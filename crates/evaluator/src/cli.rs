use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};

use serde_json;

use crate::{config::DomainsConfig, lc_verify::LcVerifier, parse::load_transactions, score::score};

#[derive(Parser, Debug)]
#[command(author, version, about = "EnsoBench evaluator")]
pub struct EvaluatorCli {
    /// Path to per_tx.jsonl artifact
    #[arg(long)]
    pub per_tx: PathBuf,
    /// Path to domains configuration YAML
    #[arg(long, default_value = "dataset/domains.enso.yaml")]
    pub domains: PathBuf,
    /// Optional ground truth JSON for LC verification
    #[arg(long)]
    pub lc_ground_truth: Option<PathBuf>,
    /// Output format
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

impl EvaluatorCli {
    pub fn run(&self) -> Result<()> {
        let domains = DomainsConfig::load(&self.domains)?;
        let transactions = load_transactions(&self.per_tx)?;
        let report = score(&transactions, &domains);

        match self.format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
            OutputFormat::Text => println!("Coverage score: {:.2}", report.final_score),
        }

        if let Some(path) = &self.lc_ground_truth {
            let verifier = LcVerifier::from_file(path)?;
            let result = verifier.verify(&transactions);
            match self.format {
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&result)?),
                OutputFormat::Text => {
                    println!("LC verification: {}", if result.pass { "PASS" } else { "FAIL" })
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}
