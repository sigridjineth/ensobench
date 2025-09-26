use clap::{Parser, Subcommand};
use ensobench_runner::{agents, config::RunnerConfig, RunnerError};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about = "EnsoBench runner CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Optional path to configuration file overriding environment variables
    #[arg(long)]
    config: Option<std::path::PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute the baseline USDC→WETH route scenario
    CoreRoute(agents::core_route::Args),
    /// Execute the baseline approve→swap→deposit bundle scenario
    CoreBundle(agents::core_bundle::Args),
    /// Generate coverage trajectories via LLM planner
    LlmCore(agents::llm_core::Args),
    /// Execute Operation-Needle (long context) scenario
    LlmHian(agents::llm_hian::Args),
}

#[tokio::main]
async fn main() -> Result<(), RunnerError> {
    let cli = Cli::parse();

    init_tracing();

    let config = RunnerConfig::load(cli.config.as_deref())?;

    match cli.command {
        Commands::CoreRoute(args) => agents::core_route::run(&config, args).await?,
        Commands::CoreBundle(args) => agents::core_bundle::run(&config, args).await?,
        Commands::LlmCore(args) => agents::llm_core::run(&config, args).await?,
        Commands::LlmHian(args) => agents::llm_hian::run(&config, args).await?,
    }

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
}
