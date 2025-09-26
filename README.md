# EnsoBench Workspace

This repository hosts the Rust workspace for running EnsoBench coverage, long-context (Operation Needle), and artifact generation workflows. The tooling mirrors the design captured in `docs/SPEC.md` and `docs/TECH_PLAN.md`.

## Workspace layout

```
crates/
  runner/         # ensobench-runner CLI
  evaluator/      # ensobench-evaluator CLI
  hian-gen/       # Haystack-in-a-needle prompt generator
```

Supporting datasets and docs live under `dataset/` and `docs/`.

## Getting started

1. Install Rust (1.75+) and Foundry's toolchain (we rely on `anvil` for local forks). Example:
   ```bash
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```
2. Export required env vars:
   - `ENSO_API_KEY`: Enso Shortcuts API key.
   - `ENSO_BASE_URL` (optional): Override base API URL.
   - `ENSO_ARTIFACTS_DIR` (optional): Directory for run artifacts (defaults to `runs/`).
   - `ENSO_FORK_URL_<CHAIN_ID>` (optional): Map chain IDs to RPCs for Anvil forking.
   - `OPENROUTER_API_KEY` (optional): Enable LLM planner calls via OpenRouter.
3. Build the workspace: `cargo build --workspace`.
4. Run baseline agents (these commands spin up ephemeral Anvil forks, execute the returned tx, and write execution traces into `runs/`):
   - `cargo run -p ensobench-runner -- core-route --scenario dataset/coverage/r1_usdc_weth_route.yaml`
   - `cargo run -p ensobench-runner -- core-bundle --scenario dataset/coverage/r2_usdc_weth_aave_bundle.yaml`

Artifacts land under `runs/<timestamp>-<label>/` with `per_tx.jsonl`, `trajectory.jsonl`, and metadata.

## Evaluating

After producing artifacts, score coverage and verify Operation Needle prompts:

```
cargo run -p ensobench-evaluator -- \
  --per-tx runs/<timestamp>-core-route/per_tx.jsonl \
  --domains dataset/domains.enso.yaml \
  --lc-ground-truth dataset/lc/swap_usdc_weth/ground_truth.json
```

Outputs a JSON report aligning with the scoring rules in the spec. When `--lc-ground-truth` is provided, Operation‑Needle verification now checks recipients and `min_out` requirements by decoding ERC‑20 `Transfer` logs from the simulation.

## Generating HIAN scenarios

Use `ensobench-hian-gen` to create prompts and ground-truth bundles:

```
cargo run -p ensobench-hian-gen -- make-prompt --output dataset/lc/custom/prompt.txt
cargo run -p ensobench-hian-gen -- ground-truth \
  --chain-id 1 --token-in 0x... --token-out 0x... --amount 100000000 \
  --recipient 0x... --output dataset/lc/custom/ground_truth.json
```

## Make commands

`Makefile` offers helpers: `core-route`, `core-bundle`, `evaluator`, `hian`, `fmt`, `lint`.

## Testing

The workspace builds with `cargo check` (Anvil must be on `PATH`). Unit tests live in `ensobench-evaluator` and can be run with `cargo test -p ensobench-evaluator`; note the sandboxed CLI may block downloading new dev dependencies, so run tests locally if network access is required.

## CI outline

The repo ships with `.github/workflows/ci.yml`, which currently builds, lints, runs the baseline route scenario (using Anvil), and evaluates artifacts. Upcoming work is to gate coverage/LC scores once a broader fixture set lands.

1. Check out repo and install Foundry (Anvil).
2. Cache cargo builds.
3. Run `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
4. Execute targeted tests / evaluations (today: baseline run + evaluator; future: add coverage/LC gates when datasets expand).

## Next steps

- Flesh out integration/system tests that replay stored artifacts
- Grow coverage & LC fixture sets, then enforce score gates in CI
- Optional: implement the Tenderly execution backend alongside Anvil
- Harden anti-gaming controls (cooldowns per protocol, cross-chain bonus caps, slippage sanity checks)
