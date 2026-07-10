#!/usr/bin/env bash
set -euo pipefail

output_dir="${1:-artifacts/wp00-acceptance}"
if [[ -e "$output_dir" ]]; then
  echo "acceptance output already exists: $output_dir" >&2
  exit 2
fi
mkdir -p "$output_dir"

log="$output_dir/commands.log"
start_epoch="$(date +%s)"

run() {
  printf '$'
  printf ' %q' "$@"
  printf '\n'
  "$@"
}

{
  run cargo fmt --all -- --check
  run cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
  run cargo test --locked --workspace --all-targets --all-features
  run cargo test --locked --workspace --doc
} 2>&1 | tee "$log"

printf '%s\n' '$ cargo run --locked --quiet -p cairn-check -- check . --json' | tee -a "$log"
cargo run --locked --quiet -p cairn-check -- check . --json \
  > "$output_dir/check-report.json"

printf '%s\n' '$ cargo run --locked --quiet -p cairn-check --bin cairn-attest -- --out <evidence>/runtime-attestation.json --path .' | tee -a "$log"
cargo run --locked --quiet -p cairn-check --bin cairn-attest -- \
  --out "$output_dir/runtime-attestation.json" --path . | tee -a "$log"

printf '%s\n' '$ bash scripts/validate_schemas.sh' | tee -a "$log"
bash scripts/validate_schemas.sh 2>&1 | tee -a "$log"

jq -e '.violations | length == 0' "$output_dir/check-report.json" >/dev/null
jq -e '.duration_ms < 5000' "$output_dir/check-report.json" >/dev/null
jq -e '.prohibited_runtime_findings | length == 0' \
  "$output_dir/runtime-attestation.json" >/dev/null

end_epoch="$(date +%s)"
git_commit="$(git rev-parse HEAD 2>/dev/null || true)"
if [[ -z "$git_commit" ]]; then
  git_commit="unavailable"
fi

jq -n \
  --arg status "PASS" \
  --arg work_package "WP00" \
  --arg git_commit "$git_commit" \
  --argjson started_at_unix "$start_epoch" \
  --argjson completed_at_unix "$end_epoch" \
  --slurpfile check "$output_dir/check-report.json" \
  --slurpfile attest "$output_dir/runtime-attestation.json" \
  '{
    schema_version: 1,
    work_package: $work_package,
    status: $status,
    git_commit: $git_commit,
    started_at_unix: $started_at_unix,
    completed_at_unix: $completed_at_unix,
    checker: {
      scanned_files: $check[0].scanned_files,
      duration_ms: $check[0].duration_ms,
      violations: ($check[0].violations | length)
    },
    runtime: {
      cpu_model: $attest[0].cpu_model,
      physical_cpu_count: $attest[0].physical_cpu_count,
      logical_cpu_count: $attest[0].logical_cpu_count,
      memory_total_bytes: $attest[0].memory_total_bytes,
      prohibited_findings: ($attest[0].prohibited_runtime_findings | length)
    },
    checks: [
      "format",
      "clippy_warnings_denied",
      "all_target_tests",
      "doc_tests",
      "repository_policy",
      "runtime_attestation",
      "schema_validation",
      "checker_under_5_seconds"
    ]
  }' > "$output_dir/acceptance-summary.json"

sha256sum \
  "$output_dir/acceptance-summary.json" \
  "$output_dir/check-report.json" \
  "$output_dir/runtime-attestation.json" \
  > "$output_dir/SHA256SUMS"

printf 'WP00 acceptance PASS; evidence: %s\n' "$output_dir" | tee -a "$log"

