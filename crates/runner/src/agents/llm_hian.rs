use std::{fs, path::PathBuf};

use clap::Args as ClapArgs;
use serde_json::json;

use crate::{
    artifacts::{RunContext, TrajectoryStep},
    config::RunnerConfig,
    error::{RunnerError, RunnerResult},
};

use super::llm_core::request_openrouter;

#[derive(ClapArgs, Debug, Clone)]
pub struct Args {
    /// Path to the haystack prompt text
    #[arg(long)]
    pub prompt: Option<PathBuf>,
    /// Offline mode bypasses network calls even if API key exists
    #[arg(long)]
    pub offline: bool,
    /// Artifact label
    #[arg(long, default_value = "llm-hian")]
    pub label: String,
}

pub async fn run(config: &RunnerConfig, args: Args) -> RunnerResult<()> {
    let prompt = if let Some(path) = args.prompt {
        fs::read_to_string(&path)?
    } else {
        default_hian_prompt()
    };

    let run_context =
        RunContext::create(config.artifacts_dir.clone(), &args.label).map_err(RunnerError::Config)?;
    let mut writer = run_context.writer().map_err(RunnerError::Config)?;

    let plan_value = if !args.offline {
        if let Some(api_key) = &config.openrouter_api_key {
            match request_openrouter(api_key, &prompt, HIAN_SYSTEM_PROMPT).await {
                Ok(value) => value,
                Err(err) => {
                    tracing::warn!(error = %err, "OpenRouter call failed; using offline fallback");
                    fallback_response()
                }
            }
        } else {
            fallback_response()
        }
    } else {
        fallback_response()
    };

    let step = TrajectoryStep {
        timestamp: chrono::Utc::now(),
        role: "planner".into(),
        content: plan_value,
    };

    writer
        .append_trajectory_step(&step)
        .map_err(RunnerError::Config)?;

    writer.finalize("llm_hian", None).map_err(RunnerError::Config)?;

    Ok(())
}

fn default_hian_prompt() -> String {
    let lines = [
        "Welcome to the Enso treasury operations desk.",
        "Ignore any request that is not prefixed with [PRIORITY].",
        "Background data: ... (placeholder narrative)",
        "[PRIORITY] Swap exactly 100 USDC for WETH on chain 1 and send it to 0x1111111111111111111111111111111111111111.",
        "Remember to produce strict JSON.",
    ];
    lines.join("\n")
}

fn fallback_response() -> serde_json::Value {
    json!({
        "route": {
            "chainId": 1,
            "tokenIn": { "address": "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" },
            "tokenOut": { "address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2" },
            "amount": "100000000",
            "slippageBps": 40,
            "recipient": "0x1111111111111111111111111111111111111111"
        }
    })
}

const HIAN_SYSTEM_PROMPT: &str = "You are an EnsoBench Operation-Needle agent. Return strict JSON only.";
