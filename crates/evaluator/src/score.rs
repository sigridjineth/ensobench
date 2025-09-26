use indexmap::{IndexMap, IndexSet};

use crate::{
    config::DomainsConfig,
    model::{ActionKind, ActionSig, ScoreMeta, ScoreReport},
    parse::{ExecutionStatus, ParsedTransaction},
};

const COMPOSITION_BONUS_PER_EXTRA_ACTION: f64 = 0.25;

pub fn score(transactions: &[ParsedTransaction], domains: &DomainsConfig) -> ScoreReport {
    let mut domain_sets: IndexMap<String, IndexSet<ActionSig>> = IndexMap::new();
    let mut bonus = 0.0;
    let mut counted_transactions = 0usize;

    for tx in transactions {
        if matches!(tx.execution_status, ExecutionStatus::Revert) {
            continue;
        }

        counted_transactions += 1;

        let mut kinds_in_tx: IndexSet<ActionKind> = IndexSet::new();
        for action in &tx.actions {
            kinds_in_tx.insert(action.action.clone());
            let domain = domains
                .domain_for_kind(&action.action)
                .unwrap_or_else(|| "unclassified".to_string());
            domain_sets.entry(domain).or_default().insert(action.clone());
        }

        if !kinds_in_tx.is_empty() {
            let unique = kinds_in_tx.len().min(domains.per_tx_action_limit);
            if unique > 1 {
                bonus += COMPOSITION_BONUS_PER_EXTRA_ACTION * (unique as f64 - 1.0);
            }
        }
    }

    let mut by_domain = IndexMap::new();
    let mut unique_sigs = IndexMap::new();
    let mut base = 0.0;

    for (domain, set) in domain_sets {
        let weight = domains.domains.get(&domain).map(|cfg| cfg.weight).unwrap_or(1.0);
        let score = weight * set.len() as f64;
        base += score;
        by_domain.insert(domain.clone(), score);
        unique_sigs.insert(domain, set.into_iter().collect());
    }

    let final_score = base + bonus; // penalty hooks in future

    let unique_total: usize = unique_sigs
        .values()
        .map(|v: &Vec<ActionSig>| v.len())
        .sum::<usize>();

    ScoreReport {
        final_score,
        by_domain,
        bonus,
        penalty: 0.0,
        unique_sigs,
        metadata: ScoreMeta {
            total_transactions: counted_transactions,
            unique_action_signatures: unique_total,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DomainConfig, DomainsConfig};
    use crate::model::{ActionKind, ActionSig};
    use crate::parse::{ExecutionStatus, ParsedTransaction};
    use indexmap::IndexMap;

    #[test]
    fn applies_domain_weights_and_bonus() {
        let domains = DomainsConfig {
            version: "0.1.0".into(),
            per_tx_action_limit: 6,
            domains: IndexMap::from([
                (
                    "dex".into(),
                    DomainConfig {
                        weight: 1.0,
                        allow: vec![super::super::config::AllowEntry {
                            action: ActionKind::Swap,
                        }],
                    },
                ),
                (
                    "lending".into(),
                    DomainConfig {
                        weight: 1.25,
                        allow: vec![super::super::config::AllowEntry {
                            action: ActionKind::Deposit,
                        }],
                    },
                ),
            ]),
        };

        let tx = ParsedTransaction {
            envelope: crate::parse::TransactionEnvelope {
                to: "0x0".into(),
                data: "0x".into(),
                value: "0".into(),
                chain_id: 1,
            },
            actions: vec![
                ActionSig::new(1, ActionKind::Swap, Some("uniswap".into()), None, None),
                ActionSig::new(1, ActionKind::Deposit, Some("aave".into()), None, None),
            ],
            execution_status: ExecutionStatus::Success,
            execution_logs: Vec::new(),
            request_recipient: None,
        };

        let report = score(&[tx], &domains);
        assert!(report.final_score > 1.0);
        assert!(report.bonus > 0.0);
    }
}
