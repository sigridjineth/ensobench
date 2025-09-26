use std::{fs, path::PathBuf};

use clap::Args as ClapArgs;
use url::Url;

use crate::{
    artifacts::{ExecutionRecord, ExecutionStatus, RunContext},
    config::RunnerConfig,
    enso_client::{EnsoClient, RouteRequest, TokenRef},
    error::{RunnerError, RunnerResult},
    txexec::{AnvilExecutor, AnvilExecutorConfig},
};

#[derive(ClapArgs, Debug, Clone)]
pub struct Args {
    /// Optional path to a JSON/YAML scenario file overriding the default USDC→WETH swap
    #[arg(long)]
    pub scenario: Option<PathBuf>,
    /// Whether to run the transaction through Anvil for simulation
    #[arg(long)]
    pub simulate: bool,
    /// Optional explicit Anvil fork URL; defaults to config per chain-id
    #[arg(long)]
    pub fork_url: Option<Url>,
    /// Optional label for the artifact folder name
    #[arg(long, default_value = "core-route")]
    pub label: String,
}

pub async fn run(config: &RunnerConfig, args: Args) -> RunnerResult<()> {
    let client = EnsoClient::from_config(config).map_err(RunnerError::Config)?;

    let request = if let Some(path) = args.scenario {
        load_route_request(path)?
    } else {
        default_route_request()
    };

    let run_context =
        RunContext::create(config.artifacts_dir.clone(), &args.label).map_err(RunnerError::Config)?;
    let mut writer = run_context.writer().map_err(RunnerError::Config)?;

    tracing::info!(chain_id = request.chain_id, "requesting /shortcuts/route");
    let response = client.post_route(&request).await?;

    let execution = if args.simulate {
        let fork_url = args
            .fork_url
            .or_else(|| config.default_fork_urls.get(&request.chain_id).cloned());

        let executor = AnvilExecutor::new(AnvilExecutorConfig {
            chain_id: request.chain_id,
            fork_url,
        });

        match executor.execute(&response.tx, "core-route").await {
            Ok(record) => Some(record),
            Err(err) => {
                tracing::warn!(error = %err, "Anvil execution failed; marking as skipped");
                Some(ExecutionRecord {
                    label: "core-route".into(),
                    envelope: response.tx.clone(),
                    status: ExecutionStatus::Skipped,
                    gas_used: None,
                    transaction_hash: None,
                    logs: Vec::new(),
                    traces: Vec::new(),
                })
            }
        }
    } else {
        None
    };

    writer
        .append_route(&request, &response, execution.as_ref())
        .map_err(RunnerError::Config)?;

    writer.finalize("core_route", None).map_err(RunnerError::Config)?;

    Ok(())
}

fn load_route_request(path: PathBuf) -> RunnerResult<RouteRequest> {
    let data = fs::read_to_string(&path)?;
    if matches!(path.extension().and_then(|s| s.to_str()), Some("yaml" | "yml")) {
        Ok(serde_yaml::from_str(&data)?)
    } else {
        Ok(serde_json::from_str(&data)?)
    }
}

fn default_route_request() -> RouteRequest {
    RouteRequest::new(
        1,
        TokenRef::by_address("0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
        TokenRef::by_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        "100000000",
    )
}
