#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/assets/fonts/NotoSansSC-Regular.otf"
url="https://raw.githubusercontent.com/googlefonts/noto-cjk/Sans2.004/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf"
expected_sha256="faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9"
mkdir -p "$(dirname "$dest")"

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    echo "Neither sha256sum nor shasum is available" >&2
    exit 1
  fi
}

if [[ -f "$dest" ]]; then
  actual_sha256="$(sha256_file "$dest")"
  if [[ "$actual_sha256" == "$expected_sha256" ]]; then
    echo "Font already present at $dest"
    exit 0
  fi
fi

tmp="$(mktemp "${TMPDIR:-/tmp}/symm-font.XXXXXX")"
trap 'rm -f "$tmp"' EXIT
curl -fsSL --retry 3 --retry-delay 1 --retry-connrefused -o "$tmp" "$url"
actual_sha256="$(sha256_file "$tmp")"
if [[ "$actual_sha256" != "$expected_sha256" ]]; then
  echo "Noto Sans SC hash mismatch: expected $expected_sha256, got $actual_sha256" >&2
  exit 1
fi
mv "$tmp" "$dest"
echo "Wrote $dest ($(du -h "$dest" | cut -f1))"
