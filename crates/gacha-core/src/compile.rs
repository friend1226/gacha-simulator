use crate::expr::{compile_expr, eval, Program};
use crate::ir::{
    Entity, Grant, LeafPredicate, ModelIr, NestingPolicy, NumericBackend, StateRole, Transition, Trigger,
};
use crate::rational::Rational;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity { Error, Warning, Info }

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
    pub clamp_events: u64,
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
    pub tracked_leaves: Vec<usize>,
    pub state_leaves: Vec<usize>,
    pub prob_table: ProbTable,
    pub transitions: Vec<CompiledTransition>,
    pub triggers: Vec<CompiledTrigger>,
    pub condition: Option<Program>,
    pub diagnostics: Vec<Diagnostic>,
    pub analysis: MarkovAnalysis,
    pub exact_lcm: BigInt,
}

impl CompiledModel {
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
        let ti = if self.prob_table.trial_dependent { trial.saturating_sub(1) as usize } else { 0 };
        &self.prob_table.entries[ci][ti]
    }

    pub fn apply_transitions(&self, control: &mut [u32], leaf: usize, trial: u32) {
        let before = control.to_vec();
        for transition in &self.transitions {
            if !transition.predicate.matches(leaf) { continue; }
            for (index, program) in &transition.assignments {
                if let Ok(value) = eval(program, |name| {
                    self.control_ids.iter().position(|id| id == name)
                        .map(|i| Rational::from_integer(before[i].into()))
                }, trial).and_then(|v| v.number()) {
                    let next = value.to_integer().to_i64().unwrap_or(i64::MAX)
                        .clamp(0, self.control_max[*index] as i64);
                    control[*index] = next as u32;
                }
            }
        }
    }

    pub fn apply_triggers(&self, control: &mut [u32], counts: &mut [u32], trial: u32) {
        for trigger in self.triggers.iter().filter(|t| t.trial_count == trial) {
            let before = control.to_vec();
            for (index, program) in &trigger.assignments {
                if let Ok(value) = eval(program, |name| {
                    self.control_ids.iter().position(|id| id == name)
                        .map(|i| Rational::from_integer(before[i].into()))
                }, trial).and_then(|v| v.number()) {
                    control[*index] = value.to_integer().to_u32().unwrap_or(u32::MAX)
                        .min(self.control_max[*index]);
                }
            }
            if let Some(grant) = &trigger.grant {
                counts[grant.leaf] = counts[grant.leaf].saturating_add(grant.amount);
                if grant.applies_transitions {
                    for _ in 0..grant.amount {
                        self.apply_transitions(control, grant.leaf, trial);
                    }
                }
            }
        }
    }

    pub fn entity_count(&self, counts: &[u32], entity: &str) -> Option<u32> {
        let mut found = false;
        let total = self.leaves.iter().enumerate().filter_map(|(i, leaf)| {
            let matches = leaf.id == entity || leaf.ancestors.iter().any(|a| a == entity);
            found |= matches;
            matches.then_some(counts[i])
        }).sum();
        found.then_some(total)
    }

    pub fn entity_count_sparse(&self, counts: &[u32], entity: &str) -> Option<u32> {
        let mut found = false;
        let total = self.state_leaves.iter().zip(counts).filter_map(|(leaf_index, count)| {
            let leaf = &self.leaves[*leaf_index];
            let matches = leaf.id == entity || leaf.ancestors.iter().any(|a| a == entity);
            found |= matches;
            matches.then_some(*count)
        }).sum();
        found.then_some(total)
    }

    pub fn apply_triggers_sparse(&self, control: &mut [u32], counts: &mut [u32], trial: u32) {
        for trigger in self.triggers.iter().filter(|t| t.trial_count == trial) {
            let before = control.to_vec();
            for (index, program) in &trigger.assignments {
                if let Ok(value) = eval(program, |name| {
                    self.control_ids.iter().position(|id| id == name)
                        .map(|i| Rational::from_integer(before[i].into()))
                }, trial).and_then(|v| v.number()) {
                    control[*index] = value.to_integer().to_u32().unwrap_or(u32::MAX)
                        .min(self.control_max[*index]);
                }
            }
            if let Some(grant) = &trigger.grant {
                if let Some(position) = self.state_leaves.iter().position(|leaf| *leaf == grant.leaf) {
                    counts[position] = counts[position].saturating_add(grant.amount);
                }
                if grant.applies_transitions {
                    for _ in 0..grant.amount {
                        self.apply_transitions(control, grant.leaf, trial);
                    }
                }
            }
        }
    }
}

