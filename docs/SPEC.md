Below is a concrete EnsoBench spec that reuses your SuiBench architecture and scoring ideas while adapting to Enso’s Shortcuts / Route / Bundle model. I’ve read the Enso docs, Swagger, SDK, and template pages you shared and mapped them into a Rust-first benchmark you can build quickly. I also anchor the mapping to your SuiBench deck so judges can see continuity in method and rigor.

0) TL;DR

Unit of evaluation (Sui → Enso):
MoveCall signature inside a PTB ⟶ Action/Route signature returned by Enso /shortcuts/route or /shortcuts/bundle (the API returns executable calldata + route metadata). 
Enso Developer Documentation
+1

Execution: Instead of Sui localnet, EnsoBench simulates or broadcasts the returned tx (to, data, value, chainId) on an EVM fork (Anvil/Tenderly) for reproducibility. Optional: use Enso’s own sim workflows/Tenderly. 
Enso Developer Documentation
+2
Blog | Tenderly
+2

Scoring (Coverage): Weighted count of unique action kinds and protocols touched + a composition bonus when Enso bundles multiple actions into one atomic tx (analog of Sui PTB composition). No‑ops and reverts ignored. 
Enso Developer Documentation

LC/Needle track: Long noisy prompt that hides a single actionable DeFi instruction (e.g., “bridge+swap and send to X”). Pass/Fail by checking sim outcome vs ground truth. (Same philosophy as your Operation‑Needle track.)

APIs used: Shortcuts (/route, /bundle), Tokens (/api/v1/tokens), Wallet balances (/api/v1/wallet/balances) and SDK‑TS as optional reference. 
Enso Developer Documentation
+3
Enso Finance
+3
Enso Developer Documentation
+3

1) Enso mental model (for the benchmarker)

What Enso gives you: a connectivity layer and intent/shortcut abstraction: you call Shortcuts APIs to get executable tx (calldata, to, value, chainId) that atomically performs DeFi workflows (swaps, deposits, lending ops, etc.), optionally cross‑chain and bundled. 
Enso Developer Documentation
+1

Two core endpoints:

POST /shortcuts/route — pathfinding between “positions” (e.g., token ↔ token, token ↔ vault). Returns optimal multi‑step route and one tx to execute. Best for swaps / simple in‑out movements. 
Enso Developer Documentation

POST /shortcuts/bundle — compose your own sequence of actions (harvest, borrow, repay, etc.) into one atomic tx. Changelog: bundle responses now also include route info when a bundled action involves routing. 
Enso Developer Documentation
+1

Router vs Delegate contracts: don’t hardcode addresses; use the response.tx.to from the API and pick strategy per docs (router vs delegate). 
Enso Developer Documentation
+1

SDK: JS/TS SDK wraps approvals, routing, quoting, balances; we’ll call the HTTP APIs directly from Rust (reqwest) but keep SDK semantics as a reference during test design. 
Enso Developer Documentation

Metadata: Tokens and Balances endpoints power “what’s tradable” and “does the wallet have funds.” Useful for Needle prompts and validation. 
Enso Developer Documentation
+1

2) Mapping SuiBench → EnsoBench (design equivalences)
SuiBench concept	Enso equivalent	Why / how we measure
PTB (programmable tx block)	Route/Bundle response.tx (single atomic EVM tx)	Both are “multi‑step in one tx” abstractions. We score in‑tx composition. 
Enso Developer Documentation
+1

MoveCall signature (pkg::mod::fn<tys>)	Action/Route signature (chainId, actionKind, protocolId?, tokenIn/out?)	Normalize the route/bundle metadata to an action signature; count unique. 
Enso Developer Documentation

PTB composition bonus	Bundle composition bonus	Reward multi‑action atomicity (e.g., approve→swap→deposit). 
Enso Developer Documentation

Localnet	Anvil/Tenderly simulation or forked EVM	Deterministic, fast, revert/no‑op filters. 
GitHub
+1

Domains (core/kiosk)	Domains (dex/lending/yield/bridge/derivatives)	Weight per domain; spread scores beyond swaps.
No‑op filter	Revert/no‑effect filter	Ignore failed or no‑change results (sim outcome).

We’ll re‑use your slides and scoring notation; judges will see continuity across chains/contexts.

3) Tracks & scoring
3.1 Coverage Track (breadth + composition)

Unique unit:
ActionSig = (chainId, actionKind, protocolId?, routeTag?)

