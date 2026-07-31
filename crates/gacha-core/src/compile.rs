use crate::expr::{compile_expr, eval, Op, Program};
use crate::ir::{
    ClampPolicy, Entity, Grant, LeafPredicate, ModelIr, NestingPolicy, NumericBackend, StateRole,
    Transition, TrialSeriesMode, Trigger,
};
use crate::rational::{parse_literal, Rational};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub block_id: Option<String>,
}

#[derive(Debug, Error)]
#[error("model compilation failed")]
pub struct CompileError {
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leaf {
    pub id: String,
    pub name: String,
    pub ancestors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LeafProbs {
    pub exact: Vec<Rational>,
}

#[derive(Debug, Clone)]
pub struct ProbTable {
    pub entries: Vec<Vec<LeafProbs>>,
    pub trial_dependent: bool,
    pub entry_control_invariant: bool,
    pub control_entry_indices: BTreeMap<usize, usize>,
    pub control_invariant: bool,
    pub clamp_events: u64,
}

#[derive(Debug, Clone)]
pub struct TransitionTable {
    pub entries: Vec<Vec<Vec<usize>>>,
    pub control_entry_indices: BTreeMap<usize, usize>,
    pub trial_dependent: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct AccumulatorTransition {
    pub value: u32,
    pub clamped: bool,
}

#[derive(Debug, Clone)]
pub struct AccumulatorTable {
    // accumulator -> control state -> trial -> leaf -> current value
    pub entries: Vec<Vec<Vec<Vec<Vec<AccumulatorTransition>>>>>,
    pub trial_dependent: Vec<bool>,
    pub control_dependent: Vec<bool>,
}

const ACCUMULATOR_TABLE_WARNING_ENTRIES: u128 = 500_000;
const ACCUMULATOR_TABLE_MAX_ENTRIES: u128 = 10_000_000;
const PROBABILITY_TABLE_WARNING_ENTRIES: u128 = 500_000;
const PROBABILITY_TABLE_MAX_ENTRIES: u128 = 10_000_000;
pub const DP_CONTROL_STATE_LIMIT: u64 = 10_000_000;
pub const DP_ESTIMATED_STATE_LIMIT: u64 = 50_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackedDimension {
    Leaf(usize),
    Accumulator(usize),
    DerivedAccumulator(usize),
}

#[derive(Debug, Clone)]
pub struct CompiledTransition {
    pub predicate: CompiledPredicate,
    pub assignments: Vec<(usize, Program)>,
}

#[derive(Debug, Clone)]
pub enum CompiledPredicate {
    LeafSet(BTreeSet<usize>),
    Not(Box<CompiledPredicate>),
    And(Vec<CompiledPredicate>),
    Or(Vec<CompiledPredicate>),
}

impl CompiledPredicate {
    pub fn matches(&self, leaf: usize) -> bool {
        match self {
            Self::LeafSet(set) => set.contains(&leaf),
            Self::Not(inner) => !inner.matches(leaf),
            Self::And(items) => items.iter().all(|p| p.matches(leaf)),
            Self::Or(items) => items.iter().any(|p| p.matches(leaf)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledTrigger {
    pub trial_count: u32,
    pub grant: Option<CompiledGrant>,
    pub assignments: Vec<(usize, Program)>,
}

#[derive(Debug, Clone)]
pub struct CompiledGrant {
    pub leaf: usize,
    pub amount: u32,
    pub consumes_trial: bool,
    pub applies_transitions: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkovAnalysis {
    pub dp_available: bool,
    pub blockers: Vec<String>,
    pub control_states: u64,
    pub stat_states: u64,
    pub total_states: u64,
    pub est_bytes_per_layer: u64,
    pub exact_available: bool,
}

#[derive(Debug, Clone)]
pub struct CompiledModel {
    pub name: String,
    pub max_trials: u32,
    pub numeric: NumericBackend,
    pub leaves: Vec<Leaf>,
    pub control_ids: Vec<String>,
    pub control_init: Vec<u32>,
    pub control_max: Vec<u32>,
    pub accumulator_ids: Vec<String>,
    pub accumulator_names: Vec<String>,
    pub accumulator_init: Vec<u32>,
    pub accumulator_max: Vec<u32>,
    pub derived_accumulator_ids: Vec<String>,
    pub derived_accumulator_leaves: Vec<Vec<usize>>,
    pub tracked_leaves: Vec<usize>,
    pub tracked_dimensions: Vec<TrackedDimension>,
    pub tracked_ids: Vec<String>,
    pub state_leaves: Vec<usize>,
    pub state_leaf_positions: Vec<Option<usize>>,
    pub state_count_max: Vec<u32>,
    pub prob_table: ProbTable,
    pub transition_table: TransitionTable,
    pub accumulator_table: AccumulatorTable,
    pub transitions: Vec<CompiledTransition>,
    pub triggers: Vec<CompiledTrigger>,
    pub condition: Option<Program>,
    pub trial_series: TrialSeriesMode,
    pub series_checkpoints: BTreeSet<u32>,
    pub diagnostics: Vec<Diagnostic>,
    pub analysis: MarkovAnalysis,
    pub exact_lcm: BigInt,
    pub model_hash: [u8; 32],
}

impl CompiledModel {
    pub fn model_hash_hex(&self) -> String {
        self.model_hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn control_index(&self, state: &[u32]) -> usize {
        let mut index = 0usize;
        let mut stride = 1usize;
        for (value, max) in state.iter().zip(&self.control_max) {
            index += *value as usize * stride;
            stride *= *max as usize + 1;
        }
        index
    }

    pub fn probabilities(&self, control: &[u32], trial: u32) -> &LeafProbs {
        let ci = self.control_index(control);
        let ci = self.probability_table_index(ci);
        let ti = if self.prob_table.trial_dependent {
            trial.saturating_sub(1) as usize
        } else {
            0
        };
        &self.prob_table.entries[ci][ti]
    }

    pub fn probability_table_index(&self, control_index: usize) -> usize {
        if self.prob_table.entry_control_invariant {
            0
        } else if self.prob_table.control_entry_indices.is_empty() {
            control_index
        } else {
            *self
                .prob_table
                .control_entry_indices
                .get(&control_index)
                .expect("runtime control state must be present in the probability table")
        }
    }

    pub fn apply_transitions(&self, control: &mut [u32], leaf: usize, trial: u32) {
        let before = control.to_vec();
        apply_compiled_transitions(
            &self.transitions,
            &self.control_ids,
            &self.control_max,
            control,
            &before,
            leaf,
            trial,
        );
    }

    pub fn transition_control_index(&self, control_index: usize, leaf: usize, trial: u32) -> usize {
        if self.transition_table.entries.is_empty() {
            return control_index;
        }
        let ti = if self.transition_table.trial_dependent {
            trial.saturating_sub(1) as usize
        } else {
            0
        };
        let entry_index = if self.transition_table.control_entry_indices.is_empty() {
            control_index
        } else {
            *self
                .transition_table
                .control_entry_indices
                .get(&control_index)
                .expect("runtime control state must be present in the transition table")
        };
        self.transition_table.entries[entry_index][ti][leaf]
    }

    pub fn accumulator_transition(
        &self,
        accumulator: usize,
        control_index: usize,
        current: u32,
        leaf: usize,
        trial: u32,
    ) -> AccumulatorTransition {
        let ci = if self.accumulator_table.control_dependent[accumulator] {
            control_index
        } else {
            0
        };
        let ti = if self.accumulator_table.trial_dependent[accumulator] {
            trial.saturating_sub(1) as usize
        } else {
            0
        };
        self.accumulator_table.entries[accumulator][ci][ti][leaf][current as usize]
    }

    pub fn apply_accumulators(
        &self,
        control: &[u32],
        accumulators: &mut [u32],
        leaf: usize,
        trial: u32,
    ) -> u64 {
        let control_index = self.control_index(control);
        let before = accumulators.to_vec();
        let mut clamp_events = 0;
        for (index, value) in accumulators.iter_mut().enumerate() {
            let transition =
                self.accumulator_transition(index, control_index, before[index], leaf, trial);
            *value = transition.value;
            clamp_events += u64::from(transition.clamped);
        }
        clamp_events
    }

    pub fn apply_transitions_buffered(
        &self,
        control: &mut [u32],
        before: &mut [u32],
        leaf: usize,
        trial: u32,
    ) {
        before.copy_from_slice(control);
        apply_compiled_transitions(
            &self.transitions,
            &self.control_ids,
            &self.control_max,
            control,
            before,
            leaf,
            trial,
        );
    }

    pub fn packed_transition_fast_path(&self) -> bool {
        self.triggers.is_empty() && self.condition.is_none()
    }

    pub fn state_leaf_position(&self, leaf: usize) -> Option<usize> {
        self.state_leaf_positions[leaf]
    }

    pub fn consumed_trials_after(&self, trial: u32) -> u32 {
        let available = self.max_trials.saturating_sub(trial);
        let requested = self
            .triggers
            .iter()
            .filter(|trigger| trigger.trial_count == trial)
            .filter(|trigger| {
                trigger
                    .grant
                    .as_ref()
                    .is_some_and(|grant| grant.consumes_trial)
            })
            .count() as u32;
        requested.min(available)
    }

    pub fn apply_triggers(
        &self,
        control: &mut [u32],
        accumulators: &mut [u32],
        counts: &mut [u32],
        trial: u32,
        mut after_grant: impl FnMut(&[u32], u32),
    ) -> (u32, u64) {
        let max_consumed = self.consumed_trials_after(trial);
        let mut consumed = 0u32;
        let mut accumulator_clamps = 0u64;
        for trigger in self.triggers.iter().filter(|t| t.trial_count == trial) {
            let action_trial = trial + consumed;
            let before = control.to_vec();
            for (index, program) in &trigger.assignments {
                // Assignment references are compile-time validated; keep this fallible
                // evaluation as a defensive runtime guard.
                if let Ok(value) = eval(
                    program,
                    |name| {
                        self.control_ids
                            .iter()
                            .position(|id| id == name)
                            .map(|i| Rational::from_integer(before[i].into()))
                    },
                    action_trial,
                )
                .and_then(|v| v.number())
                {
                    control[*index] = value
                        .to_integer()
                        .to_u32()
                        .unwrap_or(u32::MAX)
                        .min(self.control_max[*index]);
                }
            }
            if let Some(grant) = &trigger.grant {
                if grant.consumes_trial {
                    if consumed >= max_consumed {
                        continue;
                    }
                    consumed += 1;
                }
                let grant_trial = trial + consumed;
                counts[grant.leaf] = counts[grant.leaf].saturating_add(grant.amount);
                if grant.applies_transitions {
                    for _ in 0..grant.amount {
                        self.apply_transitions(control, grant.leaf, grant_trial);
                        accumulator_clamps +=
                            self.apply_accumulators(control, accumulators, grant.leaf, grant_trial);
                    }
                }
                after_grant(counts, grant_trial);
            }
        }
        (consumed, accumulator_clamps)
    }

    pub fn entity_count(&self, counts: &[u32], entity: &str) -> Option<u32> {
        let mut found = false;
        let total = self
            .leaves
            .iter()
            .enumerate()
            .filter_map(|(i, leaf)| {
                let matches = leaf.id == entity || leaf.ancestors.iter().any(|a| a == entity);
                found |= matches;
                matches.then_some(counts[i])
            })
            .sum();
        found.then_some(total)
    }

    pub fn entity_count_sparse(&self, counts: &[u32], entity: &str) -> Option<u32> {
        let mut found = false;
        let total = self
            .state_leaves
            .iter()
            .zip(counts)
            .filter_map(|(leaf_index, count)| {
                let leaf = &self.leaves[*leaf_index];
                let matches = leaf.id == entity || leaf.ancestors.iter().any(|a| a == entity);
                found |= matches;
                matches.then_some(*count)
            })
            .sum();
        found.then_some(total)
    }

    pub fn condition_matches_sparse(&self, counts: &[u32], trial: u32) -> bool {
        let Some(program) = &self.condition else {
            return false;
        };
        eval(
            program,
            |name| {
                let entity = name
                    .strip_prefix('n')
                    .map(lower_first)
                    .unwrap_or_else(|| name.to_owned());
                self.entity_count_sparse(counts, &entity)
                    .map(|value| Rational::from_integer(value.into()))
            },
            trial,
        )
        .and_then(|value| value.boolean())
        .unwrap_or(false)
    }

    pub fn apply_triggers_sparse(
        &self,
        control: &mut [u32],
        accumulators: &mut [u32],
        counts: &mut [u32],
        trial: u32,
        mut after_grant: impl FnMut(&[u32], u32),
    ) -> (u32, u64) {
        let max_consumed = self.consumed_trials_after(trial);
        let mut consumed = 0u32;
        let mut accumulator_clamps = 0u64;
        for trigger in self.triggers.iter().filter(|t| t.trial_count == trial) {
            let action_trial = trial + consumed;
            let before = control.to_vec();
            for (index, program) in &trigger.assignments {
                // Assignment references are compile-time validated; keep this fallible
                // evaluation as a defensive runtime guard.
                if let Ok(value) = eval(
                    program,
                    |name| {
                        self.control_ids
                            .iter()
                            .position(|id| id == name)
                            .map(|i| Rational::from_integer(before[i].into()))
                    },
                    action_trial,
                )
                .and_then(|v| v.number())
                {
                    control[*index] = value
                        .to_integer()
                        .to_u32()
                        .unwrap_or(u32::MAX)
                        .min(self.control_max[*index]);
                }
            }
            if let Some(grant) = &trigger.grant {
                if grant.consumes_trial {
                    if consumed >= max_consumed {
                        continue;
                    }
                    consumed += 1;
                }
                let grant_trial = trial + consumed;
                if let Some(position) = self
                    .state_leaves
                    .iter()
                    .position(|leaf| *leaf == grant.leaf)
                {
                    counts[position] = counts[position].saturating_add(grant.amount);
                }
                if grant.applies_transitions {
                    for _ in 0..grant.amount {
                        self.apply_transitions(control, grant.leaf, grant_trial);
                        accumulator_clamps +=
                            self.apply_accumulators(control, accumulators, grant.leaf, grant_trial);
                    }
                }
                after_grant(counts, grant_trial);
            }
        }
        (consumed, accumulator_clamps)
    }
}

fn apply_compiled_transitions(
    transitions: &[CompiledTransition],
    control_ids: &[String],
    control_max: &[u32],
    control: &mut [u32],
    before: &[u32],
    leaf: usize,
    trial: u32,
) {
    for transition in transitions {
        if !transition.predicate.matches(leaf) {
            continue;
        }
        for (index, program) in &transition.assignments {
            // Assignment references are compile-time validated; keep this fallible
            // evaluation as a defensive runtime guard.
            if let Ok(value) = eval(
                program,
                |name| {
                    control_ids
                        .iter()
                        .position(|id| id == name)
                        .map(|i| Rational::from_integer(before[i].into()))
                },
                trial,
            )
            .and_then(|v| v.number())
            {
                let next = value
                    .to_integer()
                    .to_i64()
                    .unwrap_or(i64::MAX)
                    .clamp(0, control_max[*index] as i64);
                control[*index] = next as u32;
            }
        }
    }
}

struct EntityDef {
    id: String,
    program: Program,
    children: Vec<EntityDef>,
    leaf_indices: BTreeSet<usize>,
    self_leaf: Option<usize>,
}

pub fn compile(ir: &ModelIr) -> Result<CompiledModel, CompileError> {
    let mut diagnostics = Vec::new();
    let trial_series = ir.run.trial_series.unwrap_or(if ir.ir_version >= 2 {
        TrialSeriesMode::Marginal
    } else {
        TrialSeriesMode::None
    });
    if !(1..=2).contains(&ir.ir_version) {
        diagnostics.push(error(
            "E000",
            format!("unsupported irVersion {}", ir.ir_version),
            None,
        ));
    }

    let mut ids = BTreeSet::new();
    validate_entity_ids(&ir.entities, &mut ids, &mut diagnostics);
    let mut control_ids = Vec::new();
    let mut control_init = Vec::new();
    let mut control_max = Vec::new();
    let mut accumulator_ids = Vec::new();
    let mut accumulator_names = Vec::new();
    let mut accumulator_init = Vec::new();
    let mut accumulator_max = Vec::new();
    for var in &ir.state_vars {
        match (var.role, var.max) {
            (StateRole::Stat, _) => diagnostics.push(error(
                "E009",
                format!(
                    "stat variable '{}' must not be declared; leaf counters are automatic",
                    var.id
                ),
                var.block_id.clone(),
            )),
            (role, None) => diagnostics.push(error(
                "E004",
                format!(
                    "{} variable '{}' needs max",
                    if role == StateRole::Control {
                        "control"
                    } else {
                        "accumulator"
                    },
                    var.id
                ),
                var.block_id.clone(),
            )),
            (_, Some(max)) if var.init < 0 || var.init as u64 > max as u64 => {
                diagnostics.push(error(
                    "E004",
                    format!("initial value for '{}' is outside 0..={max}", var.id),
                    var.block_id.clone(),
                ))
            }
            (StateRole::Control, Some(max)) => {
                control_ids.push(var.id.clone());
                control_init.push(var.init as u32);
                control_max.push(max);
            }
            (StateRole::Accumulator, Some(max)) => {
                accumulator_ids.push(var.id.clone());
                accumulator_names.push(var.name.clone().unwrap_or_else(|| var.id.clone()));
                accumulator_init.push(var.init as u32);
                accumulator_max.push(max);
            }
        }
    }
    let mut leaves = Vec::new();
    collect_leaves(&ir.entities, &mut Vec::new(), &mut leaves);
    leaves.push(Leaf {
        id: "__other__".into(),
        name: "그외".into(),
        ancestors: Vec::new(),
    });
    let mut derived_accumulator_ids = Vec::new();
    let mut derived_accumulator_leaves = Vec::new();
    for index in (0..accumulator_ids.len()).rev() {
        let var = ir
            .state_vars
            .iter()
            .find(|var| var.role == StateRole::Accumulator && var.id == accumulator_ids[index])
            .expect("compiled accumulator comes from stateVars");
        let Some(target) = simple_counter_target(var) else {
            continue;
        };
        let matching: Vec<_> = leaves
            .iter()
            .enumerate()
            .filter_map(|(leaf, definition)| {
                (definition.id == target
                    || definition
                        .ancestors
                        .iter()
                        .any(|ancestor| ancestor == target))
                .then_some(leaf)
            })
            .collect();
        if matching.is_empty() {
            continue;
        }
        let possible_count_max = ir
            .triggers
            .iter()
            .filter_map(|trigger| trigger.grant.as_ref())
            .filter(|grant| matching.iter().any(|leaf| leaves[*leaf].id == grant.leaf))
            .fold(ir.run.max_trials, |maximum, grant| {
                maximum.saturating_add(grant.amount)
            });
        if accumulator_max[index] < possible_count_max {
            continue;
        }
        derived_accumulator_ids.push(accumulator_ids.remove(index));
        derived_accumulator_leaves.push(matching);
        accumulator_names.remove(index);
        accumulator_init.remove(index);
        accumulator_max.remove(index);
        diagnostics.push(warning(
            "W008",
            format!(
                "accumulator '{}' duplicates an automatic leaf counter and was derived without a state axis",
                var.id
            ),
            var.block_id.clone(),
        ));
    }
    derived_accumulator_ids.reverse();
    derived_accumulator_leaves.reverse();

    let rule_map: HashMap<_, _> = ir
        .prob_rules
        .iter()
        .map(|r| (r.target.as_str(), &r.expr))
        .collect();
    let mut exact_available = true;
    let entity_defs = match compile_entities(
        &ir.entities,
        &rule_map,
        &leaves,
        &mut exact_available,
        &mut diagnostics,
    ) {
        Some(v) => v,
        None => Vec::new(),
    };
    for rule in &ir.prob_rules {
        if !ids.contains(&rule.target) {
            diagnostics.push(error(
                "E005",
                format!("probability rule targets unknown entity '{}'", rule.target),
                rule.block_id.clone(),
            ));
        }
    }
    validate_probability_ranges(
        &ir.entities,
        &ir.prob_rules,
        &control_ids,
        &control_max,
        ir.run.max_trials,
        &mut diagnostics,
    );

    let control_states = control_max
        .iter()
        .fold(1u64, |n, max| n.saturating_mul(*max as u64 + 1));
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }

    let leaf_lookup: HashMap<_, _> = leaves
        .iter()
        .enumerate()
        .map(|(i, l)| (l.id.as_str(), i))
        .collect();
    let transitions = compile_transitions(&ir.transitions, &leaves, &control_ids, &mut diagnostics);
    let triggers = compile_triggers(
        &ir.triggers,
        &leaves,
        &leaf_lookup,
        &control_ids,
        &mut diagnostics,
    );
    let transition_trial_dependent = transitions
        .iter()
        .flat_map(|transition| &transition.assignments)
        .any(|(_, program)| program.trial_dependent);
    let transition_trials = if transition_trial_dependent {
        ir.run.max_trials.max(1)
    } else {
        1
    };
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }

    let trial_dependent = entity_defs.iter().any(entity_trial_dependent);
    let probability_control_dependent = entity_defs_control_dependent(&entity_defs, &control_ids);
    let table_trials = if trial_dependent {
        ir.run.max_trials.max(1)
    } else {
        1
    };
    let initial_control_index = encode_control(&control_init, &control_max);
    let probability_control_indices = if probability_control_dependent {
        let entries_per_control = u128::from(table_trials).saturating_mul(leaves.len() as u128);
        let reachable_limit =
            (PROBABILITY_TABLE_MAX_ENTRIES / entries_per_control).min(usize::MAX as u128) as usize;
        match reachable_control_indices(
            &transitions,
            &control_ids,
            &control_init,
            &control_max,
            leaves.len(),
            transition_trial_dependent,
            &triggers,
            ir.run.max_trials,
            reachable_limit,
        ) {
            Ok(indices) => indices,
            Err(discovered_controls) => {
                push_probability_table_size_diagnostic(
                    discovered_controls as u64,
                    table_trials,
                    leaves.len(),
                    true,
                    &mut diagnostics,
                );
                return Err(CompileError { diagnostics });
            }
        }
    } else {
        vec![initial_control_index]
    };
    let probability_table_controls = probability_control_indices.len() as u64;
    if !push_probability_table_size_diagnostic(
        probability_table_controls,
        table_trials,
        leaves.len(),
        false,
        &mut diagnostics,
    ) {
        return Err(CompileError { diagnostics });
    }

    let mut probability_control_entry_indices = if probability_control_dependent {
        probability_control_indices
            .iter()
            .enumerate()
            .map(|(entry_index, control_index)| (*control_index, entry_index))
            .collect()
    } else {
        BTreeMap::new()
    };
    let mut entries = Vec::with_capacity(probability_control_indices.len());
    let mut clamp_events = 0u64;
    for ci in &probability_control_indices {
        let control = decode_control(*ci as u64, &control_max);
        let mut by_trial = Vec::with_capacity(table_trials as usize);
        for trial in 1..=table_trials {
            match calculate_leaf_probs(
                &entity_defs,
                leaves.len(),
                &control_ids,
                &control,
                trial,
                ir.nesting_policy,
                &mut clamp_events,
            ) {
                Ok(probs) => by_trial.push(LeafProbs { exact: probs }),
                Err(message) => diagnostics.push(error("E003", message, None)),
            }
        }
        entries.push(by_trial);
    }
    if clamp_events > 0 {
        diagnostics.push(warning(
            "W001",
            format!("child probabilities were adjusted {clamp_events} time(s)"),
            None,
        ));
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }
    diagnostics.push(info(
        "W002",
        "the implicit __other__ leaf completes total probability to 1",
        None,
    ));
    let probability_control_invariant = entries.first().is_none_or(|first| {
        entries.iter().skip(1).all(|candidate| {
            candidate.len() == first.len()
                && candidate
                    .iter()
                    .zip(first)
                    .all(|(left, right)| left.exact == right.exact)
        })
    });
    if probability_control_invariant {
        entries.truncate(1);
        probability_control_entry_indices.clear();
    }

    let accumulator_table = compile_accumulator_table(
        ir,
        &leaves,
        &control_ids,
        &control_max,
        &accumulator_ids,
        &accumulator_max,
        ir.run.max_trials,
        &mut diagnostics,
    );
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }
    let control_invariant = probability_control_invariant
        && !accumulator_table
            .control_dependent
            .iter()
            .any(|dependent| *dependent);
    let mut transition_entries = Vec::new();
    let mut transition_control_entry_indices = BTreeMap::new();
    if !transitions.is_empty() && !control_invariant {
        let transition_control_indices: Vec<usize> = if probability_control_dependent {
            probability_control_indices.clone()
        } else {
            (0..control_states)
                .map(|control_index| control_index as usize)
                .collect()
        };
        if probability_control_dependent {
            transition_control_entry_indices = transition_control_indices
                .iter()
                .enumerate()
                .map(|(entry_index, control_index)| (*control_index, entry_index))
                .collect();
        }
        transition_entries.reserve(transition_control_indices.len());
        for ci in transition_control_indices {
            let control = decode_control(ci as u64, &control_max);
            let mut by_trial = Vec::with_capacity(transition_trials as usize);
            for trial in 1..=transition_trials {
                let mut by_leaf = Vec::with_capacity(leaves.len());
                for leaf in 0..leaves.len() {
                    let mut next = control.clone();
                    apply_compiled_transitions(
                        &transitions,
                        &control_ids,
                        &control_max,
                        &mut next,
                        &control,
                        leaf,
                        trial,
                    );
                    by_leaf.push(encode_control(&next, &control_max));
                }
                by_trial.push(by_leaf);
            }
            transition_entries.push(by_trial);
        }
    }
    let transition_table = TransitionTable {
        entries: transition_entries,
        control_entry_indices: transition_control_entry_indices,
        trial_dependent: transition_trial_dependent,
    };
    warn_unapplied_consuming_grants(&ir.triggers, ir.run.max_trials, &mut diagnostics);
    let (tracked_leaves, tracked_dimensions, tracked_ids) = expand_tracked(
        &ir.run.track_joint,
        &leaves,
        &accumulator_ids,
        &derived_accumulator_ids,
        &derived_accumulator_leaves,
        &mut diagnostics,
    );
    let mut state_leaf_set: BTreeSet<usize> = tracked_leaves.iter().copied().collect();
    if let Some(condition_expr) = &ir.run.condition {
        let mut variables = Vec::new();
        collect_expr_variables(condition_expr, &mut variables);
        for variable in variables {
            let entity = variable
                .strip_prefix('n')
                .map(lower_first)
                .unwrap_or(variable);
            for (index, leaf) in leaves.iter().enumerate() {
                if leaf.id == entity || leaf.ancestors.iter().any(|a| a == &entity) {
                    state_leaf_set.insert(index);
                }
            }
        }
    }
    let state_leaves: Vec<_> = state_leaf_set.into_iter().collect();
    let mut state_leaf_positions = vec![None; leaves.len()];
    for (position, leaf) in state_leaves.iter().enumerate() {
        state_leaf_positions[*leaf] = Some(position);
    }
    let state_count_max: Vec<_> = state_leaves
        .iter()
        .map(|leaf| {
            triggers
                .iter()
                .filter_map(|trigger| trigger.grant.as_ref())
                .filter(|grant| grant.leaf == *leaf)
                .fold(ir.run.max_trials, |maximum, grant| {
                    maximum.saturating_add(grant.amount)
                })
        })
        .collect();
    let condition = ir
        .run
        .condition
        .as_ref()
        .and_then(|expr| match compile_expr(expr) {
            Ok(p) => Some(p),
            Err(e) => {
                diagnostics.push(error("E006", e.to_string(), None));
                None
            }
        });
    if condition.is_some() {
        if let Some(false) = condition_could_be_true(
            ir.run
                .condition
                .as_ref()
                .expect("compiled condition source"),
            ir.run.max_trials,
            &leaves,
            &entries,
            &triggers,
        ) {
            diagnostics.push(warning(
                "W003",
                "run condition is statically unsatisfiable within maxTrials",
                None,
            ));
        }
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }

    let exact_lcm = entries
        .iter()
        .flat_map(|v| v.iter())
        .flat_map(|p| &p.exact)
        .fold(BigInt::one(), |lcm, p| lcm.lcm(p.denom()));
    let l_bits = exact_lcm.bits();
    if l_bits > 64 {
        diagnostics.push(warning(
            "W005",
            format!("exact common denominator requires {l_bits} bits"),
            None,
        ));
    }
    if l_bits > 128 {
        exact_available = false;
    }

    if trial_series == TrialSeriesMode::Checkpoints {
        if ir.run.series_checkpoints.len() > 20 {
            diagnostics.push(error(
                "E006",
                "seriesCheckpoints accepts at most 20 trial numbers",
                None,
            ));
        }
        for checkpoint in &ir.run.series_checkpoints {
            if *checkpoint == 0 || *checkpoint > ir.run.max_trials {
                diagnostics.push(error(
                    "E006",
                    format!(
                        "series checkpoint {checkpoint} is outside 1..={}",
                        ir.run.max_trials
                    ),
                    None,
                ));
            }
        }
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }

    let count_states = state_count_max.iter().fold(1u64, |states, maximum| {
        states.saturating_mul(u64::from(*maximum) + 1)
    });
    let accumulator_states = accumulator_max.iter().fold(1u64, |states, maximum| {
        states.saturating_mul(u64::from(*maximum) + 1)
    });
    let stat_states = count_states.saturating_mul(accumulator_states);
    let effective_control_states = if control_invariant {
        1
    } else if probability_control_dependent {
        probability_control_indices.len() as u64
    } else {
        control_states
    };
    let total_states = effective_control_states.saturating_mul(stat_states);
    let state_encoding_available = crate::state::StateCodec::with_accumulators(
        &control_max,
        &accumulator_max,
        &state_count_max,
    )
    .is_ok();
    let dp_available = state_encoding_available
        && effective_control_states <= DP_CONTROL_STATE_LIMIT
        && total_states <= DP_ESTIMATED_STATE_LIMIT;
    let blockers = if dp_available {
        Vec::new()
    } else if !state_encoding_available {
        vec!["mixed-radix state space exceeds u64".into()]
    } else {
        vec!["estimated state space exceeds DP limit".into()]
    };
    let analysis = MarkovAnalysis {
        dp_available,
        blockers,
        control_states: effective_control_states,
        stat_states,
        total_states,
        est_bytes_per_layer: total_states.saturating_mul(40),
        exact_available,
    };
    let model_hash: [u8; 32] =
        Sha256::digest(serde_json::to_vec(ir).expect("serializable Model IR")).into();
    Ok(CompiledModel {
        name: ir.name.clone(),
        max_trials: ir.run.max_trials,
        numeric: ir.run.numeric,
        leaves,
        control_ids,
        control_init,
        control_max,
        accumulator_ids,
        accumulator_names,
        accumulator_init,
        accumulator_max,
        derived_accumulator_ids,
        derived_accumulator_leaves,
        tracked_leaves,
        tracked_dimensions,
        tracked_ids,
        state_leaves,
        state_leaf_positions,
        state_count_max,
        prob_table: ProbTable {
            entries,
            trial_dependent,
            entry_control_invariant: probability_control_invariant,
            control_entry_indices: probability_control_entry_indices,
            control_invariant,
            clamp_events,
        },
        transition_table,
        accumulator_table,
        transitions,
        triggers,
        condition,
        trial_series,
        series_checkpoints: ir.run.series_checkpoints.iter().copied().collect(),
        diagnostics,
        analysis,
        exact_lcm,
        model_hash,
    })
}

fn validate_entity_ids(
    entities: &[Entity],
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for entity in entities {
        if !ids.insert(entity.id.clone()) {
            diagnostics.push(error(
                "E001",
                format!("duplicate entity id '{}'", entity.id),
                entity.block_id.clone(),
            ));
        }
        validate_entity_ids(&entity.children, ids, diagnostics);
    }
}

#[derive(Clone)]
struct AffineExpr {
    constant: Rational,
    variables: BTreeMap<String, Rational>,
    trial: Rational,
}

impl AffineExpr {
    fn constant(value: Rational) -> Self {
        Self {
            constant: value,
            variables: BTreeMap::new(),
            trial: Rational::zero(),
        }
    }

    fn variable(name: String) -> Self {
        Self {
            constant: Rational::zero(),
            variables: BTreeMap::from([(name, Rational::one())]),
            trial: Rational::zero(),
        }
    }

    fn trial() -> Self {
        Self {
            constant: Rational::zero(),
            variables: BTreeMap::new(),
            trial: Rational::one(),
        }
    }

    fn add(mut self, other: Self) -> Self {
        self.constant += other.constant;
        self.trial += other.trial;
        for (name, coefficient) in other.variables {
            let value = self
                .variables
                .entry(name.clone())
                .or_insert_with(Rational::zero);
            *value += coefficient;
            if value.is_zero() {
                self.variables.remove(&name);
            }
        }
        self
    }

    fn scale(mut self, factor: &Rational) -> Self {
        self.constant *= factor;
        self.trial *= factor;
        for coefficient in self.variables.values_mut() {
            *coefficient *= factor;
        }
        self
    }

    fn is_constant(&self) -> bool {
        self.variables.is_empty() && self.trial.is_zero()
    }
}

fn affine_expr(expr: &serde_json::Value) -> Option<AffineExpr> {
    let object = expr.as_object()?;
    if let Some(literal) = object.get("lit").and_then(serde_json::Value::as_str) {
        return parse_literal(literal).ok().map(AffineExpr::constant);
    }
    if let Some(variable) = object.get("var").and_then(serde_json::Value::as_str) {
        return Some(AffineExpr::variable(variable.to_owned()));
    }
    if object.get("trial").is_some() {
        return Some(AffineExpr::trial());
    }
    let unary = |name: &str| object.get(name).and_then(affine_expr);
    let binary = |name: &str| {
        let arguments = object.get(name)?.as_array()?;
        (arguments.len() == 2).then(|| (&arguments[0], &arguments[1]))
    };
    if let Some(value) = unary("neg") {
        return Some(value.scale(&-Rational::one()));
    }
    if let Some((left, right)) = binary("add") {
        return Some(affine_expr(left)?.add(affine_expr(right)?));
    }
    if let Some((left, right)) = binary("sub") {
        return Some(affine_expr(left)?.add(affine_expr(right)?.scale(&-Rational::one())));
    }
    if let Some((left, right)) = binary("mul") {
        let left = affine_expr(left)?;
        let right = affine_expr(right)?;
        if left.is_constant() {
            let factor = left.constant;
            return Some(right.scale(&factor));
        }
        if right.is_constant() {
            let factor = right.constant;
            return Some(left.scale(&factor));
        }
        return None;
    }
    if let Some((left, right)) = binary("div") {
        let numerator = affine_expr(left)?;
        let denominator = affine_expr(right)?;
        if !denominator.is_constant() || denominator.constant.is_zero() {
            return None;
        }
        return Some(numerator.scale(&(Rational::one() / denominator.constant)));
    }
    None
}

fn affine_range(
    expression: &AffineExpr,
    variable_bounds: &HashMap<&str, u32>,
    max_trials: u32,
) -> Option<(Rational, Rational)> {
    let mut lower = expression.constant.clone();
    let mut upper = expression.constant.clone();
    for (name, coefficient) in &expression.variables {
        let maximum = Rational::from_integer((*variable_bounds.get(name.as_str())?).into());
        if coefficient.is_negative() {
            lower += coefficient * &maximum;
        } else {
            upper += coefficient * &maximum;
        }
    }
    let first_trial = Rational::one();
    let last_trial = Rational::from_integer(max_trials.max(1).into());
    if expression.trial.is_negative() {
        lower += &expression.trial * last_trial;
        upper += &expression.trial * first_trial;
    } else {
        lower += &expression.trial * first_trial;
        upper += &expression.trial * last_trial;
    }
    Some((lower, upper))
}

fn validate_probability_ranges(
    entities: &[Entity],
    rules: &[crate::ir::ProbRule],
    control_ids: &[String],
    control_max: &[u32],
    max_trials: u32,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let variable_bounds: HashMap<_, _> = control_ids
        .iter()
        .zip(control_max)
        .map(|(name, maximum)| (name.as_str(), *maximum))
        .collect();
    for entity in entities {
        let rule = rules.iter().rev().find(|rule| rule.target == entity.id);
        let expression = rule.map(|rule| &rule.expr).unwrap_or(&entity.prob);
        let block_id = rule
            .and_then(|rule| rule.block_id.clone())
            .or_else(|| entity.block_id.clone());
        let negative = if let Some(affine) = affine_expr(expression) {
            affine_range(&affine, &variable_bounds, max_trials)
                .map(|(lower, _)| lower)
                .filter(|minimum| minimum.is_negative())
        } else {
            exact_negative_probability(expression, control_ids, control_max, max_trials)
        };
        if let Some(minimum) = negative {
            diagnostics.push(error(
                "E002",
                format!(
                    "probability expression for '{}' can be negative (observed {minimum})",
                    entity.id
                ),
                block_id,
            ));
        }
        validate_probability_ranges(
            &entity.children,
            rules,
            control_ids,
            control_max,
            max_trials,
            diagnostics,
        );
    }
}

fn exact_negative_probability(
    expression: &serde_json::Value,
    control_ids: &[String],
    control_max: &[u32],
    max_trials: u32,
) -> Option<Rational> {
    let program = compile_expr(expression).ok()?;
    let control_states = control_max.iter().fold(1u64, |states, maximum| {
        states.saturating_mul(*maximum as u64 + 1)
    });
    let trials = if program.trial_dependent {
        max_trials.max(1)
    } else {
        1
    };
    if control_states.saturating_mul(trials as u64) > 1_000_000 {
        return None;
    }
    let mut minimum: Option<Rational> = None;
    for control_index in 0..control_states {
        let control = decode_control(control_index, control_max);
        for trial in 1..=trials {
            let Ok(value) = eval(
                &program,
                |name| {
                    control_ids
                        .iter()
                        .position(|id| id == name)
                        .map(|index| Rational::from_integer(control[index].into()))
                },
                trial,
            )
            .and_then(|value| value.number()) else {
                continue;
            };
            if value.is_negative() && minimum.as_ref().is_none_or(|current| value < *current) {
                minimum = Some(value);
            }
        }
    }
    minimum
}

#[derive(Clone, Copy)]
struct TruthPossibility {
    can_be_true: bool,
    can_be_false: bool,
}

impl TruthPossibility {
    fn unknown() -> Self {
        Self {
            can_be_true: true,
            can_be_false: true,
        }
    }
}

fn condition_could_be_true(
    condition: &serde_json::Value,
    max_trials: u32,
    leaves: &[Leaf],
    probability_entries: &[Vec<LeafProbs>],
    triggers: &[CompiledTrigger],
) -> Option<bool> {
    if max_trials == 0 {
        return Some(false);
    }
    condition_possibility(condition, max_trials, leaves, probability_entries, triggers)
        .map(|possibility| possibility.can_be_true)
}

fn condition_possibility(
    condition: &serde_json::Value,
    max_trials: u32,
    leaves: &[Leaf],
    probability_entries: &[Vec<LeafProbs>],
    triggers: &[CompiledTrigger],
) -> Option<TruthPossibility> {
    let object = condition.as_object()?;
    for relation in ["eq", "ne", "lt", "le", "gt", "ge"] {
        let Some(arguments) = object.get(relation).and_then(serde_json::Value::as_array) else {
            continue;
        };
        if arguments.len() != 2 {
            return None;
        }
        let difference =
            affine_expr(&arguments[0])?.add(affine_expr(&arguments[1])?.scale(&-Rational::one()));
        let (lower, upper) = condition_affine_range(
            &difference,
            max_trials,
            leaves,
            probability_entries,
            triggers,
        )?;
        let zero = Rational::zero();
        return Some(match relation {
            "eq" => TruthPossibility {
                can_be_true: lower <= zero && zero <= upper,
                can_be_false: lower != zero || upper != zero,
            },
            "ne" => TruthPossibility {
                can_be_true: lower != zero || upper != zero,
                can_be_false: lower <= zero && zero <= upper,
            },
            "lt" => TruthPossibility {
                can_be_true: lower < zero,
                can_be_false: upper >= zero,
            },
            "le" => TruthPossibility {
                can_be_true: lower <= zero,
                can_be_false: upper > zero,
            },
            "gt" => TruthPossibility {
                can_be_true: upper > zero,
                can_be_false: lower <= zero,
            },
            "ge" => TruthPossibility {
                can_be_true: upper >= zero,
                can_be_false: lower < zero,
            },
            _ => unreachable!(),
        });
    }
    if let Some(inner) = object.get("not") {
        let inner =
            condition_possibility(inner, max_trials, leaves, probability_entries, triggers)?;
        return Some(TruthPossibility {
            can_be_true: inner.can_be_false,
            can_be_false: inner.can_be_true,
        });
    }
    for operation in ["and", "or", "xor"] {
        let Some(arguments) = object.get(operation).and_then(serde_json::Value::as_array) else {
            continue;
        };
        if arguments.len() != 2 {
            return None;
        }
        let left = condition_possibility(
            &arguments[0],
            max_trials,
            leaves,
            probability_entries,
            triggers,
        )?;
        let right = condition_possibility(
            &arguments[1],
            max_trials,
            leaves,
            probability_entries,
            triggers,
        )?;
        return Some(match operation {
            "and" => TruthPossibility {
                can_be_true: left.can_be_true && right.can_be_true,
                can_be_false: left.can_be_false || right.can_be_false,
            },
            "or" => TruthPossibility {
                can_be_true: left.can_be_true || right.can_be_true,
                can_be_false: left.can_be_false && right.can_be_false,
            },
            "xor" => TruthPossibility {
                can_be_true: (left.can_be_true && right.can_be_false)
                    || (left.can_be_false && right.can_be_true),
                can_be_false: (left.can_be_true && right.can_be_true)
                    || (left.can_be_false && right.can_be_false),
            },
            _ => unreachable!(),
        });
    }
    Some(TruthPossibility::unknown())
}

fn condition_affine_range(
    expression: &AffineExpr,
    max_trials: u32,
    leaves: &[Leaf],
    probability_entries: &[Vec<LeafProbs>],
    triggers: &[CompiledTrigger],
) -> Option<(Rational, Rational)> {
    let mut leaf_coefficients = vec![Rational::zero(); leaves.len()];
    for (variable, coefficient) in &expression.variables {
        let entity = variable
            .strip_prefix('n')
            .map(lower_first)
            .unwrap_or_else(|| variable.to_owned());
        let mut found = false;
        for (index, leaf) in leaves.iter().enumerate() {
            if leaf.id == entity || leaf.ancestors.iter().any(|ancestor| ancestor == &entity) {
                leaf_coefficients[index] += coefficient;
                found = true;
            }
        }
        if !found {
            return None;
        }
    }

    let possible_normal_leaf: Vec<_> = (0..leaves.len())
        .map(|leaf| {
            probability_entries
                .iter()
                .flat_map(|by_trial| by_trial)
                .any(|probabilities| probabilities.exact[leaf].is_positive())
        })
        .collect();
    let mut lower = expression.constant.clone();
    let mut upper = expression.constant.clone();
    let normal_minimum = leaf_coefficients
        .iter()
        .zip(&possible_normal_leaf)
        .filter(|(_, possible)| **possible)
        .map(|(coefficient, _)| coefficient)
        .filter(|coefficient| coefficient.is_negative())
        .min()
        .cloned()
        .unwrap_or_else(Rational::zero);
    let normal_maximum = leaf_coefficients
        .iter()
        .zip(&possible_normal_leaf)
        .filter(|(_, possible)| **possible)
        .map(|(coefficient, _)| coefficient)
        .filter(|coefficient| coefficient.is_positive())
        .max()
        .cloned()
        .unwrap_or_else(Rational::zero);
    let trial_budget = Rational::from_integer(max_trials.into());
    lower += normal_minimum * &trial_budget;
    upper += normal_maximum * &trial_budget;

    for grant in triggers.iter().filter_map(|trigger| trigger.grant.as_ref()) {
        let contribution =
            &leaf_coefficients[grant.leaf] * Rational::from_integer(grant.amount.into());
        if contribution.is_negative() {
            lower += contribution;
        } else {
            upper += contribution;
        }
    }

    let first_trial = Rational::one();
    let last_trial = Rational::from_integer(max_trials.into());
    if expression.trial.is_negative() {
        lower += &expression.trial * last_trial;
        upper += &expression.trial * first_trial;
    } else {
        lower += &expression.trial * first_trial;
        upper += &expression.trial * last_trial;
    }
    Some((lower, upper))
}

fn collect_leaves(entities: &[Entity], ancestors: &mut Vec<String>, leaves: &mut Vec<Leaf>) {
    for entity in entities {
        if entity.children.is_empty() {
            leaves.push(Leaf {
                id: entity.id.clone(),
                name: entity.name.clone(),
                ancestors: ancestors.clone(),
            });
        } else {
            ancestors.push(entity.id.clone());
            collect_leaves(&entity.children, ancestors, leaves);
            ancestors.pop();
            leaves.push(Leaf {
                id: format!("{}__self", entity.id),
                name: format!("{}(전용)", entity.name),
                ancestors: ancestors
                    .iter()
                    .cloned()
                    .chain(std::iter::once(entity.id.clone()))
                    .collect(),
            });
        }
    }
}

fn compile_entities(
    entities: &[Entity],
    rules: &HashMap<&str, &serde_json::Value>,
    leaves: &[Leaf],
    exact_available: &mut bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<EntityDef>> {
    let mut result = Vec::new();
    for entity in entities {
        let expr = rules
            .get(entity.id.as_str())
            .copied()
            .unwrap_or(&entity.prob);
        let program = match compile_expr(expr) {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(error("E006", e.to_string(), entity.block_id.clone()));
                return None;
            }
        };
        *exact_available &= program.exact_safe;
        let children = compile_entities(
            &entity.children,
            rules,
            leaves,
            exact_available,
            diagnostics,
        )?;
        let leaf_indices = leaves
            .iter()
            .enumerate()
            .filter_map(|(i, leaf)| {
                (leaf.id == entity.id || leaf.ancestors.iter().any(|a| a == &entity.id))
                    .then_some(i)
            })
            .collect();
        let self_leaf = (!entity.children.is_empty()).then(|| {
            leaves
                .iter()
                .position(|l| l.id == format!("{}__self", entity.id))
                .unwrap()
        });
        result.push(EntityDef {
            id: entity.id.clone(),
            program,
            children,
            leaf_indices,
            self_leaf,
        });
    }
    Some(result)
}

fn entity_trial_dependent(entity: &EntityDef) -> bool {
    entity.program.trial_dependent || entity.children.iter().any(entity_trial_dependent)
}

fn entity_defs_control_dependent(entities: &[EntityDef], control_ids: &[String]) -> bool {
    entities.iter().any(|entity| {
        entity
            .program
            .ops
            .iter()
            .any(|op| matches!(op, Op::PushVar(name) if control_ids.contains(name)))
            || entity_defs_control_dependent(&entity.children, control_ids)
    })
}

fn calculate_leaf_probs(
    roots: &[EntityDef],
    leaf_count: usize,
    control_ids: &[String],
    control: &[u32],
    trial: u32,
    policy: NestingPolicy,
    clamp_events: &mut u64,
) -> Result<Vec<Rational>, String> {
    let mut out = vec![Rational::zero(); leaf_count];
    let mut top_total = Rational::zero();
    for root in roots {
        let p = eval_probability(root, control_ids, control, trial)?;
        if p.is_negative() {
            return Err(format!("probability for '{}' is negative", root.id));
        }
        let effective = split_entity(
            root,
            p,
            control_ids,
            control,
            trial,
            policy,
            &mut out,
            clamp_events,
        )?;
        top_total += effective;
    }
    if top_total > Rational::one() {
        return Err(format!("top-level probability sum {} exceeds 1", top_total));
    }
    out[leaf_count - 1] = Rational::one() - top_total;
    let sum: Rational = out.iter().cloned().sum();
    if sum != Rational::one() {
        return Err(format!("leaf probability mass is {sum}, expected 1"));
    }
    Ok(out)
}

fn split_entity(
    entity: &EntityDef,
    mut parent_p: Rational,
    control_ids: &[String],
    control: &[u32],
    trial: u32,
    policy: NestingPolicy,
    out: &mut [Rational],
    clamp_events: &mut u64,
) -> Result<Rational, String> {
    if entity.children.is_empty() {
        let leaf = *entity.leaf_indices.iter().next().expect("leaf index");
        out[leaf] += &parent_p;
        return Ok(parent_p);
    }
    let mut child_layers = Vec::with_capacity(entity.children.len());
    let mut child_total = Rational::zero();
    for child in &entity.children {
        let child_probability = eval_probability(child, control_ids, control, trial)?;
        if child_probability.is_negative() {
            return Err(format!("negative child probability below '{}'", entity.id));
        }
        let mut child_out = vec![Rational::zero(); out.len()];
        let effective = split_entity(
            child,
            child_probability,
            control_ids,
            control,
            trial,
            policy,
            &mut child_out,
            clamp_events,
        )?;
        child_total += &effective;
        child_layers.push(child_out);
    }
    if child_total > parent_p {
        *clamp_events += 1;
        match policy {
            NestingPolicy::ClampChildren | NestingPolicy::ScaleSiblings => {
                let scale = &parent_p / &child_total;
                for layer in &mut child_layers {
                    for probability in layer {
                        *probability *= &scale;
                    }
                }
                child_total = parent_p.clone();
            }
            NestingPolicy::ExpandParent => parent_p = child_total.clone(),
            NestingPolicy::Error => {
                return Err(format!(
                    "children of '{}' exceed parent probability",
                    entity.id
                ))
            }
        }
    }
    out[entity.self_leaf.expect("internal entity self leaf")] += &parent_p - &child_total;
    for layer in child_layers {
        for (target, probability) in out.iter_mut().zip(layer) {
            *target += probability;
        }
    }
    Ok(parent_p)
}

fn eval_probability(
    entity: &EntityDef,
    ids: &[String],
    control: &[u32],
    trial: u32,
) -> Result<Rational, String> {
    eval(
        &entity.program,
        |name| {
            ids.iter()
                .position(|id| id == name)
                .map(|i| Rational::from_integer(control[i].into()))
        },
        trial,
    )
    .and_then(|v| v.number())
    .map_err(|e| e.to_string())
}

fn decode_control(mut index: u64, maxes: &[u32]) -> Vec<u32> {
    maxes
        .iter()
        .map(|max| {
            let radix = *max as u64 + 1;
            let value = (index % radix) as u32;
            index /= radix;
            value
        })
        .collect()
}

fn encode_control(values: &[u32], maxes: &[u32]) -> usize {
    let mut index = 0usize;
    let mut stride = 1usize;
    for (value, maximum) in values.iter().zip(maxes) {
        index += *value as usize * stride;
        stride *= *maximum as usize + 1;
    }
    index
}

fn reachable_control_indices(
    transitions: &[CompiledTransition],
    control_ids: &[String],
    control_init: &[u32],
    control_max: &[u32],
    leaf_count: usize,
    transition_trial_dependent: bool,
    triggers: &[CompiledTrigger],
    max_trials: u32,
    reachable_limit: usize,
) -> Result<Vec<usize>, usize> {
    let initial_index = encode_control(control_init, control_max);
    if reachable_limit == 0 {
        return Err(1);
    }
    let trigger_changes_control = triggers.iter().any(|trigger| {
        !trigger.assignments.is_empty()
            || trigger
                .grant
                .as_ref()
                .is_some_and(|grant| grant.applies_transitions && grant.amount > 0)
    });
    if transitions.is_empty() && !trigger_changes_control {
        return Ok(vec![initial_index]);
    }

    let mut reachable = BTreeSet::from([initial_index]);
    let mut layer = BTreeSet::from([initial_index]);
    let mut completed_trials = 0u32;
    while completed_trials < max_trials {
        let draw_trial = completed_trials + 1;
        let mut next_layer = BTreeSet::new();
        let mut added_reachable = false;
        for control_index in layer {
            let control = decode_control(control_index as u64, control_max);
            for leaf in 0..leaf_count {
                let mut next = control.clone();
                apply_compiled_transitions(
                    transitions,
                    control_ids,
                    control_max,
                    &mut next,
                    &control,
                    leaf,
                    draw_trial,
                );
                apply_compiled_triggers_to_control(
                    triggers,
                    transitions,
                    control_ids,
                    control_max,
                    &mut next,
                    draw_trial,
                    max_trials,
                );
                let next_index = encode_control(&next, control_max);
                next_layer.insert(next_index);
                if reachable.insert(next_index) {
                    added_reachable = true;
                    if reachable.len() > reachable_limit {
                        return Err(reachable.len());
                    }
                }
            }
        }
        let next_completed_trials =
            draw_trial + compiled_consumed_trials_after(triggers, draw_trial, max_trials);
        layer = next_layer;
        if !transition_trial_dependent && !added_reachable {
            let next_trigger_trial = triggers
                .iter()
                .map(|trigger| trigger.trial_count)
                .filter(|trial| *trial > next_completed_trials && *trial <= max_trials)
                .min();
            let Some(next_trigger_trial) = next_trigger_trial else {
                break;
            };
            completed_trials = next_trigger_trial - 1;
            layer = reachable.clone();
        } else {
            completed_trials = next_completed_trials;
        }
    }
    Ok(reachable.into_iter().collect())
}

fn compiled_consumed_trials_after(
    triggers: &[CompiledTrigger],
    trial: u32,
    max_trials: u32,
) -> u32 {
    let requested = triggers
        .iter()
        .filter(|trigger| trigger.trial_count == trial)
        .filter(|trigger| {
            trigger
                .grant
                .as_ref()
                .is_some_and(|grant| grant.consumes_trial)
        })
        .count() as u32;
    requested.min(max_trials.saturating_sub(trial))
}

fn apply_compiled_triggers_to_control(
    triggers: &[CompiledTrigger],
    transitions: &[CompiledTransition],
    control_ids: &[String],
    control_max: &[u32],
    control: &mut [u32],
    trial: u32,
    max_trials: u32,
) {
    let max_consumed = compiled_consumed_trials_after(triggers, trial, max_trials);
    let mut consumed = 0u32;
    for trigger in triggers
        .iter()
        .filter(|trigger| trigger.trial_count == trial)
    {
        let action_trial = trial + consumed;
        let before = control.to_vec();
        for (index, program) in &trigger.assignments {
            // Assignment references are compile-time validated; keep this fallible
            // evaluation as a defensive runtime guard.
            if let Ok(value) = eval(
                program,
                |name| {
                    control_ids
                        .iter()
                        .position(|id| id == name)
                        .map(|i| Rational::from_integer(before[i].into()))
                },
                action_trial,
            )
            .and_then(|value| value.number())
            {
                control[*index] = value
                    .to_integer()
                    .to_u32()
                    .unwrap_or(u32::MAX)
                    .min(control_max[*index]);
            }
        }
        if let Some(grant) = &trigger.grant {
            if grant.consumes_trial {
                if consumed >= max_consumed {
                    continue;
                }
                consumed += 1;
            }
            if grant.applies_transitions {
                let grant_trial = trial + consumed;
                for _ in 0..grant.amount {
                    let before = control.to_vec();
                    apply_compiled_transitions(
                        transitions,
                        control_ids,
                        control_max,
                        control,
                        &before,
                        grant.leaf,
                        grant_trial,
                    );
                }
            }
        }
    }
}

fn push_probability_table_size_diagnostic(
    control_count: u64,
    trial_count: u32,
    leaf_count: usize,
    is_lower_bound: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let entry_count = u128::from(control_count)
        .saturating_mul(u128::from(trial_count))
        .saturating_mul(leaf_count as u128);
    let axes = if is_lower_bound {
        format!(
            "control>={control_count}, trials={trial_count}, leaves={leaf_count}, entries>={entry_count}"
        )
    } else {
        format!(
            "control={control_count}, trials={trial_count}, leaves={leaf_count}, entries={entry_count}"
        )
    };
    if entry_count > PROBABILITY_TABLE_MAX_ENTRIES {
        let requirement = if is_lower_bound {
            format!("requires at least {entry_count} entries")
        } else {
            format!("requires {entry_count} entries")
        };
        let axes_label = if is_lower_bound {
            "lower-bound axes"
        } else {
            "axes"
        };
        diagnostics.push(error(
            "E012",
            format!(
                "probability precompute table {requirement}, exceeding hard limit {PROBABILITY_TABLE_MAX_ENTRIES}; {axes_label}: {axes}; reduce reachable control states, maxTrials, or probability leaves"
            ),
            None,
        ));
        return false;
    }
    if entry_count >= PROBABILITY_TABLE_WARNING_ENTRIES {
        let requirement = if is_lower_bound {
            format!("requires at least {entry_count} entries")
        } else {
            format!("requires {entry_count} entries")
        };
        let axes_label = if is_lower_bound {
            "lower-bound axes"
        } else {
            "axes"
        };
        diagnostics.push(warning(
            "W010",
            format!(
                "probability precompute table {requirement}; {axes_label}: {axes}; reduce reachable control states, maxTrials, or probability leaves"
            ),
            None,
        ));
    }
    true
}

fn compile_transitions(
    transitions: &[Transition],
    leaves: &[Leaf],
    control_ids: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<CompiledTransition> {
    transitions
        .iter()
        .filter_map(|transition| {
            let predicate = compile_predicate(
                &transition.when,
                leaves,
                diagnostics,
                transition.block_id.clone(),
            )?;
            let assignments = compile_assignments(
                &transition.set,
                control_ids,
                diagnostics,
                transition.block_id.clone(),
            );
            Some(CompiledTransition {
                predicate,
                assignments,
            })
        })
        .collect()
}

fn compile_predicate(
    predicate: &LeafPredicate,
    leaves: &[Leaf],
    diagnostics: &mut Vec<Diagnostic>,
    block_id: Option<String>,
) -> Option<CompiledPredicate> {
    Some(match predicate {
        LeafPredicate::LeafOf { leaf_of } => {
            let set: BTreeSet<_> = leaves
                .iter()
                .enumerate()
                .filter_map(|(i, l)| {
                    (l.id == *leaf_of || l.ancestors.iter().any(|a| a == leaf_of)).then_some(i)
                })
                .collect();
            if set.is_empty() {
                diagnostics.push(error(
                    "E006",
                    format!("unknown entity '{leaf_of}' in transition"),
                    block_id,
                ));
                return None;
            }
            CompiledPredicate::LeafSet(set)
        }
        LeafPredicate::LeafIs { leaf_is } => {
            let set = leaves
                .iter()
                .position(|l| l.id == *leaf_is)
                .into_iter()
                .collect();
            CompiledPredicate::LeafSet(set)
        }
        LeafPredicate::Not { not } => CompiledPredicate::Not(Box::new(compile_predicate(
            not,
            leaves,
            diagnostics,
            block_id,
        )?)),
        LeafPredicate::And { and } => CompiledPredicate::And(
            and.iter()
                .filter_map(|p| compile_predicate(p, leaves, diagnostics, block_id.clone()))
                .collect(),
        ),
        LeafPredicate::Or { or } => CompiledPredicate::Or(
            or.iter()
                .filter_map(|p| compile_predicate(p, leaves, diagnostics, block_id.clone()))
                .collect(),
        ),
    })
}

fn compile_accumulator_table(
    ir: &ModelIr,
    leaves: &[Leaf],
    control_ids: &[String],
    control_max: &[u32],
    accumulator_ids: &[String],
    accumulator_max: &[u32],
    max_trials: u32,
    diagnostics: &mut Vec<Diagnostic>,
) -> AccumulatorTable {
    struct AccumulatorSpec {
        id: String,
        block_id: Option<String>,
        clamp_policy: ClampPolicy,
        rules: Vec<(CompiledPredicate, Program)>,
        depends_on_trial: bool,
        depends_on_control: bool,
        control_states: u64,
        table_trials: u32,
        maximum: u32,
        entry_count: u128,
    }

    let mut specs = Vec::with_capacity(accumulator_ids.len());
    for (accumulator_index, accumulator_id) in accumulator_ids.iter().enumerate() {
        let var = ir
            .state_vars
            .iter()
            .find(|var| var.role == StateRole::Accumulator && var.id == *accumulator_id)
            .expect("compiled accumulator comes from stateVars");
        let mut rules = Vec::new();
        for update in &var.update {
            let Some(predicate) =
                compile_predicate(&update.when, leaves, diagnostics, var.block_id.clone())
            else {
                continue;
            };
            match compile_expr(&update.set) {
                Ok(program) => {
                    let mut invalid = None;
                    for op in &program.ops {
                        if let Op::PushVar(name) = op {
                            if name != accumulator_id && !control_ids.contains(name) {
                                invalid = Some(name.clone());
                                break;
                            }
                        }
                    }
                    if let Some(name) = invalid {
                        diagnostics.push(error(
                            "E006",
                            format!(
                                "accumulator '{}' update references unsupported variable '{}'",
                                accumulator_id, name
                            ),
                            var.block_id.clone(),
                        ));
                    } else {
                        rules.push((predicate, program));
                    }
                }
                Err(error_value) => {
                    diagnostics.push(error("E006", error_value.to_string(), var.block_id.clone()))
                }
            }
        }
        let depends_on_trial = rules.iter().any(|(_, program)| program.trial_dependent);
        let depends_on_control = rules.iter().any(|(_, program)| {
            program
                .ops
                .iter()
                .any(|op| matches!(op, Op::PushVar(name) if control_ids.contains(name)))
        });
        let control_states = if depends_on_control {
            control_max.iter().fold(1u64, |states, max| {
                states.saturating_mul(u64::from(*max) + 1)
            })
        } else {
            1
        };
        let table_trials = if depends_on_trial {
            max_trials.max(1)
        } else {
            1
        };
        let maximum = accumulator_max[accumulator_index];
        let entry_count = u128::from(control_states)
            .saturating_mul(u128::from(table_trials))
            .saturating_mul(leaves.len() as u128)
            .saturating_mul(u128::from(maximum) + 1);
        specs.push(AccumulatorSpec {
            id: accumulator_id.clone(),
            block_id: var.block_id.clone(),
            clamp_policy: var.clamp_policy,
            rules,
            depends_on_trial,
            depends_on_control,
            control_states,
            table_trials,
            maximum,
            entry_count,
        });
    }

    let total_entries = specs
        .iter()
        .fold(0u128, |total, spec| total.saturating_add(spec.entry_count));
    let axes = specs
        .iter()
        .map(|spec| {
            format!(
                "{}(control={}, trials={}, leaves={}, current=max+1={}, entries={})",
                spec.id,
                spec.control_states,
                spec.table_trials,
                leaves.len(),
                u128::from(spec.maximum) + 1,
                spec.entry_count,
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let trial_dependent = specs
        .iter()
        .map(|spec| spec.depends_on_trial)
        .collect::<Vec<_>>();
    let control_dependent = specs
        .iter()
        .map(|spec| spec.depends_on_control)
        .collect::<Vec<_>>();
    if total_entries > ACCUMULATOR_TABLE_MAX_ENTRIES {
        diagnostics.push(error(
            "E010",
            format!(
                "accumulator precompute table requires {total_entries} entries, exceeding hard limit {ACCUMULATOR_TABLE_MAX_ENTRIES}; axes: {axes}; reduce max or remove control/trial dependency"
            ),
            specs.first().and_then(|spec| spec.block_id.clone()),
        ));
        return AccumulatorTable {
            entries: (0..specs.len()).map(|_| Vec::new()).collect(),
            trial_dependent,
            control_dependent,
        };
    }
    if total_entries >= ACCUMULATOR_TABLE_WARNING_ENTRIES {
        diagnostics.push(warning(
            "W009",
            format!(
                "accumulator precompute table requires {total_entries} entries; axes: {axes}; reduce max or remove control/trial dependency"
            ),
            specs.first().and_then(|spec| spec.block_id.clone()),
        ));
    }

    let mut all_entries = Vec::with_capacity(specs.len());
    for spec in specs {
        let AccumulatorSpec {
            id: accumulator_id,
            block_id,
            clamp_policy,
            rules,
            depends_on_control,
            control_states,
            table_trials,
            maximum,
            ..
        } = spec;
        let mut by_control = Vec::with_capacity(control_states as usize);
        let mut error_reported = false;
        for control_index in 0..control_states {
            let control = if depends_on_control {
                decode_control(control_index, control_max)
            } else {
                vec![0; control_ids.len()]
            };
            let mut by_trial = Vec::with_capacity(table_trials as usize);
            for trial in 1..=table_trials {
                let mut by_leaf = Vec::with_capacity(leaves.len());
                for leaf in 0..leaves.len() {
                    let mut by_current = Vec::with_capacity(maximum as usize + 1);
                    for current in 0..=maximum {
                        let mut next = current as i64;
                        for (predicate, program) in &rules {
                            if !predicate.matches(leaf) {
                                continue;
                            }
                            match eval(
                                program,
                                |name| {
                                    if name == accumulator_id {
                                        Some(Rational::from_integer(current.into()))
                                    } else {
                                        control_ids.iter().position(|id| id == name).map(|index| {
                                            Rational::from_integer(control[index].into())
                                        })
                                    }
                                },
                                trial,
                            )
                            .and_then(|value| value.number())
                            {
                                Ok(value) => next = value.to_integer().to_i64().unwrap_or(i64::MAX),
                                Err(error_value) if !error_reported => {
                                    diagnostics.push(error(
                                        "E006",
                                        error_value.to_string(),
                                        block_id.clone(),
                                    ));
                                    error_reported = true;
                                }
                                Err(_) => {}
                            }
                        }
                        let clamped = next < 0 || next > i64::from(maximum);
                        if clamped && clamp_policy == ClampPolicy::Error && !error_reported {
                            diagnostics.push(error(
                                "E004",
                                format!(
                                    "accumulator '{}' update can exceed 0..={maximum} with clampPolicy error",
                                    accumulator_id
                                ),
                                block_id.clone(),
                            ));
                            error_reported = true;
                        }
                        by_current.push(AccumulatorTransition {
                            value: next.clamp(0, i64::from(maximum)) as u32,
                            clamped,
                        });
                    }
                    by_leaf.push(by_current);
                }
                by_trial.push(by_leaf);
            }
            by_control.push(by_trial);
        }
        all_entries.push(by_control);
    }
    AccumulatorTable {
        entries: all_entries,
        trial_dependent,
        control_dependent,
    }
}

fn simple_counter_target(var: &crate::ir::StateVar) -> Option<&str> {
    if var.init != 0 || var.update.len() != 1 {
        return None;
    }
    let update = &var.update[0];
    let target = match &update.when {
        LeafPredicate::LeafOf { leaf_of } => leaf_of.as_str(),
        LeafPredicate::LeafIs { leaf_is } => leaf_is.as_str(),
        _ => return None,
    };
    let add = update.set.get("add")?.as_array()?;
    if add.len() != 2 {
        return None;
    }
    let is_self = |value: &serde_json::Value| {
        value.get("var").and_then(serde_json::Value::as_str) == Some(var.id.as_str())
    };
    let is_one = |value: &serde_json::Value| {
        value.get("lit").and_then(serde_json::Value::as_str) == Some("1")
    };
    ((is_self(&add[0]) && is_one(&add[1])) || (is_one(&add[0]) && is_self(&add[1])))
        .then_some(target)
}

fn compile_triggers(
    triggers: &[Trigger],
    leaves: &[Leaf],
    lookup: &HashMap<&str, usize>,
    control_ids: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<CompiledTrigger> {
    triggers
        .iter()
        .filter_map(|trigger| {
            let grant = trigger.grant.as_ref().and_then(|grant| {
                compile_grant(grant, leaves, lookup, diagnostics, trigger.block_id.clone())
            });
            let assignments = compile_assignments(
                &trigger.set,
                control_ids,
                diagnostics,
                trigger.block_id.clone(),
            );
            Some(CompiledTrigger {
                trial_count: trigger.at.trial_count,
                grant,
                assignments,
            })
        })
        .collect()
}

fn warn_unapplied_consuming_grants(
    triggers: &[Trigger],
    max_trials: u32,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut consuming_by_trial = BTreeMap::<u32, u32>::new();
    for trigger in triggers {
        let Some(grant) = &trigger.grant else {
            continue;
        };
        if !grant.consumes_trial {
            continue;
        }
        let requested = consuming_by_trial
            .entry(trigger.at.trial_count)
            .or_default();
        *requested += 1;
        let available = max_trials.saturating_sub(trigger.at.trial_count);
        if *requested > available {
            diagnostics.push(warning(
                "W007",
                format!(
                    "consumesTrial grant at trialCount {} has no remaining logical trial slot within maxTrials {max_trials} and will not be applied",
                    trigger.at.trial_count,
                ),
                trigger.block_id.clone(),
            ));
        }
    }
}

fn compile_grant(
    grant: &Grant,
    leaves: &[Leaf],
    lookup: &HashMap<&str, usize>,
    diagnostics: &mut Vec<Diagnostic>,
    block_id: Option<String>,
) -> Option<CompiledGrant> {
    let Some(&leaf) = lookup.get(grant.leaf.as_str()) else {
        let internal = leaves
            .iter()
            .any(|l| l.ancestors.iter().any(|a| a == &grant.leaf));
        diagnostics.push(error(
            "E007",
            if internal {
                format!("grant target '{}' is an internal entity", grant.leaf)
            } else {
                format!("unknown grant leaf '{}'", grant.leaf)
            },
            block_id,
        ));
        return None;
    };
    Some(CompiledGrant {
        leaf,
        amount: grant.amount,
        consumes_trial: grant.consumes_trial,
        applies_transitions: grant.applies_transitions,
    })
}

fn compile_assignments(
    assignments: &BTreeMap<String, serde_json::Value>,
    control_ids: &[String],
    diagnostics: &mut Vec<Diagnostic>,
    block_id: Option<String>,
) -> Vec<(usize, Program)> {
    assignments
        .iter()
        .filter_map(|(name, expr)| {
            let Some(index) = control_ids.iter().position(|id| id == name) else {
                diagnostics.push(error(
                    "E008",
                    format!("assignment to undeclared/non-control variable '{name}'"),
                    block_id.clone(),
                ));
                return None;
            };
            match compile_expr(expr) {
                Ok(program) => {
                    if let Some(reference) = program.ops.iter().find_map(|op| match op {
                        Op::PushVar(reference) if !control_ids.contains(reference) => {
                            Some(reference)
                        }
                        _ => None,
                    }) {
                        diagnostics.push(error(
                            "E008",
                            format!(
                                "assignment to '{name}' references undeclared/non-control variable '{reference}'"
                            ),
                            block_id.clone(),
                        ));
                        None
                    } else {
                        Some((index, program))
                    }
                }
                Err(e) => {
                    diagnostics.push(error("E006", e.to_string(), block_id.clone()));
                    None
                }
            }
        })
        .collect()
}

fn expand_tracked(
    names: &[String],
    leaves: &[Leaf],
    accumulator_ids: &[String],
    derived_accumulator_ids: &[String],
    derived_accumulator_leaves: &[Vec<usize>],
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<usize>, Vec<TrackedDimension>, Vec<String>) {
    let mut leaf_result = BTreeSet::new();
    let mut dimensions = Vec::new();
    let mut ids = Vec::new();
    for name in names {
        let before = dimensions.len();
        if let Some(index) = accumulator_ids.iter().position(|id| id == name) {
            let dimension = TrackedDimension::Accumulator(index);
            if !dimensions.contains(&dimension) {
                dimensions.push(dimension);
                ids.push(name.clone());
            }
        }
        if let Some(index) = derived_accumulator_ids.iter().position(|id| id == name) {
            let dimension = TrackedDimension::DerivedAccumulator(index);
            if !dimensions.contains(&dimension) {
                dimensions.push(dimension);
                ids.push(name.clone());
            }
            leaf_result.extend(derived_accumulator_leaves[index].iter().copied());
        }
        for (i, leaf) in leaves.iter().enumerate() {
            if leaf.id == *name || leaf.ancestors.iter().any(|a| a == name) {
                leaf_result.insert(i);
                let dimension = TrackedDimension::Leaf(i);
                if !dimensions.contains(&dimension) {
                    dimensions.push(dimension);
                    ids.push(leaf.id.clone());
                }
            }
        }
        if dimensions.len() == before {
            diagnostics.push(warning(
                "W006",
                format!("trackJoint item '{name}' matches no entity, leaf, or accumulator"),
                None,
            ));
        }
    }
    (leaf_result.into_iter().collect(), dimensions, ids)
}

fn collect_expr_variables(expr: &serde_json::Value, out: &mut Vec<String>) {
    match expr {
        serde_json::Value::Object(object) => {
            if let Some(name) = object.get("var").and_then(serde_json::Value::as_str) {
                out.push(name.to_owned());
            }
            for value in object.values() {
                collect_expr_variables(value, out);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_expr_variables(value, out);
            }
        }
        _ => {}
    }
}

fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|c| c.to_lowercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

fn error(code: &str, message: impl Into<String>, block_id: Option<String>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        message: message.into(),
        block_id,
    }
}
fn warning(code: &str, message: impl Into<String>, block_id: Option<String>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Warning,
        message: message.into(),
        block_id,
    }
}
fn info(code: &str, message: impl Into<String>, block_id: Option<String>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Info,
        message: message.into(),
        block_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn blue_archive_tree_compiles_to_exclusive_leaves() {
        let ir: ModelIr = serde_json::from_value(json!({
            "irVersion":1, "name":"BA",
            "entities":[{"id":"star3","name":"3성","prob":{"lit":"0.03"},
                "children":[{"id":"pickup","name":"픽업","prob":{"lit":"0.007"}}]}],
            "nestingPolicy":"clampChildren", "stateVars":[], "probRules":[],
            "transitions":[], "triggers":[],
            "run":{"maxTrials":10,"trackJoint":["pickup"],"numeric":"scaled"}
        }))
        .unwrap();
        let model = compile(&ir).unwrap();
        let probs = &model.prob_table.entries[0][0].exact;
        assert_eq!(probs[0], Rational::new(7.into(), 1000.into()));
        assert_eq!(probs[1], Rational::new(23.into(), 1000.into()));
        assert_eq!(probs[2], Rational::new(970.into(), 1000.into()));
    }

    #[test]
    fn probability_table_size_diagnostics_cover_warning_and_hard_limits() {
        let mut diagnostics = Vec::new();
        assert!(push_probability_table_size_diagnostic(
            250_000,
            1,
            2,
            false,
            &mut diagnostics,
        ));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W010");
        assert!(diagnostics[0].message.contains("entries=500000"));

        diagnostics.clear();
        assert!(!push_probability_table_size_diagnostic(
            5_000_001,
            1,
            2,
            false,
            &mut diagnostics,
        ));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E012");
        assert!(diagnostics[0].message.contains("entries=10000002"));

        diagnostics.clear();
        assert!(!push_probability_table_size_diagnostic(
            5_000_001,
            1,
            2,
            true,
            &mut diagnostics,
        ));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E012");
        assert!(diagnostics[0]
            .message
            .contains("requires at least 10000002 entries"));
        assert!(diagnostics[0]
            .message
            .contains("lower-bound axes: control>=5000001"));
    }
}
