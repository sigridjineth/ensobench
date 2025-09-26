use std::collections::HashMap;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{config::RunnerConfig, error::RunnerError};

#[derive(Clone)]
pub struct EnsoClient {
    http: reqwest::Client,
    base_url: Url,
    api_key: String,
}

impl EnsoClient {
    pub fn from_config(config: &RunnerConfig) -> Result<Self> {
        Self::new(config.enso_base_url.clone(), config.enso_api_key.clone())
    }

    pub fn new(base_url: Url, api_key: impl Into<String>) -> Result<Self> {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .context("invalid ENSO API key for Authorization header")?,
        );

        let http = reqwest::Client::builder().default_headers(headers).build()?;

        Ok(Self {
            http,
            base_url,
            api_key,
        })
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    #[instrument(name = "enso.get_tokens", skip(self))]
    pub async fn get_tokens(&self) -> Result<Vec<TokenMetadata>, RunnerError> {
        let url = self.base_url.join("/api/v1/tokens")?;
        let response = self.http.get(url).send().await?.error_for_status()?;
        let tokens: Vec<TokenMetadata> = response.json().await?;
        Ok(tokens)
    }

    #[instrument(name = "enso.get_balances", skip(self))]
    pub async fn get_wallet_balances(
        &self,
        request: &WalletBalancesRequest,
    ) -> Result<WalletBalancesResponse, RunnerError> {
        let url = self.base_url.join("/api/v1/wallet/balances")?;
        let response = self
            .http
            .get(url)
            .query(&request)
            .send()
            .await?
            .error_for_status()?;
        let balances = response.json().await?;
        Ok(balances)
    }

    #[instrument(name = "enso.post_route", skip(self, request))]
    pub async fn post_route(&self, request: &RouteRequest) -> Result<RouteResponse, RunnerError> {
        let url = self.base_url.join("/shortcuts/route")?;
        let response = self
            .http
            .post(url)
            .json(request)
            .send()
            .await?
            .error_for_status()?;
        let body = response.json().await?;
        Ok(body)
    }

    #[instrument(name = "enso.post_bundle", skip(self, request))]
    pub async fn post_bundle(&self, request: &BundleRequest) -> Result<BoxedBundleResponse, RunnerError> {
        let url = self.base_url.join("/shortcuts/bundle")?;
        let response = self
            .http
            .post(url)
            .json(request)
            .send()
            .await?
            .error_for_status()?;
        let body = response.json().await?;
        Ok(body)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteRequest {
    pub chain_id: u64,
    pub token_in: TokenRef,
    pub token_out: TokenRef,
    pub amount: String,
    #[serde(default)]
    pub slippage_bps: Option<u16>,
    #[serde(default)]
    pub recipient: Option<String>,
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl RouteRequest {
    pub fn new(chain_id: u64, token_in: TokenRef, token_out: TokenRef, amount: impl Into<String>) -> Self {
        Self {
            chain_id,
            token_in,
            token_out,
            amount: amount.into(),
            slippage_bps: Some(30),
            recipient: None,
            extra: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteResponse {
    pub tx: TransactionEnvelope,
    #[serde(default)]
    pub route: Option<RouteMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteMetadata {
    #[serde(default)]
    pub steps: Vec<RouteStep>,
    #[serde(default)]
    pub estimated_gas: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteStep {
    Swap {
        protocol: Option<String>,
        token_in: Option<String>,
        token_out: Option<String>,
        pool: Option<String>,
    },
    Bridge {
        protocol: Option<String>,
        source_chain: Option<u64>,
        destination_chain: Option<u64>,
    },
    Unknown {
        #[serde(flatten)]
        details: HashMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleRequest {
    pub chain_id: u64,
    pub actions: Vec<ActionRequest>,
    #[serde(default)]
    pub recipient: Option<String>,
    #[serde(default)]
    pub routing_strategy: Option<String>,
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ActionRequest {
    Approve {
        token: String,
        spender: String,
    },
    Swap {
        token_in: String,
        token_out: String,
        amount: String,
        #[serde(default)]
        slippage_bps: Option<u16>,
    },
    Deposit {
        protocol: String,
        asset: String,
        #[serde(default)]
        amount_source: Option<String>,
    },
    Borrow {
        protocol: String,
        asset: String,
        amount: String,
    },
    Repay {
        protocol: String,
        asset: String,
        amount: String,
    },
    Stake {
        protocol: String,
        asset: String,
        amount: String,
    },
    Harvest {
        protocol: String,
    },
    Bridge {
        protocol: String,
        amount: String,
        destination_chain: u64,
        recipient: String,
    },
    Custom {
        #[serde(flatten)]
        inner: HashMap<String, serde_json::Value>,
    },
}

pub type BoxedBundleResponse = BundleResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleResponse {
    pub tx: TransactionEnvelope,
    #[serde(default)]
    pub bundle: Option<BundleMetadata>,
    #[serde(default)]
    pub route: Option<RouteMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleMetadata {
    #[serde(default)]
    pub actions: Vec<ActionMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ActionMetadata {
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
        details: HashMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEnvelope {
    pub to: String,
    pub data: String,
    pub value: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(default)]
    pub gas: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRef {
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
}

impl TokenRef {
    pub fn by_address(address: impl Into<String>) -> Self {
        Self {
            address: Some(address.into()),
            symbol: None,
        }
    }

    pub fn by_symbol(symbol: impl Into<String>) -> Self {
        Self {
            address: None,
            symbol: Some(symbol.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_request_defaults_slippage() {
        let req = RouteRequest::new(
            1,
            TokenRef::by_address("0x0"),
            TokenRef::by_address("0x1"),
            "1000",
        );
        assert_eq!(req.slippage_bps, Some(30));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenMetadata {
    pub address: String,
    pub chain_id: u64,
    pub symbol: String,
    pub decimals: u8,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBalancesRequest {
    pub wallet: String,
    #[serde(default)]
    pub use_eoa: Option<bool>,
    #[serde(default)]
    pub chain_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBalancesResponse {
    pub balances: Vec<TokenBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalance {
    pub token: TokenMetadata,
    pub balance: String,
    #[serde(default)]
    pub usd_value: Option<f64>,
}
