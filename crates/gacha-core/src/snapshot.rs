use crate::compile::CompiledModel;
use crate::numeric::Prob;
use crate::state::StateCodec;
use num_bigint::{BigInt, Sign};
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use sysinfo::System;

const MAGIC: &[u8; 4] = b"GCHS";
const VERSION: u16 = 1;
const WARNING_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotPolicy {
    Aggregate,
    Checkpoint,
    Full,
}

#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    pub output_dir: PathBuf,
    pub policy: SnapshotPolicy,
    pub pinned_layers: BTreeSet<u32>,
    pub confirm_full: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotManifest {
    pub policy: String,
    pub estimated_bytes: u64,
    pub warning: Option<String>,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotHeader {
    pub version: u16,
    pub numeric_backend: u8,
    pub model_hash: [u8; 32],
    pub n_trials: u32,
    pub layer_index: u32,
    pub state_dims: Vec<(String, u32)>,
    pub cell_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSnapshot {
    pub header: SnapshotHeader,
    pub denominator: Option<BigInt>,
    pub cells: Vec<(u64, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("full snapshot policy requires explicit confirmation")]
    FullConfirmationRequired,
    #[error("estimated snapshot size {estimated} exceeds 50% of available memory {available}; use aggregate policy")]
    Capacity { estimated: u64, available: u64 },
    #[error("snapshot model hash does not match the compiled model")]
    ModelHashMismatch,
    #[error("invalid GCHS snapshot: {0}")]
    Invalid(String),
    #[error("snapshot I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub(crate) struct SnapshotSession {
    options: SnapshotOptions,
    manifest: SnapshotManifest,
    error: Option<SnapshotError>,
}

impl SnapshotSession {
    pub(crate) fn new(
        model: &CompiledModel,
        options: SnapshotOptions,
    ) -> Result<Self, SnapshotError> {
        if options.policy == SnapshotPolicy::Full && !options.confirm_full {
            return Err(SnapshotError::FullConfirmationRequired);
        }
        let estimated_bytes = estimate_bytes(model, options.policy, &options.pinned_layers);
        let mut system = System::new();
        system.refresh_memory();
        let available = system.available_memory();
        if available > 0 && estimated_bytes > available / 2 {
            return Err(SnapshotError::Capacity {
                estimated: estimated_bytes,
                available,
            });
        }
        let warning = (estimated_bytes > WARNING_BYTES).then(|| format!(
            "estimated snapshot size is {} MB; aggregate policy is the recommended smaller alternative",
            estimated_bytes / 1024 / 1024,
        ));
        fs::create_dir_all(&options.output_dir)?;
        Ok(Self {
            manifest: SnapshotManifest {
                policy: policy_name(options.policy).into(),
                estimated_bytes,
                warning,
                files: Vec::new(),
            },
            options,
            error: None,
        })
    }

    pub(crate) fn on_approx_layer<P: Prob>(
        &mut self,
        model: &CompiledModel,
        codec: &StateCodec,
        layer_index: u32,
        cells: &FxHashMap<u64, P>,
    ) {
        if self.error.is_some()
            || !stores_layer(
                self.options.policy,
                layer_index,
                &self.options.pinned_layers,
            )
        {
            return;
        }
        let values = if self.options.policy == SnapshotPolicy::Aggregate {
            let maxima: Vec<_> = model
                .accumulator_max
                .iter()
                .chain(&model.state_count_max)
                .copied()
                .collect();
            aggregate_approx(codec, &maxima, cells)
        } else {
            cells
                .iter()
                .map(|(state, value)| (*state, value.to_decimal_string(17)))
                .collect()
        };
        if let Err(error) = self.write(model, layer_index, values, None) {
            self.error = Some(error);
        }
    }

    pub(crate) fn on_exact_layer(
        &mut self,
        model: &CompiledModel,
        codec: &StateCodec,
        layer_index: u32,
        cells: &FxHashMap<u64, BigInt>,
        denominator: &BigInt,
    ) {
        if self.error.is_some()
            || !stores_layer(
                self.options.policy,
                layer_index,
                &self.options.pinned_layers,
            )
        {
            return;
        }
        let values = if self.options.policy == SnapshotPolicy::Aggregate {
            let maxima: Vec<_> = model
                .accumulator_max
                .iter()
                .chain(&model.state_count_max)
                .copied()
                .collect();
            aggregate_exact(codec, &maxima, cells)
        } else {
            cells
                .iter()
                .map(|(state, value)| (*state, value.to_string()))
                .collect()
        };
        if let Err(error) = self.write(model, layer_index, values, Some(denominator)) {
            self.error = Some(error);
        }
    }

    fn write(
        &mut self,
        model: &CompiledModel,
        layer_index: u32,
        mut cells: Vec<(u64, String)>,
        denominator: Option<&BigInt>,
    ) -> Result<(), SnapshotError> {
        cells.sort_by_key(|cell| cell.0);
        let state_dims = state_dims(model, self.options.policy);
        let header = SnapshotHeader {
            version: VERSION,
            numeric_backend: backend_code(model),
            model_hash: model.model_hash,
            n_trials: model.max_trials,
            layer_index,
            state_dims,
            cell_count: cells.len() as u64,
        };
        let bytes = encode_snapshot(&header, denominator, &cells)?;
        let path = self
            .options
            .output_dir
            .join(format!("layer-{layer_index:06}.gchs"));
        fs::write(&path, bytes)?;
        self.manifest.files.push(path);
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<SnapshotManifest, SnapshotError> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        Ok(self.manifest)
    }
}

pub fn load_snapshot(
    path: impl AsRef<Path>,
    expected_model_hash: [u8; 32],
) -> Result<LoadedSnapshot, SnapshotError> {
    let bytes = fs::read(path)?;
    decode_snapshot(&bytes, expected_model_hash)
}

pub fn nearest_checkpoint(target: u32, available: &[u32]) -> Option<u32> {
    available
        .iter()
        .copied()
        .filter(|layer| *layer <= target)
        .max()
}

fn stores_layer(policy: SnapshotPolicy, layer: u32, pinned: &BTreeSet<u32>) -> bool {
    match policy {
        SnapshotPolicy::Aggregate | SnapshotPolicy::Full => true,
        SnapshotPolicy::Checkpoint => {
            layer == 0 || pinned.contains(&layer) || is_log_checkpoint(layer)
        }
    }
}

fn is_log_checkpoint(layer: u32) -> bool {
    if layer == 0 {
        return true;
    }
    let mut normalized = layer;
    while normalized.is_multiple_of(10) {
        normalized /= 10;
    }
    matches!(normalized, 1 | 2 | 5)
}

fn estimate_bytes(model: &CompiledModel, policy: SnapshotPolicy, pinned: &BTreeSet<u32>) -> u64 {
    let layers = match policy {
        SnapshotPolicy::Aggregate | SnapshotPolicy::Full => u64::from(model.max_trials) + 1,
        SnapshotPolicy::Checkpoint => (0..=model.max_trials)
            .filter(|layer| stores_layer(policy, *layer, pinned))
            .count() as u64,
    };
    let per_layer = match policy {
        SnapshotPolicy::Aggregate => (model.state_count_max.len() as u64)
            .saturating_mul(2_560)
            .max(128),
        SnapshotPolicy::Checkpoint | SnapshotPolicy::Full => model
            .analysis
            .est_bytes_per_layer
            .clamp(128, 4 * 1024 * 1024),
    };
    layers.saturating_mul(per_layer)
}

fn policy_name(policy: SnapshotPolicy) -> &'static str {
    match policy {
        SnapshotPolicy::Aggregate => "aggregate",
        SnapshotPolicy::Checkpoint => "checkpoint",
        SnapshotPolicy::Full => "full",
    }
}

fn backend_code(model: &CompiledModel) -> u8 {
    match model.numeric {
        crate::ir::NumericBackend::F64 => 0,
        crate::ir::NumericBackend::Scaled => 1,
        crate::ir::NumericBackend::Exact => 2,
    }
}

fn state_dims(model: &CompiledModel, policy: SnapshotPolicy) -> Vec<(String, u32)> {
    if policy == SnapshotPolicy::Aggregate {
        return model
            .accumulator_ids
            .iter()
            .cloned()
            .zip(model.accumulator_max.iter().copied())
            .chain(
                model
                    .state_leaves
                    .iter()
                    .zip(&model.state_count_max)
                    .map(|(leaf, maximum)| (model.leaves[*leaf].id.clone(), *maximum)),
            )
            .collect();
    }
    model
        .control_ids
        .iter()
        .cloned()
        .zip(model.control_max.iter().copied())
        .chain(
            model
                .accumulator_ids
                .iter()
                .cloned()
                .zip(model.accumulator_max.iter().copied()),
        )
        .chain(
            model
                .state_leaves
                .iter()
                .zip(&model.state_count_max)
                .map(|(leaf, maximum)| (model.leaves[*leaf].id.clone(), *maximum)),
        )
        .collect()
}

fn aggregate_approx<P: Prob>(
    codec: &StateCodec,
    maxima: &[u32],
    cells: &FxHashMap<u64, P>,
) -> Vec<(u64, String)> {
    let mut margins: Vec<FxHashMap<u32, P>> = maxima.iter().map(|_| FxHashMap::default()).collect();
    for (state, probability) in cells {
        let (_, accumulators, counts) = codec.decode_full(*state);
        for (dimension, count) in accumulators.into_iter().chain(counts).enumerate() {
            margins[dimension]
                .entry(count)
                .or_insert_with(P::zero)
                .add_assign(probability);
        }
    }
    flatten_margins(
        maxima,
        margins
            .into_iter()
            .map(|margin| {
                margin
                    .into_iter()
                    .map(|(count, value)| (count, value.to_decimal_string(17)))
                    .collect()
            })
            .collect(),
    )
}

fn aggregate_exact(
    codec: &StateCodec,
    maxima: &[u32],
    cells: &FxHashMap<u64, BigInt>,
) -> Vec<(u64, String)> {
    let mut margins: Vec<FxHashMap<u32, BigInt>> =
        maxima.iter().map(|_| FxHashMap::default()).collect();
    for (state, numerator) in cells {
        let (_, accumulators, counts) = codec.decode_full(*state);
        for (dimension, count) in accumulators.into_iter().chain(counts).enumerate() {
            *margins[dimension].entry(count).or_default() += numerator;
        }
    }
    flatten_margins(
        maxima,
        margins
            .into_iter()
            .map(|margin| {
                margin
                    .into_iter()
                    .map(|(count, value)| (count, value.to_string()))
                    .collect()
            })
            .collect(),
    )
}

fn flatten_margins(maxima: &[u32], margins: Vec<Vec<(u32, String)>>) -> Vec<(u64, String)> {
    let mut offset = 0u64;
    let mut cells = Vec::new();
    for (maximum, margin) in maxima.iter().zip(margins) {
        cells.extend(
            margin
                .into_iter()
                .map(|(count, value)| (offset + u64::from(count), value)),
        );
        offset += u64::from(*maximum) + 1;
    }
    cells
}

fn encode_snapshot(
    header: &SnapshotHeader,
    denominator: Option<&BigInt>,
    cells: &[(u64, String)],
) -> Result<Vec<u8>, SnapshotError> {
    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&header.version.to_le_bytes());
    output.push(header.numeric_backend);
    output.extend_from_slice(&header.model_hash);
    output.extend_from_slice(&header.n_trials.to_le_bytes());
    output.extend_from_slice(&header.layer_index.to_le_bytes());
    write_varint(header.state_dims.len() as u64, &mut output);
    for (id, maximum) in &header.state_dims {
        write_varint(id.len() as u64, &mut output);
        output.extend_from_slice(id.as_bytes());
        output.extend_from_slice(&maximum.to_le_bytes());
    }
    output.extend_from_slice(&header.cell_count.to_le_bytes());
    let mut body = Vec::new();
    match denominator {
        Some(value) => {
            let (sign, bytes) = value.to_bytes_be();
            body.push(match sign {
                Sign::Minus => 1,
                Sign::NoSign => 0,
                Sign::Plus => 2,
            });
            write_varint(bytes.len() as u64, &mut body);
            body.extend_from_slice(&bytes);
        }
        None => body.push(0xff),
    }
    let mut previous = 0u64;
    for (state, value) in cells {
        write_varint(state - previous, &mut body);
        previous = *state;
        write_varint(value.len() as u64, &mut body);
        body.extend_from_slice(value.as_bytes());
    }
    output.extend_from_slice(&zstd::stream::encode_all(Cursor::new(body), 3)?);
    Ok(output)
}