struct EntityDef {
    id: String,
    name: String,
    program: Program,
    children: Vec<EntityDef>,
    leaf_indices: BTreeSet<usize>,
    self_leaf: Option<usize>,
}

pub fn compile(ir: &ModelIr) -> Result<CompiledModel, CompileError> {
    let mut diagnostics = Vec::new();
    if ir.ir_version != 1 {
        diagnostics.push(error("E000", format!("unsupported irVersion {}", ir.ir_version), None));
    }

    let mut ids = BTreeSet::new();
    validate_entity_ids(&ir.entities, &mut ids, &mut diagnostics);
    let mut control_ids = Vec::new();
    let mut control_init = Vec::new();
    let mut control_max = Vec::new();
    for var in &ir.state_vars {
        if var.role != StateRole::Control {
            diagnostics.push(error("E008", format!("stat variable '{}' must not be declared; leaf counters are automatic", var.id), var.block_id.clone()));
            continue;
        }
        match var.max {
            None => diagnostics.push(error("E004", format!("control variable '{}' needs max", var.id), var.block_id.clone())),
            Some(max) if var.init < 0 || var.init as u64 > max as u64 =>
                diagnostics.push(error("E004", format!("initial value for '{}' is outside 0..={max}", var.id), var.block_id.clone())),
            Some(max) => {
                control_ids.push(var.id.clone());
                control_init.push(var.init as u32);
                control_max.push(max);
            }
        }
    }
    let mut leaves = Vec::new();
    collect_leaves(&ir.entities, &mut Vec::new(), &mut leaves);
    leaves.push(Leaf { id: "__other__".into(), name: "그외".into(), ancestors: Vec::new() });

    let rule_map: HashMap<_, _> = ir.prob_rules.iter().map(|r| (r.target.as_str(), &r.expr)).collect();
    let mut exact_available = true;
    let entity_defs = match compile_entities(&ir.entities, &rule_map, &leaves, &mut exact_available, &mut diagnostics) {
        Some(v) => v,
        None => Vec::new(),
    };
    for rule in &ir.prob_rules {
        if !ids.contains(&rule.target) {
            diagnostics.push(error("E005", format!("probability rule targets unknown entity '{}'", rule.target), rule.block_id.clone()));
        }
    }

    let control_states = control_max.iter().fold(1u64, |n, max| n.saturating_mul(*max as u64 + 1));
    if control_states > 10_000_000 {
        diagnostics.push(warning("W004", format!("control space {control_states} exceeds precompute limit"), None));
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }

    let trial_dependent = entity_defs.iter().any(entity_trial_dependent);
    let table_trials = if trial_dependent { ir.run.max_trials.max(1) } else { 1 };
    let mut entries = Vec::with_capacity(control_states as usize);
    let mut clamp_events = 0u64;
    for ci in 0..control_states {
        let control = decode_control(ci, &control_max);
        let mut by_trial = Vec::with_capacity(table_trials as usize);
        for trial in 1..=table_trials {
            match calculate_leaf_probs(
                &entity_defs, leaves.len(), &control_ids, &control, trial,
                ir.nesting_policy, &mut clamp_events,
            ) {
                Ok(probs) => by_trial.push(LeafProbs { exact: probs }),
                Err(message) => diagnostics.push(error("E003", message, None)),
            }
        }
        entries.push(by_trial);
    }
    if clamp_events > 0 {
        diagnostics.push(warning("W001", format!("child probabilities were adjusted {clamp_events} time(s)"), None));
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }
    diagnostics.push(info("W002", "the implicit __other__ leaf completes total probability to 1", None));

    let leaf_lookup: HashMap<_, _> = leaves.iter().enumerate().map(|(i, l)| (l.id.as_str(), i)).collect();
    let transitions = compile_transitions(&ir.transitions, &leaves, &control_ids, &mut diagnostics);
    let triggers = compile_triggers(&ir.triggers, &leaves, &leaf_lookup, &control_ids, &mut diagnostics);
    let tracked_leaves = expand_tracked(&ir.run.track_joint, &leaves, &mut diagnostics);
    let mut state_leaf_set: BTreeSet<usize> = tracked_leaves.iter().copied().collect();
    if let Some(condition_expr) = &ir.run.condition {
        let mut variables = Vec::new();
        collect_expr_variables(condition_expr, &mut variables);
        for variable in variables {
            let entity = variable.strip_prefix('n').map(lower_first).unwrap_or(variable);
            for (index, leaf) in leaves.iter().enumerate() {
                if leaf.id == entity || leaf.ancestors.iter().any(|a| a == &entity) {
                    state_leaf_set.insert(index);
                }
            }
        }
    }
    let state_leaves: Vec<_> = state_leaf_set.into_iter().collect();
    let condition = ir.run.condition.as_ref().and_then(|expr| match compile_expr(expr) {
        Ok(p) => Some(p),
        Err(e) => { diagnostics.push(error("E006", e.to_string(), None)); None }
    });
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(CompileError { diagnostics });
    }

    let exact_lcm = entries.iter().flat_map(|v| v.iter()).flat_map(|p| &p.exact)
        .fold(BigInt::one(), |lcm, p| lcm.lcm(p.denom()));
    let l_bits = exact_lcm.bits();
    if l_bits > 64 {
        diagnostics.push(warning("W005", format!("exact common denominator requires {l_bits} bits"), None));
    }
    if l_bits > 128 { exact_available = false; }

    let stat_states = (ir.run.max_trials as u64 + 1)
        .saturating_pow(state_leaves.len().saturating_sub(1) as u32);
    let total_states = control_states.saturating_mul(stat_states);
    let dp_available = control_states <= 10_000_000 && total_states <= 50_000_000;
    let blockers = if dp_available { Vec::new() } else { vec!["estimated state space exceeds DP limit".into()] };
    let analysis = MarkovAnalysis {
        dp_available,
        blockers,
        control_states,
        stat_states,
        total_states,
        est_bytes_per_layer: total_states.saturating_mul(40),
        exact_available,
    };
    Ok(CompiledModel {
        name: ir.name.clone(),
        max_trials: ir.run.max_trials,
        numeric: ir.run.numeric,
        leaves,
        control_ids,
        control_init,
        control_max,
        tracked_leaves,
        state_leaves,
        prob_table: ProbTable { entries, trial_dependent, clamp_events },
        transitions,
        triggers,
        condition,
        diagnostics,
        analysis,
        exact_lcm,
    })
}

