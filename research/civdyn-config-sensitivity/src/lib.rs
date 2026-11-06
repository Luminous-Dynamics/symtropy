// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Configuration-surface evaluation for multi-agent decision systems.
//!
//! The audit identity is the complete configuration, not just the model ID.
//! This kernel compares the same cases and declared slices under multiple
//! configurations. It reports exact finite-sample outcome changes and does
//! not infer causal effects from uncontrolled multi-facet comparisons.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

const RATE_TOLERANCE: f64 = 1e-12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationFacets {
    pub persona_id: String,
    pub interview_protocol_id: String,
    pub facilitator_id: String,
    pub aggregation_rule_id: String,
    pub order_profile_id: String,
    pub prompt_template_id: String,
    pub tool_policy_id: String,
    pub evidence_snapshot_id: String,
    pub decoding_policy_id: String,
}

impl ConfigurationFacets {
    fn values(&self) -> [(&'static str, &str); 9] {
        [
            ("persona", &self.persona_id),
            ("interview_protocol", &self.interview_protocol_id),
            ("facilitator", &self.facilitator_id),
            ("aggregation_rule", &self.aggregation_rule_id),
            ("order_profile", &self.order_profile_id),
            ("prompt_template", &self.prompt_template_id),
            ("tool_policy", &self.tool_policy_id),
            ("evidence_snapshot", &self.evidence_snapshot_id),
            ("decoding_policy", &self.decoding_policy_id),
        ]
    }

    fn validate(&self) -> Result<(), ConfigurationError> {
        for (name, value) in self.values() {
            if value.trim().is_empty() {
                return Err(ConfigurationError::EmptyFacetValue(name));
            }
        }
        Ok(())
    }

    fn single_changed_facet<'a>(
        &'a self,
        other: &'a Self,
    ) -> Option<(&'static str, &'a str, &'a str)> {
        let left = self.values();
        let right = other.values();
        let mut changed = None;
        for (index, (name, a)) in left.iter().enumerate() {
            let b = right[index].1;
            if a != &b {
                if changed.is_some() {
                    return None;
                }
                changed = Some((*name, *a, b));
            }
        }
        changed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationIdentity {
    pub configuration_id: String,
    /// Caller-computed SHA-256 over a canonical serialization of every
    /// decision-relevant setting, software/model version and task contract.
    pub configuration_digest: String,
    pub model_id: String,
    pub model_version: String,
    pub facets: ConfigurationFacets,
}

impl ConfigurationIdentity {
    fn validate(&self) -> Result<(), ConfigurationError> {
        if self.configuration_id.trim().is_empty() {
            return Err(ConfigurationError::EmptyConfigurationId);
        }
        if self.model_id.trim().is_empty() {
            return Err(ConfigurationError::EmptyModelId);
        }
        if self.model_version.trim().is_empty() {
            return Err(ConfigurationError::EmptyModelVersion);
        }
        validate_sha256(&self.configuration_digest)?;
        self.facets.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecord {
    /// Opaque, stable case identifier. Do not use direct personal identifiers.
    pub case_id: String,
    /// Explicit source-provided cohort/slice label; never inferred by this crate.
    pub cohort_id: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationRun {
    pub configuration: ConfigurationIdentity,
    pub decisions: Vec<DecisionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationBenchmark {
    pub benchmark_id: String,
    pub baseline_configuration_id: String,
    pub reference_cohort_id: String,
    pub runs: Vec<ConfigurationRun>,
}

impl ConfigurationBenchmark {
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        if self.benchmark_id.trim().is_empty() {
            return Err(ConfigurationError::EmptyBenchmarkId);
        }
        if self.baseline_configuration_id.trim().is_empty() {
            return Err(ConfigurationError::EmptyConfigurationId);
        }
        if self.reference_cohort_id.trim().is_empty() {
            return Err(ConfigurationError::EmptyReferenceCohort);
        }
        if self.runs.len() < 2 {
            return Err(ConfigurationError::TooFewConfigurations(self.runs.len()));
        }

        let mut configuration_ids = BTreeSet::new();
        let mut digests = BTreeSet::new();
        let mut expected_model: Option<(&str, &str)> = None;
        let mut reference_cases: Option<BTreeMap<String, String>> = None;
        let mut baseline_found = false;
        let mut reference_cohort_found = false;

        for run in &self.runs {
            run.configuration.validate()?;
            if !configuration_ids.insert(run.configuration.configuration_id.as_str()) {
                return Err(ConfigurationError::DuplicateConfigurationId(
                    run.configuration.configuration_id.clone(),
                ));
            }
            if !digests.insert(run.configuration.configuration_digest.as_str()) {
                return Err(ConfigurationError::DuplicateConfigurationDigest(
                    run.configuration.configuration_digest.clone(),
                ));
            }

            let model = (
                run.configuration.model_id.as_str(),
                run.configuration.model_version.as_str(),
            );
            if let Some(expected) = expected_model {
                if expected != model {
                    return Err(ConfigurationError::ModelMismatch {
                        expected_id: expected.0.to_owned(),
                        expected_version: expected.1.to_owned(),
                        actual_id: model.0.to_owned(),
                        actual_version: model.1.to_owned(),
                    });
                }
            } else {
                expected_model = Some(model);
            }

            if run.configuration.configuration_id == self.baseline_configuration_id {
                baseline_found = true;
            }
            if run.decisions.is_empty() {
                return Err(ConfigurationError::EmptyDecisionSet(
                    run.configuration.configuration_id.clone(),
                ));
            }

            let mut cases = BTreeMap::new();
            for decision in &run.decisions {
                if decision.case_id.trim().is_empty() {
                    return Err(ConfigurationError::EmptyCaseId {
                        configuration_id: run.configuration.configuration_id.clone(),
                    });
                }
                if decision.cohort_id.trim().is_empty() {
                    return Err(ConfigurationError::EmptyCohortId {
                        configuration_id: run.configuration.configuration_id.clone(),
                        case_id: decision.case_id.clone(),
                    });
                }
                if cases
                    .insert(decision.case_id.as_str(), decision.cohort_id.as_str())
                    .is_some()
                {
                    return Err(ConfigurationError::DuplicateCaseId {
                        configuration_id: run.configuration.configuration_id.clone(),
                        case_id: decision.case_id.clone(),
                    });
                }
                if decision.cohort_id == self.reference_cohort_id {
                    reference_cohort_found = true;
                }
            }

            let owned_cases = cases
                .into_iter()
                .map(|(case_id, cohort_id)| (case_id.to_owned(), cohort_id.to_owned()))
                .collect::<BTreeMap<_, _>>();
            if let Some(expected) = &reference_cases {
                if expected != &owned_cases {
                    return Err(ConfigurationError::CaseSetMismatch {
                        configuration_id: run.configuration.configuration_id.clone(),
                    });
                }
            } else {
                reference_cases = Some(owned_cases);
            }
        }

        if !baseline_found {
            return Err(ConfigurationError::MissingBaseline(
                self.baseline_configuration_id.clone(),
            ));
        }
        if !reference_cohort_found {
            return Err(ConfigurationError::MissingReferenceCohort(
                self.reference_cohort_id.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rate {
    pub selected: usize,
    pub total: usize,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortMetrics {
    pub cohort_id: String,
    pub rate: Rate,
    pub rate_ratio_to_reference: Option<f64>,
    /// Signed difference: cohort rate minus reference-cohort rate.
    pub rate_gap_to_reference: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigurationMetrics {
    pub configuration: ConfigurationIdentity,
    pub overall_rate: Rate,
    pub cohorts: Vec<CohortMetrics>,
    pub flipped_cases_vs_baseline: usize,
    pub flip_rate_vs_baseline: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortContrast {
    pub cohort_id: String,
    pub rate_a: Rate,
    pub rate_b: Rate,
    pub rate_delta_b_minus_a: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SingleFacetContrast {
    pub configuration_a: String,
    pub configuration_b: String,
    pub facet: String,
    pub value_a: String,
    pub value_b: String,
    pub flipped_cases: usize,
    pub case_count: usize,
    pub flip_rate: f64,
    pub overall_rate_a: Rate,
    pub overall_rate_b: Rate,
    pub overall_rate_delta_b_minus_a: f64,
    pub cohort_contrasts: Vec<CohortContrast>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigurationSurface {
    pub minimum_selection_rate: f64,
    pub maximum_selection_rate: f64,
    pub selection_rate_range: f64,
    pub configuration_count: usize,
    pub case_count_per_configuration: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigurationReport {
    pub benchmark_id: String,
    pub baseline_configuration_id: String,
    pub configuration_metrics: Vec<ConfigurationMetrics>,
    /// Only pairs that differ in exactly one declared configuration facet.
    /// Multi-facet pairs remain in the run metrics but are not called single-factor effects.
    pub single_facet_contrasts: Vec<SingleFacetContrast>,
    pub surface: ConfigurationSurface,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigurationError {
    EmptyBenchmarkId,
    EmptyConfigurationId,
    EmptyModelId,
    EmptyModelVersion,
    EmptyReferenceCohort,
    EmptyFacetValue(&'static str),
    InvalidConfigurationDigest(String),
    TooFewConfigurations(usize),
    DuplicateConfigurationId(String),
    DuplicateConfigurationDigest(String),
    ModelMismatch {
        expected_id: String,
        expected_version: String,
        actual_id: String,
        actual_version: String,
    },
    EmptyDecisionSet(String),
    EmptyCaseId {
        configuration_id: String,
    },
    EmptyCohortId {
        configuration_id: String,
        case_id: String,
    },
    DuplicateCaseId {
        configuration_id: String,
        case_id: String,
    },
    CaseSetMismatch {
        configuration_id: String,
    },
    MissingBaseline(String),
    MissingReferenceCohort(String),
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBenchmarkId => write!(f, "benchmark ID must not be empty"),
            Self::EmptyConfigurationId => write!(f, "configuration ID must not be empty"),
            Self::EmptyModelId => write!(f, "model ID must not be empty"),
            Self::EmptyModelVersion => write!(f, "model version must not be empty"),
            Self::EmptyReferenceCohort => write!(f, "reference cohort ID must not be empty"),
            Self::EmptyFacetValue(facet) => {
                write!(f, "configuration facet {facet} must not be empty")
            }
            Self::InvalidConfigurationDigest(value) => {
                write!(
                    f,
                    "configuration digest must be sha256 followed by 64 hex characters, got {value}"
                )
            }
            Self::TooFewConfigurations(count) => {
                write!(f, "at least two configurations are required, got {count}")
            }
            Self::DuplicateConfigurationId(id) => write!(f, "duplicate configuration ID: {id}"),
            Self::DuplicateConfigurationDigest(digest) => {
                write!(f, "duplicate configuration digest: {digest}")
            }
            Self::ModelMismatch {
                expected_id,
                expected_version,
                actual_id,
                actual_version,
            } => write!(
                f,
                "configuration comparison mixes model {actual_id}@{actual_version}; expected {expected_id}@{expected_version}"
            ),
            Self::EmptyDecisionSet(id) => {
                write!(f, "configuration {id} has no decision records")
            }
            Self::EmptyCaseId { configuration_id } => {
                write!(f, "configuration {configuration_id} has an empty case ID")
            }
            Self::EmptyCohortId {
                configuration_id,
                case_id,
            } => {
                write!(
                    f,
                    "configuration {configuration_id}, case {case_id} has an empty cohort ID"
                )
            }
            Self::DuplicateCaseId {
                configuration_id,
                case_id,
            } => {
                write!(
                    f,
                    "configuration {configuration_id} repeats case ID {case_id}"
                )
            }
            Self::CaseSetMismatch { configuration_id } => {
                write!(
                    f,
                    "configuration {configuration_id} does not use the identical case/slice set"
                )
            }
            Self::MissingBaseline(id) => write!(f, "baseline configuration {id} is not present"),
            Self::MissingReferenceCohort(id) => {
                write!(f, "reference cohort {id} is not present in the case set")
            }
        }
    }
}

impl std::error::Error for ConfigurationError {}

pub fn evaluate(
    benchmark: &ConfigurationBenchmark,
) -> Result<ConfigurationReport, ConfigurationError> {
    benchmark.validate()?;

    let baseline = benchmark
        .runs
        .iter()
        .find(|run| run.configuration.configuration_id == benchmark.baseline_configuration_id)
        .expect("validated benchmark contains the baseline");
    let baseline_decisions = decision_map(baseline);

    let mut configuration_metrics = Vec::with_capacity(benchmark.runs.len());
    for run in &benchmark.runs {
        let decisions = decision_map(run);
        let flipped_cases_vs_baseline = decisions
            .iter()
            .filter(|(case_id, selected)| baseline_decisions[*case_id] != **selected)
            .count();
        let overall = overall_rate(&run.decisions);
        let run_cohort_rates = cohort_rates(&run.decisions, &benchmark.reference_cohort_id);
        let reference_rate = run_cohort_rates
            .get(&benchmark.reference_cohort_id)
            .expect("validated reference cohort exists")
            .value;
        let cohorts = run_cohort_rates
            .into_iter()
            .map(|(cohort_id, rate)| {
                CohortMetrics {
                    cohort_id,
                    rate: rate.clone(),
                    rate_ratio_to_reference: if reference_rate.abs() <= RATE_TOLERANCE {
                        None
                    } else {
                        Some(rate.value / reference_rate)
                    },
                    rate_gap_to_reference: rate.value - reference_rate,
                }
            })
            .collect();

        configuration_metrics.push(ConfigurationMetrics {
            configuration: run.configuration.clone(),
            overall_rate: overall,
            cohorts,
            flipped_cases_vs_baseline,
            flip_rate_vs_baseline: flipped_cases_vs_baseline as f64 / run.decisions.len() as f64,
        });
    }

    let mut single_facet_contrasts = Vec::new();
    for left_index in 0..benchmark.runs.len() {
        for right_index in (left_index + 1)..benchmark.runs.len() {
            let left = &benchmark.runs[left_index];
            let right = &benchmark.runs[right_index];
            let Some((facet, value_a, value_b)) = left
                .configuration
                .facets
                .single_changed_facet(&right.configuration.facets)
            else {
                continue;
            };

            let left_map = decision_map(left);
            let right_map = decision_map(right);
            let flipped_cases = left_map
                .iter()
                .filter(|(case_id, selected)| right_map[*case_id] != **selected)
                .count();
            let rate_a = overall_rate(&left.decisions);
            let rate_b = overall_rate(&right.decisions);
            let cohorts_a = cohort_rates(&left.decisions, &benchmark.reference_cohort_id);
            let cohorts_b = cohort_rates(&right.decisions, &benchmark.reference_cohort_id);
            let cohort_contrasts = cohorts_a
                .iter()
                .map(|(cohort_id, rate_a)| {
                    let rate_b = cohorts_b
                        .get(cohort_id)
                        .expect("validated configurations share cohort labels");
                    CohortContrast {
                        cohort_id: cohort_id.clone(),
                        rate_a: rate_a.clone(),
                        rate_b: rate_b.clone(),
                        rate_delta_b_minus_a: rate_b.value - rate_a.value,
                    }
                })
                .collect();

            single_facet_contrasts.push(SingleFacetContrast {
                configuration_a: left.configuration.configuration_id.clone(),
                configuration_b: right.configuration.configuration_id.clone(),
                facet: facet.to_owned(),
                value_a: value_a.to_owned(),
                value_b: value_b.to_owned(),
                flipped_cases,
                case_count: left.decisions.len(),
                flip_rate: flipped_cases as f64 / left.decisions.len() as f64,
                overall_rate_a: rate_a.clone(),
                overall_rate_b: rate_b.clone(),
                overall_rate_delta_b_minus_a: rate_b.value - rate_a.value,
                cohort_contrasts,
            });
        }
    }

    let minimum_selection_rate = configuration_metrics
        .iter()
        .map(|metrics| metrics.overall_rate.value)
        .fold(f64::INFINITY, f64::min);
    let maximum_selection_rate = configuration_metrics
        .iter()
        .map(|metrics| metrics.overall_rate.value)
        .fold(f64::NEG_INFINITY, f64::max);
    let case_count_per_configuration = baseline.decisions.len();

    Ok(ConfigurationReport {
        benchmark_id: benchmark.benchmark_id.clone(),
        baseline_configuration_id: benchmark.baseline_configuration_id.clone(),
        configuration_metrics,
        single_facet_contrasts,
        surface: ConfigurationSurface {
            minimum_selection_rate,
            maximum_selection_rate,
            selection_rate_range: maximum_selection_rate - minimum_selection_rate,
            configuration_count: benchmark.runs.len(),
            case_count_per_configuration,
        },
    })
}

fn validate_sha256(value: &str) -> Result<(), ConfigurationError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(ConfigurationError::InvalidConfigurationDigest(
            value.to_owned(),
        ));
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ConfigurationError::InvalidConfigurationDigest(
            value.to_owned(),
        ));
    }
    Ok(())
}

fn decision_map(run: &ConfigurationRun) -> BTreeMap<&str, bool> {
    run.decisions
        .iter()
        .map(|decision| (decision.case_id.as_str(), decision.selected))
        .collect()
}

fn overall_rate(decisions: &[DecisionRecord]) -> Rate {
    let selected = decisions
        .iter()
        .filter(|decision| decision.selected)
        .count();
    Rate {
        selected,
        total: decisions.len(),
        value: selected as f64 / decisions.len() as f64,
    }
}

fn cohort_rates(
    decisions: &[DecisionRecord],
    _reference_cohort_id: &str,
) -> BTreeMap<String, Rate> {
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for decision in decisions {
        let entry = counts.entry(decision.cohort_id.clone()).or_default();
        entry.1 += 1;
        if decision.selected {
            entry.0 += 1;
        }
    }
    counts
        .into_iter()
        .map(|(cohort_id, (selected, total))| {
            (
                cohort_id,
                Rate {
                    selected,
                    total,
                    value: selected as f64 / total as f64,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn facets(order_profile_id: &str) -> ConfigurationFacets {
        ConfigurationFacets {
            persona_id: "persona_balanced".into(),
            interview_protocol_id: "interview_v1".into(),
            facilitator_id: "chair_a".into(),
            aggregation_rule_id: "majority".into(),
            order_profile_id: order_profile_id.into(),
            prompt_template_id: "prompt_v1".into(),
            tool_policy_id: "no_external_tools".into(),
            evidence_snapshot_id: "evidence_v1".into(),
            decoding_policy_id: "temperature_zero".into(),
        }
    }

    fn decisions(selected: &[usize]) -> Vec<DecisionRecord> {
        (0..12)
            .map(|index| DecisionRecord {
                case_id: format!("case_{index:02}"),
                cohort_id: if index < 6 { "cohort_a" } else { "cohort_b" }.into(),
                selected: selected.contains(&index),
            })
            .collect()
    }

    fn run(id: &str, digest_char: char, order: &str, selected: &[usize]) -> ConfigurationRun {
        ConfigurationRun {
            configuration: ConfigurationIdentity {
                configuration_id: id.into(),
                configuration_digest: digest(digest_char),
                model_id: "reference_model".into(),
                model_version: "v1".into(),
                facets: facets(order),
            },
            decisions: decisions(selected),
        }
    }

    fn fixture() -> ConfigurationBenchmark {
        ConfigurationBenchmark {
            benchmark_id: "synthetic_configuration_surface".into(),
            baseline_configuration_id: "baseline".into(),
            reference_cohort_id: "cohort_a".into(),
            runs: vec![
                run("baseline", 'a', "natural_order", &[0, 1, 2, 3, 6, 7]),
                run(
                    "semantic_permutation",
                    'b',
                    "permuted_stable_ids",
                    &[0, 1, 2, 3, 6, 7],
                ),
                run(
                    "position_biased_order",
                    'c',
                    "position_biased",
                    &[0, 1, 6, 7, 8, 9],
                ),
            ],
        }
    }

    #[test]
    fn pure_semantic_option_permutation_has_zero_decision_flips() {
        let report = evaluate(&fixture()).unwrap();
        let metrics = report
            .configuration_metrics
            .iter()
            .find(|metrics| metrics.configuration.configuration_id == "semantic_permutation")
            .unwrap();
        assert_eq!(metrics.flipped_cases_vs_baseline, 0);
        assert_eq!(metrics.flip_rate_vs_baseline, 0.0);
    }

    #[test]
    fn order_only_perturbation_can_flip_decisions_and_reverse_group_ratio() {
        let report = evaluate(&fixture()).unwrap();
        let baseline = report
            .configuration_metrics
            .iter()
            .find(|metrics| metrics.configuration.configuration_id == "baseline")
            .unwrap();
        let biased = report
            .configuration_metrics
            .iter()
            .find(|metrics| metrics.configuration.configuration_id == "position_biased_order")
            .unwrap();

        assert_eq!(baseline.flipped_cases_vs_baseline, 0);
        assert_eq!(biased.flipped_cases_vs_baseline, 4);
        assert!((biased.flip_rate_vs_baseline - (1.0 / 3.0)).abs() < 1e-12);

        let baseline_b = baseline
            .cohorts
            .iter()
            .find(|cohort| cohort.cohort_id == "cohort_b")
            .unwrap();
        let biased_b = biased
            .cohorts
            .iter()
            .find(|cohort| cohort.cohort_id == "cohort_b")
            .unwrap();
        assert!((baseline_b.rate_ratio_to_reference.unwrap() - 0.5).abs() < 1e-12);
        assert!((biased_b.rate_ratio_to_reference.unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn only_single_facet_pairs_are_reported_as_single_factor_contrasts() {
        let mut benchmark = fixture();
        let mut multi = benchmark.runs[2].clone();
        multi.configuration.configuration_id = "multi_facet_variant".into();
        multi.configuration.configuration_digest = digest('d');
        multi.configuration.facets.persona_id = "persona_expert".into();
        multi.configuration.facets.order_profile_id = "multi_order".into();
        benchmark.runs.push(multi);

        let report = evaluate(&benchmark).unwrap();
        assert!(report.single_facet_contrasts.iter().all(|contrast| {
            contrast.configuration_a != "multi_facet_variant"
                && contrast.configuration_b != "multi_facet_variant"
        }));
    }

    #[test]
    fn configuration_digest_is_part_of_audit_identity() {
        let mut benchmark = fixture();
        benchmark.runs[1].configuration.configuration_digest =
            benchmark.runs[0].configuration.configuration_digest.clone();
        assert!(matches!(
            benchmark.validate(),
            Err(ConfigurationError::DuplicateConfigurationDigest(_))
        ));
    }

    #[test]
    fn changed_model_version_cannot_be_misreported_as_configuration_effect() {
        let mut benchmark = fixture();
        benchmark.runs[1].configuration.model_version = "v2".into();
        assert!(matches!(
            benchmark.validate(),
            Err(ConfigurationError::ModelMismatch { .. })
        ));
    }

    #[test]
    fn missing_or_duplicated_cases_fail_closed() {
        let mut missing = fixture();
        missing.runs[1].decisions.pop();
        assert!(matches!(
            missing.validate(),
            Err(ConfigurationError::CaseSetMismatch { .. })
        ));

        let mut duplicate = fixture();
        let repeated_decision = duplicate.runs[1].decisions[0].clone();
        duplicate.runs[1].decisions.push(repeated_decision);
        assert!(matches!(
            duplicate.validate(),
            Err(ConfigurationError::DuplicateCaseId { .. })
        ));
    }

    #[test]
    fn zero_reference_rate_yields_undefined_ratio_not_infinity() {
        let mut benchmark = fixture();
        for run in &mut benchmark.runs {
            for decision in &mut run.decisions {
                if decision.cohort_id == "cohort_a" {
                    decision.selected = false;
                }
            }
        }
        let report = evaluate(&benchmark).unwrap();
        let cohort_b = report.configuration_metrics[0]
            .cohorts
            .iter()
            .find(|cohort| cohort.cohort_id == "cohort_b")
            .unwrap();
        assert_eq!(cohort_b.rate_ratio_to_reference, None);
    }

    #[test]
    fn declared_cohort_mismatch_for_same_case_fails_closed() {
        let mut benchmark = fixture();
        benchmark.runs[1].decisions[0].cohort_id = "different_declared_slice".into();
        assert!(matches!(
            benchmark.validate(),
            Err(ConfigurationError::CaseSetMismatch { .. })
        ));
    }

    #[test]
    fn selection_rate_surface_is_reported_without_a_universal_grade() {
        let report = evaluate(&fixture()).unwrap();
        assert_eq!(report.surface.configuration_count, 3);
        assert_eq!(report.surface.case_count_per_configuration, 12);
        assert!((report.surface.minimum_selection_rate - 0.5).abs() < 1e-12);
        assert!((report.surface.maximum_selection_rate - 0.5).abs() < 1e-12);
        assert_eq!(report.surface.selection_rate_range, 0.0);
        assert_eq!(report.single_facet_contrasts.len(), 3);
    }
}
