.PHONY: core-route core-bundle evaluator hian prompt bridge lint fmt format check

core-route:
	cargo run -p ensobench-runner -- core-route --scenario dataset/coverage/r1_usdc_weth_route.yaml --simulate

core-bundle:
	cargo run -p ensobench-runner -- core-bundle --scenario dataset/coverage/r2_usdc_weth_aave_bundle.yaml --simulate

evaluator:
	cargo run -p ensobench-evaluator -- --per-tx runs/latest/per_tx.jsonl --domains dataset/domains.enso.yaml

hian:
	cargo run -p ensobench-hian-gen -- make-prompt --output dataset/lc/generated_prompt.txt

format:
	cargo fmt && cargo check

check:
	cargo clippy --all-targets --all-features
