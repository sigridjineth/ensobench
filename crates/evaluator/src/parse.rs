use std::{fs::File, io::BufReader, path::Path};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::model::{ActionKind, ActionSig};

pub fn load_transactions(path: impl AsRef<Path>) -> Result<Vec<ParsedTransaction>> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("failed to open artifacts at {}", path.as_ref().display()))?;
    read_transactions(BufReader::new(file))
}

pub fn read_transactions<R: std::io::Read>(reader: R) -> Result<Vec<ParsedTransaction>> {
    let stream = serde_json::Deserializer::from_reader(reader).into_iter::<RawArtifact>();
    let mut transactions = Vec::new();
    for entry in stream {
        let artifact = entry?;
        transactions.push(parse_artifact(artifact));
    }
    Ok(transactions)
}

fn parse_artifact(artifact: RawArtifact) -> ParsedTransaction {
    match artifact {
        RawArtifact::Route {
            request,
            response,
            execution,
        } => {
            let chain_id = response.tx.chain_id;
            let actions = extract_actions_from_route(chain_id, response.route.as_ref());
            ParsedTransaction {
                envelope: response.tx,
                actions,
                execution_status: execution
                    .as_ref()
                    .map(|e| e.status)
                    .unwrap_or(ExecutionStatus::Skipped),
                execution_logs: execution.map(|record| record.logs).unwrap_or_default(),
                request_recipient: extract_recipient(&request),
            }
        }
        RawArtifact::Bundle {
            request,
            response,
            execution,
        } => {
            let chain_id = response.tx.chain_id;
            let mut actions = Vec::new();
            actions.extend(extract_actions_from_bundle(chain_id, response.bundle.as_ref()));
            actions.extend(extract_actions_from_route(chain_id, response.route.as_ref()));
            ParsedTransaction {
                envelope: response.tx,
                actions,
                execution_status: execution
                    .as_ref()
                    .map(|e| e.status)
                    .unwrap_or(ExecutionStatus::Skipped),
                execution_logs: execution.map(|record| record.logs).unwrap_or_default(),
                request_recipient: extract_recipient(&request),
            }
        }
    }
}

fn extract_actions_from_route(chain_id: u64, route: Option<&RouteMetadata>) -> Vec<ActionSig> {
    let mut out = Vec::new();
    if let Some(route) = route {
        for step in &route.steps {
            match step {
                RouteStep::Swap {
                    protocol,
                    token_in,
                    token_out,
                    ..
                } => {
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Swap,
                        protocol.clone(),
                        token_pair(token_in.clone(), token_out.clone()),
                        None,
                    ));
                }
                RouteStep::Bridge {
                    protocol,
                    destination_chain,
                    ..
                } => {
                    out.push(ActionSig::new(
                        destination_chain.unwrap_or(chain_id),
                        ActionKind::Bridge,
                        protocol.clone(),
                        None,
                        None,
                    ));
                }
                RouteStep::Unknown { details } => {
                    let protocol = details
                        .get("protocol")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Unknown,
                        protocol,
                        None,
                        None,
                    ));
                }
            }
        }
    }
    out
}

