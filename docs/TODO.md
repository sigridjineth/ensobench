# EnsoBench Implementation TODO

## Repo scaffolding
- [x] Create Rust workspace with crates `runner`, `evaluator`, and `hian-gen` under `crates/` as outlined in `docs/TECH_PLAN.md` §1.
- [x] Define shared crates workspace-level config (edition 2021, clippy/rustfmt settings) and ensure binary targets for each tool.

## Runner crate (`ensobench-runner`)
- [x] Implement `enso_client.rs` with HTTP wrappers for `/shortcuts/route`, `/shortcuts/bundle`, `/api/v1/tokens`, `/api/v1/wallet/balances`; load Bearer key from env (`ENSO_API_KEY`).
- [x] Build `txexec/anvil.rs` to fork Anvil per `chainId`, fund ephemeral EOA, send returned tx (to/data/value), and record status/logs/traces.
- [x] Sketch optional `txexec/tenderly.rs` adapter gated behind feature flag, reusing request shape from Tenderly docs.
- [x] Add baseline agents: `agents/core_route.rs` (USDC→WETH route) and `agents/core_bundle.rs` (approve→swap→deposit bundle) with CLI triggers.
- [x] Implement `agents/llm_core.rs` and `agents/llm_hian.rs` shells that enforce strict JSON schema and integrate with OpenRouter (env-driven opt-in).
- [x] Create `artifacts.rs` helpers to persist `runs/<timestamp>/{per_tx.jsonl, trajectory.jsonl, tx_raw.json, meta.json}` as specified in SPEC §6.

## Evaluator crate (`ensobench-evaluator`)
- [x] Define core models (`ActionKind`, `ActionSig`, `ScoreReport`) per `docs/TECH_PLAN.md` §10.
- [x] Implement `parse.rs` to normalize route/bundle metadata into `ActionSig` instances, leveraging token symbol mapping from artifacts.
- [x] Implement `score.rs` applying domain weights, per-tx composition bonus, per-tx action limit, and optional cooldown hooks; ignore revert/no-op sims.
- [x] Build CLI entry (`cli.rs`) that ingests run artifacts and emits coverage score + breakdown JSON.
- [x] Implement `lc_verify.rs` to validate LC runs by sim artifacts: swap balance deltas, bridge metadata, deposit events per SPEC §7.3.

## HIAN generator crate (`ensobench-hian-gen`)
- [x] Implement `make_prompt.rs` to compose haystack + injected instruction (needle) with configurable noise layers.
- [x] Implement `ground_truth.rs` to emit `ground_truth.json` artifacts consumed by evaluator for LC verification.

## Dataset & configs
- [x] Populate `dataset/domains.enso.yaml` using weights and action allowances from `docs/TECH_PLAN.md` §2.2.
- [x] Author initial Coverage scenarios `dataset/coverage/*.yaml` (R1–R4 from SPEC §5.2) with token refs and expected outputs.
- [x] Author LC scenarios under `dataset/lc/` (e.g., `lc_swap`, `lc_bridge`) including `prompt.txt`, `ground_truth.json`, and `meta.json`.

## CI & tooling
- [x] Draft GitHub Actions workflow to build crates, launch Anvil forks, run coverage/LC jobs, and enforce `COVERAGE_FLOOR` + `LC_REQUIRED` gates.
- [x] Add integration test harness to execute baseline agents and evaluator end-to-end on fixtures.
- [x] Provide developer scripts (`Makefile` or `justfile`) for local run, evaluate, and artifact inspection.

## Documentation & polish
- [x] Document environment variables (API keys, OpenRouter) and run instructions in README update.
- [x] Record API response shapes and sample artifacts for reproducibility (align with SPEC §6 & Appendix).
- [x] Outline anti-gaming measures and future hooks (cooldowns, cross-chain bonus) for v0.2 roadmap.

## Follow-up items
- [x] Replace the Anvil execution stub with a real simulator (JSON-RPC flow to Anvil, receipts & gas captured).
- [x] Extend LC verification to validate balance deltas/events per SPEC §7.3 and add accompanying tests.
  - [x] Enforce matching recipient when provided.
  - [x] Fail on reverted simulation receipts.
  - [x] Check min_out via balance/log deltas.
- [ ] Add additional coverage and LC fixtures, then hook CI gates (coverage floor, LC required) once replayable runs are available.
