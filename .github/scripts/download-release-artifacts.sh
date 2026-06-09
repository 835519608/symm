#!/usr/bin/env bash
set -euo pipefail

error() {
  echo "::error::$*" >&2
}

usage() {
  echo "usage: $0 <artifact-pattern> [dest-dir]" >&2
}

download_and_merge() {
  local pattern="$1"
  local dest="${2:-dist}"
  local run_id="${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
  local repo="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
  SYMM_DOWNLOAD_TMPDIR="$(mktemp -d)"
  trap 'rm -rf "$SYMM_DOWNLOAD_TMPDIR"' EXIT

  gh run download "$run_id" \
    --repo "$repo" \
    --pattern "$pattern" \
    --dir "$SYMM_DOWNLOAD_TMPDIR"

  rm -rf "$dest"
  mkdir -p "$dest"

  local files=()
  while IFS= read -r -d '' file; do
    files+=("$file")
  done < <(find "$SYMM_DOWNLOAD_TMPDIR" -mindepth 2 -maxdepth 2 -type f -print0 | sort -z)

  if [ "${#files[@]}" -eq 0 ]; then
    error "没有下载到匹配 ${pattern} 的临时产物"
    exit 1
  fi

  local file
  for file in "${files[@]}"; do
    local target="${dest}/$(basename "$file")"
    if [ -e "$target" ]; then
      error "临时产物文件名重复：$(basename "$file")"
      exit 1
    fi
    cp "$file" "$target"
  done

  echo "Downloaded artifacts:"
  find "$dest" -maxdepth 1 -type f -printf '%f\n' | sort
}

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  usage
  exit 2
fi

download_and_merge "$@"
