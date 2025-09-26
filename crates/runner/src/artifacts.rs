use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enso_client::{BundleRequest, BundleResponse, RouteRequest, RouteResponse, TransactionEnvelope};

#[derive(Debug, Clone)]
pub struct RunContext {
    pub root: PathBuf,
    pub started_at: DateTime<Utc>,
}

impl RunContext {
    pub fn create(base: PathBuf, label: &str) -> Result<Self> {
        let started_at = Utc::now();
        let dir_name = format!(
            "{}-{}",
            started_at.format("%Y%m%dT%H%M%S"),
            label.replace(['/', ' '], "-")
        );
        let root = base.join(dir_name);
        fs::create_dir_all(&root)
            .with_context(|| format!("unable to create artifact directory at {}", root.display()))?;
        Ok(Self { root, started_at })
    }

    pub fn writer(&self) -> Result<ArtifactWriter> {
        ArtifactWriter::new(self)
    }
}

pub struct ArtifactWriter {
    per_tx: BufWriter<File>,
    trajectory: BufWriter<File>,
    meta_path: PathBuf,
    run_started_at: DateTime<Utc>,
}

impl ArtifactWriter {
    fn new(ctx: &RunContext) -> Result<Self> {
        let per_tx_path = ctx.root.join("per_tx.jsonl");
        let per_tx = BufWriter::new(File::create(&per_tx_path)?);

        let trajectory_path = ctx.root.join("trajectory.jsonl");
        let trajectory = BufWriter::new(File::create(&trajectory_path)?);

        let meta_path = ctx.root.join("meta.json");

        Ok(Self {
            per_tx,
            trajectory,
            meta_path,
            run_started_at: ctx.started_at,
        })
    }

    pub fn append_route(
        &mut self,
        request: &RouteRequest,
        response: &RouteResponse,
        execution: Option<&ExecutionRecord>,
    ) -> Result<()> {
        let artifact = RunArtifact::Route {
            timestamp: Utc::now(),
            request: request.clone(),
            response: response.clone(),
            execution: execution.cloned(),
        };
        Self::write_jsonl(&mut self.per_tx, &artifact)
    }

    pub fn append_bundle(
        &mut self,
        request: &BundleRequest,
        response: &BundleResponse,
        execution: Option<&ExecutionRecord>,
    ) -> Result<()> {
        let artifact = RunArtifact::Bundle {
            timestamp: Utc::now(),
            request: request.clone(),
            response: response.clone(),
            execution: execution.cloned(),
        };
        Self::write_jsonl(&mut self.per_tx, &artifact)
    }

    pub fn append_trajectory_step(&mut self, step: &TrajectoryStep) -> Result<()> {
        Self::write_jsonl(&mut self.trajectory, step)
    }

    pub fn finalize(&mut self, scenario: &str, notes: Option<String>) -> Result<()> {
        let metadata = RunMetadata {
            started_at: self.run_started_at,
            finished_at: Utc::now(),
            scenario: scenario.to_string(),
            notes,
        };

        let file = File::create(&self.meta_path)?;
        serde_json::to_writer_pretty(file, &metadata)?;
        Ok(())
    }

    fn write_jsonl<T: Serialize>(writer: &mut BufWriter<File>, value: &T) -> Result<()> {
        serde_json::to_writer(&mut *writer, value)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunArtifact {
    #[serde(rename = "route")]
    Route {
        timestamp: DateTime<Utc>,
        request: RouteRequest,
        response: RouteResponse,
        execution: Option<ExecutionRecord>,
    },
    #[serde(rename = "bundle")]
    Bundle {
        timestamp: DateTime<Utc>,
        request: BundleRequest,
        response: BundleResponse,
        execution: Option<ExecutionRecord>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub label: String,
    pub envelope: TransactionEnvelope,
    pub status: ExecutionStatus,
    pub gas_used: Option<u64>,
    pub transaction_hash: Option<String>,
    pub logs: Vec<String>,
    #[serde(default)]
    pub traces: Vec<TxTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxTrace {
    pub step: usize,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Revert,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub scenario: String,
    pub notes: Option<String>,
}

impl RunMetadata {}