actionKind examples: swap, deposit, redeem, borrow, repay, harvest, stake, unstake, bridge. (Route paths supply protocol/project steps; Bundle defines actions explicitly.) 
Enso Developer Documentation

Base score: Σ_d weight[d] × unique_sigs[d] where domains are dex, lending, yield, bridge, derivatives. (E.g., Uniswap/Curve steps → dex; Aave/Morpho → lending.)

Composition bonus (per‑tx): 0.25 × max(0, unique_actionKinds_in_tx − 1)—same shape as SuiBench PTB bonus.

No‑op filter: Reverts or no messages/state impact (via sim) earn 0.

Anti‑gaming (first pass): cap repeats per protocol per run; cap bonus by per_tx_action_limit.

3.2 LC / Operation‑Needle (Pass/Fail)

Prompt is a long noisy doc embedding one critical instruction + keys (token IDs/addresses, chainId, recipient). The agent must call route or bundle with the correct params and execute the returned tx on sim/fork. We Pass if the sim outcome matches ground truth (e.g., post‑swap minOut met, recipient got funds; or vault share tokens minted). (Same method you used for Sui LC.)

4) System architecture (Rust, reusing your crates)

Mirrors the three‑crate design in your deck (generator → runner → evaluator).

ensobench/
├── runner/ (Rust)
│   ├── enso_client.rs        # reqwest client for /route, /bundle, /tokens, /balances
│   ├── agents/
│   │   ├── core_route.rs     # baseline: single route (swap)
│   │   ├── core_bundle.rs    # approve→swap→deposit bundle
│   │   ├── llm_core.rs       # Coverage via LLM plan → route/bundle
│   │   └── llm_hian.rs       # LC: long-context “needle” task
│   ├── txexec/
│   │   ├── anvil.rs          # fork, fund EOA, send tx, collect trace/logs
│   │   └── tenderly.rs       # optional: remote sim API
│   └── artifacts.rs          # per_tx.jsonl (response + sim result), trajectory.jsonl
├── evaluator/ (Rust)
│   ├── parse.rs              # parse route/bundle JSON → ActionSig list
│   ├── score.rs              # Base+Bonus−Penalty; domain weights
│   ├── lc_verify.rs          # compare sim outputs vs ground_truth.json
│   └── cli.rs                # score-from-per-tx, explain
└── hian-gen/ (Rust)
    ├── make_prompt.rs        # generate haystack, place needle & keys
    └── ground_truth.rs       # expected chainId/to/token changes


APIs used: /shortcuts/route, /shortcuts/bundle (auth: Bearer key), /api/v1/tokens, /api/v1/wallet/balances. Swagger confirms the base, and docs show sandbox vs key usage. 
Enso Developer Documentation
+3
Enso Finance
+3
Enso Developer Documentation
+3

Routing strategies & deployments: use response.tx.to; pick router or delegate per page guidance—no hardcoding. 
Enso Developer Documentation
+1

Simulation layer: default Anvil fork per chainId (JSON‑RPC from env). Optional Tenderly driver to cross‑check routes, consistent with Enso’s own practice of integrating TX sims. 
GitHub
+1

5) Data & configs
5.1 domains.yaml (EnsoBench)
version: "0.1.0"
per_tx_action_limit: 6

domains:
  dex:
    weight: 1.0
    allow:
      - action: swap
  lending:
    weight: 1.25
    allow:
      - action: deposit
      - action: borrow
      - action: repay
      - action: redeem
  yield:
    weight: 1.25
    allow:
      - action: stake
      - action: harvest
      - action: claim
  bridge:
    weight: 1.5
    allow:
      - action: bridge


Action kinds map from route path steps or bundle actions; evaluator tags each ActionSig to a domain using this file.

5.2 Coverage tasks (dataset)

R1 Swap USDC→WETH (mainnet) via /route.

R2 Swap + Deposit (USDC→WETH→lp/vault) via /bundle.

R3 Cross‑chain USDC→USDC (Arb→OP) with implicit bridge callbacks.

R4 Borrow/Repay (Aave‑style) within a bundle.

The docs page clarifies when to prefer Route vs Bundle and highlights cross‑chain callbacks. 
Enso Developer Documentation
+1

5.3 LC tasks (Operation‑Needle)

LC‑Swap: noisy prompt hides “swap EXACT 100 USDC→WETH on chain 1 to Recipient A before 14:00.”

