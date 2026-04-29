#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-artifacts}"
mkdir -p "${OUT_DIR}"

REPORT_MD="${OUT_DIR}/rollback-metrics.md"
REPORT_JSON="${OUT_DIR}/rollback-metrics.json"

TESTS=(
  "add_name_conflict_rolls_back_created_link"
  "add_when_link_is_locked_and_user_cancels_fails_before_mutation"
  "add_when_link_is_locked_and_unlock_still_leaves_locks_fails"
)

total=0
passed=0

{
  echo "# Rollback Metrics"
  echo
  echo "- generated_at_utc: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "- scope: tests/cli_flow.rs rollback scenarios"
  echo
  echo "## Cases"
} > "${REPORT_MD}"

printf '{\n  "generated_at_utc": "%s",\n  "scope": "tests/cli_flow.rs rollback scenarios",\n  "cases": [\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" > "${REPORT_JSON}"

first=1
for t in "${TESTS[@]}"; do
  total=$((total + 1))
  if cargo test --test cli_flow "${t}" -- --exact; then
    status="pass"
    passed=$((passed + 1))
  else
    status="fail"
  fi

  echo "- \`${t}\`: ${status}" >> "${REPORT_MD}"

  if [[ ${first} -eq 0 ]]; then
    printf ',\n' >> "${REPORT_JSON}"
  fi
  first=0
  printf '    {"name":"%s","status":"%s"}' "${t}" "${status}" >> "${REPORT_JSON}"
done

failed=$((total - passed))
rate=$((passed * 100 / total))

{
  echo
  echo "## Summary"
  echo
  echo "- total: ${total}"
  echo "- passed: ${passed}"
  echo "- failed: ${failed}"
  echo "- rollback_success_rate_pct: ${rate}"
} >> "${REPORT_MD}"

{
  printf '\n  ],\n'
  printf '  "summary": {\n'
  printf '    "total": %d,\n' "${total}"
  printf '    "passed": %d,\n' "${passed}"
  printf '    "failed": %d,\n' "${failed}"
  printf '    "rollback_success_rate_pct": %d\n' "${rate}"
  printf '  }\n'
  printf '}\n'
} >> "${REPORT_JSON}"

echo "[rollback-metrics] report generated at ${OUT_DIR}"
