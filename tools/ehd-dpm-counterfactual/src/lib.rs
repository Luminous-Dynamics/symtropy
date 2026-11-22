//! Drift-balanced causal qualification of EHD diagnostic interventions.
//!
//! The evaluator consumes measurements from a pre-registered, randomized
//! four-period crossover. It does not generate assignments, verify signatures,
//! control the laboratory, or establish that a claimed reset is physically real.
//! Those activities require independent receipts and instrumentation.
//!
//! Each block uses ABBA (control, diagnostic, diagnostic, control) or BAAB
//! (diagnostic, control, control, diagnostic). With symmetric period timing,
//! either sequence cancels a first-order linear within-block drift. Balanced
//! assignment of ABBA and BAAB across independent clusters also counterbalances
//! a common second-order drift term in the cohort mean. Neither property
//! removes arbitrary non-stationarity, unmeasured condition changes, carryover,
//! or common-cause dependencies. Surface reset and purge are required before
//! every period; unverifiable reset/purge keeps the result unqualified.
//!
//! Effects are averaged first within the declared independent acquisition
//! cluster, then across clusters with equal weight. A conservative measurement
//! bound is added to a cluster-level t-based sampling margin. The fixed
//! t=2.093 value is the two-sided 95% critical value for 19 degrees of freedom,
//! used conservatively for n>=20 under an approximately normal cluster-mean
//! assumption. This is not a distribution-free guarantee.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

const REQUIRED_INDEPENDENT_CLUSTERS: usize = 20;
const CLUSTER_T_95_DF_19: f64 = 2.093;
const REQUIRED_PERIODS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CrossoverSequence {
    ControlDiagnosticDiagnosticControl,
    DiagnosticControlControlDiagnostic,
}

