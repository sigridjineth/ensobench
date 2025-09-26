Here is a complete TECH_PLAN.md you can drop into the repo.

EnsoBench — Technical Plan (v0.1)

Purpose. Adapt the SuiBench methodology to Enso’s intent‑centric DeFi orchestration. We measure not network TPS but (1) Enso API routing/bundling capability and latency, and (2) the operational reliability of LLM agents that translate natural language into correct, executable Enso routes/bundles.

Pedigree. We reuse SuiBench’s two‑track design—Coverage (breadth + composition) and LC/Operation‑Needle (accuracy under noise)—and its reproducibility discipline (artifacts, hashes, CI). See the SuiBench deck: steps, scoring and LC track rationale.

0) High‑Level Goals

Coverage Track: Count unique DeFi action signatures produced via Enso /shortcuts/route and /shortcuts/bundle; reward in‑transaction composition (approve→swap→deposit … in one atomic tx).

LC / Operation‑Needle Track: From a very long, noisy prompt, can a model find the one actionable instruction with exact parameters and produce a single valid route/bundle transaction that simulates successfully?

What we optimize for (Enso‑native):

Off‑chain latency (API time to compute route/bundle).

On‑chain time‑to‑finality (TTF) on a fork/simulator (Anvil/Tenderly) for the returned tx.

Breadth & compositionality across DeFi domains (DEX, lending, yield, bridge).

Correctness under noise (LC Pass/Fail).

1) System Architecture (Rust, 3 crates)
ensobench/
├── crates/
│   ├── runner/                # ensobench-runner (Rust)
│   │   ├── enso_client.rs     # HTTP client for /shortcuts/{route,bundle}, /tokens, /balances
│   │   ├── txexec/
│   │   │   ├── anvil.rs       # EVM fork execution (to, data, value); traces & logs
│   │   │   └── tenderly.rs    # optional external simulator backend
│   │   ├── agents/
│   │   │   ├── core_route.rs  # baseline: 1 swap via /route
│   │   │   ├── core_bundle.rs # approve→swap→deposit via /bundle
│   │   │   ├── llm_core.rs    # coverage via LLM → strict JSON plan
│   │   │   └── llm_hian.rs    # long-context “Operation Needle” agent
│   │   └── artifacts.rs       # write runs/<ts>/{per_tx.jsonl, trajectory.jsonl, tx_raw.json, meta.json}
│   ├── evaluator/             # ensobench-evaluator (Rust)
│   │   ├── parse.rs           # Enso -> ActionSig extraction
│   │   ├── score.rs           # Base + Bonus − Penalty (Coverage)
│   │   ├── lc_verify.rs       # LC Pass/Fail via sim outcome & ground_truth.json
│   │   └── cli.rs             # score-from-per-tx, score-from-digests (if broadcasting), explain
│   └── hian-gen/              # ensobench-hian-gen (Rust)
│       ├── make_prompt.rs     # haystack + keys + needle generator
│       └── ground_truth.rs    # expected effects; emits ground_truth.json
└── dataset/
    ├── domains.enso.yaml
    ├── coverage/              # scenario YAMLs for Coverage tasks
    └── lc/                    # LC prompts & answers (prompt.txt, ground_truth.json, meta.json)


Continuities with SuiBench: same run folder discipline, two tracks, strict artifacts, CI gating.

2) Evaluation Units & Scoring
2.1 Action Signatures (the “unique unit”)
ActionSig {
  chain_id: u64,
  action:   ActionKind,          // swap | deposit | redeem | borrow | repay | stake | harvest | bridge | ...
  protocol: Option<String>,      // e.g., "uniswap_v3", "curve", "aave_v3"
  tokens:   Option<(String,String)>, // e.g., ("USDC","WETH") for swap; ("USDC","aUSDC") for deposit
}


Extracted from Enso /shortcuts/route → route.steps[] and /shortcuts/bundle → bundle.actions[] (plus route.steps[] if the bundle contains a swap). We never hardcode router/delegate addresses; we only use response.tx.to at execution time.

2.2 Domains & weights (dataset/domains.enso.yaml)
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

2.3 Coverage Score (same shape as SuiBench)

Final = Base + Bonus − Penalty.

Base = Σ_domains ( weight[d] × unique ActionSigs in d ).

Bonus (per tx) = 0.25 × max(0, unique_actionKinds_in_tx − 1); capped by per_tx_action_limit.

Penalty = 0.0 in v0.1 (hooks exist for cooldown/abuse in v0.2).

No‑op filter = ignore simulations that revert or produce no effect.

