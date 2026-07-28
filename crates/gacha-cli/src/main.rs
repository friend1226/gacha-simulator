use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use gacha_core::{
    compile, run_dp, run_dp_with_snapshots, run_exact, run_mc, DpOptions, ExactOptions,
    McOptions, ModelIr, SnapshotOptions, SnapshotPolicy,
};
use std::{collections::BTreeSet, fs, path::PathBuf};

#[derive(Parser)]
#[command(name = "gacha", version, about = "Exact and Monte Carlo gacha probability simulator")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Validate { model: PathBuf },
    Dp {
        model: PathBuf,
        #[arg(long)]
        no_prune: bool,
    },
    Exact {
        model: PathBuf,
        #[arg(long)]
        reduce: bool,
    },
    Mc {
        model: PathBuf,
        #[arg(long, default_value_t = 100_000)]
        runs: u64,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    Snapshot {
        model: PathBuf,
        output: PathBuf,
        #[arg(long, value_enum, default_value_t = SnapshotPolicyArg::Aggregate)]
        policy: SnapshotPolicyArg,
        #[arg(long)]
        pin: Vec<u32>,
        #[arg(long)]
        confirm_full: bool,
        #[arg(long)]
        no_prune: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum SnapshotPolicyArg { Aggregate, Checkpoint, Full }

impl From<SnapshotPolicyArg> for SnapshotPolicy {
    fn from(value: SnapshotPolicyArg) -> Self {
        match value {
            SnapshotPolicyArg::Aggregate => Self::Aggregate,
            SnapshotPolicyArg::Checkpoint => Self::Checkpoint,
            SnapshotPolicyArg::Full => Self::Full,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { model } => {
            let compiled = load(&model)?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "name": compiled.name,
                "leaves": compiled.leaves,
                "diagnostics": compiled.diagnostics,
                "analysis": compiled.analysis,
                "exactCommonDenominator": compiled.exact_lcm.to_string(),
            }))?);
        }
        Command::Dp { model, no_prune } => {
            let compiled = load(&model)?;
            let result = run_dp(
                &compiled,
                DpOptions { prune_log10: (!no_prune).then_some(-18.0) },
                |done, total| {
                    if done == total || done % 100 == 0 { eprintln!("DP {done}/{total}"); }
                    true
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Exact { model, reduce } => {
            let compiled = load(&model)?;
            let result = run_exact(
                &compiled,
                ExactOptions { reduce_layers: reduce, ..Default::default() },
                |done, total| {
                    if done == total || done % 25 == 0 { eprintln!("Exact DP {done}/{total}"); }
                    true
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Mc { model, runs, seed } => {
            let compiled = load(&model)?;
            let result = run_mc(
                &compiled,
                McOptions { runs, seed, ..Default::default() },
                |done, total| {
                    if done == total || done % 100_000 == 0 { eprintln!("MC {done}/{total}"); }
                    true
                },
            );
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Snapshot { model, output, policy, pin, confirm_full, no_prune } => {
            let compiled = load(&model)?;
            let (result, manifest) = run_dp_with_snapshots(
                &compiled,
                DpOptions { prune_log10: (!no_prune).then_some(-18.0) },
                SnapshotOptions {
                    output_dir: output,
                    policy: policy.into(),
                    pinned_layers: pin.into_iter().collect::<BTreeSet<_>>(),
                    confirm_full,
                },
                |done, total| {
                    if done == total || done % 100 == 0 { eprintln!("Snapshot DP {done}/{total}"); }
                    true
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "result": result,
                "snapshot": manifest,
            }))?);
        }
    }
    Ok(())
}

fn load(path: &PathBuf) -> Result<gacha_core::CompiledModel> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let ir: ModelIr = serde_json::from_str(&source)
        .with_context(|| format!("invalid Model IR JSON in {}", path.display()))?;
    compile(&ir).map_err(|error| {
        anyhow::anyhow!(
            "{}",
            error.diagnostics.iter()
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect::<Vec<_>>().join("\n")
        )
    })
}