LC‑Bridge: noisy prompt hides “bridge 25% of USDC to Optimism and send to Recipient B.”

Ground truth encodes (chainId, tokenIn/out, minOut/recipient); evaluator reads the sim outcome & Pass/Fail.

6) Runner details (Rust)
6.1 Enso client (reqwest)

Auth: Authorization: Bearer <API_KEY> (get key from Enso dashboard). 
Enso Developer Documentation

Endpoints:

POST /shortcuts/route → { tx: { to, data, value, chainId }, route: {...} }

POST /shortcuts/bundle → { tx: {...}, bundle: {...}, route?: {...} } (route metadata present when the bundle involves routing). 
Enso Developer Documentation
+1

Helpers:

Token/chain lookup: call /api/v1/tokens to resolve symbols/contracts. 
Enso Developer Documentation

Balance check (optional): /api/v1/wallet/balances; with useEoa=true to check the funded EOA on the fork. 
Enso Developer Documentation

Swagger confirms base URL and headers; the docs note you can browse endpoints in a sandbox but need keys for building. 
Enso Finance
+1

6.2 Tx execution (Anvil fork)

Spin up Anvil with --fork-url for the target chainId.

Fund an ephemeral EOA and send the returned tx (to, data, value) against the fork.

Collect: status (success/revert), logs, traces, gas used.

Record all into runs/<ts>/per_tx.jsonl (append also the original Enso response for evaluator parsing).

If you prefer pure simulation, add the Tenderly backend; Enso publicly documents sim usage to sanitize routes. 
Blog | Tenderly

7) Evaluator details (Rust)
7.1 Parsing to ActionSig

From Route/Bundle JSON:

Extract chainId, detect actionKind(s) and, when present, protocol/project identifiers in the route metadata (e.g., step type, protocol name). (Changelog: bundles now echo route info when routing is involved.) 
Enso Developer Documentation

Normalize to:
ActionSig { chainId, actionKind, protocol: Option<String>, tokens: Option<(in,out)> }

7.2 Scoring

Base: domain‑weighted unique ActionSigs (per domains.yaml).

Composition bonus: per‑tx 0.25 × (unique_actionKinds_in_tx − 1), capped by per_tx_action_limit.

No‑op: ignore if Anvil/Tenderly marks revert or if no route/bundle produced.

7.3 LC verifier

Compare sim results to ground_truth.json:

Swap: check decoded logs or token balances on fork (pre/post) for recipient/minOut.

Bridge: check that a bridge step exists in route metadata and post‑bridge callback recipient matches.

Emit eval_hian.json with { pass: bool, reason }.

8) LLM agent & prompts

llm_core (Coverage)
System prompt forces STRICT JSON steps (no fences) with a small vocabulary:

route { chainId, tokenIn, tokenOut, amount, slippageBps }

bundle { chainId, actions: [ {approve|swap|deposit|borrow|repay|harvest|stake|bridge ...} ] }
HARD RULES: ≤5 steps; if deposit/borrow appears, must be inside a single bundle; always use tokens/addresses exactly as provided; never guess—call /tokens first if only symbols are given. We auto‑patch a missing approve when required. (Same pattern you used to enforce a final movecall in Sui.)

llm_hian (Needle)
Long context with one actionable instruction. The agent must:
(1) extract tokens/addresses/chainId, (2) choose route vs bundle appropriately, (3) return strict JSON. Pass/Fail via lc_verify.rs.

9) Anti‑gaming & difficulty knobs

Cooldowns: per actionKind+protocol, only first N instances count fully.

Domain weights: raise lending / yield above dex to motivate more complex flows.

Cross‑chain bonus: +0.25 if route metadata contains an explicit bridge step. 
Enso Developer Documentation

Per‑tx action cap: matches per_tx_action_limit to keep bonus bounded.

No‑hardcoding: evaluator rejects submissions that ignore API’s tx.to or routing strategy hints. 
Enso Developer Documentation

10) CI & reproducibility

GitHub Actions: matrix over models; each job

starts Anvil fork(s),

exports ENSO_API_KEY (repo secret),

runs ensobench-runner → ensobench-evaluator,

fails on regression.

Set COVERAGE_FLOOR and LC_REQUIRED envs like in SuiBench so teams get don’t‑trust‑verify quality gates on every PR.

11) Minimal build plan (≈1–2 days)

Day 0 (3–5h)

