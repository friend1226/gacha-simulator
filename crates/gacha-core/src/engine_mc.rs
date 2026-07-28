use crate::compile::CompiledModel;
use crate::report::{wilson, WilsonInterval};
use num_traits::ToPrimitive;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Instant;

const STREAM_RUNS: u64 = 4_096;

#[derive(Debug, Clone)]
pub struct McOptions {
    pub runs: u64,
    pub seed: u64,
    pub confidence_z: f64,
    pub batch_size: u64,
}

impl Default for McOptions {
    fn default() -> Self {
        Self { runs: 100_000, seed: 42, confidence_z: 1.959963984540054, batch_size: 10_000 }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McCell {
    pub counts: Vec<u32>,
    pub occurrences: u64,
    pub interval: WilsonInterval,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McResult {
    pub runs: u64,
    pub seed: u64,
    pub tracked_leaf_ids: Vec<String>,
    pub joint: Vec<McCell>,
    pub first_hit: Option<Vec<u64>>,
    pub elapsed_ms: u64,
    pub clamp_events: u64,
}

#[derive(Clone)]
struct AliasTable {
    probability: Vec<f64>,
    alias: Vec<usize>,
}

impl AliasTable {
    fn new(weights: &[f64]) -> Self {
        let n = weights.len();
        let mut probability = vec![0.0; n];
        let mut alias = vec![0; n];
        let mut scaled: Vec<f64> = weights.iter().map(|p| p * n as f64).collect();
        let mut small = Vec::new();
        let mut large = Vec::new();
        for (i, &p) in scaled.iter().enumerate() {
            if p < 1.0 { small.push(i); } else { large.push(i); }
        }
        while !small.is_empty() && !large.is_empty() {
            let s = small.pop().expect("small alias bucket");
            let l = large.pop().expect("large alias bucket");
            probability[s] = scaled[s];
            alias[s] = l;
            scaled[l] = scaled[l] + scaled[s] - 1.0;
            if scaled[l] < 1.0 { small.push(l); } else { large.push(l); }
        }
        for i in large.into_iter().chain(small) {
            probability[i] = 1.0;
            alias[i] = i;
        }
        Self { probability, alias }
    }

    fn sample(&self, rng: &mut Xoshiro256PlusPlus) -> usize {
        let column = rng.gen_range(0..self.probability.len());
        if rng.gen::<f64>() < self.probability[column] { column } else { self.alias[column] }
    }
}

pub fn run_mc(
    model: &CompiledModel,
    options: McOptions,
    mut progress: impl FnMut(u64, u64) -> bool,
) -> McResult {
    let started = Instant::now();
    let tables: Vec<Vec<_>> = model.prob_table.entries.iter().map(|by_trial| {
        by_trial.iter().map(|p| AliasTable::new(&p.exact.iter()
            .map(|v| v.to_f64().unwrap_or(0.0)).collect::<Vec<_>>())).collect()
    }).collect();
    let mut histogram: BTreeMap<Vec<u32>, u64> = BTreeMap::new();
    let mut first_hit = model.condition.as_ref().map(|_| vec![0u64; model.max_trials as usize + 1]);
    let chunks_per_batch = options.batch_size.max(1).div_ceil(STREAM_RUNS);
    let stream_count = options.runs.div_ceil(STREAM_RUNS);
    let mut completed = 0u64;
    let mut stream = 0u64;
    while stream < stream_count {
        let end_stream = (stream + chunks_per_batch).min(stream_count);
        for chunk in run_streams(model, &tables, &options, stream, end_stream) {
            completed += chunk.runs;
            for (key, occurrences) in chunk.histogram {
                *histogram.entry(key).or_default() += occurrences;
            }
            if let (Some(total), Some(local)) = (&mut first_hit, chunk.first_hit) {
                for (value, addition) in total.iter_mut().zip(local) {
                    *value += addition;
                }
            }
        }
        stream = end_stream;
        if !progress(completed, options.runs) { break; }
    }
    let actual_runs: u64 = histogram.values().sum();
    let joint = histogram.into_iter().map(|(counts, occurrences)| McCell {
        counts,
        occurrences,
        interval: wilson(occurrences, actual_runs, options.confidence_z),
    }).collect();
    McResult {
        runs: actual_runs,
        seed: options.seed,
        tracked_leaf_ids: model.tracked_leaves.iter().map(|i| model.leaves[*i].id.clone()).collect(),
        joint,
        first_hit,
        elapsed_ms: started.elapsed().as_millis() as u64,
        clamp_events: model.prob_table.clamp_events,
    }
}

struct ChunkResult {
    runs: u64,
    histogram: BTreeMap<Vec<u32>, u64>,
    first_hit: Option<Vec<u64>>,
}

#[cfg(feature = "parallel")]
fn run_streams(
    model: &CompiledModel,
    tables: &[Vec<AliasTable>],
    options: &McOptions,
    start: u64,
    end: u64,
) -> Vec<ChunkResult> {
    use rayon::prelude::*;
    (start..end).into_par_iter()
        .map(|stream| run_stream(model, tables, options, stream))
        .collect()
}

#[cfg(not(feature = "parallel"))]
fn run_streams(
    model: &CompiledModel,
    tables: &[Vec<AliasTable>],
    options: &McOptions,
    start: u64,
    end: u64,
) -> Vec<ChunkResult> {
    (start..end)
        .map(|stream| run_stream(model, tables, options, stream))
        .collect()
}

fn run_stream(
    model: &CompiledModel,
    tables: &[Vec<AliasTable>],
    options: &McOptions,
    stream: u64,
) -> ChunkResult {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(options.seed);
    for _ in 0..stream { rng.jump(); }
    let start = stream * STREAM_RUNS;
    let runs = (options.runs - start).min(STREAM_RUNS);
    let mut histogram = BTreeMap::new();
    let mut first_hit = model.condition.as_ref()
        .map(|_| vec![0u64; model.max_trials as usize + 1]);
    for _ in 0..runs {
        let mut control = model.control_init.clone();
        let mut counts = vec![0u32; model.leaves.len()];
        let mut hit = false;
        let mut trial = 0u32;
        while trial < model.max_trials {
            let draw_trial = trial + 1;
            let ci = model.control_index(&control);
            let ti = if model.prob_table.trial_dependent { draw_trial as usize - 1 } else { 0 };
            let leaf = tables[ci][ti].sample(&mut rng);
            counts[leaf] += 1;
            model.apply_transitions(&mut control, leaf, draw_trial);
            if !hit && condition_matches(model, &counts, draw_trial) {
                if let Some(pmf) = &mut first_hit { pmf[draw_trial as usize] += 1; }
                hit = true;
            }
            let consumed = model.apply_triggers(
                &mut control,
                &mut counts,
                draw_trial,
                |grant_counts, grant_trial| {
                    if !hit && condition_matches(model, grant_counts, grant_trial) {
                        if let Some(pmf) = &mut first_hit { pmf[grant_trial as usize] += 1; }
                        hit = true;
                    }
                },
            );
            trial = draw_trial + consumed;
        }
        let key = model.tracked_leaves.iter().map(|i| counts[*i]).collect();
        *histogram.entry(key).or_default() += 1;
    }
    ChunkResult { runs, histogram, first_hit }
}

fn condition_matches(model: &CompiledModel, counts: &[u32], trial: u32) -> bool {
    let Some(program) = &model.condition else { return false; };
    crate::expr::eval(program, |name| {
        let entity = name.strip_prefix('n').map(lower_first).unwrap_or_else(|| name.to_owned());
        model.entity_count(counts, &entity)
            .map(|v| crate::rational::Rational::from_integer(v.into()))
    }, trial).and_then(|v| v.boolean()).unwrap_or(false)
}

fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    chars.next().map(|c| c.to_lowercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "parallel")]
    use crate::{compile, ir::ModelIr};
    #[cfg(feature = "parallel")]
    use serde_json::json;

    #[test]
    fn alias_table_keeps_columns_when_one_bucket_is_empty() {
        let equal = AliasTable::new(&[0.5, 0.5]);
        assert_eq!(equal.probability, vec![1.0, 1.0]);
        assert_eq!(equal.alias, vec![0, 1]);

        let skewed = AliasTable::new(&[0.2, 0.8]);
        assert_eq!(skewed.probability, vec![0.4, 1.0]);
        assert_eq!(skewed.alias, vec![1, 1]);
    }

    #[test]
    fn alias_table_keeps_the_final_large_bucket() {
        let table = AliasTable::new(&[0.25, 0.25, 0.5]);

        assert_eq!(table.probability[2], 1.0);
        assert_eq!(table.alias[2], 2);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_mc_is_reproducible_across_thread_counts() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion": 1,
            "name": "parallel deterministic MC",
            "entities": [{"id": "hit", "name": "hit", "prob": {"lit": "1/3"}}],
            "stateVars": [],
            "probRules": [],
            "transitions": [],
            "triggers": [],
            "run": {
                "maxTrials": 8,
                "trackJoint": ["hit"],
                "numeric": "scaled",
                "condition": {"ge": [{"var": "nHit"}, {"lit": "2"}]}
            }
        })).unwrap();
        let model = compile(&ir).unwrap();
        let options = McOptions { runs: 32_777, seed: 0x5eed, batch_size: 10_000, ..Default::default() };
        let run = |threads| {
            rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap()
                .install(|| run_mc(&model, options.clone(), |_, _| true))
        };
        let one = run(1);
        let four = run(4);
        let cells = |result: &McResult| result.joint.iter()
            .map(|cell| (cell.counts.clone(), cell.occurrences))
            .collect::<Vec<_>>();

        assert_eq!(one.runs, options.runs);
        assert_eq!(one.seed, options.seed);
        assert_eq!(cells(&one), cells(&four));
        assert_eq!(one.first_hit, four.first_hit);
    }
}
