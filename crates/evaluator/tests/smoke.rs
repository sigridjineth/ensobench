use ensobench_evaluator::{config::DomainsConfig, parse::load_transactions, score::score};

#[test]
fn parses_sample_artifact() {
    let transactions =
        load_transactions("docs/examples/core_route/per_tx.jsonl").expect("parse sample artifact");
    let domains = DomainsConfig::load("dataset/domains.enso.yaml").expect("load domains");
    let report = score(&transactions, &domains);
    assert!(report.final_score >= 1.0);
}