impl CrossoverSequence {
    fn expected_diagnostic_pattern(self) -> [bool; REQUIRED_PERIODS] {
        match self {
            Self::ControlDiagnosticDiagnosticControl => [false, true, true, false],
            Self::DiagnosticControlControlDiagnostic => [true, false, false, true],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeriodObservation {
    pub period_index: u8,
    pub diagnostic_active: bool,
    /// Elapsed seconds from the start of the crossover block.
    pub elapsed_seconds: f64,
    pub outcome: f64,
    /// Conservative absolute bound in the outcome's declared unit.
    pub absolute_uncertainty_bound: f64,
    pub evidence_id: String,
    pub surface_reset_receipt_id: String,
    pub surface_reset_verified: bool,
    pub purge_receipt_id: String,
    pub purge_verified: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrossoverBlock {
    pub block_id: String,
    /// Highest defensible independent acquisition/common-cause cluster.
    /// Multiple blocks in one cluster are averaged before campaign inference.
    pub independence_cluster_id: String,
    pub operating_condition_id: String,
    pub sequence: CrossoverSequence,
    pub periods: [PeriodObservation; REQUIRED_PERIODS],
    /// External gates; this evaluator does not compute the ledgers or envelope.
    pub validation_envelope_passed: bool,
    pub conservation_ledgers_passed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CounterfactualInput {
    pub campaign_id: String,
    pub diagnostic_id: String,
    pub operating_condition_id: String,
    pub outcome_unit: String,
    pub frozen_protocol_receipt_id: String,
    pub frozen_analysis_plan_receipt_id: String,
    pub randomization_receipt_id: String,
    /// Set only by an independent audit of the allocation manifest.
    pub randomization_assignment_verified: bool,
    /// Maximum physically acceptable absolute active effect, declared pre-run.
    pub maximum_allowed_active_effect: f64,
    /// Period midpoint symmetry tolerance in seconds.
    pub time_symmetry_tolerance_seconds: f64,
    pub blocks: Vec<CrossoverBlock>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectClassification {
    WithinTolerance,
    PerturbationDetected,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterfactualDisposition {
    WithinTolerance,
    ObserverPerturbationDetected,
    ObserverPerturbationUnresolved,
    RandomizationNotVerified,
    RandomizationSequenceImbalance,
    InsufficientIndependentClusters,
    SurfaceResetUnverified,
    PurgeUnverified,
    OutOfValidationEnvelope,
    PhysicalLedgerFailure,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterEffect {
    pub independence_cluster_id: String,
    pub sequence: CrossoverSequence,
    pub block_count: usize,
    pub mean_signed_effect: f64,
    pub mean_absolute_measurement_bound: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectInterval {
    pub mean_signed_effect: f64,
    pub cluster_standard_error: Option<f64>,
    pub statistical_95_margin: Option<f64>,
    pub mean_absolute_measurement_bound: f64,
    pub combined_absolute_uncertainty: Option<f64>,
    pub lower_signed_effect: Option<f64>,
    pub upper_signed_effect: Option<f64>,
    pub lower_absolute_effect: Option<f64>,
    pub upper_absolute_effect: Option<f64>,
    pub maximum_allowed_absolute_effect: f64,
    pub classification: EffectClassification,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CounterfactualReport {
    pub campaign_id: String,
    pub diagnostic_id: String,
    pub operating_condition_id: String,
    pub outcome_unit: String,
    pub block_count: usize,
    pub independent_cluster_count: usize,
    pub abba_cluster_count: usize,
    pub baab_cluster_count: usize,
    pub sequence_allocation_balanced: bool,
    pub first_order_linear_drift_cancelled_by_design: bool,
    pub all_surface_resets_verified: bool,
    pub all_purges_verified: bool,
    pub all_validation_envelopes_passed: bool,
    pub all_conservation_ledgers_passed: bool,
    pub cluster_effects: Vec<ClusterEffect>,
    pub active_effect: EffectInterval,
    pub disposition: CounterfactualDisposition,
    pub limitations: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CounterfactualError {
    EmptyIdentity { field: &'static str },
    NoBlocks,
    InvalidTolerance,
    InvalidTimeSymmetryTolerance,
    DuplicateBlockId { block_id: String },
    DuplicateEvidenceId { evidence_id: String },
    DuplicateResetReceiptId { receipt_id: String },
    DuplicatePurgeReceiptId { receipt_id: String },
    InvalidPeriodIndex { block_id: String, period_index: u8 },
    SequencePatternMismatch { block_id: String },
    NonFinitePeriodValue { block_id: String, period_index: u8 },
    InvalidMeasurementBound { block_id: String, period_index: u8 },
    NonMonotonicPeriodTime { block_id: String },
    NonSymmetricPeriodTiming { block_id: String },
    NonFiniteDerivedValue,
    MixedSequencesWithinCluster { cluster_id: String },
}

impl fmt::Display for CounterfactualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "counterfactual {field} must be non-empty"),
            Self::NoBlocks => write!(f, "counterfactual campaign requires at least one block"),
            Self::InvalidTolerance => write!(f, "maximum allowed effect must be finite and non-negative"),
            Self::InvalidTimeSymmetryTolerance => write!(f, "time symmetry tolerance must be finite and positive"),
            Self::DuplicateBlockId { block_id } => write!(f, "duplicate crossover block identity: {block_id}"),
            Self::DuplicateEvidenceId { evidence_id } => write!(f, "duplicate period evidence identity: {evidence_id}"),
            Self::DuplicateResetReceiptId { receipt_id } => write!(f, "duplicate surface-reset receipt: {receipt_id}"),
            Self::DuplicatePurgeReceiptId { receipt_id } => write!(f, "duplicate purge receipt: {receipt_id}"),
            Self::InvalidPeriodIndex { block_id, period_index } => {
                write!(f, "block {block_id} has invalid period index {period_index}")
            }
            Self::SequencePatternMismatch { block_id } => {
                write!(f, "block {block_id} does not match its declared ABBA/BAAB intervention sequence")
            }
            Self::NonFinitePeriodValue { block_id, period_index } => {
                write!(f, "block {block_id} period {period_index} has a non-finite value or time")
            }
            Self::InvalidMeasurementBound { block_id, period_index } => {
                write!(f, "block {block_id} period {period_index} has an invalid uncertainty bound")
            }
            Self::NonMonotonicPeriodTime { block_id } => {
                write!(f, "block {block_id} period times must be strictly increasing")
            }
            Self::NonSymmetricPeriodTiming { block_id } => {
                write!(f, "block {block_id} treatment/control time midpoints are not symmetric")
            }
            Self::NonFiniteDerivedValue => write!(f, "counterfactual effect calculation is non-finite"),
            Self::MixedSequencesWithinCluster { cluster_id } => {
                write!(f, "independent cluster {cluster_id} contains mixed sequence assignments")
            }
        }
    }
}

impl Error for CounterfactualError {}

#[derive(Default)]
struct ClusterAccumulator {
    sequence: Option<CrossoverSequence>,
    effects: Vec<f64>,
    measurement_bounds: Vec<f64>,
}

fn validate_identity(field: &'static str, value: &str) -> Result<(), CounterfactualError> {
    if value.trim().is_empty() {
        return Err(CounterfactualError::EmptyIdentity { field });
    }
    Ok(())
}

fn effect_for_block(
    block: &CrossoverBlock,
    input: &CounterfactualInput,
    evidence_ids: &mut BTreeSet<String>,
    reset_ids: &mut BTreeSet<String>,
    purge_ids: &mut BTreeSet<String>,
) -> Result<(f64, f64, bool, bool), CounterfactualError> {
    validate_identity("block_id", &block.block_id)?;
    validate_identity("independence_cluster_id", &block.independence_cluster_id)?;
    validate_identity("block_operating_condition_id", &block.operating_condition_id)?;

    if block.operating_condition_id != input.operating_condition_id {
        return Err(CounterfactualError::EmptyIdentity {
            field: "matched_operating_condition_identity",
        });
    }

    let expected_pattern = block.sequence.expected_diagnostic_pattern();
    let mut control_sum = 0.0;
    let mut diagnostic_sum = 0.0;
    let mut uncertainty_sum = 0.0;
    let mut previous_time = f64::NEG_INFINITY;
    let mut all_resets_verified = true;
    let mut all_purges_verified = true;

    for (index, period) in block.periods.iter().enumerate() {
        let expected_index = (index + 1) as u8;
        if period.period_index != expected_index {
            return Err(CounterfactualError::InvalidPeriodIndex {
                block_id: block.block_id.clone(),
                period_index: period.period_index,
            });
        }
        if period.diagnostic_active != expected_pattern[index] {
            return Err(CounterfactualError::SequencePatternMismatch {
                block_id: block.block_id.clone(),
            });
        }
        if !period.elapsed_seconds.is_finite() || !period.outcome.is_finite() {
            return Err(CounterfactualError::NonFinitePeriodValue {
                block_id: block.block_id.clone(),
                period_index: period.period_index,
            });
        }
        if !period.absolute_uncertainty_bound.is_finite()
            || period.absolute_uncertainty_bound < 0.0
        {
            return Err(CounterfactualError::InvalidMeasurementBound {
                block_id: block.block_id.clone(),
                period_index: period.period_index,
            });
        }
        if period.elapsed_seconds <= previous_time {
            return Err(CounterfactualError::NonMonotonicPeriodTime {
                block_id: block.block_id.clone(),
            });
        }
        previous_time = period.elapsed_seconds;

        validate_identity("period_evidence_id", &period.evidence_id)?;
        validate_identity("surface_reset_receipt_id", &period.surface_reset_receipt_id)?;
        validate_identity("purge_receipt_id", &period.purge_receipt_id)?;

        if !evidence_ids.insert(period.evidence_id.clone()) {
            return Err(CounterfactualError::DuplicateEvidenceId {
                evidence_id: period.evidence_id.clone(),
            });
        }
        if !reset_ids.insert(period.surface_reset_receipt_id.clone()) {
            return Err(CounterfactualError::DuplicateResetReceiptId {
                receipt_id: period.surface_reset_receipt_id.clone(),
            });
        }
        if !purge_ids.insert(period.purge_receipt_id.clone()) {
            return Err(CounterfactualError::DuplicatePurgeReceiptId {
                receipt_id: period.purge_receipt_id.clone(),
            });
        }

        if period.diagnostic_active {
            diagnostic_sum += period.outcome;
        } else {
            control_sum += period.outcome;
        }
        uncertainty_sum += period.absolute_uncertainty_bound;
        all_resets_verified &= period.surface_reset_verified;
        all_purges_verified &= period.purge_verified;
    }

    let times = block.periods.each_ref().map(|period| period.elapsed_seconds);
    let control_time_sum = if expected_pattern[0] {
        times[1] + times[2]
    } else {
        times[0] + times[3]
    };
    let diagnostic_time_sum = if expected_pattern[0] {
        times[0] + times[3]
    } else {
        times[1] + times[2]
    };
    if (control_time_sum - diagnostic_time_sum).abs()
        > input.time_symmetry_tolerance_seconds * 2.0
    {
        return Err(CounterfactualError::NonSymmetricPeriodTiming {
            block_id: block.block_id.clone(),
        });
    }

    // Two measurements per state. Absolute bounds are conservatively added
    // through the difference of the two means (1/2 sum of all four bounds).
    let signed_effect = (diagnostic_sum - control_sum) / 2.0;
    let measurement_bound = uncertainty_sum / 2.0;
    if !signed_effect.is_finite() || !measurement_bound.is_finite() {
        return Err(CounterfactualError::NonFiniteDerivedValue);
    }

    Ok((
        signed_effect,
        measurement_bound,
        all_resets_verified,
        all_purges_verified,
    ))
}

fn classify_effect(
    mean_effect: f64,
    combined_uncertainty: f64,
    maximum_allowed_effect: f64,
) -> EffectClassification {
    let lower_signed = mean_effect - combined_uncertainty;
    let upper_signed = mean_effect + combined_uncertainty;
    let lower_absolute = if lower_signed <= 0.0 && upper_signed >= 0.0 {
        0.0
    } else {
        lower_signed.abs().min(upper_signed.abs())
    };
    let upper_absolute = lower_signed.abs().max(upper_signed.abs());

    if lower_absolute > maximum_allowed_effect {
        EffectClassification::PerturbationDetected
    } else if upper_absolute <= maximum_allowed_effect {
        EffectClassification::WithinTolerance
    } else {
        EffectClassification::Unresolved
    }
}

/// Evaluate a pre-registered, randomized ABBA/BAAB observer-intervention
/// campaign. No intervention is declared passive merely because the effect
/// estimate is statistically non-significant: the complete uncertainty
/// interval must lie inside the predeclared physical tolerance to qualify.
pub fn qualify_counterfactual_intervention(
    input: &CounterfactualInput,
) -> Result<CounterfactualReport, CounterfactualError> {
    for (field, value) in [
        ("campaign_id", input.campaign_id.as_str()),
        ("diagnostic_id", input.diagnostic_id.as_str()),
        ("operating_condition_id", input.operating_condition_id.as_str()),
        ("outcome_unit", input.outcome_unit.as_str()),
        ("frozen_protocol_receipt_id", input.frozen_protocol_receipt_id.as_str()),
        ("frozen_analysis_plan_receipt_id", input.frozen_analysis_plan_receipt_id.as_str()),
        ("randomization_receipt_id", input.randomization_receipt_id.as_str()),
    ] {
        validate_identity(field, value)?;
    }
    if !input.maximum_allowed_active_effect.is_finite()
        || input.maximum_allowed_active_effect < 0.0
    {
        return Err(CounterfactualError::InvalidTolerance);
    }
    if !input.time_symmetry_tolerance_seconds.is_finite()
        || input.time_symmetry_tolerance_seconds <= 0.0
    {
        return Err(CounterfactualError::InvalidTimeSymmetryTolerance);
    }
    if input.blocks.is_empty() {
        return Err(CounterfactualError::NoBlocks);
    }

    let mut block_ids = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();
    let mut reset_ids = BTreeSet::new();
    let mut purge_ids = BTreeSet::new();
    let mut clusters: BTreeMap<String, ClusterAccumulator> = BTreeMap::new();
    let mut all_surface_resets_verified = true;
    let mut all_purges_verified = true;
    let mut all_validation_envelopes_passed = true;
    let mut all_conservation_ledgers_passed = true;

    for block in &input.blocks {
        if !block_ids.insert(block.block_id.clone()) {
            return Err(CounterfactualError::DuplicateBlockId {
                block_id: block.block_id.clone(),
            });
        }
        let (effect, bound, resets_verified, purges_verified) =
            effect_for_block(block, input, &mut evidence_ids, &mut reset_ids, &mut purge_ids)?;

        let cluster = clusters.entry(block.independence_cluster_id.clone()).or_default();
        if let Some(existing) = cluster.sequence {
            if existing != block.sequence {
                return Err(CounterfactualError::MixedSequencesWithinCluster {
                    cluster_id: block.independence_cluster_id.clone(),
                });
            }
        } else {
            cluster.sequence = Some(block.sequence);
        }
        cluster.effects.push(effect);
        cluster.measurement_bounds.push(bound);

        all_surface_resets_verified &= resets_verified;
        all_purges_verified &= purges_verified;
        all_validation_envelopes_passed &= block.validation_envelope_passed;
        all_conservation_ledgers_passed &= block.conservation_ledgers_passed;
    }

    let independent_cluster_count = clusters.len();
    let mut abba_cluster_count = 0usize;
    let mut baab_cluster_count = 0usize;
    let mut cluster_effects = Vec::with_capacity(independent_cluster_count);
    let mut campaign_cluster_means = Vec::with_capacity(independent_cluster_count);
    let mut campaign_cluster_bounds = Vec::with_capacity(independent_cluster_count);

    for (cluster_id, accumulator) in clusters {
        let block_count = accumulator.effects.len();
        let effect_mean = accumulator.effects.iter().sum::<f64>() / block_count as f64;
        let bound_mean = accumulator.measurement_bounds.iter().sum::<f64>() / block_count as f64;
        let sequence = accumulator.sequence.expect("cluster sequence assigned during validation");

        match sequence {
            CrossoverSequence::ControlDiagnosticDiagnosticControl => abba_cluster_count += 1,
            CrossoverSequence::DiagnosticControlControlDiagnostic => baab_cluster_count += 1,
        }
        campaign_cluster_means.push(effect_mean);
        campaign_cluster_bounds.push(bound_mean);
        cluster_effects.push(ClusterEffect {
            independence_cluster_id: cluster_id,
            sequence,
            block_count,
            mean_signed_effect: effect_mean,
            mean_absolute_measurement_bound: bound_mean,
        });
    }

    let sequence_allocation_balanced =
        abba_cluster_count.abs_diff(baab_cluster_count) <= 1;
    let mean_signed_effect =
        campaign_cluster_means.iter().sum::<f64>() / independent_cluster_count as f64;
    let mean_absolute_measurement_bound =
        campaign_cluster_bounds.iter().sum::<f64>() / independent_cluster_count as f64;

    if !mean_signed_effect.is_finite() || !mean_absolute_measurement_bound.is_finite() {
        return Err(CounterfactualError::NonFiniteDerivedValue);
    }

    let (cluster_standard_error, statistical_95_margin, combined_absolute_uncertainty) =
        if independent_cluster_count >= REQUIRED_INDEPENDENT_CLUSTERS {
            let count = independent_cluster_count as f64;
            let squared_deviation = campaign_cluster_means
                .iter()
                .map(|value| {
                    let delta = *value - mean_signed_effect;
                    delta * delta
                })
                .sum::<f64>();
            let sample_variance = squared_deviation / (count - 1.0);
            let standard_error = (sample_variance / count).sqrt();
            let margin = CLUSTER_T_95_DF_19 * standard_error;
            (
                Some(standard_error),
                Some(margin),
                Some(margin + mean_absolute_measurement_bound),
            )
        } else {
            (None, None, None)
        };

    let (lower_signed, upper_signed, lower_absolute, upper_absolute, classification) =
        if let Some(uncertainty) = combined_absolute_uncertainty {
            let lower_signed = mean_signed_effect - uncertainty;
            let upper_signed = mean_signed_effect + uncertainty;
            let lower_absolute = if lower_signed <= 0.0 && upper_signed >= 0.0 {
                0.0
            } else {
                lower_signed.abs().min(upper_signed.abs())
            };
            let upper_absolute = lower_signed.abs().max(upper_signed.abs());
            (
                Some(lower_signed),
                Some(upper_signed),
                Some(lower_absolute),
                Some(upper_absolute),
                classify_effect(
                    mean_signed_effect,
                    uncertainty,
                    input.maximum_allowed_active_effect,
                ),
            )
        } else {
            (None, None, None, None, EffectClassification::Unresolved)
        };

    let active_effect = EffectInterval {
        mean_signed_effect,
        cluster_standard_error,
        statistical_95_margin,
        mean_absolute_measurement_bound,
        combined_absolute_uncertainty,
        lower_signed_effect: lower_signed,
        upper_signed_effect: upper_signed,
        lower_absolute_effect: lower_absolute,
        upper_absolute_effect: upper_absolute,
        maximum_allowed_absolute_effect: input.maximum_allowed_active_effect,
        classification,
    };

    // A physically invalid, out-of-envelope, unreset, or unpurged campaign
    // cannot be promoted by a small statistical effect estimate.
    let disposition = if !all_conservation_ledgers_passed {
        CounterfactualDisposition::PhysicalLedgerFailure
    } else if !all_validation_envelopes_passed {
        CounterfactualDisposition::OutOfValidationEnvelope
    } else if !input.randomization_assignment_verified {
        CounterfactualDisposition::RandomizationNotVerified
    } else if !sequence_allocation_balanced {
        CounterfactualDisposition::RandomizationSequenceImbalance
    } else if !all_surface_resets_verified {
        CounterfactualDisposition::SurfaceResetUnverified
    } else if !all_purges_verified {
        CounterfactualDisposition::PurgeUnverified
    } else if independent_cluster_count < REQUIRED_INDEPENDENT_CLUSTERS {
        CounterfactualDisposition::InsufficientIndependentClusters
    } else {
        match classification {
            EffectClassification::WithinTolerance => CounterfactualDisposition::WithinTolerance,
            EffectClassification::PerturbationDetected => {
                CounterfactualDisposition::ObserverPerturbationDetected
            }
            EffectClassification::Unresolved => {
                CounterfactualDisposition::ObserverPerturbationUnresolved
            }
        }
    };

    Ok(CounterfactualReport {
        campaign_id: input.campaign_id.clone(),
        diagnostic_id: input.diagnostic_id.clone(),
        operating_condition_id: input.operating_condition_id.clone(),
        outcome_unit: input.outcome_unit.clone(),
        block_count: input.blocks.len(),
        independent_cluster_count,
        abba_cluster_count,
        baab_cluster_count,
        sequence_allocation_balanced,
        first_order_linear_drift_cancelled_by_design: true,
        all_surface_resets_verified,
        all_purges_verified,
        all_validation_envelopes_passed,
        all_conservation_ledgers_passed,
        cluster_effects,
        active_effect,
        disposition,
        limitations: "Randomized switchback reference only. Requires independently verified randomization, physical surface reset and purge receipts. Symmetric ABBA/BAAB cancels first-order linear drift; balanced sequence allocation counterbalances a common quadratic drift term only under the stated timing and cohort assumptions. It does not remove arbitrary non-stationarity, hidden common causes, unmeasured environmental changes, or carryover that survives the declared reset. Cluster-level t interval assumes approximately normal independent cluster means; no raw PIV, tracer-transport, electrode-fouling or thrust model is implemented.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(
        index: usize,
        sequence: CrossoverSequence,
        effect: f64,
        slope: f64,
        curvature: f64,
    ) -> CrossoverBlock {
        let pattern = sequence.expected_diagnostic_pattern();
        let periods = std::array::from_fn(|period| {
            let time = period as f64;
            let baseline = 10.0 + index as f64 * 0.01;
            let treatment = if pattern[period] { effect } else { 0.0 };
            PeriodObservation {
                period_index: (period + 1) as u8,
                diagnostic_active: pattern[period],
                elapsed_seconds: time,
                outcome: baseline + slope * time + curvature * time * time + treatment,
                absolute_uncertainty_bound: 0.01,
                evidence_id: format!("evidence-{index}-{period}"),
                surface_reset_receipt_id: format!("reset-{index}-{period}"),
                surface_reset_verified: true,
                purge_receipt_id: format!("purge-{index}-{period}"),
                purge_verified: true,
            }
        });
        CrossoverBlock {
            block_id: format!("block-{index}"),
            independence_cluster_id: format!("session-{index}"),
            operating_condition_id: "needle-ring-14kv-rh50".to_owned(),
            sequence,
            periods,
            validation_envelope_passed: true,
            conservation_ledgers_passed: true,
        }
    }

    fn input(effect: f64, slope: f64, curvature: f64) -> CounterfactualInput {
        let blocks = (0..REQUIRED_INDEPENDENT_CLUSTERS)
            .map(|index| {
                let sequence = if index % 2 == 0 {
                    CrossoverSequence::ControlDiagnosticDiagnosticControl
                } else {
                    CrossoverSequence::DiagnosticControlControlDiagnostic
                };
                block(index, sequence, effect, slope, curvature)
            })
            .collect();
        CounterfactualInput {
            campaign_id: "DPM-002-MANUFACTURED".to_owned(),
            diagnostic_id: "EHD-PIV-seeding".to_owned(),
            operating_condition_id: "needle-ring-14kv-rh50".to_owned(),
            outcome_unit: "m/s".to_owned(),
            frozen_protocol_receipt_id: "protocol-sha256-example".to_owned(),
            frozen_analysis_plan_receipt_id: "analysis-plan-sha256-example".to_owned(),
            randomization_receipt_id: "allocation-manifest-example".to_owned(),
            randomization_assignment_verified: true,
            maximum_allowed_active_effect: 0.05,
            time_symmetry_tolerance_seconds: 0.01,
            blocks,
        }
    }

    #[test]
    fn crossover_detects_diagnostic_effect_despite_linear_drift() {
        let report = qualify_counterfactual_intervention(&input(0.4, 3.0, 0.0)).unwrap();
        assert!((report.active_effect.mean_signed_effect - 0.4).abs() < 1.0e-12);
        assert_eq!(report.abba_cluster_count, 10);
        assert_eq!(report.baab_cluster_count, 10);
        assert_eq!(
            report.disposition,
            CounterfactualDisposition::ObserverPerturbationDetected
        );
    }

    #[test]
    fn no_diagnostic_effect_with_linear_drift_can_meet_equivalence_tolerance() {
        let report = qualify_counterfactual_intervention(&input(0.0, 5.0, 0.0)).unwrap();
        assert!(report.active_effect.upper_absolute_effect.unwrap() <= 0.05);
        assert_eq!(report.disposition, CounterfactualDisposition::WithinTolerance);
    }

    #[test]
    fn balanced_sequences_counterbalance_common_quadratic_drift() {
        let mut campaign = input(0.0, 0.0, 0.1);
        campaign.maximum_allowed_active_effect = 0.2;
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert!(report.active_effect.mean_signed_effect.abs() < 1.0e-12);
        assert_eq!(report.disposition, CounterfactualDisposition::WithinTolerance);
    }

    #[test]
    fn uncertainty_overlap_with_equivalence_boundary_is_unresolved() {
        let campaign = input(0.06, 0.0, 0.0);
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert_eq!(
            report.active_effect.classification,
            EffectClassification::Unresolved
        );
        assert_eq!(
            report.disposition,
            CounterfactualDisposition::ObserverPerturbationUnresolved
        );
    }

    #[test]
    fn asymmetric_period_timing_is_rejected() {
        let mut campaign = input(0.1, 0.0, 0.0);
        campaign.blocks[0].periods[3].elapsed_seconds = 3.2;
        assert!(matches!(
            qualify_counterfactual_intervention(&campaign),
            Err(CounterfactualError::NonSymmetricPeriodTiming { .. })
        ));
    }

    #[test]
    fn failed_surface_reset_prevents_passivity_qualification() {
        let mut campaign = input(0.0, 0.0, 0.0);
        campaign.blocks[0].periods[2].surface_reset_verified = false;
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert_eq!(report.disposition, CounterfactualDisposition::SurfaceResetUnverified);
    }

    #[test]
    fn failed_purge_prevents_passivity_qualification() {
        let mut campaign = input(0.0, 0.0, 0.0);
        campaign.blocks[0].periods[1].purge_verified = false;
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert_eq!(report.disposition, CounterfactualDisposition::PurgeUnverified);
    }

    #[test]
    fn randomization_must_be_independently_verified() {
        let mut campaign = input(0.0, 0.0, 0.0);
        campaign.randomization_assignment_verified = false;
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert_eq!(report.disposition, CounterfactualDisposition::RandomizationNotVerified);
    }

    #[test]
    fn sequence_assignment_must_be_balanced_across_independent_clusters() {
        let mut campaign = input(0.0, 0.0, 0.0);
        for block in campaign.blocks.iter_mut().skip(10) {
            block.sequence = CrossoverSequence::ControlDiagnosticDiagnosticControl;
            let pattern = block.sequence.expected_diagnostic_pattern();
            for (period, active) in block.periods.iter_mut().zip(pattern) {
                period.diagnostic_active = active;
            }
        }
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert_eq!(report.disposition, CounterfactualDisposition::RandomizationSequenceImbalance);
    }

    #[test]
    fn many_blocks_inside_one_session_do_not_create_independent_clusters() {
        let mut campaign = input(0.0, 0.0, 0.0);
        campaign.blocks.truncate(20);
        for (index, block) in campaign.blocks.iter_mut().enumerate() {
            block.independence_cluster_id = "same-lab-session".to_owned();
            block.sequence = CrossoverSequence::ControlDiagnosticDiagnosticControl;
            let pattern = block.sequence.expected_diagnostic_pattern();
            for (period, active) in block.periods.iter_mut().zip(pattern) {
                period.diagnostic_active = active;
            }
            block.block_id = format!("repeated-block-{index}");
        }
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert_eq!(report.independent_cluster_count, 1);
        assert_eq!(report.disposition, CounterfactualDisposition::InsufficientIndependentClusters);
    }

    #[test]
    fn physical_ledger_failure_has_priority_over_small_effect() {
        let mut campaign = input(0.0, 0.0, 0.0);
        campaign.blocks[0].conservation_ledgers_pass = false;
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert_eq!(report.disposition, CounterfactualDisposition::PhysicalLedgerFailure);
    }

    #[test]
    fn out_of_envelope_cannot_be_rescued_by_small_effect() {
        let mut campaign = input(0.0, 0.0, 0.0);
        campaign.blocks[0].validation_envelope_passed = false;
        let report = qualify_counterfactual_intervention(&campaign).unwrap();
        assert_eq!(report.disposition, CounterfactualDisposition::OutOfValidationEnvelope);
    }

    #[test]
    fn duplicate_evidence_is_rejected() {
        let mut campaign = input(0.0, 0.0, 0.0);
        campaign.blocks[0].periods[1].evidence_id =
            campaign.blocks[0].periods[0].evidence_id.clone();
        assert!(matches!(
            qualify_counterfactual_intervention(&campaign),
            Err(CounterfactualError::DuplicateEvidenceId { .. })
        ));
    }

    #[test]
    fn crossover_sequence_mismatch_is_rejected() {
        let mut campaign = input(0.0, 0.0, 0.0);
        campaign.blocks[0].periods[1].diagnostic_active = false;
        assert!(matches!(
            qualify_counterfactual_intervention(&campaign),
            Err(CounterfactualError::SequencePatternMismatch { .. })
        ));
    }
}
