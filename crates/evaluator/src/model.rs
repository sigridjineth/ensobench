use std::{fmt, str::FromStr};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Swap,
    Deposit,
    Redeem,
    Borrow,
    Repay,
    Stake,
    Harvest,
    Claim,
    Bridge,
    Approve,
    Unknown,
}

impl FromStr for ActionKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "swap" => Ok(Self::Swap),
            "deposit" => Ok(Self::Deposit),
            "redeem" => Ok(Self::Redeem),
            "borrow" => Ok(Self::Borrow),
            "repay" => Ok(Self::Repay),
            "stake" => Ok(Self::Stake),
            "harvest" => Ok(Self::Harvest),
            "claim" => Ok(Self::Claim),
            "bridge" => Ok(Self::Bridge),
            "approve" => Ok(Self::Approve),
            _ => Ok(Self::Unknown),
        }
    }
}

impl fmt::Display for ActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
        )
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionSig {
    pub chain_id: u64,
    pub action: ActionKind,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub tokens: Option<(String, String)>,
    #[serde(default)]
    pub recipient: Option<String>,
}

impl ActionSig {
    pub fn new(
        chain_id: u64,
        action: ActionKind,
        protocol: Option<String>,
        tokens: Option<(String, String)>,
        recipient: Option<String>,
    ) -> Self {
        Self {
            chain_id,
            action,
            protocol,
            tokens,
            recipient,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreReport {
    pub final_score: f64,
    pub by_domain: IndexMap<String, f64>,
    pub bonus: f64,
    pub penalty: f64,
    pub unique_sigs: IndexMap<String, Vec<ActionSig>>,
    pub metadata: ScoreMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreMeta {
    pub total_transactions: usize,
    pub unique_action_signatures: usize,
}