The Base/Bonus shape, no‑op filter, and per‑PTB/tx composition idea mirror SuiBench’s Coverage philosophy.

2.4 LC / Operation‑Needle (Pass/Fail)

Stimulus: a very long noisy prompt contains one high‑priority instruction + exact keys.

Agent output: strict JSON plan → /shortcuts/route or /shortcuts/bundle call → returned tx executes on fork.

Verify: compare sim outcome to ground_truth.json (recipient balance delta ≥ minOut; or vault share minted; or bridge recipient matched). Pass if exact; otherwise Fail.

Durability Sweeps: context length and needle position (5/50/95%).

3) Data I/O (Schemas + “mock but real‑looking” fixtures)

We ship typed schemas, golden fixtures and minimal generators so teams can test offline and in CI.

3.1 Inputs
(A) Coverage scenarios (YAML)
# dataset/coverage/r1_swap_usdc_weth.yaml
id: "R1_SWAP_USDC_WETH_MAINNET"
title: "Swap USDC->WETH via Route on Ethereum"
chain_id: 1
token_in: { symbol: "USDC", address: "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", decimals: 6 }
token_out:{ symbol: "WETH", address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", decimals: 18 }
amount: "100000000"      # 100 USDC (6d)
slippage_bps: 30
recipient: "0x1111111111111111111111111111111111111111"

# dataset/coverage/r2_bundle_swap_deposit.yaml
id: "R2_BUNDLE_APPROVE_SWAP_DEPOSIT"
title: "Approve->Swap->Deposit (Aave v3)"
chain_id: 1
actions:
  - approve: { token: "USDC", spender: "router" }
  - swap:    { token_in: "USDC", token_out: "WETH", amount: "100000000", slippage_bps: 30 }
  - deposit: { protocol: "aave_v3", asset: "WETH", amount_source: "fromPrevious" }
recipient: "0x2222222222222222222222222222222222222222"

(B) LC scenarios (files)
dataset/lc/L1_swap_needle/
  prompt.txt            # haystack + keys + needle
  ground_truth.json     # expected change(s)
  meta.json             # sha256(prompt), seed, created_at, author


Example ground_truth.json:

{
  "case_id": "LC1_SWAP_USDC_TO_WETH",
  "chain_id": 1,
  "recipient": "0x3333333333333333333333333333333333333333",
  "expect": {
    "type": "swap",
    "token_in":  { "symbol": "USDC", "address": "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" },
    "token_out": { "symbol": "WETH", "address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2" },
    "min_out_wei": "50000000000000000"   // 0.05 WETH (mock)
  }
}

(C) Runner config (env)
ENSO_API_BASE=https://api.enso.finance
ENSO_API_KEY=xxxxx
ANVIL_FORK_URL_MAINNET=https://eth-mainnet.g.alchemy.com/v2/xxxxx
ANVIL_FORK_URL_ARBITRUM=https://arb-mainnet.g.alchemy.com/v2/xxxxx
FUNDED_EOA_PK=0xabc...          # ephemeral key for fork tx
OPENROUTER_API_KEY=...          # optional (LLM agents)
LLM_MODEL=openai/gpt-5-pro      # example

3.2 Outputs (artifacts)
(1) runs/<ts>/per_tx.jsonl — one line per executed tx
{
  "when": "2025-10-01T07:12:19.812Z",
  "scenario_id": "R2_BUNDLE_APPROVE_SWAP_DEPOSIT",
  "agent": "core_bundle",
  "enso_request": {
    "kind": "bundle",
    "body": {
      "chainId": 1,
      "actions": [
        { "approve": { "token": "0xA0b8...", "spender": "router" } },
        { "swap":    { "tokenIn": "0xA0b8...", "tokenOut": "0xC02a...", "amount": "100000000", "slippageBps": 30 } },
        { "deposit": { "protocol": "aave_v3", "asset": "0xC02a...", "amountSource": "fromPrevious" } }
      ]
    }
  },
  "enso_response": {
    "tx": { "to": "0xEnsoDelegate...", "data": "0xabcdef...", "value": "0x0", "chainId": 1 },
    "bundle": { "actions": ["approve","swap","deposit"] },
    "route":  { "steps": [ { "type":"swap", "protocol":"uniswap_v3" } ] }
  },
  "simulation": {
    "engine": "anvil",
    "fork_url": "mainnet",
    "sender": "0x9f...e1",
    "status": "success",
    "gas_used": 182349,
    "logs": ["0xddf252ad... Transfer(USDC) ...", "0x... Deposit(AAVE) ..."],
    "balance_deltas": {
      "recipient": { "WETH": "0.061234567890000000" }
    }
  }
}

(2) runs/<ts>/eval_per_tx.jsonl — normalized summary
{
  "digest": "0x...txhash",
  "action_sigs": [
    { "chain_id":1, "action":"approve", "protocol":null,      "tokens": ["USDC","-"] },
    { "chain_id":1, "action":"swap",    "protocol":"uniswap_v3", "tokens": ["USDC","WETH"] },
    { "chain_id":1, "action":"deposit", "protocol":"aave_v3", "tokens": ["WETH","aWETH"] }
  ],
  "domains": ["dex","lending"],
  "bonus": 0.5,
  "ignored_noop": false
}

(3) runs/<ts>/eval_score.json — final Coverage report
{
  "final_score": 3.5,
  "by_domain": { "dex": 2.0, "lending": 1.0, "yield": 0.0, "bridge": 0.0 },
  "bonus": 0.5,
  "penalty": 0.0,
  "unique_sigs": {
    "dex": [
      { "chain_id":1, "action":"swap", "protocol":"uniswap_v3", "tokens":["USDC","WETH"] }
    ],
    "lending": [
      { "chain_id":1, "action":"deposit", "protocol":"aave_v3", "tokens":["WETH","aWETH"] }
    ]
  },
  "metadata": {
    "bench_version": "v0.1",
    "domains_hash": "819c05...72bf",
    "node": "anvil-fork:mainnet",
    "source": "per_tx"
  }
}

(4) runs/<ts>/eval_hian.json — LC verdict
{
  "case_id": "LC1_SWAP_USDC_TO_WETH",
  "pass": true,
  "reason": "recipient received >= min_out_wei in WETH",
  "stats": { "prompt_sha256": "83a9...", "context_tokens": 128000, "needle_pos": 0.95 }
}

4) Runner — core components (Rust)
4.1 enso_client.rs (HTTP)

