#!/usr/bin/env bash
set -euo pipefail

error() {
  echo "::error::$*" >&2
}

selected_any() {
  $build_windows_x64 || $build_windows_arm64 || $build_windows_x86 ||
    $build_linux_x64 || $build_linux_arm64 ||
    $build_macos_x64 || $build_macos_arm64
}

append_expected_assets() {
  expected_assets=()
  if $build_windows_x64; then
    expected_assets+=("symm-setup-windows-x64.exe" "symm-portable-windows-x64.zip")
  fi
  if $build_windows_arm64; then
    expected_assets+=("symm-portable-windows-arm64.zip")
  fi
  if $build_windows_x86; then
    expected_assets+=("symm-portable-windows-x86.zip")
  fi
  if $build_linux_x64; then
    expected_assets+=("symm-portable-linux-x64.zip")
  fi
  if $build_linux_arm64; then
    expected_assets+=("symm-portable-linux-arm64.zip")
  fi
  if $build_macos_x64; then
    expected_assets+=("symm-portable-macos-x64.zip")
  fi
  if $build_macos_arm64; then
    expected_assets+=("symm-portable-macos-arm64.zip")
  fi
}

write_outputs() {
  local output_file="${GITHUB_OUTPUT:-/dev/stdout}"
  {
    echo "release_tag=${RELEASE_TAG}"
    echo "build_windows_x64=${build_windows_x64}"
    echo "build_windows_arm64=${build_windows_arm64}"
    echo "build_windows_x86=${build_windows_x86}"
    echo "build_linux_x64=${build_linux_x64}"
    echo "build_linux_arm64=${build_linux_arm64}"
    echo "build_macos_x64=${build_macos_x64}"
    echo "build_macos_arm64=${build_macos_arm64}"
    echo "expected_assets<<ASSETS_EOF"
    printf '%s\n' "${expected_assets[@]}"
    echo "ASSETS_EOF"
    echo "expected_assets_with_checksums<<ASSETS_WITH_CHECKSUMS_EOF"
    printf '%s\n' "${expected_assets[@]}"
    echo "SHA256SUMS"
    echo "ASSETS_WITH_CHECKSUMS_EOF"
  } >> "$output_file"
}