fn validate_entity_ids(entities: &[Entity], ids: &mut BTreeSet<String>, diagnostics: &mut Vec<Diagnostic>) {
    for entity in entities {
        if !ids.insert(entity.id.clone()) {
            diagnostics.push(error("E001", format!("duplicate entity id '{}'", entity.id), entity.block_id.clone()));
        }
        validate_entity_ids(&entity.children, ids, diagnostics);
    }
}

fn collect_leaves(entities: &[Entity], ancestors: &mut Vec<String>, leaves: &mut Vec<Leaf>) {
    for entity in entities {
        if entity.children.is_empty() {
            leaves.push(Leaf { id: entity.id.clone(), name: entity.name.clone(), ancestors: ancestors.clone() });
        } else {
            ancestors.push(entity.id.clone());
            collect_leaves(&entity.children, ancestors, leaves);
            ancestors.pop();
            leaves.push(Leaf {
                id: format!("{}__self", entity.id),
                name: format!("{}(전용)", entity.name),
                ancestors: ancestors.iter().cloned().chain(std::iter::once(entity.id.clone())).collect(),
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
        let expr = rules.get(entity.id.as_str()).copied().unwrap_or(&entity.prob);
        let program = match compile_expr(expr) {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(error("E006", e.to_string(), entity.block_id.clone()));
                return None;
            }
        };
        *exact_available &= program.exact_safe;
        let children = compile_entities(&entity.children, rules, leaves, exact_available, diagnostics)?;
        let leaf_indices = leaves.iter().enumerate().filter_map(|(i, leaf)| {
            (leaf.id == entity.id || leaf.ancestors.iter().any(|a| a == &entity.id)).then_some(i)
        }).collect();
        let self_leaf = (!entity.children.is_empty()).then(|| {
            leaves.iter().position(|l| l.id == format!("{}__self", entity.id)).unwrap()
        });
        result.push(EntityDef {
            id: entity.id.clone(), name: entity.name.clone(), program, children, leaf_indices, self_leaf,
        });
    }
    Some(result)
}

fn entity_trial_dependent(entity: &EntityDef) -> bool {
    entity.program.trial_dependent || entity.children.iter().any(entity_trial_dependent)
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
        if p.is_negative() { return Err(format!("probability for '{}' is negative", root.id)); }
        let effective = split_entity(root, p, control_ids, control, trial, policy, &mut out, clamp_events)?;
        top_total += effective;
    }
    if top_total > Rational::one() {
        return Err(format!("top-level probability sum {} exceeds 1", top_total));
    }
    out[leaf_count - 1] = Rational::one() - top_total;
    let sum: Rational = out.iter().cloned().sum();
    if sum != Rational::one() { return Err(format!("leaf probability mass is {sum}, expected 1")); }
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
            child, child_probability, control_ids, control, trial, policy,
            &mut child_out, clamp_events,
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
                    for probability in layer { *probability *= &scale; }
                }
                child_total = parent_p.clone();
            }
            NestingPolicy::ExpandParent => parent_p = child_total.clone(),
            NestingPolicy::Error => return Err(format!("children of '{}' exceed parent probability", entity.id)),
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