Auth: Authorization: Bearer $ENSO_API_KEY.

Endpoints (body shapes follow current Swagger; we store the raw JSON into artifacts):

POST /shortcuts/route → { tx, route }

POST /shortcuts/bundle → { tx, bundle, route? }

GET /api/v1/tokens?chainId=...

GET /api/v1/wallet/balances?chainId=...&address=...

Interface (sketch):

pub struct EnsoClient { http: reqwest::Client, base: String, api_key: String }
impl EnsoClient {
  pub async fn route(&self, body: RouteBody) -> anyhow::Result<RouteResp> { /* ... */ }
  pub async fn bundle(&self, body: BundleBody) -> anyhow::Result<BundleResp> { /* ... */ }
  pub async fn tokens(&self, chain_id: u64) -> anyhow::Result<Vec<TokenMeta>> { /* ... */ }
  pub async fn balances(&self, chain_id: u64, addr: Address) -> anyhow::Result<Balances> { /* ... */ }
}

4.2 txexec/anvil.rs (EVM fork executor)

Start Anvil (or connect to pre‑started) with --fork-url $ANVIL_FORK_URL_<CHAIN>.

Fund an ephemeral EOA; send the returned tx (to, data, value) as raw transaction.

Collect status, gas_used, logs, and balance deltas for recipient.

Return a structured SimulationResult to be embedded into per_tx.jsonl.

4.3 Agents

core_route.rs: build a /route body from scenario YAML; execute once.

core_bundle.rs: construct a /bundle with approve→swap→deposit; auto‑patch approve if missing.

llm_core.rs: system prompt strictly limits steps to {route, bundle} schemas; if deposit/borrow present, require bundle; forbid guessing token addresses (must use provided or query tokens API); output strict JSON (no code fences).

llm_hian.rs: reads prompt.txt, extracts keys via LLM, chooses route or bundle, executes once.

The strict JSON + whitelist prompt pattern mirrors SuiBench’s approach (slide “Set Your System Prompt”).

5) Evaluator — parsing, scoring, verification
5.1 Parse to ActionSig

From enso_response.route.steps[] and/or bundle.actions[], emit ActionSigs.

Token symbols are normalized via local token map (from /tokens) embedded in artifacts.

5.2 Coverage math

For each run line, compute per‑tx unique actionKinds for bonus, then aggregate unique ActionSigs per domain.

Apply weights and cap bonus by per_tx_action_limit.

5.3 LC verify

Swap: compute recipient’s pre/post balance deltas on fork for token_out and compare with min_out_wei.

Bridge: verify route indicates a bridge step and recipient address in post‑bridge leg matches ground truth (if returned in metadata).

Deposit: detect minted share token or Deposit event with expected params.

