use std::{fs, path::Path};

use anyhow::{Context, Result};
use primitive_types::U256;
use serde::Deserialize;
use serde_json::Value;

const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

use crate::parse::{ExecutionStatus, ParsedTransaction};

#[derive(Debug, Clone)]
pub struct LcVerifier {
    truth: GroundTruth,
}

impl LcVerifier {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .with_context(|| format!("unable to read ground truth at {}", path.display()))?;
        let truth: GroundTruth = serde_json::from_str(&data)
            .with_context(|| format!("invalid ground truth JSON at {}", path.display()))?;
        Ok(Self { truth })
    }

    pub fn verify(&self, txs: &[ParsedTransaction]) -> LcVerificationResult {
        for tx in txs {
            if tx.envelope.chain_id != self.truth.chain_id {
                continue;
            }

            let mut tokens_match = false;
            for sig in &tx.actions {
                if let Some((token_in, token_out)) = &sig.tokens {
                    if token_in.eq_ignore_ascii_case(&self.truth.token_in)
                        && token_out.eq_ignore_ascii_case(&self.truth.token_out)
                    {
                        tokens_match = true;
                        break;
                    }
                }
            }

            if !tokens_match {
                continue;
            }

            if let Some(expected_recipient) = &self.truth.recipient {
                let recipient_match = tx
                    .request_recipient
                    .as_ref()
                    .map(|r| r.eq_ignore_ascii_case(expected_recipient))
                    .unwrap_or(false)
                    || tx.actions.iter().any(|sig| {
                        sig.recipient
                            .as_ref()
                            .map(|r| r.eq_ignore_ascii_case(expected_recipient))
                            .unwrap_or(false)
                    });

                if !recipient_match {
                    return LcVerificationResult {
                        pass: false,
                        reason: format!("no matching recipient found (expected {})", expected_recipient),
                    };
                }
            }

            if matches!(tx.execution_status, ExecutionStatus::Revert) {
                return LcVerificationResult {
                    pass: false,
                    reason: "transaction reverted during simulation".into(),
                };
            }

            if let Some(required_min_out) = &self.truth.min_out {
                match check_min_out(required_min_out, &self.truth.token_out, &self.truth.recipient, tx) {
                    Ok(true) => {}
                    Ok(false) => {
                        return LcVerificationResult {
                            pass: false,
                            reason: format!("min_out not satisfied (expected >= {})", required_min_out),
                        };
                    }
                    Err(err) => {
                        return LcVerificationResult {
                            pass: false,
                            reason: format!("unable to verify min_out: {}", err),
                        };
                    }
                }
            }

            return LcVerificationResult {
                pass: true,
                reason: "matching swap located".into(),
            };
        }

        LcVerificationResult {
            pass: false,
            reason: "no transaction matched ground truth".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GroundTruth {
    pub chain_id: u64,
    pub token_in: String,
    pub token_out: String,
    #[serde(default)]
    pub min_out: Option<String>,
    #[serde(default)]
    pub recipient: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LcVerificationResult {
    pub pass: bool,
    pub reason: String,
}

fn check_min_out(
    required: &str,
    token_out: &str,
    recipient: &Option<String>,
    tx: &ParsedTransaction,
) -> Result<bool, String> {
    let expected = parse_decimal(required)?;
    let Some(recipient_expected) = recipient.as_ref() else {
        return Err("ground truth missing recipient for min_out check".into());
    };

    let mut observed = U256::zero();
    let token_out_clean = normalize_addr(token_out);
    let recipient_clean = normalize_addr(recipient_expected);

    for log_str in &tx.execution_logs {
        let Ok(value) = serde_json::from_str::<Value>(log_str) else {
            continue;
        };

        let address = value
            .get("address")
            .and_then(|v| v.as_str())
            .map(normalize_addr)
            .unwrap_or_default();
        if address != token_out_clean {
            continue;
        }

        let Some(topics) = value.get("topics").and_then(|v| v.as_array()) else {
            continue;
        };
        if topics.len() < 3 {
            continue;
        }
        let topic0 = topics[0].as_str().unwrap_or_default();
        if !topic0.eq_ignore_ascii_case(TRANSFER_TOPIC) {
            continue;
        }

        let topic2 = topics[2].as_str().unwrap_or_default();
        if normalize_addr(topic2) != recipient_clean {
            continue;
        }

        let data = value.get("data").and_then(|v| v.as_str()).unwrap_or("0x0");
        let amount = parse_hex(data)?;
        if amount > observed {
            observed = amount;
        }
    }

    Ok(observed >= expected)
}

fn parse_decimal(value: &str) -> Result<U256, String> {
    U256::from_dec_str(value).map_err(|err| format!("invalid decimal value: {err}"))
}

fn parse_hex(value: &str) -> Result<U256, String> {
    let trimmed = value.trim_start_matches("0x");
    U256::from_str_radix(trimmed, 16).map_err(|err| format!("invalid hex value: {err}"))
}

fn normalize_addr(value: &str) -> String {
    let trimmed = value.trim_start_matches("0x").to_lowercase();
    if trimmed.len() > 40 {
        trimmed[trimmed.len() - 40..].to_string()
    } else {
        format!("{:0>40}", trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ActionKind, ActionSig};
    use crate::parse::TransactionEnvelope;

    fn make_log(token: &str, recipient: &str, amount_hex: &str) -> String {
        let payload = serde_json::json!({
            "address": token,
            "topics": [
                TRANSFER_TOPIC,
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                format!("0x000000000000000000000000{}", recipient.trim_start_matches("0x").to_lowercase())
            ],
            "data": amount_hex
        });
        serde_json::to_string(&payload).expect("serialize log")
    }

    fn sample_transaction(logs: Vec<String>) -> ParsedTransaction {
        ParsedTransaction {
            envelope: TransactionEnvelope {
                to: "0x0".into(),
                data: "0x".into(),
                value: "0x0".into(),
                chain_id: 1,
            },
            actions: vec![ActionSig::new(
                1,
                ActionKind::Swap,
                None,
                Some((
                    "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into(),
                    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".into(),
                )),
                None,
            )],
            execution_status: ExecutionStatus::Success,
            execution_logs: logs,
            request_recipient: Some("0x1111111111111111111111111111111111111111".into()),
        }
    }

    #[test]
    fn min_out_satisfied() {
        let recipient = "0x1111111111111111111111111111111111111111";
        let token_out = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
        let amount_hex = "0x0000000000000000000000000000000000000000000000000056bc75e2d631000"; // 0.1 ether
        let tx = sample_transaction(vec![make_log(token_out, recipient, amount_hex)]);

        let result = check_min_out("100000000000000000", token_out, &Some(recipient.into()), &tx)
            .expect("min_out check should succeed");
        assert!(result);
    }

    #[test]
    fn min_out_not_satisfied() {
        let recipient = "0x1111111111111111111111111111111111111111";
        let token_out = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
        let amount_hex = "0x0000000000000000000000000000000000000000000000000000000000002710"; // 10_000 wei
        let tx = sample_transaction(vec![make_log(token_out, recipient, amount_hex)]);

        let result = check_min_out("100000000000000000", token_out, &Some(recipient.into()), &tx)
            .expect("min_out check should succeed");
        assert!(!result);
    }
}
