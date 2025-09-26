use std::{fs, path::PathBuf};

use clap::Args as ClapArgs;
use serde_json::json;

use crate::{
    artifacts::{RunContext, TrajectoryStep},
    config::RunnerConfig,
    error::{RunnerError, RunnerResult},
};

#[derive(ClapArgs, Debug, Clone)]
pub struct Args {
    /// Optional prompt override file (JSON or plain text)
    #[arg(long)]
    pub prompt: Option<PathBuf>,
    /// Artifact label
    #[arg(long, default_value = "llm-core")]
    pub label: String,
    /// Use stubbed planner even if OPENROUTER_API_KEY is present
    #[arg(long)]
    pub offline: bool,
}

pub async fn run(config: &RunnerConfig, args: Args) -> RunnerResult<()> {
    let prompt = if let Some(path) = args.prompt {
        fs::read_to_string(&path)?
    } else {
        default_prompt()
    };

    let run_context =
        RunContext::create(config.artifacts_dir.clone(), &args.label).map_err(RunnerError::Config)?;
    let mut writer = run_context.writer().map_err(RunnerError::Config)?;

    let plan_value = if !args.offline {
        if let Some(api_key) = &config.openrouter_api_key {
            match request_openrouter(api_key, &prompt, SYSTEM_PROMPT).await {
                Ok(value) => value,
                Err(err) => {
                    tracing::warn!(error = %err, "OpenRouter call failed; falling back to offline plan");
                    stub_plan()
                }
            }
        } else {
            stub_plan()
        }
    } else {
        stub_plan()
    };

    let step = TrajectoryStep {
        timestamp: chrono::Utc::now(),
        role: "planner".into(),
        content: plan_value,
    };

    writer
        .append_trajectory_step(&step)
        .map_err(RunnerError::Config)?;

    writer.finalize("llm_core", None).map_err(RunnerError::Config)?;

    Ok(())
}

pub(crate) async fn request_openrouter(
    api_key: &str,
    prompt: &str,
    system_prompt: &str,
) -> RunnerResult<serde_json::Value> {
    let client = reqwest::Client::new();
    let body = json!({
        "model": "openai/gpt-4.1-mini",
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": prompt }
        ],
        "response_format": { "type": "json_object" }
    });

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://github.com/enso-org/ensobench")
        .header("X-Title", "EnsoBench Runner")
        .json(&body)
        .send()
        .await
        .map_err(|err| RunnerError::Llm(format!("send error: {err}")))?
        .error_for_status()
        .map_err(|err| RunnerError::Llm(format!("OpenRouter HTTP error: {err}")))?;

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|err| RunnerError::Llm(format!("decode error: {err}")))?;

    Ok(value)
}

fn default_prompt() -> String {
    "Plan a coverage transaction using Enso Shortcuts that swaps 100 USDC to WETH on chain 1.".to_string()
}

const SYSTEM_PROMPT: &str = "You are an EnsoBench planner. Respond with strict JSON.";

fn stub_plan() -> serde_json::Value {
    json!({
        "route": {
            "chainId": 1,
            "tokenIn": { "address": "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" },
            "tokenOut": { "address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2" },
            "amount": "100000000",
            "slippageBps": 30,
            "recipient": "0x0000000000000000000000000000000000000000"
        }
    })
}
