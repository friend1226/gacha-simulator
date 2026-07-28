pub mod compile;
pub mod engine_dp;
pub mod engine_exact;
pub mod engine_mc;
pub mod expr;
pub mod ir;
pub mod numeric;
pub mod rational;
pub mod report;
pub mod state;
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot;

pub use compile::{compile, CompiledModel, Diagnostic, Severity};
pub use engine_dp::{run_dp, DpOptions, DpResult, DpRunResult};
#[cfg(not(target_arch = "wasm32"))]
pub use engine_dp::{restore_dp_snapshot, run_dp_with_snapshots, SnapshotRunError};
pub use engine_exact::{
    run_exact, ExactFirstHitResult, ExactOptions, ExactProbability, ExactResult,
};
pub use engine_mc::{run_mc, McOptions, McResult};
pub use ir::ModelIr;
#[cfg(not(target_arch = "wasm32"))]
pub use snapshot::{
    load_snapshot, LoadedSnapshot, SnapshotError, SnapshotHeader, SnapshotManifest,
    SnapshotOptions, SnapshotPolicy,
};
