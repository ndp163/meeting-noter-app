#!/bin/bash
# Emit the mlx manifest.json consumed by src-tauri/src/mlx/mod.rs.
#
# Usage: make-manifest.sh <libMlxBridge.dylib> <mlx.metallib> <model-dir> > manifest.json
#
# Each entry: { url (CDN path under mlx/v1/), dest (local path under mlx/),
#               size, sha256 }. The dylib + metallib sit at the mlx root; weight
# files go under model/.
set -euo pipefail

DYLIB="$1"
METALLIB="$2"
MODEL_DIR="$3"
PREFIX="mlx/v1"

entry() { # <file> <url> <dest> <trailing-comma?>
  local f="$1" url="$2" dest="$3" comma="$4"
  local size sha
  size=$(stat -f%z "$f")
  sha=$(shasum -a 256 "$f" | awk '{print $1}')
  printf '    { "url": "%s", "dest": "%s", "size": %s, "sha256": "%s" }%s\n' \
    "$url" "$dest" "$size" "$sha" "$comma"
}

echo '{'
echo '  "files": ['
entry "$DYLIB"    "$PREFIX/libMlxBridge.dylib" "libMlxBridge.dylib" ","
entry "$METALLIB" "$PREFIX/mlx.metallib"        "mlx.metallib"        ","

# Model weights: every regular file in the model dir → model/<relative path>.
# Skip Hugging Face's local cache bookkeeping (`.cache/…`) — repo files only.
files=$(cd "$MODEL_DIR" && find . -type f -not -path './.cache/*' | sed 's|^\./||' | sort)
count=$(echo "$files" | grep -c .)
i=0
while IFS= read -r rel; do
  i=$((i + 1))
  comma=","; [ "$i" -eq "$count" ] && comma=""
  entry "$MODEL_DIR/$rel" "$PREFIX/model/$rel" "model/$rel" "$comma"
done <<< "$files"

echo '  ]'
echo '}'
