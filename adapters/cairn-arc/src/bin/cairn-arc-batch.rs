use cairn_arc::run_training_batch;
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use std::env;
use std::path::PathBuf;

fn main() {
    let Some(directory) = env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!(
            "usage: cairn-arc-batch TRAINING_DIRECTORY [TASK_LIMIT] [CANDIDATE_LIMIT] [REPORT.json]"
        );
        std::process::exit(2);
    };
    let task_limit = argument(2, 10_usize);
    let candidate_limit = argument(3, 100_000_u64);
    let report = run_training_batch(
        &directory,
        task_limit,
        InventionLimits {
            maximum_cost: 9,
            maximum_candidates: candidate_limit,
            maximum_expression_nodes: 10_000,
            execution: ExecLimits::default(),
        },
        ExecLimits::default(),
    );
    let output = env::args_os().nth(4).map(PathBuf::from);
    match report.and_then(|report| serde_json::to_string_pretty(&report).map_err(|e| e.to_string()))
    {
        Ok(json) => {
            if let Some(path) = output {
                if let Err(error) = std::fs::write(path, format!("{json}\n")) {
                    eprintln!("ARC report write failed: {error}");
                    std::process::exit(1);
                }
            } else {
                println!("{json}");
            }
        }
        Err(error) => {
            eprintln!("ARC batch failed: {error}");
            std::process::exit(1);
        }
    }
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
