use cairn_arc::{attempt_task, parse_task};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use std::env;
use std::fs;

fn main() {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: cairn-arc-attempt TASK.json");
        std::process::exit(2);
    };
    let result = fs::read(path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| parse_task(&bytes).map_err(|error| format!("{error:?}")))
        .and_then(|task| {
            attempt_task(
                &task,
                InventionLimits {
                    maximum_cost: 9,
                    maximum_candidates: 1_000_000,
                    maximum_expression_nodes: 10_000,
                    execution: ExecLimits::default(),
                },
                ExecLimits::default(),
            )
            .map_err(|error| format!("{error:?}"))
        });
    match result {
        Ok(attempt) => println!("{attempt:#?}"),
        Err(error) => {
            eprintln!("ARC attempt failed: {error}");
            std::process::exit(1);
        }
    }
}
