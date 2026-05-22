#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/assets/fonts/NotoSansSC-Regular.otf"
url="https://raw.githubusercontent.com/googlefonts/noto-cjk/Sans2.004/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf"
mkdir -p "$(dirname "$dest")"
curl -fsSL -o "$dest" "$url"
echo "Wrote $dest ($(du -h "$dest" | cut -f1))"
