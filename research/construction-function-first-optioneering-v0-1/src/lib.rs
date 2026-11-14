//! Evidence-gated, function-first construction option evaluation.
//!
//! The evaluator compares explicit Avoid / Switch / Improve alternatives for the
//! same functional brief. Every required hard gate must have qualified-pass evidence.
//! It reports worst-case scenario metric vectors and an exact Pareto frontier; it
//! deliberately does not invent probabilities or collapse trade-offs into one score.
//!
//! This is a finite synthetic decision oracle. It is not a structural, fire, legal,
//! environmental, safety, accessibility or professional engineering verifier.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptionTier {
    Avoid,
    Switch,
    Improve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetricKey {
    CapitalCostMinorUnits,
    WholeLifeCostMinorUnits,
    EmbodiedCarbonGramsCo2e,
    OperationalCarbonGramsCo2e,
    CrewMinutes,
    DeliveryDurationMinutes,
    ServiceDisruptionMinutes,
    MaterialMassGrams,
    VirginMaterialMassGrams,
    WaterMillilitres,
    EnergyWattHours,
    WasteMassGrams,
    MaintenanceCrewMinutes,
    ReplacementCount,
    ReuseFractionBasisPoints,
    RepairabilityBasisPoints,
    DisassemblyCrewMinutes,
}

impl MetricKey {
    pub const fn lower_is_better(self) -> bool {
        !matches!(self, Self::ReuseFractionBasisPoints | Self::RepairabilityBasisPoints)
    }

