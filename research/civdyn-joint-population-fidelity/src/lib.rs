// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub mod cluster_bootstrap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairObservation {
    pub observation_id: String,
    pub independent_unit_id: String,
    /// Declared stratum label; never inferred from free text.
    pub stratum: String,
    pub left_category: String,
    pub right_category: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairFidelityReport {
    pub observed_rows: usize,
    pub simulated_rows: usize,
    pub observed_independent_units: usize,
    pub simulated_independent_units: usize,
    pub left_marginal_tv: f64,
    pub right_marginal_tv: f64,
    pub joint_tv: f64,
    pub observed_mutual_information_nats: f64,
    pub simulated_mutual_information_nats: f64,
    pub mutual_information_abs_gap_nats: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FidelityError {
    EmptyDataset,
    EmptyIdentity {
        field: &'static str,
    },
    DuplicateObservationId {
        id: String,
    },
    MissingStratum {
        stratum: String,
        population: &'static str,
    },
    InsufficientIndependentUnits {
        population: &'static str,
        units: usize,
    },
    InsufficientBootstrapReplicates {
        minimum: usize,
        actual: usize,
    },
}

impl fmt::Display for FidelityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDataset => write!(f, "dataset must contain at least one observation"),
            Self::EmptyIdentity { field } => write!(f, "{field} must not be empty"),
            Self::DuplicateObservationId { id } => write!(f, "duplicate observation id: {id}"),
            Self::MissingStratum {
                stratum,
                population,
            } => {
                write!(
                    f,
                    "stratum {stratum} is missing from {population} population"
                )
            }
            Self::InsufficientIndependentUnits { population, units } => {
                write!(
                    f,
                    "{population} population requires at least two independent units, found {units}"
                )
            }
            Self::InsufficientBootstrapReplicates { minimum, actual } => {
                write!(
                    f,
                    "bootstrap requires at least {minimum} replicates, received {actual}"
                )
            }
        }
    }
}
impl std::error::Error for FidelityError {}

#[derive(Default)]
struct Counts {
    left: BTreeMap<String, u64>,
    right: BTreeMap<String, u64>,
    joint: BTreeMap<(String, String), u64>,
    independent_units: BTreeSet<String>,
    rows: usize,
}

fn validate(rows: &[PairObservation]) -> Result<(), FidelityError> {
    if rows.is_empty() {
        return Err(FidelityError::EmptyDataset);
    }
    let mut ids = BTreeSet::new();
    for row in rows {
        for (field, value) in [
            ("observation_id", row.observation_id.as_str()),
            ("independent_unit_id", row.independent_unit_id.as_str()),
            ("stratum", row.stratum.as_str()),
            ("left_category", row.left_category.as_str()),
            ("right_category", row.right_category.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(FidelityError::EmptyIdentity { field });
            }
        }
        if !ids.insert(row.observation_id.as_str()) {
            return Err(FidelityError::DuplicateObservationId {
                id: row.observation_id.clone(),
            });
        }
    }
    Ok(())
}

fn count_rows(rows: &[PairObservation]) -> Counts {
    let mut c = Counts {
        rows: rows.len(),
        ..Counts::default()
    };
    for row in rows {
        *c.left.entry(row.left_category.clone()).or_default() += 1;
        *c.right.entry(row.right_category.clone()).or_default() += 1;
        *c.joint
            .entry((row.left_category.clone(), row.right_category.clone()))
            .or_default() += 1;
        c.independent_units.insert(row.independent_unit_id.clone());
    }
    c
}

fn total<K: Ord>(counts: &BTreeMap<K, u64>) -> u64 {
    counts.values().sum()
}

fn tv<K: Ord + Clone>(left: &BTreeMap<K, u64>, right: &BTreeMap<K, u64>) -> f64 {
    let nl = total(left) as f64;
    let nr = total(right) as f64;
    let keys: BTreeSet<K> = left.keys().chain(right.keys()).cloned().collect();
    0.5 * keys
        .iter()
        .map(|key| {
            let p = *left.get(key).unwrap_or(&0) as f64 / nl;
            let q = *right.get(key).unwrap_or(&0) as f64 / nr;
            (p - q).abs()
        })
        .sum::<f64>()
}

fn mi(c: &Counts) -> f64 {
    let n = c.rows as f64;
    c.joint
        .iter()
        .map(|((x, y), count)| {
            let pxy = *count as f64 / n;
            let px = c.left[x] as f64 / n;
            let py = c.right[y] as f64 / n;
            pxy * (pxy / (px * py)).ln()
        })
        .sum()
}

fn make_report(o: &Counts, s: &Counts) -> PairFidelityReport {
    let oi = mi(o);
    let si = mi(s);
    PairFidelityReport {
        observed_rows: o.rows,
        simulated_rows: s.rows,
        observed_independent_units: o.independent_units.len(),
        simulated_independent_units: s.independent_units.len(),
        left_marginal_tv: tv(&o.left, &s.left),
        right_marginal_tv: tv(&o.right, &s.right),
        joint_tv: tv(&o.joint, &s.joint),
        observed_mutual_information_nats: oi,
        simulated_mutual_information_nats: si,
        mutual_information_abs_gap_nats: (oi - si).abs(),
    }
}

