use cairn_check::{collect_attestation, write_attestation};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("cairn-attest: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let mut output = None;
    let mut repository = PathBuf::from(".");
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--out" => {
                output = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--out requires a JSON path".to_owned())?,
                ));
            }
            "--path" => {
                repository = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--path requires a repository path".to_owned())?,
                );
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            _ => return Err(format!("unknown argument `{argument}`")),
        }
    }
    let output = output.ok_or_else(|| "--out is required".to_owned())?;
    let attestation = collect_attestation(&repository).map_err(|error| error.to_string())?;
    write_attestation(&output, &attestation).map_err(|error| error.to_string())?;
    println!("wrote runtime attestation to {}", output.display());
    Ok(())
}

fn print_usage() {
    println!("Usage: cairn-attest --out PATH [--path REPOSITORY]");
}