fn extract_actions_from_bundle(chain_id: u64, bundle: Option<&BundleMetadata>) -> Vec<ActionSig> {
    let mut out = Vec::new();
    if let Some(bundle) = bundle {
        for action in &bundle.actions {
            match action {
                ActionMetadata::Swap {
                    protocol,
                    token_in,
                    token_out,
                } => {
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Swap,
                        protocol.clone(),
                        token_pair(token_in.clone(), token_out.clone()),
                        None,
                    ));
                }
                ActionMetadata::Approve { token, spender } => {
                    let tokens = token.clone().map(|t| (t, spender.clone().unwrap_or_default()));
                    out.push(ActionSig::new(chain_id, ActionKind::Approve, None, tokens, None));
                }
                ActionMetadata::Deposit { protocol, asset } => {
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Deposit,
                        protocol.clone(),
                        asset.clone().map(|a| (a.clone(), a)),
                        None,
                    ));
                }
                ActionMetadata::Borrow { protocol, asset } => {
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Borrow,
                        protocol.clone(),
                        asset.clone().map(|a| (a.clone(), a)),
                        None,
                    ));
                }
                ActionMetadata::Repay { protocol, asset } => {
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Repay,
                        protocol.clone(),
                        asset.clone().map(|a| (a.clone(), a)),
                        None,
                    ));
                }
                ActionMetadata::Stake { protocol, asset } => {
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Stake,
                        protocol.clone(),
                        asset.clone().map(|a| (a.clone(), a)),
                        None,
                    ));
                }
                ActionMetadata::Harvest { protocol } => {
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Harvest,
                        protocol.clone(),
                        None,
                        None,
                    ));
                }
                ActionMetadata::Bridge {
                    protocol,
                    destination_chain,
                    recipient,
                } => {
                    out.push(ActionSig::new(
                        destination_chain.unwrap_or(chain_id),
                        ActionKind::Bridge,
                        protocol.clone(),
                        None,
                        recipient.clone(),
                    ));
                }
                ActionMetadata::Unknown { details } => {
                    let protocol = details
                        .get("protocol")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    out.push(ActionSig::new(
                        chain_id,
                        ActionKind::Unknown,
                        protocol,
                        None,
                        None,
                    ));
                }
            }
        }
    }
    out
}

fn token_pair(a: Option<String>, b: Option<String>) -> Option<(String, String)> {
    match (a, b) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RawArtifact {
    Route {
        request: serde_json::Value,
        response: RouteResponse,
        #[serde(default)]
        execution: Option<ExecutionRecord>,
    },
    Bundle {
        request: serde_json::Value,
        response: BundleResponse,
        #[serde(default)]
        execution: Option<ExecutionRecord>,
    },
}

#[derive(Debug, Deserialize)]
struct RouteResponse {
    tx: TransactionEnvelope,
    #[serde(default)]
    route: Option<RouteMetadata>,
}

#[derive(Debug, Deserialize)]
struct BundleResponse {
    tx: TransactionEnvelope,
    #[serde(default)]
    bundle: Option<BundleMetadata>,
    #[serde(default)]
    route: Option<RouteMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionEnvelope {
    pub to: String,
    pub data: String,
    pub value: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
}

#[derive(Debug, Deserialize)]
struct RouteMetadata {
    #[serde(default)]
    steps: Vec<RouteStep>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RouteStep {
    Swap {
        protocol: Option<String>,
        token_in: Option<String>,
        token_out: Option<String>,
        #[serde(default)]
        _pool: Option<String>,
    },
    Bridge {
        protocol: Option<String>,
        _source_chain: Option<u64>,
        destination_chain: Option<u64>,
    },
    Unknown {
        #[serde(flatten)]
        details: IndexMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Deserialize)]
struct BundleMetadata {
    #[serde(default)]
    actions: Vec<ActionMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ActionMetadata {
    Swap {
        protocol: Option<String>,
        token_in: Option<String>,
        token_out: Option<String>,
    },
    Approve {
        token: Option<String>,
        spender: Option<String>,
    },
    Deposit {
        protocol: Option<String>,
        asset: Option<String>,
    },
    Borrow {
        protocol: Option<String>,
        asset: Option<String>,
    },
    Repay {
        protocol: Option<String>,
        asset: Option<String>,
    },
    Stake {
        protocol: Option<String>,
        asset: Option<String>,
    },
    Harvest {
        protocol: Option<String>,
    },
    Bridge {
        protocol: Option<String>,
        destination_chain: Option<u64>,
        recipient: Option<String>,
    },
    Unknown {
        #[serde(flatten)]
        details: IndexMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Revert,
    Skipped,
}

#[derive(Debug, Deserialize)]
struct ExecutionRecord {
    status: ExecutionStatus,
    #[serde(default)]
    logs: Vec<String>,
}

#[derive(Debug)]
pub struct ParsedTransaction {
    pub envelope: TransactionEnvelope,
    pub actions: Vec<ActionSig>,
    pub execution_status: ExecutionStatus,
    pub execution_logs: Vec<String>,
    pub request_recipient: Option<String>,
}

fn extract_recipient(request: &serde_json::Value) -> Option<String> {
    request
        .get("recipient")
        .and_then(|value| value.as_str())
        .map(|s| s.to_string())
}
