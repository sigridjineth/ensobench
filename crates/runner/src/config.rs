use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub enso_base_url: Url,
    pub enso_api_key: String,
    pub artifacts_dir: PathBuf,
    pub default_fork_urls: HashMap<u64, Url>,
    pub openrouter_api_key: Option<String>,
}

impl RunnerConfig {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let file_cfg = if let Some(path) = path {
            Self::read_file(path)?
        } else {
            FileConfig::default()
        };

        let enso_api_key = std::env::var("ENSO_API_KEY")
            .ok()
            .or_else(|| file_cfg.enso.api_key.clone())
            .context("ENSO_API_KEY environment variable or config entry is required")?;

        let base_url = std::env::var("ENSO_BASE_URL")
            .ok()
            .or_else(|| file_cfg.enso.base_url.clone())
            .unwrap_or_else(|| "https://api.enso.finance".to_string());

        let enso_base_url = Url::parse(&base_url).context("Invalid ENSO_BASE_URL")?;

        let artifacts_dir = std::env::var("ENSO_ARTIFACTS_DIR")
            .ok()
            .map(PathBuf::from)
            .or(file_cfg.artifacts_dir.clone())
            .unwrap_or_else(|| PathBuf::from("runs"));

        let mut default_fork_urls = file_cfg.forks.unwrap_or_default();

        for (key, value) in std::env::vars() {
            if let Some(chain_id) = key.strip_prefix("ENSO_FORK_URL_") {
                if let Ok(id) = chain_id.parse::<u64>() {
                    if let Ok(url) = Url::parse(&value) {
                        default_fork_urls.insert(id, url);
                    }
                }
            }
        }

        let openrouter_api_key = std::env::var("OPENROUTER_API_KEY")
            .ok()
            .or(file_cfg.openrouter_api_key.clone());

        Ok(Self {
            enso_base_url,
            enso_api_key,
            artifacts_dir,
            default_fork_urls,
            openrouter_api_key,
        })
    }

    fn read_file(path: &Path) -> Result<FileConfig> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("unable to read config file at {}", path.display()))?;
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
            .unwrap_or(false)
        {
            let cfg: FileConfig = serde_yaml::from_str(&contents)
                .with_context(|| format!("invalid YAML config at {}", path.display()))?;
            Ok(cfg)
        } else {
            let cfg: FileConfig = serde_json::from_str(&contents)
                .with_context(|| format!("invalid JSON config at {}", path.display()))?;
            Ok(cfg)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct FileConfig {
    #[serde(default)]
    enso: EnsoSection,
    #[serde(default)]
    artifacts_dir: Option<PathBuf>,
    #[serde(default)]
    forks: Option<HashMap<u64, Url>>,
    #[serde(default)]
    openrouter_api_key: Option<String>,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            enso: EnsoSection::default(),
            artifacts_dir: Some(PathBuf::from("runs")),
            forks: Some(HashMap::new()),
            openrouter_api_key: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct EnsoSection {
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
}

impl Default for EnsoSection {
    fn default() -> Self {
        Self {
            base_url: Some("https://api.enso.finance".to_string()),
            api_key: None,
        }
    }
}
