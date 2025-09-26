use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("configuration error: {0}")]
    Config(#[from] anyhow::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("serialization error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("executor error: {0}")]
    Executor(String),
    #[error("LLM error: {0}")]
    Llm(String),
}

pub type RunnerResult<T> = Result<T, RunnerError>;
