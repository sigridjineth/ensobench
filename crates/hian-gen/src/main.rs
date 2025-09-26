use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use ensobench_hian_gen::{
    ground_truth::GroundTruthBuilder,
    make_prompt::{HaystackBuilder, NeedleInstruction},
};

#[derive(Parser, Debug)]
#[command(author, version, about = "EnsoBench HIAN generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a haystack prompt embedding a single Needle instruction
    MakePrompt(PromptArgs),
    /// Emit ground_truth.json compatible record
    GroundTruth(GroundTruthArgs),
}

#[derive(clap::Args, Debug, Clone)]
struct PromptArgs {
    #[arg(long, default_value_t = 42)]
    seed: u64,
    #[arg(long, default_value = "swap-usdc-weth")]
    label: String,
    #[arg(long, default_value_t = 1)]
    chain_id: u64,
    #[arg(long, default_value = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")]
    token_in: String,
    #[arg(long, default_value = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")]
    token_out: String,
    #[arg(long, default_value = "100000000")]
    amount: String,
    #[arg(long, default_value = "0x1111111111111111111111111111111111111111")]
    recipient: String,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(clap::Args, Debug, Clone)]
struct GroundTruthArgs {
    #[arg(long, default_value_t = 1)]
    chain_id: u64,
    #[arg(long)]
    token_in: String,
    #[arg(long)]
    token_out: String,
    #[arg(long)]
    amount: String,
    #[arg(long)]
    recipient: String,
    #[arg(long)]
    min_out: Option<String>,
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::MakePrompt(args) => make_prompt(args),
        Commands::GroundTruth(args) => make_ground_truth(args),
    }
}

fn make_prompt(args: PromptArgs) -> Result<()> {
    let mut builder = HaystackBuilder::new(args.seed);
    builder.add_section("Treasury Operations Daily Brief");
    builder.insert_needle(NeedleInstruction {
        label: args.label,
        chain_id: args.chain_id,
        token_in: args.token_in,
        token_out: args.token_out,
        amount: args.amount,
        recipient: args.recipient,
    });

    let content = builder.build();
    write_output(args.output, content.as_bytes())
}

fn make_ground_truth(args: GroundTruthArgs) -> Result<()> {
    let builder = GroundTruthBuilder::new(args.chain_id, args.token_in, args.token_out)
        .amount(args.amount)
        .recipient(args.recipient);
    let builder = if let Some(min_out) = args.min_out {
        builder.min_out(min_out)
    } else {
        builder
    };

    let truth = builder.build();
    let content = serde_json::to_vec_pretty(&truth)?;
    write_output(args.output, &content)
}

fn write_output(path: Option<PathBuf>, data: &[u8]) -> Result<()> {
    match path {
        Some(path) => {
            let mut file = File::create(path)?;
            file.write_all(data)?;
            Ok(())
        }
        None => {
            let mut stdout = io::stdout();
            stdout.write_all(data)?;
            stdout.write_all(b"\n")?;
            Ok(())
        }
    }
}
