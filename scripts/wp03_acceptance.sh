#!/usr/bin/env bash
set -euo pipefail

output_dir="${1:-artifacts/wp03-acceptance}"
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

printf '%s\n' '$ cargo run --release --locked --quiet -p cairn-reg --bin cairn-wp03-bench' | tee -a "$log"
cargo run --release --locked --quiet -p cairn-reg --bin cairn-wp03-bench \
  | tee "$output_dir/benchmark-result.json" | tee -a "$log" >/dev/null

printf '%s\n' '$ cargo run --locked --quiet -p cairn-check -- check . --json' | tee -a "$log"
cargo run --locked --quiet -p cairn-check -- check . --json \
  > "$output_dir/check-report.json"

printf '%s\n' '$ cargo run --locked --quiet -p cairn-check --bin cairn-attest -- --out <evidence>/runtime-attestation.json --path .' | tee -a "$log"
cargo run --locked --quiet -p cairn-check --bin cairn-attest -- \
  --out "$output_dir/runtime-attestation.json" --path . | tee -a "$log"

run bash scripts/validate_schemas.sh 2>&1 | tee -a "$log"
run bash scripts/handoff_manifest.sh --check 2>&1 | tee -a "$log"

jq -e '
  .nodes == 2000 and
  .facts == 10000 and
  .probe_iterations == 10000000 and
  .indexed_probes_per_second > 10000000 and
  .median_two_join_ns < 50000 and
  .rss_kib < 20480
' "$output_dir/benchmark-result.json" >/dev/null
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
  --arg work_package "WP03" \
  --arg semantics_version "1.0.0" \
  --arg git_commit "$git_commit" \
  --argjson started_at_unix "$start_epoch" \
  --argjson completed_at_unix "$end_epoch" \
  --slurpfile benchmark "$output_dir/benchmark-result.json" \
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
    benchmark: $benchmark[0],
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
      "typed_nodes_relations_and_hyperedges",
      "append_only_acyclic_derivations",
      "node_fact_value_and_temporal_indexes",
      "deterministic_conjunctive_joins",
      "bounded_paths_filters_and_aggregates",
      "snapshot_concurrency_and_extension_isolation",
      "one_hundred_random_graph_reference_comparisons",
      "canonical_decode_reencode_and_index_reconstruction",
      "hostile_decode_limits_and_semantic_order_rejection",
      "ten_million_indexed_probes",
      "workspace_policy_schemas_and_handoff_integrity"
    ],
    declared_limits: [
      "snapshot extension currently clones the immutable base before append",
      "specialized dense binary adjacency is deferred because sparse index gates pass",
      "hosted CI not exercised in this workspace"
    ]
  }' > "$output_dir/acceptance-summary.json"

sha256sum \
  "$output_dir/acceptance-summary.json" \
  "$output_dir/benchmark-result.json" \
  "$output_dir/check-report.json" \
  "$output_dir/runtime-attestation.json" \
  > "$output_dir/SHA256SUMS"

printf 'WP03 acceptance PASS; evidence: %s\n' "$output_dir" | tee -a "$log"
