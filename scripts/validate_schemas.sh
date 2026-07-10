#!/usr/bin/env bash
set -euo pipefail

for schema in schemas/*.json docs/CAIRN_handoff/schemas/*.json; do
  jq -e 'type == "object" and has("$schema")' "$schema" >/dev/null
done

schema_python=""
for candidate in python3 python3.11 /usr/bin/python3.11; do
  if command -v "$candidate" >/dev/null 2>&1 && "$candidate" -c 'import jsonschema' >/dev/null 2>&1; then
    schema_python="$candidate"
    break
  fi
done
if [[ -n "$schema_python" ]]; then
  "$schema_python" - <<'PY'
import json
from pathlib import Path
from jsonschema import Draft202012Validator

for pattern in ("schemas/*.json", "docs/CAIRN_handoff/schemas/*.json"):
    for path in sorted(Path().glob(pattern)):
        Draft202012Validator.check_schema(json.loads(path.read_text()))
PY
fi

attestation="$(mktemp)"
check_report="$(mktemp)"
trap 'rm -f "$attestation" "$check_report"' EXIT
cargo run --locked --quiet -p cairn-check --bin cairn-attest -- --out "$attestation" --path .
cargo run --locked --quiet -p cairn-check -- check . --json > "$check_report"
jq -e '
  .schema_version == 1 and
  (.logical_cpu_count | type == "number") and
  (.process.pid | type == "number") and
  (.process.tree | type == "array" and length > 0) and
  (.loaded_native_libraries | type == "array") and
  (.prohibited_runtime_findings | type == "array") and
  (.network.interfaces | type == "array") and
  (.build | has("cargo_lock_blake3"))
' "$attestation" >/dev/null
jq -e '
  .schema_version == 1 and
  (.root | type == "string") and
  (.scanned_files | type == "number") and
  (.violations | type == "array" and length == 0)
' "$check_report" >/dev/null

if command -v check-jsonschema >/dev/null 2>&1; then
  check-jsonschema --schemafile schemas/runtime-attestation.schema.json "$attestation"
  check-jsonschema --schemafile schemas/check-report.schema.json "$check_report"
elif command -v jsonschema >/dev/null 2>&1; then
  jsonschema -i "$attestation" schemas/runtime-attestation.schema.json
  jsonschema -i "$check_report" schemas/check-report.schema.json
fi
