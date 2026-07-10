#!/usr/bin/env bash
set -euo pipefail

workspace="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="$workspace/docs/CAIRN_handoff"
manifest="$root/manifest.json"
digest_file="$root/manifest.sha256"

collect_paths() {
  find "$root" -type f \
    ! -path "$manifest" \
    ! -path "$digest_file" \
    -printf '%P\n' | LC_ALL=C sort
}

check_manifest() {
  (cd "$root" && sha256sum --check "$(basename "$digest_file")")

  local expected actual
  expected="$(mktemp)"
  actual="$(mktemp)"
  jq -r '.files[].path' "$manifest" | LC_ALL=C sort > "$expected"
  collect_paths > "$actual"
  diff -u "$expected" "$actual"

  while IFS=$'\t' read -r relative bytes digest; do
    test "$(stat -c '%s' "$root/$relative")" = "$bytes"
    test "$(sha256sum "$root/$relative" | cut -d ' ' -f 1)" = "$digest"
  done < <(jq -r '.files[] | [.path, (.bytes | tostring), .sha256] | @tsv' "$manifest")

  test "$(jq '.files | length' "$manifest")" = "$(jq '.file_count' "$manifest")"
  test "$(jq '[.files[].bytes] | add // 0' "$manifest")" = "$(jq '.total_bytes' "$manifest")"
  echo "handoff manifest: verified $(jq '.file_count' "$manifest") files"
  rm -f "$expected" "$actual"
}

regenerate_manifest() {
  local records output source_archives generated_utc
  records="$(mktemp)"
  output="$(mktemp)"
  source_archives="$(jq -c '.source_archives' "$manifest")"
  generated_utc="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"

  while IFS= read -r relative; do
    jq -cn \
      --arg path "$relative" \
      --argjson bytes "$(stat -c '%s' "$root/$relative")" \
      --arg sha256 "$(sha256sum "$root/$relative" | cut -d ' ' -f 1)" \
      '{path: $path, bytes: $bytes, sha256: $sha256}' >> "$records"
  done < <(collect_paths)

  jq -n \
    --arg generated_utc "$generated_utc" \
    --argjson source_archives "$source_archives" \
    --slurpfile files "$records" \
    '{
      schema_version: "1.0",
      bundle: "CAIRN_ARCHITECTURE_HANDOFF",
      architecture: "CAIRN — Causal Analogical Induction over Relational Networks",
      generated_utc: $generated_utc,
      hash_algorithm: "SHA-256",
      manifest_excludes: ["manifest.json", "manifest.sha256"],
      file_count: ($files | length),
      total_bytes: ([$files[].bytes] | add // 0),
      source_archives: $source_archives,
      files: $files
    }' > "$output"
  mv "$output" "$manifest"
  (cd "$root" && sha256sum "$(basename "$manifest")" > "$(basename "$digest_file")")
  rm -f "$records"
  check_manifest
}

case "${1:---regenerate}" in
  --check) check_manifest ;;
  --regenerate) regenerate_manifest ;;
  *) echo "usage: $0 [--regenerate|--check]" >&2; exit 2 ;;
esac
