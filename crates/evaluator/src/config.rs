use std::{fs, path::Path};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::model::ActionKind;

#[derive(Debug, Clone, Deserialize)]
pub struct DomainsConfig {
    pub version: String,
    pub per_tx_action_limit: usize,
    pub domains: IndexMap<String, DomainConfig>,
}

impl DomainsConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("unable to read domains file at {}", path.display()))?;
        let config: Self = serde_yaml::from_str(&contents)
            .with_context(|| format!("invalid domains YAML at {}", path.display()))?;
        Ok(config)
    }

    pub fn domain_for_kind(&self, kind: &ActionKind) -> Option<String> {
        self.domains
            .iter()
            .find_map(|(domain, cfg)| cfg.allows(kind).then(|| domain.clone()))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainConfig {
    pub weight: f64,
    pub allow: Vec<AllowEntry>,
}

impl DomainConfig {
    fn allows(&self, kind: &ActionKind) -> bool {
        self.allow.iter().any(|entry| &entry.action == kind)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AllowEntry {
    pub action: ActionKind,
}