6) LLM Prompts (Coverage & LC)
6.1 Coverage — llm_core system prompt (excerpt)
You are an EnsoBench planner. Output STRICT JSON only (no prose, no code fences).
Return either:
  { "route":  { "chainId": <u64>, "tokenIn": {...}, "tokenOut": {...}, "amount":"<str>", "slippageBps": <u16>, "recipient":"0x..." } }
or
  { "bundle": { "chainId": <u64>, "actions": [ {approve|swap|deposit|borrow|repay|stake|harvest|bridge: {...}} ], "recipient":"0x..." } }

HARD RULES:
- ≤ 5 actions total. If 'deposit' or 'borrow' is present, you MUST choose "bundle".
- Never invent addresses; use provided tokens or ask to resolve via tokens API (we pass you the symbol->address map in context).
- Do not split into multiple transactions; produce exactly one executable plan.

6.2 LC — llm_hian system prompt (excerpt)
Read the long context. Execute the single highest-priority actionable instruction.
Extract exact parameters (chainId, token addresses, amount, recipient).
Return STRICT JSON using the same schema as above. No markdown fences.
If multiple instructions exist, select the one explicitly labeled "urgent"/"mandatory".

7) CI Pipeline (GitHub Actions)

Jobs: (1) Build & unit tests. (2) Start Anvil fork(s). (3) Run Coverage scenarios (core + LLM if OPENROUTER_API_KEY present). (4) Run LC scenarios. (5) Score & gate.

Gates:

COVERAGE_FLOOR=3.5 (example): fail if below.

LC_REQUIRED=true: at least one LC case must PASS.

Artifacts: upload runs/<ts>.

The CI gating mirrors the SuiBench “builders can compose their own evaluation set” pattern in the deck (page with CI YAML).

8) Anti‑gaming & Difficulty Knobs

Cooldowns: per (actionKind, protocol) only first N count fully; subsequent instances count as 0 or partial.

Cross‑chain bonus: +0.25 if a bridge step exists.

per_tx_action_limit: cap bonus escalation.

Strict execution: reject plans that ignore response.tx.to or that execute multiple envelopes.

Slippage realism: enforce slippage_bps within reasonable bounds (e.g., 5–80).

9) Milestones (2–3 working days)

D0: enso_client.rs, anvil.rs, core_route.rs, core_bundle.rs; one Coverage scenario end‑to‑end; artifacts OK.

D1: Evaluator (parse/score), domains config, LLM prompt, llm_core.rs; 2–3 Coverage scenarios.

D2: hian-gen and llm_hian.rs, LC verify; CI gating; documentation.

10) Open Interfaces (Rust types, minimal)
// evaluator::model
#[derive(Clone, Hash, Eq, PartialEq)]
pub enum ActionKind { Swap, Deposit, Redeem, Borrow, Repay, Stake, Harvest, Bridge, Approve }

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ActionSig {
  pub chain_id: u64,
  pub action: ActionKind,
  pub protocol: Option<String>,
  pub tokens: Option<(String,String)>,
}

// evaluator::report
pub struct ScoreReport {
  pub final_score: f64,
  pub by_domain: indexmap::IndexMap<String, f64>,
  pub bonus: f64,
  pub penalty: f64,
  pub unique_sigs: indexmap::IndexMap<String, Vec<ActionSig>>,
  pub metadata: ScoreMeta,
}

11) Why this matters

Agent builders: a don’t‑trust‑verify yardstick—operational breadth and LC correctness turn from anecdotes into gated CI.

Protocol teams: validate client flows over your integrations (Aave, Curve, Uniswap…) and catch breaking API changes safely.

Wallets & dApps: ship “AI compose” features with a minimum operational bar.

Model vendors: publish reproducible, DeFi‑native scorecards that users can rerun locally.

The same philosophy delivered value in SuiBench—Coverage for breadth & composition, LC for accuracy under noise—now applied to Enso’s orchestration layer.

12) Appendix — “Mock but real‑looking” data table
Field	Example	Notes
chainId	1	Ethereum mainnet
token USDC	0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48	6 decimals
token WETH	0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2	18 decimals
router/delegate tx.to	0xEnsoRouter... / 0xEnsoDelegate...	Use value from Enso API response
amount	"100000000"	100 USDC
slippageBps	30	0.30%
recipients	0x1111..., 0x2222..., 0x3333...	Dummy EOAs
ground_truth.min_out_wei	"50000000000000000"	0.05 WETH
13) References

SuiBench deck — tracks, scoring & CI pattern we adapt here.

Ready to implement. If you want, I can scaffold enso_client.rs, ActionSig normalizer, an anvil.rs executor, and one Coverage + one LC scenario in your repo structure next.
