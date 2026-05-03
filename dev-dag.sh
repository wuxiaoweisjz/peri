#!/bin/bash
set -e

cd "$(dirname "$0")"

# Defaults
export PORT="${PORT:-3000}"
export DATABASE_URL="${DATABASE_URL:-sqlite:peri-dag.db?mode=rwc}"
export RUST_LOG="${RUST_LOG:-peri_dag=debug,info}"

echo "=== peri-dag dev server ==="
echo "  http://0.0.0.0:${PORT}"
echo "  db: ${DATABASE_URL}"
echo

cargo run -p peri-dag -- --workflow-dir ./peri-dag/examples/
