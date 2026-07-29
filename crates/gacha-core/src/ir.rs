use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub type Expr = Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelIr {
    pub ir_version: u32,
    pub name: String,
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub nesting_policy: NestingPolicy,
    #[serde(default)]
    pub state_vars: Vec<StateVar>,
    #[serde(default)]
    pub prob_rules: Vec<ProbRule>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    pub run: RunConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub prob: Expr,
    #[serde(default)]
    pub children: Vec<Entity>,
    #[serde(default)]
    pub block_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NestingPolicy {
    #[default]
    ClampChildren,
    ExpandParent,
    ScaleSiblings,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateVar {
    pub id: String,
    pub init: i64,
    pub max: Option<u32>,
    pub role: StateRole,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub update: Vec<AccumulatorUpdate>,
    #[serde(default)]
    pub clamp_policy: ClampPolicy,
    #[serde(default)]
    pub block_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StateRole {
    Control,
    Stat,
    Accumulator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccumulatorUpdate {
    pub when: LeafPredicate,
    pub set: Expr,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClampPolicy {
    #[default]
    Saturate,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbRule {
    pub target: String,
    pub expr: Expr,
    #[serde(default)]
    pub block_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub when: LeafPredicate,
    pub set: BTreeMap<String, Expr>,
    #[serde(default)]
    pub block_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LeafPredicate {
    LeafOf {
        #[serde(rename = "leafOf")]
        leaf_of: String,
    },
    LeafIs {
        #[serde(rename = "leafIs")]
        leaf_is: String,
    },
    Not {
        not: Box<LeafPredicate>,
    },
    And {
        and: Vec<LeafPredicate>,
    },
    Or {
        or: Vec<LeafPredicate>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trigger {
    pub at: TriggerAt,
    #[serde(default)]
    pub grant: Option<Grant>,
    #[serde(default)]
    pub set: BTreeMap<String, Expr>,
    #[serde(default)]
    pub block_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerAt {
    pub trial_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Grant {
    pub leaf: String,
    #[serde(default = "one")]
    pub amount: u32,
    #[serde(default)]
    pub consumes_trial: bool,
    #[serde(default = "default_true")]
    pub applies_transitions: bool,
}

fn one() -> u32 {
    1
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    pub max_trials: u32,
    #[serde(default)]
    pub track_joint: Vec<String>,
    #[serde(default)]
    pub numeric: NumericBackend,
    #[serde(default)]
    pub condition: Option<Expr>,
    #[serde(default)]
    pub trial_series: Option<TrialSeriesMode>,
    #[serde(default)]
    pub series_checkpoints: Vec<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrialSeriesMode {
    None,
    Marginal,
    Checkpoints,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NumericBackend {
    F64,
    #[default]
    Scaled,
    Exact,
}