enso_client.rs: POST /shortcuts/route & /shortcuts/bundle; GET /api/v1/tokens, /api/v1/wallet/balances; Bearer auth loaded from env. 
Enso Finance
+2
Enso Developer Documentation
+2

txexec/anvil.rs: fork by chainId, fund EOA, eth_sendRawTransaction, gather traces.

Day 1 (4–6h)

agents/core_route.rs (USDC→WETH swap) & core_bundle.rs (approve→swap→deposit).

Evaluator: parse route/bundle JSON → ActionSig; Coverage scoring; no‑op filter.

Day 2 (5–6h)

llm_core + strict JSON plan (OpenRouter) and auto‑patch approvals.

hian-gen and llm_hian (one LC scenario).

lc_verify.rs (swap Pass/Fail by balance delta for recipient).

12) Example request/response shapes (for your Rust client)

Note: endpoints shown here match the docs & Swagger; supply API key via Bearer. 
Enso Finance

Route (swap)

POST /shortcuts/route
{
  "chainId": 1,
  "tokenIn": { "address": "0xA0b86991..." },
  "tokenOut": { "address": "0xC02aaa39..." },
  "amount": "100000000", 
  "slippageBps": 30
}
-- response --
{
  "tx": { "to": "0xEnsoRouter...", "data": "0x...", "value": "0x0", "chainId": 1 },
  "route": {
    "steps": [
      { "type": "swap", "protocol": "uniswap_v3", "pool": "...", "tokenIn": "USDC", "tokenOut": "WETH" }
    ]
  }
}


Bundle (approve + swap + deposit)

POST /shortcuts/bundle
{
  "chainId": 1,
  "actions": [
    { "approve": { "token": "0xA0b8...", "spender": "router" } },
    { "swap": { "tokenIn": "...", "tokenOut": "...", "amount": "100000000", "slippageBps": 30 } },
    { "deposit": { "protocol": "aave_v3", "asset": "0xA0b8...", "amountSource": "fromPrevious" } }
  ]
}
-- response --
{
  "tx": { "to": "0xDelegateShortcuts...", "data": "0x...", "value": "0x0", "chainId": 1 },
  "bundle": { "actions": [ "...normalized..." ] },
  "route": { "steps": [ { "type": "swap", "protocol": "uniswap_v3" } ] }  // when bundle includes a swap
}


(Exact field names come from the live API; the key point for the evaluator is tx and route/bundle metadata for signature extraction.) 
Enso Developer Documentation
+1

13) Risks & mitigations

Token availability on fork: seed forked EOAs or impersonate whales for positive‑path demos; for CI, prefer simulation (no state commitments). 
GitHub

API evolution: pin Enso API version & record the exact JSON alongside every run for replay. Swagger indicates API‑key auth and versioned endpoints. 
Enso Finance

Cross‑chain complexity: start with single‑chain routes; treat bridge flows as bonus until LC & Coverage are solid. 
Enso Developer Documentation

Hardcoding contracts: forbidden—always use response.tx.to. 
Enso Developer Documentation

14) Why EnsoBench matters (value)

For agent builders: get a chain‑agnostic, CI‑friendly measure of operational capability over real DeFi workflows (not toy swaps).

For protocol teams: publish domain weights and example bundles to validate integrations; catch approval & allowance pitfalls before user funds are at risk.

For model vendors: show a DeFi‑native scorecard that customers can reproduce locally, across Coverage and Long‑Context tasks, not just chat benchmarks.

For end users: don’t trust—verify that any “AI copilot” actually executes the right tx with the right parameters under noise and drift, before it touches funds.

Appendix — Key doc pointers used in this spec

Dev Home & API overview: what Shortcuts are, sandbox vs API‑key. 
Enso Developer Documentation
+1

Shortcuts / Route / Bundle: semantics, when to use which, and bundle’s new route info. 
Enso Developer Documentation
+2
Enso Developer Documentation
+2

Routing strategies & deployments: router vs delegate, use response.tx.to. 
Enso Developer Documentation
+1

Tokens/Balances metadata: /api/v1/tokens & /api/v1/wallet/balances. 
Enso Developer Documentation
+1

SDK reference (optional adjunct to HTTP): approvals/quoting/routing helpers. 
Enso Developer Documentation

Simulation precedent: Enso case study + Enso’s own “temper” simulator. 
Blog | Tenderly
+1

If you want, I can draft the Rust scaffolding for enso_client.rs, the ActionSig normalizer, and a minimal Anvil executor in your repo’s style next.