    pub const fn unit(self) -> &'static str {
        match self {
            Self::CapitalCostMinorUnits | Self::WholeLifeCostMinorUnits => "currency_minor_units",
            Self::EmbodiedCarbonGramsCo2e | Self::OperationalCarbonGramsCo2e => "gCO2e",
            Self::CrewMinutes
            | Self::DeliveryDurationMinutes
            | Self::ServiceDisruptionMinutes
            | Self::MaintenanceCrewMinutes
            | Self::DisassemblyCrewMinutes => "minute",
            Self::MaterialMassGrams | Self::VirginMaterialMassGrams | Self::WasteMassGrams => "g",
            Self::WaterMillilitres => "mL",
            Self::EnergyWattHours => "Wh",
            Self::ReplacementCount => "count",
            Self::ReuseFractionBasisPoints | Self::RepairabilityBasisPoints => "basis_points",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateResult {
    QualifiedPass { evidence_ref: String },
    QualifiedFail { evidence_ref: String, reason: String },
    Unknown { reason: String },
    Stale { reason: String },
}

impl GateResult {
    fn validate(&self) -> Result<(), OptioningError> {
        match self {
            Self::QualifiedPass { evidence_ref } => {
                require_non_empty("gate_evidence_ref", evidence_ref)
            }
            Self::QualifiedFail {
                evidence_ref,
                reason,
            } => {
                require_non_empty("gate_evidence_ref", evidence_ref)?;
                require_non_empty("gate_failure_reason", reason)
            }
            Self::Unknown { reason } => require_non_empty("unknown_gate_reason", reason),
            Self::Stale { reason } => require_non_empty("stale_gate_reason", reason),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionalBrief {
    pub brief_id: String,
    pub revision_digest: String,
    pub required_gate_ids: Vec<String>,
    pub scenario_ids: Vec<String>,
    pub analysis_horizon_months: u32,
    pub discount_rate_basis_points: u32,
    /// ISO 4217 alphabetic currency code for cost metrics (for example ZAR or USD).
    pub currency_code: String,
    pub benchmark_profile_id: String,
    pub benchmark_profile_digest: String,
    pub fixture_id: String,
    pub fixture_digest: String,
    pub claim_ceiling: String,
}

impl FunctionalBrief {
    fn validate(&self) -> Result<(), OptioningError> {
        require_non_empty("brief_id", &self.brief_id)?;
        require_non_empty("revision_digest", &self.revision_digest)?;
        if self.currency_code.len() != 3
            || !self.currency_code.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err(OptioningError::InvalidCurrencyCode);
        }
        require_non_empty("benchmark_profile_id", &self.benchmark_profile_id)?;
        require_non_empty("benchmark_profile_digest", &self.benchmark_profile_digest)?;
        require_non_empty("fixture_id", &self.fixture_id)?;
        require_non_empty("fixture_digest", &self.fixture_digest)?;
        require_non_empty("claim_ceiling", &self.claim_ceiling)?;

        if self.analysis_horizon_months == 0 {
            return Err(OptioningError::InvalidAnalysisHorizon);
        }
        validate_unique_non_empty("required_gate_id", &self.required_gate_ids)?;
        validate_unique_non_empty("scenario_id", &self.scenario_ids)?;
        if self.required_gate_ids.is_empty() {
            return Err(OptioningError::NoRequiredGates);
        }
        if self.scenario_ids.is_empty() {
            return Err(OptioningError::NoScenarios);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioMetrics {
    pub scenario_id: String,
    pub values: BTreeMap<MetricKey, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryOption {
    pub option_id: String,
    pub functional_brief_id: String,
    /// Exact functional-brief revision this alternative was evaluated against.
    pub brief_revision_digest: String,
    pub tier: OptionTier,
    pub delivery_method: String,
    /// Exact design/model revision unique to this alternative.
    pub design_revision_digest: String,
    /// Every required gate must be present. Additional gates are rejected rather
    /// than silently treated as irrelevant or passed.
    pub gate_results: BTreeMap<String, GateResult>,
    pub scenario_metrics: BTreeMap<String, ScenarioMetrics>,
    pub assumptions_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionDisposition {
    QualifiedFeasible,
    Infeasible { failed_gate_ids: Vec<String> },
    NotEvaluable {
        unknown_gate_ids: Vec<String>,
        stale_gate_ids: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionAssessment {
    pub option_id: String,
    pub tier: OptionTier,
    pub disposition: OptionDisposition,
    pub gate_results: BTreeMap<String, GateResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParetoRelationship {
    pub option_id: String,
    /// Every option listed here is an exact robust Pareto dominator.
    pub dominated_by: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptioningReport {
    pub brief_id: String,
    pub revision_digest: String,
    pub analysis_horizon_months: u32,
    pub discount_rate_basis_points: u32,
    pub currency_code: String,
    pub benchmark_profile_id: String,
    pub benchmark_profile_digest: String,
    pub fixture_id: String,
    pub fixture_digest: String,
    pub claim_ceiling: String,
    pub assessments: Vec<OptionAssessment>,
    pub pareto_option_ids: Vec<String>,
    pub dominated_options: Vec<ParetoRelationship>,
    /// Original per-scenario metric vectors, retained so reviewers can inspect
    /// scenario variation rather than seeing only a worst-case projection.
    pub scenario_metrics_by_option: BTreeMap<String, BTreeMap<String, BTreeMap<MetricKey, u64>>>,
    /// Per-option, per-metric worst case over all declared scenarios.
    /// For lower-is-better metrics this is the maximum; for higher-is-better
    /// metrics this is the minimum. No scenario probability is implied.
    pub worst_case_metrics: BTreeMap<String, BTreeMap<MetricKey, u64>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptioningError {
    EmptyField(&'static str),
    InvalidAnalysisHorizon,
    NoRequiredGates,
    NoScenarios,
    DuplicateIdentifier { field: &'static str, value: String },
    WrongFunctionalBrief { option_id: String },
    WrongBriefRevision { option_id: String },
    InvalidCurrencyCode,
    MissingRequiredGate { option_id: String, gate_id: String },
    UnexpectedGate { option_id: String, gate_id: String },
    MissingScenarioMetrics { option_id: String, scenario_id: String },
    UnexpectedScenarioMetrics { option_id: String, scenario_id: String },
    EmptyMetricVector { option_id: String, scenario_id: String },
    InconsistentMetricSchema { option_id: String, scenario_id: String },
    InconsistentMetricUnits { metric: MetricKey },
    NoOptions,
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), OptioningError> {
    if value.trim().is_empty() {
        return Err(OptioningError::EmptyField(field));
    }
    Ok(())
}

fn validate_unique_non_empty(
    field: &'static str,
    values: &[String],
) -> Result<(), OptioningError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_non_empty(field, value)?;
        if !seen.insert(value.as_str()) {
            return Err(OptioningError::DuplicateIdentifier {
                field,
                value: value.clone(),
            });
        }
    }
    Ok(())
}

/// Return the worst value for one metric across all declared scenarios.
fn worst_case_value(
    key: MetricKey,
    scenario_values: impl Iterator<Item = u64>,
) -> Option<u64> {
    if key.lower_is_better() {
        scenario_values.max()
    } else {
        scenario_values.min()
    }
}

fn validate_option(
    brief: &FunctionalBrief,
    option: &DeliveryOption,
    metric_schema: &mut Option<BTreeSet<MetricKey>>,
) -> Result<OptionAssessment, OptioningError> {
    require_non_empty("option_id", &option.option_id)?;
    require_non_empty("functional_brief_id", &option.functional_brief_id)?;
    require_non_empty("delivery_method", &option.delivery_method)?;
    require_non_empty("option_design_revision_digest", &option.design_revision_digest)?;
    require_non_empty("assumptions_ref", &option.assumptions_ref)?;

    require_non_empty("option_brief_revision_digest", &option.brief_revision_digest)?;
    if option.functional_brief_id != brief.brief_id {
        return Err(OptioningError::WrongFunctionalBrief {
            option_id: option.option_id.clone(),
        });
    }
    if option.brief_revision_digest != brief.revision_digest {
        return Err(OptioningError::WrongBriefRevision {
            option_id: option.option_id.clone(),
        });
    }

    for gate_id in &brief.required_gate_ids {
        let result = option
            .gate_results
            .get(gate_id)
            .ok_or_else(|| OptioningError::MissingRequiredGate {
                option_id: option.option_id.clone(),
                gate_id: gate_id.clone(),
            })?;
        result.validate()?;
    }
    for gate_id in option.gate_results.keys() {
        if !brief.required_gate_ids.contains(gate_id) {
            return Err(OptioningError::UnexpectedGate {
                option_id: option.option_id.clone(),
                gate_id: gate_id.clone(),
            });
        }
    }

    let mut failed_gate_ids = Vec::new();
    let mut unknown_gate_ids = Vec::new();
    let mut stale_gate_ids = Vec::new();
    for gate_id in &brief.required_gate_ids {
        match option
            .gate_results
            .get(gate_id)
            .expect("all required gates validated")
        {
            GateResult::QualifiedPass { .. } => {}
            GateResult::QualifiedFail { .. } => failed_gate_ids.push(gate_id.clone()),
            GateResult::Unknown { .. } => unknown_gate_ids.push(gate_id.clone()),
            GateResult::Stale { .. } => stale_gate_ids.push(gate_id.clone()),
        }
    }

    let mut schema_for_option: Option<BTreeSet<MetricKey>> = None;
    for scenario_id in &brief.scenario_ids {
        let metrics = option
            .scenario_metrics
            .get(scenario_id)
            .ok_or_else(|| OptioningError::MissingScenarioMetrics {
                option_id: option.option_id.clone(),
                scenario_id: scenario_id.clone(),
            })?;
        if metrics.scenario_id != *scenario_id {
            return Err(OptioningError::UnexpectedScenarioMetrics {
                option_id: option.option_id.clone(),
                scenario_id: scenario_id.clone(),
            });
        }
        if metrics.values.is_empty() {
            return Err(OptioningError::EmptyMetricVector {
                option_id: option.option_id.clone(),
                scenario_id: scenario_id.clone(),
            });
        }
        let current_schema: BTreeSet<MetricKey> = metrics.values.keys().copied().collect();
        if let Some(existing_schema) = &schema_for_option {
            if existing_schema != &current_schema {
                return Err(OptioningError::InconsistentMetricSchema {
                    option_id: option.option_id.clone(),
                    scenario_id: scenario_id.clone(),
                });
            }
        } else {
            schema_for_option = Some(current_schema.clone());
        }

        if let Some(global_schema) = metric_schema {
            if global_schema != &current_schema {
                return Err(OptioningError::InconsistentMetricSchema {
                    option_id: option.option_id.clone(),
                    scenario_id: scenario_id.clone(),
                });
            }
        } else {
            *metric_schema = Some(current_schema);
        }
    }
    for scenario_id in option.scenario_metrics.keys() {
        if !brief.scenario_ids.contains(scenario_id) {
            return Err(OptioningError::UnexpectedScenarioMetrics {
                option_id: option.option_id.clone(),
                scenario_id: scenario_id.clone(),
            });
        }
    }

    let disposition = if !failed_gate_ids.is_empty() {
        OptionDisposition::Infeasible { failed_gate_ids }
    } else if !unknown_gate_ids.is_empty() || !stale_gate_ids.is_empty() {
        OptionDisposition::NotEvaluable {
            unknown_gate_ids,
            stale_gate_ids,
        }
    } else {
        OptionDisposition::QualifiedFeasible
    };

    Ok(OptionAssessment {
        option_id: option.option_id.clone(),
        tier: option.tier,
        disposition,
        gate_results: option.gate_results.clone(),
    })
}

fn build_worst_case_profile(
    brief: &FunctionalBrief,
    option: &DeliveryOption,
) -> BTreeMap<MetricKey, u64> {
    let schema = option
        .scenario_metrics
        .get(&brief.scenario_ids[0])
        .expect("validated scenario metrics")
        .values
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();

    schema
        .into_iter()
        .filter_map(|metric| {
            let value = worst_case_value(
                metric,
                brief.scenario_ids.iter().map(|scenario_id| {
                    option.scenario_metrics[scenario_id].values[&metric]
                }),
            )?;
            Some((metric, value))
        })
        .collect()
}

/// A robust Pareto comparison over per-metric worst-case values.
///
/// A dominates B only when A is no worse on every declared metric and strictly
/// better on at least one. Metrics retain their native direction; no weighted
/// sum, arbitrary trade-off coefficient or scenario probability is introduced.
fn dominates(
    candidate: &BTreeMap<MetricKey, u64>,
    other: &BTreeMap<MetricKey, u64>,
) -> bool {
    if candidate.keys().ne(other.keys()) {
        return false;
    }

    let mut strictly_better = false;
    for (metric, candidate_value) in candidate {
        let other_value = other[metric];
        if metric.lower_is_better() {
            if candidate_value > &other_value {
                return false;
            }
            strictly_better |= candidate_value < &other_value;
        } else {
            if candidate_value < &other_value {
                return false;
            }
            strictly_better |= candidate_value > &other_value;
        }
    }
    strictly_better
}

/// Evaluate the finite option catalogue, fail closed on all hard gates, and
/// return an exact robust Pareto frontier over feasible alternatives only.
pub fn evaluate_options(
    brief: &FunctionalBrief,
    options: &[DeliveryOption],
) -> Result<OptioningReport, OptioningError> {
    brief.validate()?;
    if options.is_empty() {
        return Err(OptioningError::NoOptions);
    }

    let mut seen_options = BTreeSet::new();
    let mut metric_schema = None;
    let mut assessments = Vec::with_capacity(options.len());
    for option in options {
        if !seen_options.insert(option.option_id.as_str()) {
            return Err(OptioningError::DuplicateIdentifier {
                field: "option_id",
                value: option.option_id.clone(),
            });
        }
        assessments.push(validate_option(brief, option, &mut metric_schema)?);
    }

    let option_by_id: BTreeMap<&str, &DeliveryOption> = options
        .iter()
        .map(|option| (option.option_id.as_str(), option))
        .collect();
    let assessment_by_id: BTreeMap<&str, &OptionAssessment> = assessments
        .iter()
        .map(|assessment| (assessment.option_id.as_str(), assessment))
        .collect();

    let mut worst_case_metrics = BTreeMap::new();
    let mut scenario_metrics_by_option = BTreeMap::new();
    for assessment in &assessments {
        if assessment.disposition == OptionDisposition::QualifiedFeasible {
            let option = option_by_id[assessment.option_id.as_str()];
            worst_case_metrics.insert(
                assessment.option_id.clone(),
                build_worst_case_profile(brief, option),
            );
            scenario_metrics_by_option.insert(
                assessment.option_id.clone(),
                option
                    .scenario_metrics
                    .iter()
                    .map(|(scenario_id, metrics)| {
                        (scenario_id.clone(), metrics.values.clone())
                    })
                    .collect(),
            );
        }
    }

    let feasible_ids: Vec<String> = assessments
        .iter()
        .filter(|assessment| {
            assessment.disposition == OptionDisposition::QualifiedFeasible
        })
        .map(|assessment| assessment.option_id.clone())
        .collect();

    let mut pareto_option_ids = Vec::new();
    let mut dominated_options = Vec::new();
    for option_id in &feasible_ids {
        let candidate = &worst_case_metrics[option_id];
        let dominators: Vec<String> = feasible_ids
            .iter()
            .filter(|other_id| {
                *other_id != option_id
                    && dominates(&worst_case_metrics[*other_id], candidate)
            })
            .cloned()
            .collect();

        if dominators.is_empty() {
            pareto_option_ids.push(option_id.clone());
        } else {
            dominated_options.push(ParetoRelationship {
                option_id: option_id.clone(),
                dominated_by: dominators,
            });
        }
    }

    pareto_option_ids.sort();
    dominated_options.sort_by(|left, right| left.option_id.cmp(&right.option_id));
    assessments.sort_by(|left, right| left.option_id.cmp(&right.option_id));

    // Keep a defensive assertion close to the report assembly: every Pareto member
    // must be qualified feasible, and every option ID must have a disposition.
    debug_assert!(pareto_option_ids.iter().all(|id| {
        assessment_by_id[id.as_str()].disposition == OptionDisposition::QualifiedFeasible
    }));

    Ok(OptioningReport {
        brief_id: brief.brief_id.clone(),
        revision_digest: brief.revision_digest.clone(),
        analysis_horizon_months: brief.analysis_horizon_months,
        discount_rate_basis_points: brief.discount_rate_basis_points,
        currency_code: brief.currency_code.clone(),
        benchmark_profile_id: brief.benchmark_profile_id.clone(),
        benchmark_profile_digest: brief.benchmark_profile_digest.clone(),
        fixture_id: brief.fixture_id.clone(),
        fixture_digest: brief.fixture_digest.clone(),
        claim_ceiling: brief.claim_ceiling.clone(),
        assessments,
        pareto_option_ids,
        dominated_options,
        scenario_metrics_by_option,
        worst_case_metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brief() -> FunctionalBrief {
        FunctionalBrief {
            brief_id: "housing-service-level-01".to_owned(),
            revision_digest: "brief-revision-01".to_owned(),
            required_gate_ids: vec![
                "structural-adequacy".to_owned(),
                "fire-safety".to_owned(),
                "accessibility".to_owned(),
            ],
            scenario_ids: vec![
                "normal".to_owned(),
                "supply-delay".to_owned(),
                "maintenance-stress".to_owned(),
            ],
            analysis_horizon_months: 600,
            discount_rate_basis_points: 350,
            currency_code: "ZAR".to_owned(),
            benchmark_profile_id: "FUNCTION-FIRST-OPTIONEERING-001".to_owned(),
            benchmark_profile_digest: "git-blob-sha1:profile-fixture-01".to_owned(),
            fixture_id: "housing-adaptation-options-01".to_owned(),
            fixture_digest: "git-blob-sha1:fixture-01".to_owned(),
            claim_ceiling: "synthetic_robust_pareto_optioning_only".to_owned(),
        }
    }

    fn passing_gates() -> BTreeMap<String, GateResult> {
        BTreeMap::from([
            (
                "structural-adequacy".to_owned(),
                GateResult::QualifiedPass {
                    evidence_ref: "evidence:structural-review-01".to_owned(),
                },
            ),
            (
                "fire-safety".to_owned(),
                GateResult::QualifiedPass {
                    evidence_ref: "evidence:fire-review-01".to_owned(),
                },
            ),
            (
                "accessibility".to_owned(),
                GateResult::QualifiedPass {
                    evidence_ref: "evidence:accessibility-review-01".to_owned(),
                },
            ),
        ])
    }

    fn option(
        id: &str,
        tier: OptionTier,
        values: [[u64; 6]; 3],
    ) -> DeliveryOption {
        let keys = [
            MetricKey::WholeLifeCostMinorUnits,
            MetricKey::EmbodiedCarbonGramsCo2e,
            MetricKey::CrewMinutes,
            MetricKey::DeliveryDurationMinutes,
            MetricKey::ServiceDisruptionMinutes,
            MetricKey::RepairabilityBasisPoints,
        ];
        let scenario_ids = ["normal", "supply-delay", "maintenance-stress"];
        let scenario_metrics = scenario_ids
            .into_iter()
            .zip(values)
            .map(|(scenario_id, values)| {
                let values = keys
                    .iter()
                    .copied()
                    .zip(values)
                    .collect::<BTreeMap<_, _>>();
                (
                    scenario_id.to_owned(),
                    ScenarioMetrics {
                        scenario_id: scenario_id.to_owned(),
                        values,
                    },
                )
            })
            .collect();

        DeliveryOption {
            option_id: id.to_owned(),
            functional_brief_id: "housing-service-level-01".to_owned(),
            brief_revision_digest: "brief-revision-01".to_owned(),
            tier,
            delivery_method: format!("method:{id}"),
            design_revision_digest: format!("design-revision:{id}"),
            gate_results: passing_gates(),
            scenario_metrics,
            assumptions_ref: format!("assumptions:{id}"),
        }
    }

    fn assess(report: &OptioningReport, id: &str) -> &OptionAssessment {
        report
            .assessments
            .iter()
            .find(|assessment| assessment.option_id == id)
            .expect("option assessment exists")
    }

    #[test]
    fn existing_asset_adaptation_can_avoid_new_build_when_all_gates_pass() {
        let avoid = option(
            "adapt-existing-asset",
            OptionTier::Avoid,
            [
                [800, 4_000, 2_000, 1_000, 100, 8_000],
                [850, 4_100, 2_100, 1_100, 110, 8_000],
                [900, 4_200, 2_200, 1_200, 120, 8_000],
            ],
        );
        let switch = option(
            "new-modular-build",
            OptionTier::Switch,
            [
                [1_200, 5_000, 2_500, 1_300, 300, 9_000],
                [1_500, 6_000, 2_800, 1_500, 400, 9_000],
                [1_700, 6_500, 3_000, 1_600, 450, 7_000],
            ],
        );
        let improve = option(
            "robot-optimized-new-build",
            OptionTier::Improve,
            [
                [1_100, 5_200, 1_400, 1_100, 250, 7_000],
                [1_400, 6_200, 1_600, 1_400, 350, 7_000],
                [1_600, 6_700, 1_800, 1_500, 400, 7_000],
            ],
        );

        let report = evaluate_options(&brief(), &[improve, switch, avoid])
            .expect("valid option report");

        assert_eq!(
            report.pareto_option_ids,
            vec![
                "adapt-existing-asset".to_owned(),
                "robot-optimized-new-build".to_owned()
            ]
        );
        assert_eq!(report.dominated_options.len(), 1);
        assert_eq!(
            report.dominated_options[0].option_id,
            "new-modular-build"
        );
        assert_eq!(assess(&report, "adapt-existing-asset").disposition, OptionDisposition::QualifiedFeasible);
    }

    #[test]
    fn failed_gate_cannot_be_compensated_by_cheaper_cost_or_faster_duration() {
        let mut unsafe_option = option(
            "cheap-but-failed",
            OptionTier::Avoid,
            [
                [1, 1, 1, 1, 1, 10_000],
                [1, 1, 1, 1, 1, 10_000],
                [1, 1, 1, 1, 1, 10_000],
            ],
        );
        unsafe_option.gate_results.insert(
            "structural-adequacy".to_owned(),
            GateResult::QualifiedFail {
                evidence_ref: "evidence:failed-review".to_owned(),
                reason: "declared capacity requirement not met".to_owned(),
            },
        );
        let valid = option(
            "valid-switch",
            OptionTier::Switch,
            [
                [1_000, 2_000, 2_000, 2_000, 100, 8_000],
                [1_100, 2_100, 2_100, 2_100, 110, 8_000],
                [1_200, 2_200, 2_200, 2_200, 120, 8_000],
            ],
        );

        let report = evaluate_options(&brief(), &[unsafe_option, valid])
            .expect("valid report");
        assert_eq!(
            assess(&report, "cheap-but-failed").disposition,
            OptionDisposition::Infeasible {
                failed_gate_ids: vec!["structural-adequacy".to_owned()]
            }
        );
        assert!(!report.pareto_option_ids.contains(&"cheap-but-failed".to_owned()));
        assert!(!report.worst_case_metrics.contains_key("cheap-but-failed"));
    }

    #[test]
    fn unknown_and_stale_gates_are_not_evaluable_not_feasible() {
        let mut unknown = option(
            "unknown-adaptation",
            OptionTier::Avoid,
            [[1, 1, 1, 1, 1, 1]; 3],
        );
        unknown.gate_results.insert(
            "fire-safety".to_owned(),
            GateResult::Unknown {
                reason: "existing drawings do not establish compartmentation".to_owned(),
            },
        );
        let mut stale = option(
            "stale-design",
            OptionTier::Switch,
            [[1, 1, 1, 1, 1, 1]; 3],
        );
        stale.gate_results.insert(
            "accessibility".to_owned(),
            GateResult::Stale {
                reason: "source model revision predates current brief".to_owned(),
            },
        );

        let report = evaluate_options(&brief(), &[unknown, stale]).expect("valid report");
        assert_eq!(
            assess(&report, "unknown-adaptation").disposition,
            OptionDisposition::NotEvaluable {
                unknown_gate_ids: vec!["fire-safety".to_owned()],
                stale_gate_ids: vec![],
            }
        );
        assert_eq!(
            assess(&report, "stale-design").disposition,
            OptionDisposition::NotEvaluable {
                unknown_gate_ids: vec![],
                stale_gate_ids: vec!["accessibility".to_owned()],
            }
        );
        assert!(report.pareto_option_ids.is_empty());
    }

    #[test]
    fn robust_frontier_uses_worst_case_metrics_across_all_scenarios() {
        let stable = option(
            "stable-option",
            OptionTier::Switch,
            [
                [1_100, 2_000, 1_000, 1_100, 100, 7_000],
                [1_200, 2_100, 1_100, 1_200, 120, 7_000],
                [1_300, 2_200, 1_200, 1_300, 130, 7_000],
            ],
        );
        let fragile = option(
            "fragile-option",
            OptionTier::Improve,
            [
                [700, 1_500, 700, 800, 80, 8_000],
                [900, 2_000, 900, 1_000, 90, 8_000],
                [2_500, 8_000, 2_000, 3_000, 600, 5_000],
            ],
        );
        let report = evaluate_options(&brief(), &[stable, fragile]).expect("robust report");

        assert_eq!(
            report.worst_case_metrics["stable-option"][&MetricKey::WholeLifeCostMinorUnits],
            1_300
        );
        assert_eq!(
            report.worst_case_metrics["fragile-option"][&MetricKey::WholeLifeCostMinorUnits],
            2_500
        );
        assert_eq!(
            report.worst_case_metrics["stable-option"][&MetricKey::RepairabilityBasisPoints],
            7_000
        );
        assert_eq!(
            report.worst_case_metrics["fragile-option"][&MetricKey::RepairabilityBasisPoints],
            5_000
        );
        assert_eq!(report.pareto_option_ids, vec!["stable-option".to_owned()]);
    }

    #[test]
    fn repairability_uses_higher_is_better_direction() {
        let repairable = option(
            "repairable",
            OptionTier::Switch,
            [[1_000, 2_000, 1_000, 1_000, 100, 9_000]; 3],
        );
        let disposable = option(
            "disposable",
            OptionTier::Improve,
            [[1_000, 2_000, 1_000, 1_000, 100, 2_000]; 3],
        );
        let report = evaluate_options(&brief(), &[repairable, disposable])
            .expect("repairability report");
        assert_eq!(report.pareto_option_ids, vec!["repairable".to_owned()]);
    }

    #[test]
    fn equal_metric_options_remain_tied_on_the_pareto_frontier() {
        let first = option("same-a", OptionTier::Avoid, [[1, 2, 3, 4, 5, 6]; 3]);
        let second = option("same-b", OptionTier::Switch, [[1, 2, 3, 4, 5, 6]; 3]);
        let report = evaluate_options(&brief(), &[first, second]).expect("equal report");
        assert_eq!(
            report.pareto_option_ids,
            vec!["same-a".to_owned(), "same-b".to_owned()]
        );
        assert!(report.dominated_options.is_empty());
    }

    #[test]
    fn missing_gate_and_missing_scenario_are_typed_errors() {
        let mut missing_gate = option(
            "missing-gate",
            OptionTier::Avoid,
            [[1, 2, 3, 4, 5, 6]; 3],
        );
        missing_gate.gate_results.remove("fire-safety");
        assert_eq!(
            evaluate_options(&brief(), &[missing_gate]),
            Err(OptioningError::MissingRequiredGate {
                option_id: "missing-gate".to_owned(),
                gate_id: "fire-safety".to_owned(),
            })
        );

        let mut missing_scenario = option(
            "missing-scenario",
            OptionTier::Switch,
            [[1, 2, 3, 4, 5, 6]; 3],
        );
        missing_scenario.scenario_metrics.remove("supply-delay");
        assert_eq!(
            evaluate_options(&brief(), &[missing_scenario]),
            Err(OptioningError::MissingScenarioMetrics {
                option_id: "missing-scenario".to_owned(),
                scenario_id: "supply-delay".to_owned(),
            })
        );
    }

    #[test]
    fn wrong_brief_revision_and_unexpected_gate_are_rejected() {
        let mut wrong_brief = option(
            "wrong-brief",
            OptionTier::Avoid,
            [[1, 2, 3, 4, 5, 6]; 3],
        );
        wrong_brief.functional_brief_id = "other-brief".to_owned();
        assert_eq!(
            evaluate_options(&brief(), &[wrong_brief]),
            Err(OptioningError::WrongFunctionalBrief {
                option_id: "wrong-brief".to_owned()
            })
        );

        let mut wrong_revision = option(
            "wrong-revision",
            OptionTier::Switch,
            [[1, 2, 3, 4, 5, 6]; 3],
        );
        wrong_revision.brief_revision_digest = "old-revision".to_owned();
        assert_eq!(
            evaluate_options(&brief(), &[wrong_revision]),
            Err(OptioningError::WrongBriefRevision {
                option_id: "wrong-revision".to_owned()
            })
        );

        let mut unexpected_gate = option(
            "unexpected-gate",
            OptionTier::Improve,
            [[1, 2, 3, 4, 5, 6]; 3],
        );
        unexpected_gate.gate_results.insert(
            "unlisted-gate".to_owned(),
            GateResult::Unknown {
                reason: "not declared by functional brief".to_owned(),
            },
        );
        assert_eq!(
            evaluate_options(&brief(), &[unexpected_gate]),
            Err(OptioningError::UnexpectedGate {
                option_id: "unexpected-gate".to_owned(),
                gate_id: "unlisted-gate".to_owned(),
            })
        );
    }

    #[test]
    fn duplicate_options_briefs_and_scenarios_are_rejected() {
        let first = option("duplicate", OptionTier::Avoid, [[1, 2, 3, 4, 5, 6]; 3]);
        let second = first.clone();
        assert_eq!(
            evaluate_options(&brief(), &[first, second]),
            Err(OptioningError::DuplicateIdentifier {
                field: "option_id",
                value: "duplicate".to_owned()
            })
        );

        let mut duplicate_brief = brief();
        duplicate_brief.required_gate_ids.push("fire-safety".to_owned());
        assert!(matches!(
            evaluate_options(&duplicate_brief, &[option("one", OptionTier::Avoid, [[1, 2, 3, 4, 5, 6]; 3])]),
            Err(OptioningError::DuplicateIdentifier { field: "required_gate_id", .. })
        ));

        let mut duplicate_scenario = brief();
        duplicate_scenario.scenario_ids.push("normal".to_owned());
        assert!(matches!(
            evaluate_options(&duplicate_scenario, &[option("one", OptionTier::Avoid, [[1, 2, 3, 4, 5, 6]; 3])]),
            Err(OptioningError::DuplicateIdentifier { field: "scenario_id", .. })
        ));
    }

    #[test]
    fn exact_frontier_is_independent_of_input_order() {
        let options = vec![
            option("option-c", OptionTier::Improve, [[1, 1, 1, 1, 1, 1]; 3]),
            option("option-a", OptionTier::Avoid, [[3, 3, 3, 3, 3, 3]; 3]),
            option("option-b", OptionTier::Switch, [[2, 2, 2, 2, 2, 2]; 3]),
        ];
        let forward = evaluate_options(&brief(), &options).expect("forward report");
        let reverse = evaluate_options(
            &brief(),
            &options.into_iter().rev().collect::<Vec<_>>(),
        )
        .expect("reverse report");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn report_carries_analysis_horizon_and_exact_provenance() {
        let one = option("avoid", OptionTier::Avoid, [[1, 2, 3, 4, 5, 6]; 3]);
        let report = evaluate_options(&brief(), &[one]).expect("provenance report");
        assert_eq!(report.brief_id, "housing-service-level-01");
        assert_eq!(report.revision_digest, "brief-revision-01");
        assert_eq!(report.analysis_horizon_months, 600);
        assert_eq!(report.discount_rate_basis_points, 350);
        assert_eq!(report.currency_code, "ZAR");
        assert_eq!(
            report.scenario_metrics_by_option["avoid"]["normal"]
                [&MetricKey::WholeLifeCostMinorUnits],
            900
        );
        assert_eq!(
            assess(&report, "avoid").gate_results["structural-adequacy"],
            GateResult::QualifiedPass {
                evidence_ref: "evidence:structural-review-01".to_owned()
            }
        );
        assert_eq!(report.benchmark_profile_id, "FUNCTION-FIRST-OPTIONEERING-001");
        assert_eq!(report.fixture_id, "housing-adaptation-options-01");
        assert_eq!(report.claim_ceiling, "synthetic_robust_pareto_optioning_only");
    }
}
