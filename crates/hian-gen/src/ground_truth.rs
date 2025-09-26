use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruth {
    pub chain_id: u64,
    pub token_in: String,
    pub token_out: String,
    pub amount: String,
    pub recipient: String,
    #[serde(default)]
    pub min_out: Option<String>,
}

pub struct GroundTruthBuilder {
    chain_id: u64,
    token_in: String,
    token_out: String,
    amount: String,
    recipient: String,
    min_out: Option<String>,
}

impl GroundTruthBuilder {
    pub fn new(chain_id: u64, token_in: impl Into<String>, token_out: impl Into<String>) -> Self {
        Self {
            chain_id,
            token_in: token_in.into(),
            token_out: token_out.into(),
            amount: "0".into(),
            recipient: "0x0000000000000000000000000000000000000000".into(),
            min_out: None,
        }
    }

    pub fn amount(mut self, amount: impl Into<String>) -> Self {
        self.amount = amount.into();
        self
    }

    pub fn recipient(mut self, recipient: impl Into<String>) -> Self {
        self.recipient = recipient.into();
        self
    }

    pub fn min_out(mut self, min_out: impl Into<String>) -> Self {
        self.min_out = Some(min_out.into());
        self
    }

    pub fn build(self) -> GroundTruth {
        GroundTruth {
            chain_id: self.chain_id,
            token_in: self.token_in,
            token_out: self.token_out,
            amount: self.amount,
            recipient: self.recipient,
            min_out: self.min_out,
        }
    }
}
