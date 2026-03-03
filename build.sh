#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
CODE_DIR="$REPO_ROOT/code"

echo "==> Building grift-site with trunk..."
cd "$CODE_DIR"
trunk build --release --public-url ./

echo "==> Removing old build artifacts from repo root..."
rm -f "$REPO_ROOT"/*.js "$REPO_ROOT"/*.wasm "$REPO_ROOT"/style-*.css "$REPO_ROOT/index.html"

echo "==> Copying new build output to repo root..."
cp "$CODE_DIR/dist/"* "$REPO_ROOT/"

echo "==> Done! Built files:"
ls -lh "$REPO_ROOT"/*.html "$REPO_ROOT"/*.js "$REPO_ROOT"/*.wasm "$REPO_ROOT"/*.css 2>/dev/null
