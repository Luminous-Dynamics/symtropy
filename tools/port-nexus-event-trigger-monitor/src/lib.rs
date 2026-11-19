//! Deterministic, read-only event-trigger monitor for port logistics,
//! grid, compute and cooling coordination.
//!
//! The monitor emits a request for an existing owner optimizer to re-plan.
//! It never creates a schedule, dispatches storage, moves workloads, changes
//! resource reservations, sends grid instructions, or actuates equipment.
//!
//! Inputs are caller-supplied snapshots. Evidence references and quality labels
//! are carried through, but are not authenticated by this crate.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalDomain {
    Logistics,
    ShorePower,
    Terminal,
    Grid,
    Renewables,
    Compute,
    Cooling,
    ThermalStore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceQuality {
    Qualified,
    Synthetic,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HardBound {
    Upper { limit: f64 },
    Lower { limit: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SignalDefinition {
    pub signal_id: String,
    pub domain: SignalDomain,
    pub unit: String,
    pub baseline_value: f64,
    pub soft_absolute_threshold: f64,
    pub soft_relative_threshold: f64,
    pub max_age_ms: u64,
    pub critical: bool,
    pub hard_bound: Option<HardBound>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SignalMeasurement {
    pub signal_id: String,
    pub observed_value: f64,
    /// Time the physical observation applies to.
    pub observed_at_ms: u64,
    /// Earliest time this exact observation was available to the decision maker.
    pub first_available_at_ms: u64,
    pub evidence_ref: String,
    pub quality: EvidenceQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggerSnapshot {
    pub snapshot_id: String,
    /// Caller-supplied digest of the canonical snapshot bytes.
    pub snapshot_digest: String,
    pub site_id: String,
    pub baseline_plan_id: String,
    pub sequence: u64,
    pub decision_time_ms: u64,
    pub plan_generated_at_ms: u64,
    pub measurements: Vec<SignalMeasurement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggerPolicy {
    pub normal_cooldown_ms: u64,
    pub max_plan_age_ms: u64,
    /// Reset the soft-trigger latch only when every measured signal returns
    /// below this fraction of its own trigger threshold.
    pub hysteresis_reset_ratio: f64,
}

impl TriggerPolicy {
    pub fn validate(&self) -> Result<(), TriggerError> {
        if self.max_plan_age_ms == 0 {
            return Err(TriggerError::InvalidPolicy(
                "maximum plan age must be positive",
            ));
        }
        if !self.hysteresis_reset_ratio.is_finite()
            || !(0.0..1.0).contains(&self.hysteresis_reset_ratio)
        {
            return Err(TriggerError::InvalidPolicy(
                "hysteresis reset ratio must be finite and in [0, 1)",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TriggerState {
    pub baseline_plan_id: Option<String>,
    pub last_snapshot_id: Option<String>,
    pub last_snapshot_digest: Option<String>,
    pub last_snapshot_sequence: Option<u64>,
    pub last_decision_time_ms: Option<u64>,
    pub last_trigger_time_ms: Option<u64>,
    pub soft_latched: bool,
    /// Sorted reason signatures currently active for hard events.
    pub active_hard_reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerPriority {
    Hard,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressionReason {
    Cooldown,
    Hysteresis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerDecision {
    NoTrigger,
    ReplanRequested { priority: TriggerPriority },
    DataIncomplete,
    Suppressed { reason: SuppressionReason },
    DuplicateSnapshotIgnored,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SignalReceipt {
    pub signal_id: String,
    pub domain: SignalDomain,
    pub unit: String,
    pub observed_value: f64,
    pub baseline_value: f64,
    pub absolute_deviation: f64,
    pub soft_threshold: f64,
    pub normalized_deviation: f64,
    pub age_ms: u64,
    pub hard_violation: bool,
    pub evidence_ref: String,
    pub evidence_quality: EvidenceQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggerEvaluation {
    pub snapshot_id: String,
    pub baseline_plan_id: String,
    pub decision: TriggerDecision,
    pub reason_codes: Vec<String>,
    pub evidence_refresh_required: bool,
    pub signal_receipts: Vec<SignalReceipt>,
    pub next_state: TriggerState,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerError {
    MissingIdentity(&'static str),
    InvalidPolicy(&'static str),
    InvalidSignal { signal_id: String, reason: &'static str },
    DuplicateId(String),
    UnknownMeasurement(String),
    MissingMeasurement(String),
    FutureObservation(String),
    InvalidChronology(&'static str),
    NumericalOverflow(&'static str),
}

impl fmt::Display for TriggerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity(field) => write!(f, "{field} must not be empty"),
            Self::InvalidPolicy(reason) => write!(f, "invalid trigger policy: {reason}"),
            Self::InvalidSignal { signal_id, reason } => {
                write!(f, "invalid signal {signal_id}: {reason}")
            }
            Self::DuplicateId(id) => write!(f, "duplicate ID: {id}"),
            Self::UnknownMeasurement(id) => write!(f, "measurement has unknown signal {id}"),
            Self::MissingMeasurement(id) => write!(f, "missing measurement for signal {id}"),
            Self::FutureObservation(id) => {
                write!(f, "signal {id} was not available at the decision cutoff")
            }
            Self::InvalidChronology(reason) => write!(f, "invalid snapshot chronology: {reason}"),
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite arithmetic during {operation}")
            }
        }
    }
}

impl Error for TriggerError {}

fn require_id(field: &'static str, value: &str) -> Result<(), TriggerError> {
    if value.trim().is_empty() {
        return Err(TriggerError::MissingIdentity(field));
    }
    Ok(())
}

fn require_finite(signal_id: &str, value: f64, field: &'static str) -> Result<(), TriggerError> {
    if !value.is_finite() {
        return Err(TriggerError::InvalidSignal {
            signal_id: signal_id.to_owned(),
            reason: field,
        });
    }
    Ok(())
}

fn threshold_for(definition: &SignalDefinition) -> Result<f64, TriggerError> {
    let relative = definition.soft_relative_threshold * definition.baseline_value.abs();
    let threshold = definition.soft_absolute_threshold.max(relative);
    if !threshold.is_finite() {
        return Err(TriggerError::NumericalOverflow("soft trigger threshold"));
    }
    Ok(threshold)
}

fn normalized_deviation(deviation: f64, threshold: f64) -> f64 {
    if threshold == 0.0 {
        if deviation == 0.0 { 0.0 } else { f64::MAX }
    } else {
        (deviation / threshold).min(f64::MAX)
    }
}

fn hard_bound_violated(bound: Option<HardBound>, value: f64) -> bool {
    match bound {
        Some(HardBound::Upper { limit }) => value > limit,
        Some(HardBound::Lower { limit }) => value < limit,
        None => false,
    }
}

fn validate_definitions(
    definitions: &[SignalDefinition],
) -> Result<BTreeMap<String, SignalDefinition>, TriggerError> {
    if definitions.is_empty() {
        return Err(TriggerError::InvalidPolicy(
            "at least one signal definition is required",
        ));
    }
    let mut map = BTreeMap::new();
    for definition in definitions {
        require_id("signal ID", &definition.signal_id)?;
        require_id("signal unit", &definition.unit)?;
        require_finite(
            &definition.signal_id,
            definition.baseline_value,
            "baseline must be finite",
        )?;
        require_finite(
            &definition.signal_id,
            definition.soft_absolute_threshold,
            "absolute threshold must be finite",
        )?;
        require_finite(
            &definition.signal_id,
            definition.soft_relative_threshold,
            "relative threshold must be finite",
        )?;
        if definition.soft_absolute_threshold < 0.0
            || definition.soft_relative_threshold < 0.0
        {
            return Err(TriggerError::InvalidSignal {
                signal_id: definition.signal_id.clone(),
                reason: "soft thresholds must be non-negative",
            });
        }
        if definition.max_age_ms == 0 {
            return Err(TriggerError::InvalidSignal {
                signal_id: definition.signal_id.clone(),
                reason: "maximum age must be positive",
            });
        }
        match definition.hard_bound {
            Some(HardBound::Upper { limit }) | Some(HardBound::Lower { limit }) => {
                require_finite(&definition.signal_id, limit, "hard limit must be finite")?;
            }
            None => {}
        }
        threshold_for(definition)?;
        if map.insert(definition.signal_id.clone(), definition.clone()).is_some() {
            return Err(TriggerError::DuplicateId(definition.signal_id.clone()));
        }
    }
    Ok(map)
}

fn update_snapshot_state(
    state: &TriggerState,
    snapshot: &TriggerSnapshot,
    baseline_changed: bool,
) -> TriggerState {
    let mut next = state.clone();
    next.baseline_plan_id = Some(snapshot.baseline_plan_id.clone());
    next.last_snapshot_id = Some(snapshot.snapshot_id.clone());
    next.last_snapshot_digest = Some(snapshot.snapshot_digest.clone());
    next.last_snapshot_sequence = Some(snapshot.sequence);
    next.last_decision_time_ms = Some(snapshot.decision_time_ms);
    if baseline_changed {
        next.last_trigger_time_ms = None;
        next.soft_latched = false;
        next.active_hard_reasons.clear();
    }
    next
}

/// Evaluate whether a new plan should be requested.
///
/// Missing or stale critical evidence returns DataIncomplete with
/// evidence_refresh_required=true; it never returns NoTrigger.
/// Hard-boundary events bypass cooldown and the soft hysteresis latch.
/// Repeated snapshots of the same persistent hard event do not emit duplicate
/// requests; a changed hard-reason signature emits a new request.
///
/// The monitor is read-only. The caller remains responsible for asking the
/// appropriate optimizer to re-plan and for independently verifying any result.
pub fn evaluate_trigger(
    snapshot: &TriggerSnapshot,
    definitions: &[SignalDefinition],
    policy: &TriggerPolicy,
    previous_state: &TriggerState,
) -> Result<TriggerEvaluation, TriggerError> {
    policy.validate()?;
    require_id("snapshot ID", &snapshot.snapshot_id)?;
    require_id("snapshot digest", &snapshot.snapshot_digest)?;
    require_id("site ID", &snapshot.site_id)?;
    require_id("baseline plan ID", &snapshot.baseline_plan_id)?;
    if snapshot.plan_generated_at_ms > snapshot.decision_time_ms {
        return Err(TriggerError::InvalidChronology(
            "plan generation is later than decision time",
        ));
    }

    if let Some(last_time) = previous_state.last_decision_time_ms {
        if snapshot.decision_time_ms < last_time {
            return Err(TriggerError::InvalidChronology(
                "decision time moved backwards",
            ));
        }
    }
    if let Some(last_sequence) = previous_state.last_snapshot_sequence {
        if snapshot.sequence < last_sequence {
            return Err(TriggerError::InvalidChronology(
                "snapshot sequence moved backwards",
            ));
        }
        if snapshot.sequence == last_sequence {
            if previous_state.baseline_plan_id.as_deref()
                == Some(snapshot.baseline_plan_id.as_str())
                && previous_state.last_snapshot_id.as_deref()
                    == Some(snapshot.snapshot_id.as_str())
                && previous_state.last_snapshot_digest.as_deref()
                    == Some(snapshot.snapshot_digest.as_str())
            {
                return Ok(TriggerEvaluation {
                    snapshot_id: snapshot.snapshot_id.clone(),
                    baseline_plan_id: snapshot.baseline_plan_id.clone(),
                    decision: TriggerDecision::DuplicateSnapshotIgnored,
                    reason_codes: vec!["DUPLICATE_SNAPSHOT".to_owned()],
                    evidence_refresh_required: false,
                    signal_receipts: Vec::new(),
                    next_state: previous_state.clone(),
                    claim_ceiling: "Read-only event-trigger evaluation over caller-supplied signal definitions and snapshots only; no schedule or physical action is authorized.",
                });
            }
            return Err(TriggerError::InvalidChronology(
                "same sequence cannot identify different snapshot digest or baseline",
            ));
        }
    }

    let definition_map = validate_definitions(definitions)?;
    let mut measurement_map = BTreeMap::new();
    for measurement in &snapshot.measurements {
        require_id("measurement signal ID", &measurement.signal_id)?;
        require_id("measurement evidence reference", &measurement.evidence_ref)?;
        require_finite(
            &measurement.signal_id,
            measurement.observed_value,
            "observed value must be finite",
        )?;
        if !definition_map.contains_key(&measurement.signal_id) {
            return Err(TriggerError::UnknownMeasurement(
                measurement.signal_id.clone(),
            ));
        }
        if measurement_map
            .insert(measurement.signal_id.clone(), measurement.clone())
            .is_some()
        {
            return Err(TriggerError::DuplicateId(measurement.signal_id.clone()));
        }
        if measurement.observed_at_ms > snapshot.decision_time_ms
            || measurement.first_available_at_ms > snapshot.decision_time_ms
            || measurement.first_available_at_ms < measurement.observed_at_ms
        {
            return Err(TriggerError::FutureObservation(
                measurement.signal_id.clone(),
            ));
        }
    }

    let baseline_changed = previous_state.baseline_plan_id.as_deref()
        != Some(snapshot.baseline_plan_id.as_str());
    let mut next_state = update_snapshot_state(previous_state, snapshot, baseline_changed);
    let plan_age = snapshot.decision_time_ms - snapshot.plan_generated_at_ms;

    let mut incomplete_reasons = Vec::new();
    let mut receipts = Vec::new();
    let mut soft_reasons = Vec::new();
    let mut hard_reasons = Vec::new();
    let mut all_below_reset = true;

    for (signal_id, definition) in &definition_map {
        let Some(measurement) = measurement_map.get(signal_id) else {
            if definition.critical {
                incomplete_reasons.push(format!("MISSING_CRITICAL_SIGNAL:{signal_id}"));
            }
            all_below_reset = false;
            continue;
        };

        let age_ms = snapshot.decision_time_ms - measurement.observed_at_ms;
        if age_ms > definition.max_age_ms {
            if definition.critical {
                incomplete_reasons.push(format!("STALE_CRITICAL_SIGNAL:{signal_id}"));
            }
            all_below_reset = false;
            continue;
        }
        if measurement.quality == EvidenceQuality::Unknown {
            if definition.critical {
                incomplete_reasons.push(format!("UNKNOWN_CRITICAL_EVIDENCE:{signal_id}"));
            }
            all_below_reset = false;
            continue;
        }

        let deviation = (measurement.observed_value - definition.baseline_value).abs();
        if !deviation.is_finite() {
            return Err(TriggerError::NumericalOverflow("signal deviation"));
        }
        let threshold = threshold_for(definition)?;
        let normalized = normalized_deviation(deviation, threshold);
        let hard_violation = hard_bound_violated(
            definition.hard_bound,
            measurement.observed_value,
        );

        receipts.push(SignalReceipt {
            signal_id: signal_id.clone(),
            domain: definition.domain,
            unit: definition.unit.clone(),
            observed_value: measurement.observed_value,
            baseline_value: definition.baseline_value,
            absolute_deviation: deviation,
            soft_threshold: threshold,
            normalized_deviation: normalized,
            age_ms,
            hard_violation,
            evidence_ref: measurement.evidence_ref.clone(),
            evidence_quality: measurement.quality,
        });

        if hard_violation {
            hard_reasons.push(format!("HARD_BOUNDARY_BREACH:{signal_id}"));
        }
        if normalized > 1.0 {
            soft_reasons.push(format!("SOFT_DEVIATION:{signal_id}"));
        }
        if normalized > policy.hysteresis_reset_ratio {
            all_below_reset = false;
        }
    }

    incomplete_reasons.sort();
    soft_reasons.sort();
    hard_reasons.sort();
    receipts.sort_by(|a, b| a.signal_id.cmp(&b.signal_id));

    if !incomplete_reasons.is_empty() {
        return Ok(TriggerEvaluation {
            snapshot_id: snapshot.snapshot_id.clone(),
            baseline_plan_id: snapshot.baseline_plan_id.clone(),
            decision: TriggerDecision::DataIncomplete,
            reason_codes: incomplete_reasons,
            evidence_refresh_required: true,
            signal_receipts: receipts,
            next_state,
            claim_ceiling: "Read-only event-trigger evaluation over caller-supplied signal definitions and snapshots only; no schedule or physical action is authorized.",
        });
    }

    if plan_age > policy.max_plan_age_ms {
        hard_reasons.push("PLAN_STALE".to_owned());
    }
    hard_reasons.sort();
    hard_reasons.dedup();

    let current_hard: BTreeSet<String> = hard_reasons.iter().cloned().collect();
    let previous_hard: BTreeSet<String> =
        next_state.active_hard_reasons.iter().cloned().collect();
    let new_hard: Vec<String> = current_hard
        .difference(&previous_hard)
        .cloned()
        .collect();

    if !new_hard.is_empty() {
        next_state.last_trigger_time_ms = Some(snapshot.decision_time_ms);
        next_state.soft_latched = true;
        next_state.active_hard_reasons = current_hard.into_iter().collect();
        let mut reasons = hard_reasons;
        reasons.sort();
        return Ok(TriggerEvaluation {
            snapshot_id: snapshot.snapshot_id.clone(),
            baseline_plan_id: snapshot.baseline_plan_id.clone(),
            decision: TriggerDecision::ReplanRequested {
                priority: TriggerPriority::Hard,
            },
            reason_codes: reasons,
            evidence_refresh_required: false,
            signal_receipts: receipts,
            next_state,
            claim_ceiling: "Read-only event-trigger evaluation over caller-supplied signal definitions and snapshots only; no schedule or physical action is authorized.",
        });
    }

    next_state.active_hard_reasons = current_hard.into_iter().collect();

    if !soft_reasons.is_empty() {
        if next_state.soft_latched {
            return Ok(TriggerEvaluation {
                snapshot_id: snapshot.snapshot_id.clone(),
                baseline_plan_id: snapshot.baseline_plan_id.clone(),
                decision: TriggerDecision::Suppressed {
                    reason: SuppressionReason::Hysteresis,
                },
                reason_codes: soft_reasons,
                evidence_refresh_required: false,
                signal_receipts: receipts,
                next_state,
                claim_ceiling: "Read-only event-trigger evaluation over caller-supplied signal definitions and snapshots only; no schedule or physical action is authorized.",
            });
        }

        let cooldown_active = next_state
            .last_trigger_time_ms
            .is_some_and(|last| snapshot.decision_time_ms.saturating_sub(last) < policy.normal_cooldown_ms);
        if cooldown_active {
            return Ok(TriggerEvaluation {
                snapshot_id: snapshot.snapshot_id.clone(),
                baseline_plan_id: snapshot.baseline_plan_id.clone(),
                decision: TriggerDecision::Suppressed {
                    reason: SuppressionReason::Cooldown,
                },
                reason_codes: soft_reasons,
                evidence_refresh_required: false,
                signal_receipts: receipts,
                next_state,
                claim_ceiling: "Read-only event-trigger evaluation over caller-supplied signal definitions and snapshots only; no schedule or physical action is authorized.",
            });
        }

        next_state.last_trigger_time_ms = Some(snapshot.decision_time_ms);
        next_state.soft_latched = true;
        return Ok(TriggerEvaluation {
            snapshot_id: snapshot.snapshot_id.clone(),
            baseline_plan_id: snapshot.baseline_plan_id.clone(),
            decision: TriggerDecision::ReplanRequested {
                priority: TriggerPriority::Normal,
            },
            reason_codes: soft_reasons,
            evidence_refresh_required: false,
            signal_receipts: receipts,
            next_state,
            claim_ceiling: "Read-only event-trigger evaluation over caller-supplied signal definitions and snapshots only; no schedule or physical action is authorized.",
        });
    }

    if next_state.soft_latched && all_below_reset && next_state.active_hard_reasons.is_empty() {
        next_state.soft_latched = false;
    }

    Ok(TriggerEvaluation {
        snapshot_id: snapshot.snapshot_id.clone(),
        baseline_plan_id: snapshot.baseline_plan_id.clone(),
        decision: TriggerDecision::NoTrigger,
        reason_codes: Vec::new(),
        evidence_refresh_required: false,
        signal_receipts: receipts,
        next_state,
        claim_ceiling: "Read-only event-trigger evaluation over caller-supplied signal definitions and snapshots only; no schedule or physical action is authorized.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definitions() -> Vec<SignalDefinition> {
        vec![
            SignalDefinition {
                signal_id: "berth_arrival_delay_min".to_owned(),
                domain: SignalDomain::Logistics,
                unit: "min".to_owned(),
                baseline_value: 0.0,
                soft_absolute_threshold: 10.0,
                soft_relative_threshold: 0.0,
                max_age_ms: 60_000,
                critical: true,
                hard_bound: None,
            },
            SignalDefinition {
                signal_id: "pcc_headroom_kw".to_owned(),
                domain: SignalDomain::Grid,
                unit: "kW".to_owned(),
                baseline_value: 400.0,
                soft_absolute_threshold: 80.0,
                soft_relative_threshold: 0.15,
                max_age_ms: 30_000,
                critical: true,
                hard_bound: Some(HardBound::Lower { limit: 100.0 }),
            },
            SignalDefinition {
                signal_id: "compute_deadline_slack_min".to_owned(),
                domain: SignalDomain::Compute,
                unit: "min".to_owned(),
                baseline_value: 120.0,
                soft_absolute_threshold: 30.0,
                soft_relative_threshold: 0.0,
                max_age_ms: 60_000,
                critical: true,
                hard_bound: Some(HardBound::Lower { limit: 15.0 }),
            },
            SignalDefinition {
                signal_id: "cooling_discharge_margin_c".to_owned(),
                domain: SignalDomain::Cooling,
                unit: "degC".to_owned(),
                baseline_value: 6.0,
                soft_absolute_threshold: 1.5,
                soft_relative_threshold: 0.0,
                max_age_ms: 60_000,
                critical: true,
                hard_bound: Some(HardBound::Lower { limit: 1.0 }),
            },
            SignalDefinition {
                signal_id: "thermal_store_soc_pct".to_owned(),
                domain: SignalDomain::ThermalStore,
                unit: "%".to_owned(),
                baseline_value: 80.0,
                soft_absolute_threshold: 10.0,
                soft_relative_threshold: 0.0,
                max_age_ms: 60_000,
                critical: true,
                hard_bound: Some(HardBound::Lower { limit: 20.0 }),
            },
        ]
    }

    fn snapshot(
        sequence: u64,
        decision_time_ms: u64,
        overrides: &[(&str, f64)],
    ) -> TriggerSnapshot {
        let defs = definitions();
        let measurements = defs
            .iter()
            .map(|definition| {
                let value = overrides
                    .iter()
                    .find(|(id, _)| *id == definition.signal_id)
                    .map(|(_, value)| *value)
                    .unwrap_or(definition.baseline_value);
                SignalMeasurement {
                    signal_id: definition.signal_id.clone(),
                    observed_value: value,
                    observed_at_ms: decision_time_ms - 1_000,
                    first_available_at_ms: decision_time_ms - 500,
                    evidence_ref: format!("synthetic:{}", definition.signal_id),
                    quality: EvidenceQuality::Synthetic,
                }
            })
            .collect();
        TriggerSnapshot {
            snapshot_id: format!("snapshot-{sequence}"),
            snapshot_digest: format!("sha256:synthetic-snapshot-{sequence}"),
            site_id: "synthetic-port".to_owned(),
            baseline_plan_id: "plan-v1".to_owned(),
            sequence,
            decision_time_ms,
            plan_generated_at_ms: decision_time_ms - 10_000,
            measurements,
        }
    }

    fn policy() -> TriggerPolicy {
        TriggerPolicy {
            normal_cooldown_ms: 30_000,
            max_plan_age_ms: 3_600_000,
            hysteresis_reset_ratio: 0.5,
        }
    }

    #[test]
    fn stable_snapshot_does_not_trigger() {
        let input = snapshot(1, 100_000, &[]);
        let result = evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()).unwrap();
        assert_eq!(result.decision, TriggerDecision::NoTrigger);
    }

    #[test]
    fn logistics_deviation_triggers_without_energy_deviation() {
        let input = snapshot(1, 100_000, &[("berth_arrival_delay_min", 16.0)]);
        let result = evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()).unwrap();
        assert_eq!(
            result.decision,
            TriggerDecision::ReplanRequested { priority: TriggerPriority::Normal }
        );
        assert!(result.reason_codes.contains(&"SOFT_DEVIATION:berth_arrival_delay_min".to_owned()));
    }

    #[test]
    fn compute_queue_deviation_triggers() {
        let input = snapshot(1, 100_000, &[("compute_deadline_slack_min", 80.0)]);
        let result = evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()).unwrap();
        assert!(matches!(result.decision, TriggerDecision::ReplanRequested { .. }));
    }

    #[test]
    fn cooling_and_thermal_store_can_trigger_without_compute_shift() {
        let input = snapshot(1, 100_000, &[
            ("cooling_discharge_margin_c", 4.0),
            ("thermal_store_soc_pct", 68.0)
        ]);
        let result = evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()).unwrap();
        assert!(matches!(result.decision, TriggerDecision::ReplanRequested { .. }));
        assert_eq!(result.reason_codes.len(), 2);
    }

    #[test]
    fn hard_pcc_margin_breach_bypasses_cooldown() {
        let input = snapshot(1, 100_000, &[("pcc_headroom_kw", 75.0)]);
        let mut state = TriggerState::default();
        state.last_trigger_time_ms = Some(99_999);
        state.baseline_plan_id = Some("plan-v1".to_owned());
        let result = evaluate_trigger(&input, &definitions(), &policy(), &state).unwrap();
        assert_eq!(
            result.decision,
            TriggerDecision::ReplanRequested { priority: TriggerPriority::Hard }
        );
        assert!(result.reason_codes.contains(&"HARD_BOUNDARY_BREACH:pcc_headroom_kw".to_owned()));
    }

    #[test]
    fn persistent_hard_event_is_not_reemitted_for_every_snapshot() {
        let first = snapshot(1, 100_000, &[("pcc_headroom_kw", 75.0)]);
        let first_result = evaluate_trigger(&first, &definitions(), &policy(), &TriggerState::default()).unwrap();
        let second = snapshot(2, 110_000, &[("pcc_headroom_kw", 75.0)]);
        let second_result = evaluate_trigger(&second, &definitions(), &policy(), &first_result.next_state).unwrap();
        assert_eq!(
            second_result.decision,
            TriggerDecision::Suppressed { reason: SuppressionReason::Hysteresis }
        );
    }

    #[test]
    fn a_new_hard_reason_triggers_even_while_previous_hard_reason_persists() {
        let first = snapshot(1, 100_000, &[("pcc_headroom_kw", 75.0)]);
        let first_result = evaluate_trigger(&first, &definitions(), &policy(), &TriggerState::default()).unwrap();
        let second = snapshot(2, 110_000, &[
            ("pcc_headroom_kw", 75.0),
            ("compute_deadline_slack_min", 10.0)
        ]);
        let second_result = evaluate_trigger(&second, &definitions(), &policy(), &first_result.next_state).unwrap();
        assert_eq!(
            second_result.decision,
            TriggerDecision::ReplanRequested { priority: TriggerPriority::Hard }
        );
        assert!(second_result.reason_codes.contains(&"HARD_BOUNDARY_BREACH:compute_deadline_slack_min".to_owned()));
    }

    #[test]
    fn missing_critical_measurement_is_data_incomplete_not_no_trigger() {
        let mut input = snapshot(1, 100_000, &[]);
        input.measurements.retain(|m| m.signal_id != "pcc_headroom_kw");
        let result = evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()).unwrap();
        assert_eq!(result.decision, TriggerDecision::DataIncomplete);
        assert!(result.evidence_refresh_required);
    }

    #[test]
    fn stale_critical_measurement_is_data_incomplete() {
        let mut input = snapshot(1, 100_000, &[]);
        let measurement = input.measurements.iter_mut()
            .find(|m| m.signal_id == "pcc_headroom_kw").unwrap();
        measurement.observed_at_ms = 1;
        measurement.first_available_at_ms = 2;
        let result = evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()).unwrap();
        assert_eq!(result.decision, TriggerDecision::DataIncomplete);
    }

    #[test]
    fn future_information_is_rejected_to_prevent_lookahead() {
        let mut input = snapshot(1, 100_000, &[]);
        let measurement = input.measurements.iter_mut()
            .find(|m| m.signal_id == "pcc_headroom_kw").unwrap();
        measurement.first_available_at_ms = input.decision_time_ms + 1;
        assert!(matches!(
            evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()),
            Err(TriggerError::FutureObservation(_))
        ));
    }

    #[test]
    fn cooldown_suppresses_normal_trigger_but_not_hard_trigger() {
        let input = snapshot(1, 110_000, &[("berth_arrival_delay_min", 16.0)]);
        let mut state = TriggerState::default();
        state.baseline_plan_id = Some("plan-v1".to_owned());
        state.last_trigger_time_ms = Some(100_000);
        let result = evaluate_trigger(&input, &definitions(), &policy(), &state).unwrap();
        assert_eq!(
            result.decision,
            TriggerDecision::Suppressed { reason: SuppressionReason::Cooldown }
        );
    }

    #[test]
    fn hysteresis_prevents_retrigger_until_all_signals_recover() {
        let first = snapshot(1, 100_000, &[("berth_arrival_delay_min", 16.0)]);
        let first_result = evaluate_trigger(&first, &definitions(), &policy(), &TriggerState::default()).unwrap();
        let second = snapshot(2, 140_000, &[("berth_arrival_delay_min", 16.0)]);
        let second_result = evaluate_trigger(&second, &definitions(), &policy(), &first_result.next_state).unwrap();
        assert_eq!(
            second_result.decision,
            TriggerDecision::Suppressed { reason: SuppressionReason::Hysteresis }
        );
    }

    #[test]
    fn recovery_below_reset_threshold_rearms_monitor() {
        let first = snapshot(1, 100_000, &[("berth_arrival_delay_min", 16.0)]);
        let first_result = evaluate_trigger(&first, &definitions(), &policy(), &TriggerState::default()).unwrap();
        let recovery = snapshot(2, 110_000, &[]);
        let recovery_result = evaluate_trigger(&recovery, &definitions(), &policy(), &first_result.next_state).unwrap();
        assert_eq!(recovery_result.decision, TriggerDecision::NoTrigger);
        assert!(!recovery_result.next_state.soft_latched);
        let next = snapshot(3, 150_000, &[("berth_arrival_delay_min", 16.0)]);
        let next_result = evaluate_trigger(&next, &definitions(), &policy(), &recovery_result.next_state).unwrap();
        assert!(matches!(next_result.decision, TriggerDecision::ReplanRequested { .. }));
    }

    #[test]
    fn stale_plan_triggers_hard_replan_request() {
        let mut input = snapshot(1, 4_000_000, &[]);
        input.plan_generated_at_ms = 0;
        let result = evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()).unwrap();
        assert_eq!(
            result.decision,
            TriggerDecision::ReplanRequested { priority: TriggerPriority::Hard }
        );
        assert!(result.reason_codes.contains(&"PLAN_STALE".to_owned()));
    }

    #[test]
    fn signal_and_measurement_order_do_not_change_receipt_order() {
        let mut input = snapshot(1, 100_000, &[("berth_arrival_delay_min", 16.0)]);
        let first = evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()).unwrap();
        input.measurements.reverse();
        let mut defs = definitions();
        defs.reverse();
        let second = evaluate_trigger(&input, &defs, &policy(), &TriggerState::default()).unwrap();
        assert_eq!(first.reason_codes, second.reason_codes);
        assert_eq!(first.signal_receipts, second.signal_receipts);
    }

    #[test]
    fn duplicate_measurements_are_rejected() {
        let mut input = snapshot(1, 100_000, &[]);
        input.measurements.push(input.measurements[0].clone());
        assert!(matches!(
            evaluate_trigger(&input, &definitions(), &policy(), &TriggerState::default()),
            Err(TriggerError::DuplicateId(_))
        ));
    }

    #[test]
    fn baseline_change_resets_latch_and_cooldown() {
        let mut state = TriggerState::default();
        state.baseline_plan_id = Some("plan-v0".to_owned());
        state.last_trigger_time_ms = Some(99_999);
        state.soft_latched = true;
        let mut input = snapshot(1, 100_000, &[("berth_arrival_delay_min", 16.0)]);
        input.baseline_plan_id = "plan-v1".to_owned();
        let result = evaluate_trigger(&input, &definitions(), &policy(), &state).unwrap();
        assert_eq!(
            result.decision,
            TriggerDecision::ReplanRequested { priority: TriggerPriority::Normal }
        );
    }

    #[test]
    fn same_sequence_with_different_snapshot_id_is_invalid() {
        let first = snapshot(1, 100_000, &[]);
        let first_result = evaluate_trigger(&first, &definitions(), &policy(), &TriggerState::default()).unwrap();
        let mut second = snapshot(1, 100_000, &[]);
        second.snapshot_digest = "sha256:different-content".to_owned();
        assert!(matches!(
            evaluate_trigger(&second, &definitions(), &policy(), &first_result.next_state),
            Err(TriggerError::InvalidChronology(_))
        ));
    }

    #[test]
    fn unknown_quality_on_noncritical_signal_does_not_promote_a_pass() {
        let mut defs = definitions();
        defs[0].critical = false;
        let mut input = snapshot(1, 100_000, &[]);
        let m = input.measurements.iter_mut()
            .find(|m| m.signal_id == "berth_arrival_delay_min").unwrap();
        m.quality = EvidenceQuality::Unknown;
        let result = evaluate_trigger(&input, &defs, &policy(), &TriggerState::default()).unwrap();
        assert_eq!(result.decision, TriggerDecision::NoTrigger);
        assert!(result.signal_receipts.iter().all(|r| r.signal_id != "berth_arrival_delay_min"));
    }

    #[test]
    fn decision_time_and_sequence_cannot_move_backwards() {
        let first = snapshot(2, 100_000, &[]);
        let first_result = evaluate_trigger(&first, &definitions(), &policy(), &TriggerState::default()).unwrap();
        let second = snapshot(1, 110_000, &[]);
        assert!(matches!(
            evaluate_trigger(&second, &definitions(), &policy(), &first_result.next_state),
            Err(TriggerError::InvalidChronology(_))
        ));
    }
}
