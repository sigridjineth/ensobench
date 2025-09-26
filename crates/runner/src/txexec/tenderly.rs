use url::Url;

use crate::{artifacts::ExecutionRecord, enso_client::TransactionEnvelope, error::RunnerError};

#[derive(Debug, Clone)]
pub struct TenderlyExecutorConfig {
    pub project_slug: String,
    pub api_key: String,
    pub account: String,
    pub base_url: Url,
}

#[derive(Clone)]
pub struct TenderlyExecutor {
    pub config: TenderlyExecutorConfig,
}

impl TenderlyExecutor {
    pub fn new(config: TenderlyExecutorConfig) -> Self {
        Self { config }
    }

    pub async fn simulate(
        &self,
        _envelope: &TransactionEnvelope,
        _label: impl Into<String>,
    ) -> Result<ExecutionRecord, RunnerError> {
        Err(RunnerError::Executor(
            "Tenderly backend is not yet implemented. Enable Anvil execution instead.".to_string(),
        ))
    }
}