fn decode_snapshot(bytes: &[u8], expected_hash: [u8; 32]) -> Result<LoadedSnapshot, SnapshotError> {
    let mut cursor = Cursor::new(bytes);
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(SnapshotError::Invalid("bad magic".into()));
    }
    let version = read_u16(&mut cursor)?;
    if version != VERSION {
        return Err(SnapshotError::Invalid(format!(
            "unsupported version {version}"
        )));
    }
    let numeric_backend = read_u8(&mut cursor)?;
    let mut model_hash = [0u8; 32];
    cursor.read_exact(&mut model_hash)?;
    if model_hash != expected_hash {
        return Err(SnapshotError::ModelHashMismatch);
    }
    let n_trials = read_u32(&mut cursor)?;
    let layer_index = read_u32(&mut cursor)?;
    let dim_count = read_varint(&mut cursor)? as usize;
    let mut state_dims = Vec::with_capacity(dim_count);
    for _ in 0..dim_count {
        let length = read_varint(&mut cursor)? as usize;
        let mut id = vec![0u8; length];
        cursor.read_exact(&mut id)?;
        let id = String::from_utf8(id)
            .map_err(|_| SnapshotError::Invalid("invalid dimension ID".into()))?;
        state_dims.push((id, read_u32(&mut cursor)?));
    }
    let cell_count = read_u64(&mut cursor)?;
    let body_start = cursor.position() as usize;
    let body = zstd::stream::decode_all(Cursor::new(&bytes[body_start..]))?;
    let mut body = Cursor::new(body);
    let denominator = match read_u8(&mut body)? {
        0xff => None,
        sign_code => {
            let length = read_varint(&mut body)? as usize;
            let mut value = vec![0u8; length];
            body.read_exact(&mut value)?;
            let sign = match sign_code {
                0 => Sign::NoSign,
                1 => Sign::Minus,
                2 => Sign::Plus,
                _ => return Err(SnapshotError::Invalid("invalid denominator sign".into())),
            };
            Some(BigInt::from_bytes_be(sign, &value))
        }
    };
    let mut cells = Vec::with_capacity(cell_count as usize);
    let mut state = 0u64;
    for _ in 0..cell_count {
        state = state
            .checked_add(read_varint(&mut body)?)
            .ok_or_else(|| SnapshotError::Invalid("state delta overflow".into()))?;
        let length = read_varint(&mut body)? as usize;
        let mut value = vec![0u8; length];
        body.read_exact(&mut value)?;
        cells.push((
            state,
            String::from_utf8(value)
                .map_err(|_| SnapshotError::Invalid("invalid cell value".into()))?,
        ));
    }
    Ok(LoadedSnapshot {
        header: SnapshotHeader {
            version,
            numeric_backend,
            model_hash,
            n_trials,
            layer_index,
            state_dims,
            cell_count,
        },
        denominator,
        cells,
    })
}