/// Return marginal TV, joint TV, mutual information and support counts separately.
/// Descriptive only: no p-values, confidence intervals or causal claims.
pub fn compare_pair(
    observed: &[PairObservation],
    simulated: &[PairObservation],
) -> Result<PairFidelityReport, FidelityError> {
    validate(observed)?;
    validate(simulated)?;
    Ok(make_report(&count_rows(observed), &count_rows(simulated)))
}

/// Compute a report for every declared stratum; missing strata fail closed.
pub fn compare_by_stratum(
    observed: &[PairObservation],
    simulated: &[PairObservation],
) -> Result<BTreeMap<String, PairFidelityReport>, FidelityError> {
    validate(observed)?;
    validate(simulated)?;
    let os: BTreeSet<_> = observed.iter().map(|r| r.stratum.as_str()).collect();
    let ss: BTreeSet<_> = simulated.iter().map(|r| r.stratum.as_str()).collect();
    let strata: BTreeSet<_> = os.union(&ss).copied().collect();
    let mut result = BTreeMap::new();

    for stratum in strata {
        let o: Vec<_> = observed
            .iter()
            .filter(|r| r.stratum.as_str() == stratum)
            .cloned()
            .collect();
        let s: Vec<_> = simulated
            .iter()
            .filter(|r| r.stratum.as_str() == stratum)
            .cloned()
            .collect();
        if o.is_empty() {
            return Err(FidelityError::MissingStratum {
                stratum: stratum.to_string(),
                population: "observed",
            });
        }
        if s.is_empty() {
            return Err(FidelityError::MissingStratum {
                stratum: stratum.to_string(),
                population: "simulated",
            });
        }
        result.insert(
            stratum.to_string(),
            make_report(&count_rows(&o), &count_rows(&s)),
        );
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(prefix: &str, pairs: &[(&str, &str)], stratum: &str) -> Vec<PairObservation> {
        pairs
            .iter()
            .enumerate()
            .map(|(i, (left, right))| PairObservation {
                observation_id: format!("{prefix}-obs-{i}"),
                independent_unit_id: format!("{prefix}-unit-{i}"),
                stratum: stratum.to_string(),
                left_category: (*left).to_string(),
                right_category: (*right).to_string(),
            })
            .collect()
    }
    fn correlated() -> Vec<(&'static str, &'static str)> {
        let mut v = vec![("low", "low"); 50];
        v.extend(vec![("high", "high"); 50]);
        v
    }
    fn independent_pairs() -> Vec<(&'static str, &'static str)> {
        let mut v = vec![("low", "low"); 25];
        v.extend([("low", "high"); 25]);
        v.extend([("high", "low"); 25]);
        v.extend([("high", "high"); 25]);
        v
    }

    #[test]
    fn detects_joint_mismatch_when_both_marginals_match() {
        let o = rows("o", &correlated(), "all");
        let s = rows("s", &independent_pairs(), "all");
        let r = compare_pair(&o, &s).expect("fixture valid");
        assert_eq!(r.left_marginal_tv, 0.0);
        assert_eq!(r.right_marginal_tv, 0.0);
        assert!((r.joint_tv - 0.5).abs() < 1e-12);
        assert!((r.observed_mutual_information_nats - std::f64::consts::LN_2).abs() < 1e-12);
        assert!(r.simulated_mutual_information_nats.abs() < 1e-12);
    }

    #[test]
    fn identical_joint_distributions_have_zero_discrepancy() {
        let o = rows("o", &correlated(), "all");
        let s = rows("s", &correlated(), "all");
        let r = compare_pair(&o, &s).expect("fixture valid");
        assert_eq!(r.left_marginal_tv, 0.0);
        assert_eq!(r.right_marginal_tv, 0.0);
        assert_eq!(r.joint_tv, 0.0);
        assert_eq!(r.mutual_information_abs_gap_nats, 0.0);
    }

    #[test]
    fn stratum_reports_do_not_hide_local_association_failure() {
        let mut o = rows("oa", &correlated(), "a");
        o.extend(rows("ob", &correlated(), "b"));
        let mut s = rows("sa", &independent_pairs(), "a");
        s.extend(rows("sb", &correlated(), "b"));
        let r = compare_by_stratum(&o, &s).expect("strata supported");
        assert!((r["a"].joint_tv - 0.5).abs() < 1e-12);
        assert_eq!(r["b"].joint_tv, 0.0);
    }

    #[test]
    fn absent_stratum_fails_closed() {
        let o = rows("o", &correlated(), "all");
        let s = rows("s", &correlated(), "other");
        assert!(matches!(
            compare_by_stratum(&o, &s),
            Err(FidelityError::MissingStratum { .. })
        ));
    }

    #[test]
    fn duplicate_observation_id_is_rejected() {
        let mut o = rows("o", &correlated(), "all");
        o[1].observation_id = o[0].observation_id.clone();
        let s = rows("s", &correlated(), "all");
        assert!(matches!(
            compare_pair(&o, &s),
            Err(FidelityError::DuplicateObservationId { .. })
        ));
    }

    #[test]
    fn missing_category_fails_instead_of_becoming_zero() {
        let mut o = rows("o", &correlated(), "all");
        o[0].right_category.clear();
        let s = rows("s", &correlated(), "all");
        assert!(matches!(
            compare_pair(&o, &s),
            Err(FidelityError::EmptyIdentity {
                field: "right_category"
            })
        ));
    }
}
