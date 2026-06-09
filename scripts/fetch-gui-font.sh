#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/assets/fonts/NotoSansSC-Regular.otf"
url="https://raw.githubusercontent.com/googlefonts/noto-cjk/Sans2.004/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf"
expected_sha256="faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9"
mkdir -p "$(dirname "$dest")"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
curl -fsSL -o "$tmp" "$url"
actual_sha256="$(sha256sum "$tmp" | cut -d' ' -f1)"
if [[ "$actual_sha256" != "$expected_sha256" ]]; then
  echo "Noto Sans SC hash mismatch: expected $expected_sha256, got $actual_sha256" >&2
  exit 1
fi
mv "$tmp" "$dest"
echo "Wrote $dest ($(du -h "$dest" | cut -f1))"
