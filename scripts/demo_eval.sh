#!/usr/bin/env bash
set -euo pipefail

log() {
  printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"
}

RUN_DIR=$(find runs -maxdepth 1 -mindepth 1 -type d -name '*-demo' | sort | tail -1)
if [[ -z "$RUN_DIR" ]]; then
  log "no runs/*-demo directory found; run scripts/demo_run.sh first"
  exit 1
fi

log "Evaluating demo run at $RUN_DIR"
cargo run -p ensobench-evaluator -- \
  --per-tx "$RUN_DIR/per_tx.jsonl" \
  --domains dataset/domains.enso.yaml \
  --lc-ground-truth dataset/lc/swap_usdc_weth/ground_truth.json \
  --format json