plan() {
  if [ "${SYMM_EVENT_NAME}" = "push" ]; then
    RELEASE_TAG="${SYMM_REF_NAME}"
  else
    RELEASE_TAG="test-run-${SYMM_RUN_ID}"
  fi

  build_windows_x64=false
  build_windows_arm64=false
  build_windows_x86=false
  build_linux_x64=false
  build_linux_arm64=false
  build_macos_x64=false
  build_macos_arm64=false

  if [ "${SYMM_EVENT_NAME}" = "workflow_dispatch" ]; then
    [ "${INPUT_BUILD_WINDOWS_X64:-false}" = "true" ] && build_windows_x64=true
    [ "${INPUT_BUILD_WINDOWS_ARM64:-false}" = "true" ] && build_windows_arm64=true
    [ "${INPUT_BUILD_WINDOWS_X86:-false}" = "true" ] && build_windows_x86=true
    [ "${INPUT_BUILD_LINUX_X64:-false}" = "true" ] && build_linux_x64=true
    [ "${INPUT_BUILD_LINUX_ARM64:-false}" = "true" ] && build_linux_arm64=true
    [ "${INPUT_BUILD_MACOS_X64:-false}" = "true" ] && build_macos_x64=true
    [ "${INPUT_BUILD_MACOS_ARM64:-false}" = "true" ] && build_macos_arm64=true
  elif [[ "${RELEASE_TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-test[0-9] ]]; then
    error "测试 tag 格式无效：${RELEASE_TAG}"
    echo "勿在 test 后加数字（如 v0.1.0-test15）；应使用 vX.Y.Z-test 或 vX.Y.Z-test-windows" >&2
    exit 1
  elif [[ "${RELEASE_TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-test$ ]]; then
    build_windows_x64=true
    build_windows_arm64=true
    build_windows_x86=true
    build_linux_x64=true
    build_linux_arm64=true
    build_macos_x64=true
    build_macos_arm64=true
  elif [[ "${RELEASE_TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-test-(.+)$ ]]; then
    suffix="${BASH_REMATCH[1]}"
    IFS='-' read -ra parts <<< "${suffix}"
    for part in "${parts[@]}"; do
      case "${part}" in
        windows|win)
          build_windows_x64=true
          build_windows_arm64=true
          build_windows_x86=true
          ;;
        linux)
          build_linux_x64=true
          build_linux_arm64=true
          ;;
        macos|mac)
          build_macos_x64=true
          build_macos_arm64=true
          ;;
        *)
          error "未知平台段「${part}」（tag: ${RELEASE_TAG}）。期望形如 v0.1.0-test-windows 或 v0.1.0-test-windows-linux"
          exit 1
          ;;
      esac
    done
  elif [[ "${RELEASE_TAG}" =~ ^test-run- ]]; then
    build_windows_x64=true
    build_linux_x64=true
    build_macos_x64=true
  else
    error "测试 tag 格式无效：${RELEASE_TAG}"
    echo "期望：vX.Y.Z-test 或 vX.Y.Z-test-<platform(s)>（如 v0.1.1-test-windows）" >&2
    exit 1
  fi

  if ! selected_any; then
    error "至少选择一个目标包"
    exit 1
  fi

  append_expected_assets
  write_outputs

  echo "Release tag: ${RELEASE_TAG}"
  echo "Targets: windows-x64=${build_windows_x64} windows-arm64=${build_windows_arm64} windows-x86=${build_windows_x86} linux-x64=${build_linux_x64} linux-arm64=${build_linux_arm64} macos-x64=${build_macos_x64} macos-arm64=${build_macos_arm64}"
}

run_plan_case() {
  local name="$1"
  local expected_status="$2"
  shift 2
  local output
  local stdout
  local stderr
  output="$(mktemp)"
  stdout="$(mktemp)"
  stderr="$(mktemp)"
  set +e
  env GITHUB_OUTPUT="$output" "$@" bash "$0" >"$stdout" 2>"$stderr"
  local status=$?
  set -e
  if [ "$status" != "$expected_status" ]; then
    echo "self-test ${name} failed: expected status ${expected_status}, got ${status}" >&2
    cat "$stdout" >&2
    cat "$stderr" >&2
    exit 1
  fi
  CASE_OUTPUT="$output"
  CASE_STDERR="$stderr"
}

require_output_line() {
  local file="$1"
  local line="$2"
  if ! grep -Fxq "$line" "$file"; then
    echo "missing output line: $line" >&2
    cat "$file" >&2
    exit 1
  fi
}

self_test() {
  run_plan_case full 0 SYMM_EVENT_NAME=push SYMM_REF_NAME=v1.2.3-test SYMM_RUN_ID=1
  require_output_line "$CASE_OUTPUT" "build_windows_x64=true"
  require_output_line "$CASE_OUTPUT" "build_macos_arm64=true"
  require_output_line "$CASE_OUTPUT" "symm-portable-macos-arm64.zip"

  run_plan_case combo 0 SYMM_EVENT_NAME=push SYMM_REF_NAME=v1.2.3-test-win-linux SYMM_RUN_ID=2
  require_output_line "$CASE_OUTPUT" "build_windows_x86=true"
  require_output_line "$CASE_OUTPUT" "build_linux_arm64=true"
  require_output_line "$CASE_OUTPUT" "build_macos_x64=false"

  run_plan_case alias 0 SYMM_EVENT_NAME=push SYMM_REF_NAME=v1.2.3-test-mac SYMM_RUN_ID=3
  require_output_line "$CASE_OUTPUT" "build_macos_x64=true"
  require_output_line "$CASE_OUTPUT" "build_windows_x64=false"

  run_plan_case dispatch 0 SYMM_EVENT_NAME=workflow_dispatch SYMM_REF_NAME=master SYMM_RUN_ID=42 INPUT_BUILD_WINDOWS_X64=true INPUT_BUILD_LINUX_X64=true INPUT_BUILD_MACOS_X64=false
  require_output_line "$CASE_OUTPUT" "release_tag=test-run-42"
  require_output_line "$CASE_OUTPUT" "symm-setup-windows-x64.exe"
  require_output_line "$CASE_OUTPUT" "symm-portable-linux-x64.zip"

  run_plan_case invalid-test-number 1 SYMM_EVENT_NAME=push SYMM_REF_NAME=v1.2.3-test15 SYMM_RUN_ID=4
  if ! grep -Fq "勿在 test 后加数字" "$CASE_STDERR"; then
    echo "invalid test number did not report the expected message" >&2
    cat "$CASE_STDERR" >&2
    exit 1
  fi

  run_plan_case dispatch-none 1 SYMM_EVENT_NAME=workflow_dispatch SYMM_REF_NAME=master SYMM_RUN_ID=43
  if ! grep -Fq "至少选择一个目标包" "$CASE_STDERR"; then
    echo "dispatch without selected targets did not fail with the expected message" >&2
    cat "$CASE_STDERR" >&2
    exit 1
  fi

  echo "plan-test-release self-test passed"
}

if [ "${1:-}" = "--self-test" ]; then
  self_test
else
  plan
fi