fn eval_probability(entity: &EntityDef, ids: &[String], control: &[u32], trial: u32) -> Result<Rational, String> {
    eval(&entity.program, |name| ids.iter().position(|id| id == name)
        .map(|i| Rational::from_integer(control[i].into())), trial)
        .and_then(|v| v.number())
        .map_err(|e| e.to_string())
}

fn decode_control(mut index: u64, maxes: &[u32]) -> Vec<u32> {
    maxes.iter().map(|max| {
        let radix = *max as u64 + 1;
        let value = (index % radix) as u32;
        index /= radix;
        value
    }).collect()
}

fn compile_transitions(
    transitions: &[Transition],
    leaves: &[Leaf],
    control_ids: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<CompiledTransition> {
    transitions.iter().filter_map(|transition| {
        let predicate = compile_predicate(&transition.when, leaves, diagnostics, transition.block_id.clone())?;
        let assignments = compile_assignments(&transition.set, control_ids, diagnostics, transition.block_id.clone());
        Some(CompiledTransition { predicate, assignments })
    }).collect()
}

fn compile_predicate(
    predicate: &LeafPredicate,
    leaves: &[Leaf],
    diagnostics: &mut Vec<Diagnostic>,
    block_id: Option<String>,
) -> Option<CompiledPredicate> {
    Some(match predicate {
        LeafPredicate::LeafOf { leaf_of } => {
            let set: BTreeSet<_> = leaves.iter().enumerate().filter_map(|(i, l)| {
                (l.id == *leaf_of || l.ancestors.iter().any(|a| a == leaf_of)).then_some(i)
            }).collect();
            if set.is_empty() {
                diagnostics.push(error("E006", format!("unknown entity '{leaf_of}' in transition"), block_id));
                return None;
            }
            CompiledPredicate::LeafSet(set)
        }
        LeafPredicate::LeafIs { leaf_is } => {
            let set = leaves.iter().position(|l| l.id == *leaf_is).into_iter().collect();
            CompiledPredicate::LeafSet(set)
        }
        LeafPredicate::Not { not } => CompiledPredicate::Not(Box::new(compile_predicate(not, leaves, diagnostics, block_id)?)),
        LeafPredicate::And { and } => CompiledPredicate::And(and.iter().filter_map(|p| compile_predicate(p, leaves, diagnostics, block_id.clone())).collect()),
        LeafPredicate::Or { or } => CompiledPredicate::Or(or.iter().filter_map(|p| compile_predicate(p, leaves, diagnostics, block_id.clone())).collect()),
    })
}

fn compile_triggers(
    triggers: &[Trigger],
    leaves: &[Leaf],
    lookup: &HashMap<&str, usize>,
    control_ids: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<CompiledTrigger> {
    triggers.iter().filter_map(|trigger| {
        let grant = trigger.grant.as_ref().and_then(|grant| compile_grant(grant, leaves, lookup, diagnostics, trigger.block_id.clone()));
        let assignments = compile_assignments(&trigger.set, control_ids, diagnostics, trigger.block_id.clone());
        Some(CompiledTrigger { trial_count: trigger.at.trial_count, grant, assignments })
    }).collect()
}

fn compile_grant(
    grant: &Grant,
    leaves: &[Leaf],
    lookup: &HashMap<&str, usize>,
    diagnostics: &mut Vec<Diagnostic>,
    block_id: Option<String>,
) -> Option<CompiledGrant> {
    let Some(&leaf) = lookup.get(grant.leaf.as_str()) else {
        let internal = leaves.iter().any(|l| l.ancestors.iter().any(|a| a == &grant.leaf));
        diagnostics.push(error("E007", if internal {
            format!("grant target '{}' is an internal entity", grant.leaf)
        } else { format!("unknown grant leaf '{}'", grant.leaf) }, block_id));
        return None;
    };
    Some(CompiledGrant {
        leaf, amount: grant.amount, consumes_trial: grant.consumes_trial,
        applies_transitions: grant.applies_transitions,
    })
}

fn compile_assignments(
    assignments: &BTreeMap<String, serde_json::Value>,
    control_ids: &[String],
    diagnostics: &mut Vec<Diagnostic>,
    block_id: Option<String>,
) -> Vec<(usize, Program)> {
    assignments.iter().filter_map(|(name, expr)| {
        let Some(index) = control_ids.iter().position(|id| id == name) else {
            diagnostics.push(error("E008", format!("assignment to undeclared/non-control variable '{name}'"), block_id.clone()));
            return None;
        };
        match compile_expr(expr) {
            Ok(program) => Some((index, program)),
            Err(e) => { diagnostics.push(error("E006", e.to_string(), block_id.clone())); None }
        }
    }).collect()
}

fn expand_tracked(names: &[String], leaves: &[Leaf], diagnostics: &mut Vec<Diagnostic>) -> Vec<usize> {
    let mut result = BTreeSet::new();
    for name in names {
        let before = result.len();
        for (i, leaf) in leaves.iter().enumerate() {
            if leaf.id == *name || leaf.ancestors.iter().any(|a| a == name) { result.insert(i); }
        }
        if result.len() == before {
            diagnostics.push(warning("W006", format!("trackJoint item '{name}' matches no entity or leaf"), None));
        }
    }
    result.into_iter().collect()
}

fn collect_expr_variables(expr: &serde_json::Value, out: &mut Vec<String>) {
    match expr {
        serde_json::Value::Object(object) => {
            if let Some(name) = object.get("var").and_then(serde_json::Value::as_str) {
                out.push(name.to_owned());
            }
            for value in object.values() { collect_expr_variables(value, out); }
        }
        serde_json::Value::Array(values) => {
            for value in values { collect_expr_variables(value, out); }
        }
        _ => {}
    }
}

fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    chars.next().map(|c| c.to_lowercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

fn error(code: &str, message: impl Into<String>, block_id: Option<String>) -> Diagnostic {
    Diagnostic { code: code.into(), severity: Severity::Error, message: message.into(), block_id }
}
fn warning(code: &str, message: impl Into<String>, block_id: Option<String>) -> Diagnostic {
    Diagnostic { code: code.into(), severity: Severity::Warning, message: message.into(), block_id }
}
fn info(code: &str, message: impl Into<String>, block_id: Option<String>) -> Diagnostic {
    Diagnostic { code: code.into(), severity: Severity::Info, message: message.into(), block_id }
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
        })).unwrap();
        let model = compile(&ir).unwrap();
        let probs = &model.prob_table.entries[0][0].exact;
        assert_eq!(probs[0], Rational::new(7.into(), 1000.into()));
        assert_eq!(probs[1], Rational::new(23.into(), 1000.into()));
        assert_eq!(probs[2], Rational::new(970.into(), 1000.into()));
    }
}
