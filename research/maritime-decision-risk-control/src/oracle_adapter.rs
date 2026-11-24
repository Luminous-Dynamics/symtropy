// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Strict adapter from independently adjudicated whole-schedule oracle receipts
//! to separate decision-risk calibration matrices.
//!
//! This module does not implement physical feasibility or participant authority.
//! It accepts their independently produced dispositions, rejects unresolved or
//! unauthorized candidate outcomes, and keeps hard-failure and service losses
//! as distinct matrices.

use std::collections::{BTreeMap, BTreeSet};

const MAX_LOSS_MILLI: u16 = 1_000;
const EXPECTED_SCHEMA: &str = "maritime.oracle-outcomes.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DatasetRole {
    Calibration,
    Evaluation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityDisposition {
    /// The exact policy action was authorized under the applicable revision.
    Authorized,
    /// No authority was required because the policy made no mutating action.
    NotRequired,
    /// The policy attempted or proposed an action without valid authority.
    Denied,
    /// Authority could not be independently resolved.
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyActionClass {
    /// The candidate policy makes no mutating operational action.
    NoMutation,
    /// The candidate policy proposes or performs a mutating resource/action change.
    Mutating,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalDisposition {
    Feasible,
    Infeasible,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LossAxis {
    HardPhysicalFailure,
    ServiceLoss,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPolicyEntry {
    /// Zero-based index in the exact frozen policy-parameter order used by CRC.
    pub parameter_index: usize,
    pub policy_id: String,
    pub policy_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleDatasetContract {
    pub schema_version: String,
    pub oracle_id: String,
    pub oracle_revision: String,
    /// Human-readable scope for the independent evaluator evidence.
    pub oracle_evidence_ref: String,
    /// Evidence that the oracle does not reuse policy/production feasibility helpers.
    pub independence_evidence_ref: String,
    /// Claimed exchangeability scope; this is a declaration, not a proof.
    pub exchangeability_scope: String,
    /// All episodes in one compiled dataset must belong to this operating regime.
    pub regime_id: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    /// Ordered candidate grid; parameter_index is the CRC column/parameter order.
    pub policy_grid: Vec<FrozenPolicyEntry>,
    /// External digest reference for the canonical ordered policy grid; shape-checked here.
    pub policy_grid_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyAdjudication {
    pub policy_id: String,
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    pub action_class: PolicyActionClass,
    pub authority: AuthorityDisposition,
    pub physical: PhysicalDisposition,
    /// Independent service loss in [0, 1000], kept separate from hard feasibility.
    pub service_loss_milli: u16,
    /// SHA-256 digest of the independently produced policy × episode receipt.
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleAdjudication {
    /// One complete scheduling episode is one row of the calibration matrix.
    pub episode_id: String,
    /// Shared outage/weather/TOS incident identifier; must be unique per episode.
    pub dependence_cluster_id: String,
    pub scenario_family: String,
    pub regime_id: String,
    pub role: DatasetRole,
    /// Exactly one outcome for every frozen candidate policy.
    pub policy_outcomes: Vec<PolicyAdjudication>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleOutcomeDataset {
    pub contract: OracleDatasetContract,
    pub episodes: Vec<ScheduleAdjudication>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleAdapterError {
    EmptyDataset,
    MissingContractField(&'static str),
    UnsupportedSchema(String),
    InvalidSha256 {
        field: &'static str,
        value: String,
    },
    EmptyEpisodeField {
        episode_id: String,
        field: &'static str,
    },
    DuplicateEpisodeId(String),
    ReusedDependenceCluster(String),
    MixedOperatingRegimes {
        expected: String,
        found: String,
    },
    MissingCalibrationEpisodes,
    MissingEvaluationEpisodes,
    EmptyPolicyGrid,
    InvalidPolicyGridOrder {
        position: usize,
        parameter_index: usize,
    },
    DuplicateFrozenPolicyId(String),
    FrozenPolicyRevisionMissing(String),
    FrozenPolicyRevisionMismatch(String),
    AuthorityActionMismatch {
        episode_id: String,
        policy_id: String,
        action_class: PolicyActionClass,
        disposition: AuthorityDisposition,
    },
    DuplicatePolicyId {
        episode_id: String,
        policy_id: String,
    },
    PolicyGridDrift {
        episode_id: String,
        missing_policy_ids: Vec<String>,
        unexpected_policy_ids: Vec<String>,
    },
    MissingPolicyRevision {
        policy_id: String,
    },
    PolicyRevisionDrift {
        policy_id: String,
    },
    EvaluatorRevisionMismatch {
        episode_id: String,
        policy_id: String,
    },
    LossContractRevisionMismatch {
        episode_id: String,
        policy_id: String,
    },
    UnauthorizedPolicyOutcome {
        episode_id: String,
        policy_id: String,
        disposition: AuthorityDisposition,
    },
    UnresolvedPhysicalOutcome {
        episode_id: String,
        policy_id: String,
    },
    ServiceLossOutsideBound {
        episode_id: String,
        policy_id: String,
        value_milli: u16,
    },
    EmptyTailRiskSample,
    InvalidTailConfidence(f64),
    TailLossOutsideBound(u16),
}

#[derive(Clone, Debug, PartialEq)]
pub struct OracleCalibrationBundle {
    /// Candidate IDs in the exact frozen parameter order; column i is parameter i.
    pub policy_ids: Vec<String>,
    /// Frozen candidate revisions in the same column order.
    pub policy_revisions: Vec<String>,
    /// One row per independent calibration schedule, one column per candidate policy.
    pub calibration_hard_failure_milli: Vec<Vec<u16>>,
    pub calibration_service_loss_milli: Vec<Vec<u16>>,
    /// Separate held-out evaluation matrices, never consumed by calibration selection.
    pub evaluation_hard_failure_milli: Vec<Vec<u16>>,
    pub evaluation_service_loss_milli: Vec<Vec<u16>>,
    pub oracle_id: String,
    pub oracle_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    pub policy_grid_sha256: String,
    pub regime_id: String,
}

impl OracleCalibrationBundle {
    pub fn matrix(&self, role: DatasetRole, axis: LossAxis) -> crate::LossMatrix {
        let schedules = match (role, axis) {
            (DatasetRole::Calibration, LossAxis::HardPhysicalFailure) => {
                &self.calibration_hard_failure_milli
            }
            (DatasetRole::Calibration, LossAxis::ServiceLoss) => {
                &self.calibration_service_loss_milli
            }
            (DatasetRole::Evaluation, LossAxis::HardPhysicalFailure) => {
                &self.evaluation_hard_failure_milli
            }
            (DatasetRole::Evaluation, LossAxis::ServiceLoss) => &self.evaluation_service_loss_milli,
        };

        crate::LossMatrix {
            schedules: schedules.clone(),
        }
    }

    pub fn non_monotone_grid(
        &self,
        role: DatasetRole,
        axis: LossAxis,
    ) -> crate::non_monotone::WholeScheduleLossGrid {
        let schedules = match (role, axis) {
            (DatasetRole::Calibration, LossAxis::HardPhysicalFailure) => {
                &self.calibration_hard_failure_milli
            }
            (DatasetRole::Calibration, LossAxis::ServiceLoss) => {
                &self.calibration_service_loss_milli
            }
            (DatasetRole::Evaluation, LossAxis::HardPhysicalFailure) => {
                &self.evaluation_hard_failure_milli
            }
            (DatasetRole::Evaluation, LossAxis::ServiceLoss) => {
                &self.evaluation_service_loss_milli
            }
        };

        crate::non_monotone::WholeScheduleLossGrid {
            schedule_losses_milli: schedules.clone(),
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn require_nonempty(value: &str, field: &'static str) -> Result<(), OracleAdapterError> {
    if value.trim().is_empty() {
        return Err(OracleAdapterError::MissingContractField(field));
    }
    Ok(())
}

fn validate_contract(contract: &OracleDatasetContract) -> Result<(), OracleAdapterError> {
    if contract.schema_version != EXPECTED_SCHEMA {
        return Err(OracleAdapterError::UnsupportedSchema(
            contract.schema_version.clone(),
        ));
    }

    for (value, field) in [
        (contract.oracle_id.as_str(), "oracle_id"),
        (contract.oracle_revision.as_str(), "oracle_revision"),
        (contract.oracle_evidence_ref.as_str(), "oracle_evidence_ref"),
        (
            contract.independence_evidence_ref.as_str(),
            "independence_evidence_ref",
        ),
        (
            contract.exchangeability_scope.as_str(),
            "exchangeability_scope",
        ),
        (contract.regime_id.as_str(), "regime_id"),
        (contract.evaluator_revision.as_str(), "evaluator_revision"),
        (
            contract.loss_contract_revision.as_str(),
            "loss_contract_revision",
        ),
    ] {
        require_nonempty(value, field)?;
    }

    if contract.policy_grid.is_empty() {
        return Err(OracleAdapterError::EmptyPolicyGrid);
    }
    let mut frozen_ids = BTreeSet::new();
    for (position, entry) in contract.policy_grid.iter().enumerate() {
        if entry.parameter_index != position {
            return Err(OracleAdapterError::InvalidPolicyGridOrder {
                position,
                parameter_index: entry.parameter_index,
            });
        }
        if entry.policy_id.trim().is_empty() || entry.policy_revision.trim().is_empty() {
            return Err(OracleAdapterError::FrozenPolicyRevisionMissing(
                entry.policy_id.clone(),
            ));
        }
        if !frozen_ids.insert(entry.policy_id.clone()) {
            return Err(OracleAdapterError::DuplicateFrozenPolicyId(entry.policy_id.clone()));
        }
    }

    if !is_sha256(&contract.policy_grid_sha256) {
        return Err(OracleAdapterError::InvalidSha256 {
            field: "policy_grid_sha256",
            value: contract.policy_grid_sha256.clone(),
        });
    }

    Ok(())
}

fn validate_episode_fields(
    episode: &ScheduleAdjudication,
    expected_regime: &str,
) -> Result<(), OracleAdapterError> {
    for (value, field) in [
        (episode.episode_id.as_str(), "episode_id"),
        (
            episode.dependence_cluster_id.as_str(),
            "dependence_cluster_id",
        ),
        (episode.scenario_family.as_str(), "scenario_family"),
        (episode.regime_id.as_str(), "regime_id"),
    ] {
        if value.trim().is_empty() {
            return Err(OracleAdapterError::EmptyEpisodeField {
                episode_id: episode.episode_id.clone(),
                field,
            });
        }
    }

    if episode.regime_id != expected_regime {
        return Err(OracleAdapterError::MixedOperatingRegimes {
            expected: expected_regime.to_owned(),
            found: episode.regime_id.clone(),
        });
    }

    Ok(())
}

fn validate_policy_outcome(
    episode: &ScheduleAdjudication,
    outcome: &PolicyAdjudication,
    frozen_policy: &FrozenPolicyEntry,
    contract: &OracleDatasetContract,
) -> Result<(), OracleAdapterError> {
    if outcome.policy_id.trim().is_empty() || outcome.policy_revision.trim().is_empty() {
        return Err(OracleAdapterError::MissingPolicyRevision {
            policy_id: outcome.policy_id.clone(),
        });
    }

    if outcome.policy_id != frozen_policy.policy_id
        || outcome.policy_revision != frozen_policy.policy_revision
    {
        return Err(OracleAdapterError::FrozenPolicyRevisionMismatch(
            outcome.policy_id.clone(),
        ));
    }

    if outcome.evaluator_revision != contract.evaluator_revision {
        return Err(OracleAdapterError::EvaluatorRevisionMismatch {
            episode_id: episode.episode_id.clone(),
            policy_id: outcome.policy_id.clone(),
        });
    }

    if outcome.loss_contract_revision != contract.loss_contract_revision {
        return Err(OracleAdapterError::LossContractRevisionMismatch {
            episode_id: episode.episode_id.clone(),
            policy_id: outcome.policy_id.clone(),
        });
    }

    if !is_sha256(&outcome.receipt_sha256) {
        return Err(OracleAdapterError::InvalidSha256 {
            field: "receipt_sha256",
            value: outcome.receipt_sha256.clone(),
        });
    }

    match outcome.authority {
        AuthorityDisposition::Denied | AuthorityDisposition::Unknown => {
            return Err(OracleAdapterError::UnauthorizedPolicyOutcome {
                episode_id: episode.episode_id.clone(),
                policy_id: outcome.policy_id.clone(),
                disposition: outcome.authority,
            });
        }
        _ => {}
    }
    let authority_matches_action = matches!(
        (outcome.action_class, outcome.authority),
        (PolicyActionClass::Mutating, AuthorityDisposition::Authorized)
            | (PolicyActionClass::NoMutation, AuthorityDisposition::NotRequired)
    );
    if !authority_matches_action {
        return Err(OracleAdapterError::AuthorityActionMismatch {
            episode_id: episode.episode_id.clone(),
            policy_id: outcome.policy_id.clone(),
            action_class: outcome.action_class,
            disposition: outcome.authority,
        });
    }

    if outcome.physical == PhysicalDisposition::Unknown {
        return Err(OracleAdapterError::UnresolvedPhysicalOutcome {
            episode_id: episode.episode_id.clone(),
            policy_id: outcome.policy_id.clone(),
        });
    }

    if outcome.service_loss_milli > MAX_LOSS_MILLI {
        return Err(OracleAdapterError::ServiceLossOutsideBound {
            episode_id: episode.episode_id.clone(),
            policy_id: outcome.policy_id.clone(),
            value_milli: outcome.service_loss_milli,
        });
    }

    Ok(())
}

/// Validates and deterministically converts independent oracle receipts into
/// separate calibration/evaluation matrices. It never computes feasibility
/// itself, infers authority, estimates a distribution ratio, or drops unknown rows.
pub fn compile_oracle_outcomes(
    dataset: &OracleOutcomeDataset,
) -> Result<OracleCalibrationBundle, OracleAdapterError> {
    if dataset.episodes.is_empty() {
        return Err(OracleAdapterError::EmptyDataset);
    }
    validate_contract(&dataset.contract)?;

    let mut episode_ids = BTreeSet::new();
    let mut dependence_clusters = BTreeSet::new();
    let expected_policy_ids = dataset
        .contract
        .policy_grid
        .iter()
        .map(|entry| entry.policy_id.clone())
        .collect::<BTreeSet<_>>();
    let mut calibration_rows = Vec::new();
    let mut evaluation_rows = Vec::new();

    for episode in &dataset.episodes {
        validate_episode_fields(episode, &dataset.contract.regime_id)?;

        if !episode_ids.insert(episode.episode_id.clone()) {
            return Err(OracleAdapterError::DuplicateEpisodeId(
                episode.episode_id.clone(),
            ));
        }
        if !dependence_clusters.insert(episode.dependence_cluster_id.clone()) {
            return Err(OracleAdapterError::ReusedDependenceCluster(
                episode.dependence_cluster_id.clone(),
            ));
        }
        if episode.policy_outcomes.is_empty() {
            return Err(OracleAdapterError::EmptyPolicyGrid);
        }

        let mut current_policy_ids = BTreeSet::new();
        for outcome in &episode.policy_outcomes {
            if !current_policy_ids.insert(outcome.policy_id.clone()) {
                return Err(OracleAdapterError::DuplicatePolicyId {
                    episode_id: episode.episode_id.clone(),
                    policy_id: outcome.policy_id.clone(),
                });
            }
            let frozen_policy = dataset
                .contract
                .policy_grid
                .iter()
                .find(|entry| entry.policy_id == outcome.policy_id)
                .ok_or_else(|| OracleAdapterError::PolicyGridDrift {
                    episode_id: episode.episode_id.clone(),
                    missing_policy_ids: expected_policy_ids
                        .difference(&current_policy_ids)
                        .cloned()
                        .collect(),
                    unexpected_policy_ids: vec![outcome.policy_id.clone()],
                })?;
            validate_policy_outcome(episode, outcome, frozen_policy, &dataset.contract)?;
        }

        if expected_policy_ids != current_policy_ids {
            return Err(OracleAdapterError::PolicyGridDrift {
                episode_id: episode.episode_id.clone(),
                missing_policy_ids: expected_policy_ids
                    .difference(&current_policy_ids)
                    .cloned()
                    .collect(),
                unexpected_policy_ids: current_policy_ids
                    .difference(&expected_policy_ids)
                    .cloned()
                    .collect(),
            });
        }

        let mut by_policy = BTreeMap::new();
        for outcome in &episode.policy_outcomes {
            by_policy.insert(outcome.policy_id.as_str(), outcome);
        }

        let policy_ids = dataset
            .contract
            .policy_grid
            .iter()
            .map(|entry| entry.policy_id.clone())
            .collect::<Vec<_>>();
        let hard_row = policy_ids
            .iter()
            .map(|policy_id| {
                let outcome = by_policy[policy_id.as_str()];
                match outcome.physical {
                    PhysicalDisposition::Feasible => 0,
                    PhysicalDisposition::Infeasible => MAX_LOSS_MILLI,
                    PhysicalDisposition::Unknown => unreachable!("validated above"),
                }
            })
            .collect::<Vec<_>>();
        let service_row = policy_ids
            .iter()
            .map(|policy_id| by_policy[policy_id.as_str()].service_loss_milli)
            .collect::<Vec<_>>();

        match episode.role {
            DatasetRole::Calibration => {
                calibration_rows.push((episode.episode_id.clone(), hard_row, service_row))
            }
            DatasetRole::Evaluation => {
                evaluation_rows.push((episode.episode_id.clone(), hard_row, service_row))
            }
        }
    }

    if calibration_rows.is_empty() {
        return Err(OracleAdapterError::MissingCalibrationEpisodes);
    }
    if evaluation_rows.is_empty() {
        return Err(OracleAdapterError::MissingEvaluationEpisodes);
    }

    calibration_rows.sort_by(|left, right| left.0.cmp(&right.0));
    evaluation_rows.sort_by(|left, right| left.0.cmp(&right.0));

    let policy_ids = dataset
        .contract
        .policy_grid
        .iter()
        .map(|entry| entry.policy_id.clone())
        .collect::<Vec<_>>();
    let policy_revisions = dataset
        .contract
        .policy_grid
        .iter()
        .map(|entry| entry.policy_revision.clone())
        .collect::<Vec<_>>();

    Ok(OracleCalibrationBundle {
        policy_ids,
        policy_revisions,
        calibration_hard_failure_milli: calibration_rows.iter().map(|row| row.1.clone()).collect(),
        calibration_service_loss_milli: calibration_rows.iter().map(|row| row.2.clone()).collect(),
        evaluation_hard_failure_milli: evaluation_rows.iter().map(|row| row.1.clone()).collect(),
        evaluation_service_loss_milli: evaluation_rows.iter().map(|row| row.2.clone()).collect(),
        oracle_id: dataset.contract.oracle_id.clone(),
        oracle_revision: dataset.contract.oracle_revision.clone(),
        evaluator_revision: dataset.contract.evaluator_revision.clone(),
        loss_contract_revision: dataset.contract.loss_contract_revision.clone(),
        policy_grid_sha256: dataset.contract.policy_grid_sha256.clone(),
        regime_id: dataset.contract.regime_id.clone(),
    })
}

/// Descriptive tail metrics for one independently adjudicated policy/loss axis.
/// These statistics are diagnostics, not finite-sample risk certificates.
#[derive(Clone, Debug, PartialEq)]
pub struct PolicyTailMetrics {
    pub policy_id: String,
    pub sample_count: usize,
    pub mean_loss_milli: f64,
    /// Empirical upper-tail CVaR at the declared confidence level (e.g. 0.95).
    pub empirical_upper_cvar_milli: f64,
    pub maximum_loss_milli: u16,
}

/// Computes empirical upper-tail CVaR using fractional tail mass.
///
/// For confidence q, the upper tail has mass (1-q). Fractional weighting of
/// the boundary observation avoids rounding the tail to an integer count.
pub fn empirical_upper_cvar_milli(
    losses: &[u16],
    confidence_level: f64,
) -> Result<f64, OracleAdapterError> {
    if losses.is_empty() {
        return Err(OracleAdapterError::EmptyTailRiskSample);
    }
    if !confidence_level.is_finite() || !(0.0..1.0).contains(&confidence_level) {
        return Err(OracleAdapterError::InvalidTailConfidence(confidence_level));
    }
    if let Some(value) = losses.iter().copied().find(|value| *value > MAX_LOSS_MILLI) {
        return Err(OracleAdapterError::TailLossOutsideBound(value));
    }

    let tail_mass = (1.0 - confidence_level) * losses.len() as f64;
    let mut ordered = losses.to_vec();
    ordered.sort_unstable_by(|left, right| right.cmp(left));

    let mut remaining = tail_mass;
    let mut weighted_total = 0.0;
    for loss in ordered {
        if remaining <= 0.0 {
            break;
        }
        let weight = remaining.min(1.0);
        weighted_total += f64::from(loss) * weight;
        remaining -= weight;
    }

    Ok(weighted_total / tail_mass)
}

/// Reports mean, maximum and empirical upper-tail CVaR for each candidate policy.
/// The output axis is explicit; hard-failure and service-loss tails never combine.
pub fn tail_metrics_by_policy(
    bundle: &OracleCalibrationBundle,
    role: DatasetRole,
    axis: LossAxis,
    confidence_level: f64,
) -> Result<Vec<PolicyTailMetrics>, OracleAdapterError> {
    let matrix = bundle.matrix(role, axis);
    if matrix.schedules.is_empty() {
        return Err(OracleAdapterError::EmptyTailRiskSample);
    }

    let mut metrics = Vec::with_capacity(bundle.policy_ids.len());
    for (policy_index, policy_id) in bundle.policy_ids.iter().enumerate() {
        let losses = matrix
            .schedules
            .iter()
            .map(|row| row[policy_index])
            .collect::<Vec<_>>();
        let total = losses.iter().map(|loss| u64::from(*loss)).sum::<u64>();
        let mean_loss_milli = total as f64 / losses.len() as f64;
        let maximum_loss_milli = losses.iter().copied().max().unwrap_or(0);
        let empirical_upper_cvar_milli = empirical_upper_cvar_milli(&losses, confidence_level)?;

        metrics.push(PolicyTailMetrics {
            policy_id: policy_id.clone(),
            sample_count: losses.len(),
            mean_loss_milli,
            empirical_upper_cvar_milli,
            maximum_loss_milli,
        });
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn contract() -> OracleDatasetContract {
        OracleDatasetContract {
            schema_version: EXPECTED_SCHEMA.to_owned(),
            oracle_id: "independent-schedule-oracle".to_owned(),
            oracle_revision: "oracle-rev-1".to_owned(),
            oracle_evidence_ref: "oracle-evidence-manifest".to_owned(),
            independence_evidence_ref: "separate-enumerator-review".to_owned(),
            exchangeability_scope: "synthetic-regime-A-whole-schedule".to_owned(),
            regime_id: "regime-A".to_owned(),
            evaluator_revision: "evaluator-v1".to_owned(),
            loss_contract_revision: "loss-v1".to_owned(),
            policy_grid: vec![
                FrozenPolicyEntry {
                    parameter_index: 0,
                    policy_id: "policy-B".to_owned(),
                    policy_revision: "policy-B-rev-1".to_owned(),
                },
                FrozenPolicyEntry {
                    parameter_index: 1,
                    policy_id: "policy-A".to_owned(),
                    policy_revision: "policy-A-rev-1".to_owned(),
                },
            ],
            policy_grid_sha256: DIGEST_A.to_owned(),
        }
    }

    fn outcome(
        policy_id: &str,
        physical: PhysicalDisposition,
        service_loss_milli: u16,
        authority: AuthorityDisposition,
        receipt_sha256: &str,
    ) -> PolicyAdjudication {
        PolicyAdjudication {
            policy_id: policy_id.to_owned(),
            policy_revision: format!("{policy_id}-rev-1"),
            evaluator_revision: "evaluator-v1".to_owned(),
            loss_contract_revision: "loss-v1".to_owned(),
            action_class: if authority == AuthorityDisposition::NotRequired {
                PolicyActionClass::NoMutation
            } else {
                PolicyActionClass::Mutating
            },
            authority,
            physical,
            service_loss_milli,
            receipt_sha256: receipt_sha256.to_owned(),
        }
    }

    fn episode(
        episode_id: &str,
        cluster_id: &str,
        role: DatasetRole,
        outcomes: Vec<PolicyAdjudication>,
    ) -> ScheduleAdjudication {
        ScheduleAdjudication {
            episode_id: episode_id.to_owned(),
            dependence_cluster_id: cluster_id.to_owned(),
            scenario_family: "berth-disruption".to_owned(),
            regime_id: "regime-A".to_owned(),
            role,
            policy_outcomes: outcomes,
        }
    }

    fn valid_dataset() -> OracleOutcomeDataset {
        OracleOutcomeDataset {
            contract: contract(),
            episodes: vec![
                episode(
                    "cal-2",
                    "weather-2",
                    DatasetRole::Calibration,
                    vec![
                        outcome(
                            "policy-B",
                            PhysicalDisposition::Infeasible,
                            600,
                            AuthorityDisposition::Authorized,
                            DIGEST_B,
                        ),
                        outcome(
                            "policy-A",
                            PhysicalDisposition::Feasible,
                            200,
                            AuthorityDisposition::NotRequired,
                            DIGEST_A,
                        ),
                    ],
                ),
                episode(
                    "eval-1",
                    "outage-1",
                    DatasetRole::Evaluation,
                    vec![
                        outcome(
                            "policy-A",
                            PhysicalDisposition::Infeasible,
                            900,
                            AuthorityDisposition::Authorized,
                            DIGEST_A,
                        ),
                        outcome(
                            "policy-B",
                            PhysicalDisposition::Feasible,
                            100,
                            AuthorityDisposition::Authorized,
                            DIGEST_B,
                        ),
                    ],
                ),
                episode(
                    "cal-1",
                    "weather-1",
                    DatasetRole::Calibration,
                    vec![
                        outcome(
                            "policy-A",
                            PhysicalDisposition::Feasible,
                            250,
                            AuthorityDisposition::Authorized,
                            DIGEST_A,
                        ),
                        outcome(
                            "policy-B",
                            PhysicalDisposition::Feasible,
                            400,
                            AuthorityDisposition::Authorized,
                            DIGEST_B,
                        ),
                    ],
                ),
            ],
        }
    }

    #[test]
    fn adapter_keeps_hard_failure_and_service_loss_separate() {
        let bundle = compile_oracle_outcomes(&valid_dataset()).expect("valid oracle data");
        assert_eq!(bundle.policy_ids, vec!["policy-B", "policy-A"]);
        assert_eq!(
            bundle.policy_revisions,
            vec!["policy-B-rev-1", "policy-A-rev-1"]
        );
        assert_eq!(
            bundle.calibration_hard_failure_milli,
            vec![vec![1_000, 0], vec![0, 0]]
        );
        assert_eq!(
            bundle.calibration_service_loss_milli,
            vec![vec![400, 250], vec![600, 200]]
        );
        assert_eq!(bundle.evaluation_hard_failure_milli, vec![vec![0, 1_000]]);
        assert_eq!(bundle.evaluation_service_loss_milli, vec![vec![100, 900]]);
    }

    #[test]
    fn adapter_replays_identically_under_episode_and_policy_input_permutations() {
        let first = compile_oracle_outcomes(&valid_dataset()).expect("first compile");
        let mut permuted = valid_dataset();
        permuted.episodes.reverse();
        for schedule in &mut permuted.episodes {
            schedule.policy_outcomes.reverse();
        }
        let second = compile_oracle_outcomes(&permuted).expect("permuted compile");
        assert_eq!(first, second);
    }

    #[test]
    fn adapter_rejects_unauthorized_action_before_statistical_calibration() {
        let mut data = valid_dataset();
        data.episodes[0].policy_outcomes[0].authority = AuthorityDisposition::Denied;
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::UnauthorizedPolicyOutcome {
                disposition: AuthorityDisposition::Denied,
                ..
            })
        ));
    }

    #[test]
    fn adapter_rejects_unknown_authority() {
        let mut data = valid_dataset();
        data.episodes[0].policy_outcomes[0].authority = AuthorityDisposition::Unknown;
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::UnauthorizedPolicyOutcome {
                disposition: AuthorityDisposition::Unknown,
                ..
            })
        ));
    }

    #[test]
    fn adapter_rejects_unknown_physical_outcome_instead_of_dropping_it() {
        let mut data = valid_dataset();
        data.episodes[0].policy_outcomes[0].physical = PhysicalDisposition::Unknown;
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::UnresolvedPhysicalOutcome { .. })
        ));
    }

    #[test]
    fn adapter_rejects_service_loss_outside_the_bounded_contract() {
        let mut data = valid_dataset();
        data.episodes[0].policy_outcomes[0].service_loss_milli = 1_001;
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::ServiceLossOutsideBound {
                value_milli: 1_001,
                ..
            })
        ));
    }

    #[test]
    fn adapter_rejects_reused_dependence_cluster_across_splits() {
        let mut data = valid_dataset();
        data.episodes[1].dependence_cluster_id = "weather-1".to_owned();
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::ReusedDependenceCluster(_))
        ));
    }

    #[test]
    fn adapter_rejects_candidate_grid_drift() {
        let mut data = valid_dataset();
        data.episodes[0].policy_outcomes.pop();
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::PolicyGridDrift { .. })
        ));
    }

    #[test]
    fn adapter_rejects_policy_revision_drift() {
        let mut data = valid_dataset();
        data.episodes[2].policy_outcomes[0].policy_revision = "policy-A-rev-2".to_owned();
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::FrozenPolicyRevisionMismatch("policy-A".to_owned()))
        ));
    }

    #[test]
    fn adapter_rejects_mixed_operating_regimes() {
        let mut data = valid_dataset();
        data.episodes[1].regime_id = "regime-B".to_owned();
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::MixedOperatingRegimes { .. })
        ));
    }

    #[test]
    fn adapter_rejects_not_required_authority_for_a_mutating_action() {
        let mut data = valid_dataset();
        let outcome = &mut data.episodes[0].policy_outcomes[1];
        outcome.action_class = PolicyActionClass::Mutating;
        outcome.authority = AuthorityDisposition::NotRequired;
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::AuthorityActionMismatch {
                action_class: PolicyActionClass::Mutating,
                disposition: AuthorityDisposition::NotRequired,
                ..
            })
        ));
    }

    #[test]
    fn adapter_rejects_noncontiguous_frozen_policy_parameter_indices() {
        let mut data = valid_dataset();
        data.contract.policy_grid[1].parameter_index = 3;
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::InvalidPolicyGridOrder {
                position: 1,
                parameter_index: 3,
            })
        ));
    }

    #[test]
    fn adapter_rejects_invalid_receipt_digest() {
        let mut data = valid_dataset();
        data.episodes[0].policy_outcomes[0].receipt_sha256 = "not-a-digest".to_owned();
        assert!(matches!(
            compile_oracle_outcomes(&data),
            Err(OracleAdapterError::InvalidSha256 {
                field: "receipt_sha256",
                ..
            })
        ));
    }

    #[test]
    fn adapter_exports_calibration_matrix_without_consuming_evaluation_rows() {
        let bundle = compile_oracle_outcomes(&valid_dataset()).expect("valid oracle data");
        let calibration = bundle.matrix(DatasetRole::Calibration, LossAxis::HardPhysicalFailure);
        let evaluation = bundle.matrix(DatasetRole::Evaluation, LossAxis::HardPhysicalFailure);
        assert_eq!(calibration.schedule_count(), 2);
        assert_eq!(evaluation.schedule_count(), 1);
    }

    #[test]
    fn upper_tail_cvar_detects_a_rare_catastrophic_schedule() {
        let losses = [10, 10, 10, 20, 1_000];
        assert_eq!(
            empirical_upper_cvar_milli(&losses, 0.80).expect("CVaR"),
            1_000.0
        );
        assert_eq!(
            empirical_upper_cvar_milli(&losses, 0.50).expect("CVaR"),
            410.0
        );
    }

    #[test]
    fn empirical_cvar_rejects_empty_invalid_or_unbounded_inputs() {
        assert_eq!(
            empirical_upper_cvar_milli(&[], 0.95),
            Err(OracleAdapterError::EmptyTailRiskSample)
        );
        assert_eq!(
            empirical_upper_cvar_milli(&[1, 2], 1.0),
            Err(OracleAdapterError::InvalidTailConfidence(1.0))
        );
        assert_eq!(
            empirical_upper_cvar_milli(&[1, 1_001], 0.95),
            Err(OracleAdapterError::TailLossOutsideBound(1_001))
        );
    }

    #[test]
    fn policy_tail_metrics_keep_physical_and_service_axes_separate() {
        let bundle = compile_oracle_outcomes(&valid_dataset()).expect("valid oracle data");
        let hard = tail_metrics_by_policy(
            &bundle,
            DatasetRole::Calibration,
            LossAxis::HardPhysicalFailure,
            0.50,
        )
        .expect("hard-failure tail");
        let service = tail_metrics_by_policy(
            &bundle,
            DatasetRole::Calibration,
            LossAxis::ServiceLoss,
            0.50,
        )
        .expect("service-loss tail");

        assert_eq!(hard[0].policy_id, "policy-B");
        assert_eq!(hard[1].policy_id, "policy-A");
        assert_eq!(hard[0].maximum_loss_milli, 1_000);
        assert_eq!(service[0].maximum_loss_milli, 600);
        assert_ne!(
            hard[0].empirical_upper_cvar_milli,
            service[0].empirical_upper_cvar_milli
        );
    }

    #[test]
    fn adapter_converts_only_the_requested_axis_to_non_monotone_grid() {
        let bundle = compile_oracle_outcomes(&valid_dataset()).expect("valid oracle data");
        let hard =
            bundle.non_monotone_grid(DatasetRole::Calibration, LossAxis::HardPhysicalFailure);
        let service = bundle.non_monotone_grid(DatasetRole::Calibration, LossAxis::ServiceLoss);
        assert_ne!(hard.schedule_losses_milli, service.schedule_losses_milli);
    }
}
