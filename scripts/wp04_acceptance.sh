#!/usr/bin/env bash
set -euo pipefail

output_dir="${1:-artifacts/wp04-acceptance}"
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
  run cargo doc --locked --workspace --no-deps
} 2>&1 | tee "$log"

printf '%s\n' '$ timed 100,000-term RKL property test' | tee -a "$log"
timing="$output_dir/generator-timing.txt"
/usr/bin/time -f '%e %M' -o "$timing" \
  cargo test --locked --quiet -p cairn-rkl --test rkl \
  one_hundred_thousand_generated_terms_are_stable_and_in_bounds -- --exact \
  2>&1 | tee -a "$log"
read -r elapsed_seconds peak_rss_kib < "$timing"
jq -n \
  --argjson generated_terms 100000 \
  --argjson failures 0 \
  --argjson elapsed_seconds "$elapsed_seconds" \
  --argjson peak_rss_kib "$peak_rss_kib" \
  '{
    generated_terms: $generated_terms,
    failures: $failures,
    elapsed_seconds: $elapsed_seconds,
    peak_rss_kib: $peak_rss_kib
  }' > "$output_dir/generator-result.json"

printf '%s\n' '$ cargo run --locked --quiet -p cairn-check -- check . --json' | tee -a "$log"
cargo run --locked --quiet -p cairn-check -- check . --json \
  > "$output_dir/check-report.json"

printf '%s\n' '$ cargo run --locked --quiet -p cairn-check --bin cairn-attest -- --out <evidence>/runtime-attestation.json --path .' | tee -a "$log"
cargo run --locked --quiet -p cairn-check --bin cairn-attest -- \
  --out "$output_dir/runtime-attestation.json" --path . | tee -a "$log"

run bash scripts/validate_schemas.sh 2>&1 | tee -a "$log"
run bash scripts/handoff_manifest.sh --check 2>&1 | tee -a "$log"

jq -e '.generated_terms == 100000 and .failures == 0' \
  "$output_dir/generator-result.json" >/dev/null
jq -e '.violations | length == 0' "$output_dir/check-report.json" >/dev/null
jq -e '.prohibited_runtime_findings | length == 0' \
  "$output_dir/runtime-attestation.json" >/dev/null

end_epoch="$(date +%s)"
git_commit="$(git rev-parse HEAD 2>/dev/null || true)"
if [[ -z "$git_commit" ]]; then
  git_commit="unavailable"
fi

jq -n \
  --arg status "PASS" \
  --arg work_package "WP04" \
  --arg semantics_version "1.0.0" \
  --arg git_commit "$git_commit" \
  --argjson started_at_unix "$start_epoch" \
  --argjson completed_at_unix "$end_epoch" \
  --slurpfile generator "$output_dir/generator-result.json" \
  --slurpfile check "$output_dir/check-report.json" \
  --slurpfile attest "$output_dir/runtime-attestation.json" \
  '{
    schema_version: 1,
    work_package: $work_package,
    status: $status,
    semantics_version: $semantics_version,
    git_commit: $git_commit,
    started_at_unix: $started_at_unix,
    completed_at_unix: $completed_at_unix,
    generated_term_check: $generator[0],
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
      "typed_expression_and_theory_admission",
      "integer_range_and_collection_bounds",
      "explicit_recursion_and_fixpoint_fuel",
      "alpha_equivalent_expression_and_theory_hashes",
      "canonical_cbor_golden",
      "rule_binder_and_schedule_validation",
      "proof_annotations_excluded_from_semantic_identity",
      "one_hundred_thousand_generated_terms",
      "workspace_policy_schemas_and_handoff_integrity"
    ],
    declared_limits: [
      "WP04 defines and validates semantics; execution belongs to WP05",
      "mutual recursion is rejected; bounded direct recursion and bounded fixpoints are admitted",
      "hosted CI not exercised in this workspace"
    ]
  }' > "$output_dir/acceptance-summary.json"

sha256sum \
  "$output_dir/acceptance-summary.json" \
  "$output_dir/generator-result.json" \
  "$output_dir/check-report.json" \
  "$output_dir/runtime-attestation.json" \
  > "$output_dir/SHA256SUMS"

printf 'WP04 acceptance PASS; evidence: %s\n' "$output_dir" | tee -a "$log"
