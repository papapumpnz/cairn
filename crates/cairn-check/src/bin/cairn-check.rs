use cairn_check::{check_repository, CheckOptions};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("cairn-check: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(arguments: impl Iterator<Item = String>) -> Result<ExitCode, String> {
    let mut path = None;
    let mut json = false;
    let mut positional = Vec::new();
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--path" => {
                path =
                    Some(PathBuf::from(arguments.next().ok_or_else(|| {
                        "--path requires a repository path".to_owned()
                    })?));
            }
            "--json" => json = true,
            "--help" | "-h" => {
                print_usage();
                return Ok(ExitCode::SUCCESS);
            }
            "check" => {}
            _ if argument.starts_with('-') => {
                return Err(format!("unknown option `{argument}`"));
            }
            _ => positional.push(argument),
        }
    }
    if path.is_none() {
        path = positional.first().map(PathBuf::from);
    }
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    let report = check_repository(&CheckOptions::new(path)).map_err(|error| error.to_string())?;
    if json {
        let output = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
        println!("{output}");
    } else if report.is_clean() {
        println!(
            "CAIRN policy PASS: {} files checked in {} ms",
            report.scanned_files, report.duration_ms
        );
    } else {
        for violation in &report.violations {
            let line = violation
                .line
                .map_or_else(String::new, |value| format!(":{value}"));
            println!(
                "{:?} {}{}: {}",
                violation.code, violation.path, line, violation.message
            );
        }
        println!(
            "CAIRN policy FAIL: {} violation(s) in {} ms",
            report.violations.len(),
            report.duration_ms
        );
    }
    Ok(if report.is_clean() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn print_usage() {
    println!("Usage: cairn-check [check] [PATH] [--path PATH] [--json]");
}