fn write_varint(mut value: u64, output: &mut Vec<u8>) {
    while value >= 0x80 {
        output.push((value as u8) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn read_varint(cursor: &mut Cursor<impl AsRef<[u8]>>) -> Result<u64, SnapshotError> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = read_u8(cursor)?;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(SnapshotError::Invalid("varint overflow".into()))
}

fn read_u8(cursor: &mut Cursor<impl AsRef<[u8]>>) -> Result<u8, SnapshotError> {
    let mut value = [0u8; 1];
    cursor.read_exact(&mut value)?;
    Ok(value[0])
}

fn read_u16(cursor: &mut Cursor<impl AsRef<[u8]>>) -> Result<u16, SnapshotError> {
    let mut value = [0u8; 2];
    cursor.read_exact(&mut value)?;
    Ok(u16::from_le_bytes(value))
}

fn read_u32(cursor: &mut Cursor<impl AsRef<[u8]>>) -> Result<u32, SnapshotError> {
    let mut value = [0u8; 4];
    cursor.read_exact(&mut value)?;
    Ok(u32::from_le_bytes(value))
}

fn read_u64(cursor: &mut Cursor<impl AsRef<[u8]>>) -> Result<u64, SnapshotError> {
    let mut value = [0u8; 8];
    cursor.read_exact(&mut value)?;
    Ok(u64::from_le_bytes(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, ir::ModelIr};
    use serde_json::json;

    fn model() -> CompiledModel {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion": 1,
            "name": "snapshot test",
            "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/2"}}],
            "stateVars": [],
            "probRules": [],
            "transitions": [],
            "triggers": [],
            "run": {"maxTrials": 3, "trackJoint": ["hit"], "numeric": "scaled"}
        }))
        .unwrap();
        compile(&ir).unwrap()
    }

    #[test]
    fn full_policy_requires_confirmation() {
        let model = model();
        let options = SnapshotOptions {
            output_dir: std::env::temp_dir().join("gacha-full-unconfirmed"),
            policy: SnapshotPolicy::Full,
            pinned_layers: BTreeSet::new(),
            confirm_full: false,
        };
        assert!(matches!(
            SnapshotSession::new(&model, options),
            Err(SnapshotError::FullConfirmationRequired),
        ));
    }

    #[test]
    fn gchs_round_trip_rejects_a_different_model_hash() {
        let model = model();
        let codec = StateCodec::new(&model.control_max, &model.state_count_max).unwrap();
        let state = codec.encode(&model.control_init, &[0]).unwrap();
        let cells = FxHashMap::from_iter([(state, crate::numeric::ScaledF64::one())]);
        let directory = std::env::temp_dir().join(format!("gacha-snapshot-{}", std::process::id()));
        let options = SnapshotOptions {
            output_dir: directory.clone(),
            policy: SnapshotPolicy::Checkpoint,
            pinned_layers: BTreeSet::new(),
            confirm_full: false,
        };
        let mut session = SnapshotSession::new(&model, options).unwrap();
        session.on_approx_layer(&model, &codec, 0, &cells);
        let manifest = session.finish().unwrap();
        let loaded = load_snapshot(&manifest.files[0], model.model_hash).unwrap();
        assert_eq!(loaded.header.layer_index, 0);
        assert_eq!(loaded.cells.len(), 1);
        assert!(matches!(
            load_snapshot(&manifest.files[0], [7; 32]),
            Err(SnapshotError::ModelHashMismatch),
        ));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn snapshot_estimates_preserve_policy_order_of_magnitude() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion": 1,
            "name": "N=1000 joint snapshot estimate",
            "entities": [{
                "id": "star3",
                "name": "star3",
                "prob": {"lit": "0.03"},
                "children": [{"id": "pickup", "name": "pickup", "prob": {"lit": "0.007"}}]
            }],
            "stateVars": [{"id": "pity", "init": 0, "max": 179, "role": "control"}],
            "probRules": [],
            "transitions": [],
            "triggers": [],
            "run": {
                "maxTrials": 1000,
                "trackJoint": ["pickup", "star3__self"],
                "numeric": "scaled"
            }
        }))
        .unwrap();
        let model = compile(&ir).unwrap();
        let pins = BTreeSet::new();
        let aggregate = estimate_bytes(&model, SnapshotPolicy::Aggregate, &pins);
        let checkpoint = estimate_bytes(&model, SnapshotPolicy::Checkpoint, &pins);
        let full = estimate_bytes(&model, SnapshotPolicy::Full, &pins);
        assert!((1 * 1024 * 1024..=20 * 1024 * 1024).contains(&aggregate));
        assert!((10 * 1024 * 1024..=200 * 1024 * 1024).contains(&checkpoint));
        assert!((1 * 1024 * 1024 * 1024..=10 * 1024 * 1024 * 1024).contains(&full));
    }

    #[test]
    fn policies_write_expected_layers_and_restore_a_pinned_layer() {
        let model = model();
        let root = std::env::temp_dir().join(format!("gacha-policies-{}", std::process::id()));
        let run = |policy, confirm_full, suffix: &str| {
            let directory = root.join(suffix);
            let (_, manifest) = crate::run_dp_with_snapshots(
                &model,
                crate::DpOptions { prune_log10: None },
                SnapshotOptions {
                    output_dir: directory,
                    policy,
                    pinned_layers: BTreeSet::new(),
                    confirm_full,
                },
                |_, _| true,
            )
            .unwrap();
            manifest
        };
        let aggregate = run(SnapshotPolicy::Aggregate, false, "aggregate");
        let checkpoint = run(SnapshotPolicy::Checkpoint, false, "checkpoint");
        let full = run(SnapshotPolicy::Full, true, "full");
        assert_eq!(aggregate.files.len(), 4);
        assert_eq!(checkpoint.files.len(), 3);
        assert_eq!(full.files.len(), 4);

        let restored = crate::restore_dp_snapshot(
            &model,
            crate::DpOptions { prune_log10: None },
            root.join("restore"),
            3,
        )
        .unwrap();
        assert_eq!(restored.header.layer_index, 3);
        assert!(!restored.cells.is_empty());

        let mut exact_model = model.clone();
        exact_model.numeric = crate::ir::NumericBackend::Exact;
        let (_, exact_manifest) = crate::run_dp_with_snapshots(
            &exact_model,
            crate::DpOptions { prune_log10: None },
            SnapshotOptions {
                output_dir: root.join("exact"),
                policy: SnapshotPolicy::Checkpoint,
                pinned_layers: BTreeSet::new(),
                confirm_full: false,
            },
            |_, _| true,
        )
        .unwrap();
        let exact =
            load_snapshot(exact_manifest.files.last().unwrap(), exact_model.model_hash).unwrap();
        assert_eq!(exact.header.numeric_backend, 2);
        assert!(exact.denominator.is_some());
        let _ = fs::remove_dir_all(root);
    }
}
