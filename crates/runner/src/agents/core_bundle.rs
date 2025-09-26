use std::{fs, path::PathBuf};

use clap::Args as ClapArgs;
use url::Url;

use crate::{
    artifacts::{ExecutionRecord, ExecutionStatus, RunContext},
    config::RunnerConfig,
    enso_client::{ActionRequest, BundleRequest, EnsoClient},
    error::{RunnerError, RunnerResult},
    txexec::{AnvilExecutor, AnvilExecutorConfig},
};

#[derive(ClapArgs, Debug, Clone)]
pub struct Args {
    /// Optional path to JSON/YAML bundle scenario describing actions array
    #[arg(long)]
    pub scenario: Option<PathBuf>,
    /// Run returned tx inside Anvil fork
    #[arg(long)]
    pub simulate: bool,
    /// Override fork URL
    #[arg(long)]
    pub fork_url: Option<Url>,
    /// Artifact label
    #[arg(long, default_value = "core-bundle")]
    pub label: String,
}

pub async fn run(config: &RunnerConfig, args: Args) -> RunnerResult<()> {
    let client = EnsoClient::from_config(config).map_err(RunnerError::Config)?;

    let request = if let Some(path) = args.scenario {
        load_bundle_request(path)?
    } else {
        default_bundle_request()
    };

    let run_context =
        RunContext::create(config.artifacts_dir.clone(), &args.label).map_err(RunnerError::Config)?;
    let mut writer = run_context.writer().map_err(RunnerError::Config)?;

    let response = client.post_bundle(&request).await?;

    let execution = if args.simulate {
        let fork_url = args
            .fork_url
            .or_else(|| config.default_fork_urls.get(&request.chain_id).cloned());

        let executor = AnvilExecutor::new(AnvilExecutorConfig {
            chain_id: request.chain_id,
            fork_url,
        });

        match executor.execute(&response.tx, "core-bundle").await {
            Ok(record) => Some(record),
            Err(err) => {
                tracing::warn!(error = %err, "Anvil execution failed; marking as skipped");
                Some(ExecutionRecord {
                    label: "core-bundle".into(),
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
        .append_bundle(&request, &response, execution.as_ref())
        .map_err(RunnerError::Config)?;

    writer
        .finalize("core_bundle", None)
        .map_err(RunnerError::Config)?;

    Ok(())
}

fn load_bundle_request(path: PathBuf) -> RunnerResult<BundleRequest> {
    let data = fs::read_to_string(&path)?;
    if matches!(path.extension().and_then(|s| s.to_str()), Some("yaml" | "yml")) {
        Ok(serde_yaml::from_str(&data)?)
    } else {
        Ok(serde_json::from_str(&data)?)
    }
}

fn default_bundle_request() -> BundleRequest {
    BundleRequest {
        chain_id: 1,
        actions: vec![
            ActionRequest::Approve {
                token: "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into(),
                spender: "router".into(),
            },
            ActionRequest::Swap {
                token_in: "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into(),
                token_out: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".into(),
                amount: "100000000".into(),
                slippage_bps: Some(30),
            },
            ActionRequest::Deposit {
                protocol: "aave_v3".into(),
                asset: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".into(),
                amount_source: Some("fromPrevious".into()),
            },
        ],
        recipient: None,
        routing_strategy: Some("router".into()),
        extra: Default::default(),
    }
}
