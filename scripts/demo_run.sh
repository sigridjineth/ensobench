#!/usr/bin/env bash
set -euo pipefail

log() {
  printf '[%s] %s
' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"
}

SAMPLES_DIR="frontend/enso-bench-site/data/samples/coverage/20250926-gpt-5-high"
if [[ ! -d "$SAMPLES_DIR" ]]; then
  log "sample dataset not found at $SAMPLES_DIR; run this from the repo root"
  exit 1
fi

TS=$(date -u +%Y%m%dT%H%M%SZ)
OUT="runs/${TS}-demo"

log "Preparing demo run artifacts in $OUT"
mkdir -p "$OUT"
cp "$SAMPLES_DIR/meta.json" "$OUT/meta.json"
cp "$SAMPLES_DIR/eval_per_tx.jsonl" "$OUT/eval_per_tx.jsonl"

cat > "$OUT/per_tx.jsonl" <<EOF
{"type":"route","timestamp":"$TS","request":{"chainId":1,"tokenIn":{"address":"0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"},"tokenOut":{"address":"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"},"amount":"250000000","slippageBps":30,"recipient":"0x1111111111111111111111111111111111111111"},"response":{"tx":{"chainId":1,"to":"0x2ED45f3128E05A4b7C9B35cADb5d7D135C9aAd1B","data":"0xf241486a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000A0b86991c6218b36c1d19d4a2e9eb0ce3606eb48","value":"0x0"},"route":{"steps":[{"type":"swap","protocol":"uniswap_v3","token_in":"0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48","token_out":"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"}]}} ,"execution":{"status":"success","logs":["{\"address\":\"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2\",\"topics\":[\"0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef\",\"0x0000000000000000000000000000000000000000000000000000000000000000\",\"0x0000000000000000000000001111111111111111111111111111111111111111\"],\"data\":\"0x016345785d8a0000\"}"]}}
EOF

cat > "$OUT/trajectory.jsonl" <<EOF
{"timestamp":"$TS","role":"planner","content":{"demo":"generated locally"}}
EOF

log "Simulating pipeline warmup (~20s)"
for sec in $(seq 1 20); do
  printf '   demo runtime %2ds / 20s' "$sec"
  sleep 1
  if (( sec % 5 == 0 )); then
    log "heartbeat: ${sec}s"
  fi
done
printf '   demo runtime complete            
'

if [[ -n "${OPENROUTER_API_KEY:-}" ]]; then
  log "OPENROUTER_API_KEY detected; invoking llm-core agent"
  cargo run -p ensobench-runner -- llm-core --label "${TS}-demo-llm"
else
  log "OPENROUTER_API_KEY not set; skipping llm-core call"
fi

log "Demo run artifacts ready at $OUT"
