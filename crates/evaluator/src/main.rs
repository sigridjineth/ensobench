use clap::Parser;
use ensobench_evaluator::EvaluatorCli;

fn main() -> anyhow::Result<()> {
    let cli = EvaluatorCli::parse();
    cli.run()
}
