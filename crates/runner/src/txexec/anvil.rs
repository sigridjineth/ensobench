use std::{net::TcpListener, process::Stdio, time::Duration};

use primitive_types::U256;
use serde_json::{json, Value};
use tokio::{
    process::Command,
    time::{sleep, timeout},
};
use tracing::{debug, warn};
use url::Url;

use crate::{
    artifacts::{ExecutionRecord, ExecutionStatus, TxTrace},
    enso_client::TransactionEnvelope,
    error::RunnerError,
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const RECEIPT_POLL_INTERVAL: Duration = Duration::from_millis(250);
const RECEIPT_POLL_ATTEMPTS: usize = 40;

#[derive(Debug, Clone)]
pub struct AnvilExecutorConfig {
    pub chain_id: u64,
    pub fork_url: Option<Url>,
}

pub struct AnvilExecutor {
    config: AnvilExecutorConfig,
}

impl AnvilExecutor {
    pub fn new(config: AnvilExecutorConfig) -> Self {
        Self { config }
    }

    pub async fn execute(
        &self,
        envelope: &TransactionEnvelope,
        label: impl Into<String>,
    ) -> Result<ExecutionRecord, RunnerError> {
        let label = label.into();
        debug!(target = "ensobench::txexec", %label, chain_id = self.config.chain_id, "starting Anvil simulation");

        let port = reserve_port()?;
        let rpc_url = format!("http://127.0.0.1:{}", port);

        let mut cmd = Command::new("anvil");
        cmd.arg("--port")
            .arg(port.to_string())
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--block-time")
            .arg("0")
            .arg("--chain-id")
            .arg(self.config.chain_id.to_string())
            .arg("--silent")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(fork) = &self.config.fork_url {
            cmd.arg("--fork-url").arg(fork.as_str());
        }

        let mut child = cmd
            .spawn()
            .map_err(|err| RunnerError::Executor(format!("failed to spawn anvil: {err}")))?;

        let client = reqwest::Client::new();

        if let Err(err) = wait_for_startup(&client, &rpc_url).await {
            let _ = child.kill().await;
            return Err(err);
        }

        let from = match rpc_call(&client, &rpc_url, "eth_accounts", json!([])).await? {
            Value::Array(accounts) if !accounts.is_empty() => accounts[0]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| RunnerError::Executor("anvil returned malformed account list".into()))?,
            _ => {
                let _ = child.kill().await;
                return Err(RunnerError::Executor(
                    "no unlocked account returned by anvil".into(),
                ));
            }
        };

        let tx_params = json!([{ "from": from, "to": envelope.to, "data": envelope.data, "value": normalize_hex(&envelope.value) }]);
        let tx_hash_value = rpc_call(&client, &rpc_url, "eth_sendTransaction", tx_params).await?;
        let tx_hash = tx_hash_value
            .as_str()
            .ok_or_else(|| RunnerError::Executor("eth_sendTransaction returned non-string hash".into()))?
            .to_string();

        let mut receipt = None;
        for _ in 0..RECEIPT_POLL_ATTEMPTS {
            let value = rpc_call(&client, &rpc_url, "eth_getTransactionReceipt", json!([tx_hash])).await?;

            if !value.is_null() {
                receipt = Some(value);
                break;
            }
            sleep(RECEIPT_POLL_INTERVAL).await;
        }

        let result =
            receipt.ok_or_else(|| RunnerError::Executor("transaction receipt not available".into()))?;

        let status = parse_status(&result)?;
        let gas_used = parse_gas_used(&result);
        let logs = parse_logs(&result);

        let execution_status = if status {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Revert
        };

        let record = ExecutionRecord {
            label,
            envelope: envelope.clone(),
            status: execution_status,
            gas_used,
            transaction_hash: Some(tx_hash.clone()),
            logs,
            traces: vec![TxTrace {
                step: 0,
                detail: format!("transaction executed via Anvil at {rpc_url}"),
            }],
        };

        if let Err(err) = child.kill().await {
            warn!(target = "ensobench::txexec", "failed to kill anvil child: {err}");
        }

        Ok(record)
    }
}

fn reserve_port() -> Result<u16, RunnerError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|err| RunnerError::Executor(format!("failed to reserve port: {err}")))?;
    let port = listener
        .local_addr()
        .map_err(|err| RunnerError::Executor(format!("failed to read port: {err}")))?
        .port();
    drop(listener);
    Ok(port)
}

async fn wait_for_startup(client: &reqwest::Client, url: &str) -> Result<(), RunnerError> {
    let readiness = async {
        loop {
            match rpc_call(client, url, "net_version", json!([])).await {
                Ok(_) => break,
                Err(_) => {
                    sleep(Duration::from_millis(200)).await;
                }
            }
        }
    };

    timeout(STARTUP_TIMEOUT, readiness)
        .await
        .map_err(|_| RunnerError::Executor("timed out waiting for anvil startup".into()))?;
    Ok(())
}

async fn rpc_call(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    params: Value,
) -> Result<Value, RunnerError> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|err| RunnerError::Executor(format!("rpc send error ({method}): {err}")))?
        .error_for_status()
        .map_err(|err| RunnerError::Executor(format!("rpc http error ({method}): {err}")))?;

    let value: Value = response
        .json()
        .await
        .map_err(|err| RunnerError::Executor(format!("rpc decode error ({method}): {err}")))?;

    if let Some(error) = value.get("error") {
        return Err(RunnerError::Executor(format!("rpc error ({method}): {error}")));
    }

    Ok(value.get("result").cloned().unwrap_or(Value::Null))
}

fn parse_status(receipt: &Value) -> Result<bool, RunnerError> {
    let status_value = receipt
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| RunnerError::Executor("receipt missing status".into()))?;

    Ok(status_value.eq_ignore_ascii_case("0x1"))
}

fn parse_gas_used(receipt: &Value) -> Option<u64> {
    receipt
        .get("gasUsed")
        .and_then(Value::as_str)
        .and_then(hex_to_u64)
}

fn parse_logs(receipt: &Value) -> Vec<String> {
    receipt
        .get("logs")
        .and_then(Value::as_array)
        .map(|logs| {
            logs.iter()
                .map(|entry| serde_json::to_string(entry).unwrap_or_else(|_| "<invalid log>".into()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_hex(value: &str) -> String {
    if value.starts_with("0x") || value.starts_with("0X") {
        value.to_string()
    } else {
        match U256::from_dec_str(value) {
            Ok(parsed) => format!("0x{:x}", parsed),
            Err(_) => value.to_string(),
        }
    }
}

fn hex_to_u64(value: &str) -> Option<u64> {
    let trimmed = value.trim_start_matches("0x");
    u64::from_str_radix(trimmed, 16).ok()
}
