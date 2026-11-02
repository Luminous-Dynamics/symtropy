//! Synthetic-first inland-waterway lock passage and water-cycle accounting.
//! This is a verifier over declared evidence, not a hydraulic or gate-control model.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeInterval {
    pub start_tick: u64,
    pub end_tick: u64,
}

impl TimeInterval {
    fn is_valid(self) -> bool {
        self.start_tick < self.end_tick
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Current,
    Unknown,
    Disputed,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockAssetStatus {
    Available,
    Outage,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorApproval {
    Approved,
    Pending,
    Declined,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockProfile {
    pub lock_id: String,
    pub revision: String,
    pub usable_length_mm: u64,
    pub usable_beam_mm: u64,
    pub required_end_clearance_mm: u64,
    pub required_side_clearance_mm: u64,
    pub required_under_keel_clearance_mm: u64,
    pub datum_id: String,
    pub total_storage_capacity_ml: u64,
    pub water_balance_tolerance_ml: u64,
    pub valid_through_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VesselMeasurement {
    pub vessel_id: String,
    pub vessel_revision: String,
    pub measurement_id: String,
    pub length_mm: u64,
    pub beam_mm: u64,
    pub draft_mm: u64,
    pub observed_at_tick: u64,
    pub calibration_valid_through_tick: Option<u64>,
    pub evidence_state: EvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterLevelObservation {
    pub lock_id: String,
    pub lock_revision: String,
    pub observation_id: String,
    pub available_depth_mm: u64,
    pub datum_id: String,
    pub observed_at_tick: u64,
    pub calibration_valid_through_tick: Option<u64>,
    pub evidence_state: EvidenceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassageDisposition {
    EligibleForSyntheticPassage,
    BlockedByHardConstraint,
    UnknownReviewRequired,
    EvidenceInconsistent,
    UnsupportedOperatingProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassageReason {
    InvalidLockIdentity,
    InvalidVesselIdentity,
    InvalidObservationIdentity,
    LockCurrentnessUnknown,
    LockProfileExpired,
    VesselLockBindingMismatch,
    WaterObservationLockMismatch,
    DatumMismatch,
    MissingCalibrationCurrentness,
    CalibrationExpired,
    MeasurementFromFuture,
    MeasurementStale,
    EvidenceUnknown,
    EvidenceDisputed,
    EvidenceRevoked,
    LengthClearanceExceeded,
    BeamClearanceExceeded,
    UnderKeelClearanceExceeded,
    LockOutage,
    LockStatusUnknown,
    OperatorApprovalPending,
    OperatorApprovalDeclined,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassageEvaluation {
    pub disposition: PassageDisposition,
    pub reasons: Vec<PassageReason>,
    pub required_length_mm: u128,
    pub required_beam_mm: u128,
    pub required_depth_mm: u128,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn calibration_status(
    expiry: Option<u64>,
    evaluation_tick: u64,
    reasons: &mut Vec<PassageReason>,
) -> bool {
    match expiry {
        None => {
            reasons.push(PassageReason::MissingCalibrationCurrentness);
            false
        }
        Some(value) if value < evaluation_tick => {
            reasons.push(PassageReason::CalibrationExpired);
            false
        }
        Some(_) => true,
    }
}

fn evidence_status(state: EvidenceState, reasons: &mut Vec<PassageReason>) -> (bool, bool) {
    match state {
        EvidenceState::Current => (false, false),
        EvidenceState::Unknown => {
            reasons.push(PassageReason::EvidenceUnknown);
            (false, true)
        }
        EvidenceState::Disputed => {
            reasons.push(PassageReason::EvidenceDisputed);
            (false, true)
        }
        EvidenceState::Revoked => {
            reasons.push(PassageReason::EvidenceRevoked);
            (true, false)
        }
    }
}

/// Evaluate one vessel against one frozen lock profile and water-level observation.
#[must_use]
pub fn evaluate_passage(
    lock: &LockProfile,
    vessel: &VesselMeasurement,
    water: &WaterLevelObservation,
    asset_status: LockAssetStatus,
    operator_approval: OperatorApproval,
    evaluation_tick: u64,
    maximum_measurement_age_ticks: u64,
) -> PassageEvaluation {
    let mut reasons = Vec::new();
    let mut inconsistent = false;
    let mut unknown = false;
    let mut blocked = false;

    if !nonempty(&lock.lock_id) || !nonempty(&lock.revision) || !nonempty(&lock.datum_id) {
        reasons.push(PassageReason::InvalidLockIdentity);
        inconsistent = true;
    }
    if !nonempty(&vessel.vessel_id) || !nonempty(&vessel.vessel_revision) {
        reasons.push(PassageReason::InvalidVesselIdentity);
        inconsistent = true;
    }
    if !nonempty(&vessel.measurement_id) || !nonempty(&water.observation_id) {
        reasons.push(PassageReason::InvalidObservationIdentity);
        inconsistent = true;
    }

    let mut lock_profile_usable = true;
    let mut vessel_evidence_usable = true;
    let mut water_evidence_usable = true;

    match lock.valid_through_tick {
        None => {
            reasons.push(PassageReason::LockCurrentnessUnknown);
            unknown = true;
            lock_profile_usable = false;
        }
        Some(expiry) if expiry < evaluation_tick => {
            reasons.push(PassageReason::LockProfileExpired);
            unknown = true;
            lock_profile_usable = false;
        }
        Some(_) => {}
    }

    if water.lock_id != lock.lock_id || water.lock_revision != lock.revision {
        reasons.push(PassageReason::WaterObservationLockMismatch);
        inconsistent = true;
        water_evidence_usable = false;
    }
    if water.datum_id != lock.datum_id {
        reasons.push(PassageReason::DatumMismatch);
        inconsistent = true;
        water_evidence_usable = false;
    }

    if !calibration_status(
        vessel.calibration_valid_through_tick,
        evaluation_tick,
        &mut reasons,
    ) {
        unknown = true;
        vessel_evidence_usable = false;
    }
    if !calibration_status(
        water.calibration_valid_through_tick,
        evaluation_tick,
        &mut reasons,
    ) {
        unknown = true;
        water_evidence_usable = false;
    }

    for observed in [vessel.observed_at_tick, water.observed_at_tick] {
        if observed > evaluation_tick {
            reasons.push(PassageReason::MeasurementFromFuture);
            inconsistent = true;
            if observed == vessel.observed_at_tick {
                vessel_evidence_usable = false;
            }
            if observed == water.observed_at_tick {
                water_evidence_usable = false;
            }
        } else if evaluation_tick - observed > maximum_measurement_age_ticks {
            reasons.push(PassageReason::MeasurementStale);
            unknown = true;
            if observed == vessel.observed_at_tick {
                vessel_evidence_usable = false;
            }
            if observed == water.observed_at_tick {
                water_evidence_usable = false;
            }
        }
    }

    let (vessel_bad, vessel_review) = evidence_status(vessel.evidence_state, &mut reasons);
    inconsistent |= vessel_bad;
    unknown |= vessel_review;
    vessel_evidence_usable &= !vessel_bad && !vessel_review;
    let (water_bad, water_review) = evidence_status(water.evidence_state, &mut reasons);
    inconsistent |= water_bad;
    unknown |= water_review;
    water_evidence_usable &= !water_bad && !water_review;

    let required_length = u128::from(vessel.length_mm)
        + u128::from(lock.required_end_clearance_mm);
    let required_beam = u128::from(vessel.beam_mm)
        + u128::from(lock.required_side_clearance_mm);
    let required_depth = u128::from(vessel.draft_mm)
        + u128::from(lock.required_under_keel_clearance_mm);

    if lock_profile_usable && vessel_evidence_usable {
        if required_length > u128::from(lock.usable_length_mm) {
            reasons.push(PassageReason::LengthClearanceExceeded);
            blocked = true;
        }
        if required_beam > u128::from(lock.usable_beam_mm) {
            reasons.push(PassageReason::BeamClearanceExceeded);
            blocked = true;
        }
    }
    if lock_profile_usable && vessel_evidence_usable && water_evidence_usable
        && required_depth > u128::from(water.available_depth_mm)
    {
        reasons.push(PassageReason::UnderKeelClearanceExceeded);
        blocked = true;
    }

    match asset_status {
        LockAssetStatus::Available => {}
        LockAssetStatus::Outage => {
            reasons.push(PassageReason::LockOutage);
            blocked = true;
        }
        LockAssetStatus::Unknown => {
            reasons.push(PassageReason::LockStatusUnknown);
            unknown = true;
        }
    }
    match operator_approval {
        OperatorApproval::Approved => {}
        OperatorApproval::Pending => {
            reasons.push(PassageReason::OperatorApprovalPending);
            unknown = true;
        }
        OperatorApproval::Declined => {
            reasons.push(PassageReason::OperatorApprovalDeclined);
            blocked = true;
        }
    }

    let disposition = if inconsistent {
        PassageDisposition::EvidenceInconsistent
    }    } else if blocked {
        PassageDisposition::BlockedByHardConstraint
    } else if unknown {
        PassageDisposition::UnknownReviewRequired
    } else {
        PassageDisposition::EligibleForSyntheticPassage
    };

    PassageEvaluation {
        disposition,
        reasons,
        required_length_mm: required_length,
        required_beam_mm: required_beam,
        required_depth_mm: required_depth,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterCycleRecord {
    pub cycle_id: String,
    pub lock_id: String,
    pub lock_revision: String,
    pub support: TimeInterval,
    pub evidence_id: String,
    pub evidence_state: EvidenceState,
    pub opening_system_storage_ml: Option<u64>,
    pub inflow_from_upper_pound_ml: Option<u64>,
    pub inflow_from_lower_pound_ml: Option<u64>,
    pub outflow_to_upper_pound_ml: Option<u64>,
    pub outflow_to_lower_pound_ml: Option<u64>,
    pub declared_losses_ml: Option<u64>,
    pub closing_system_storage_ml: Option<u64>,
    pub pump_required: bool,
    pub pump_energy_wh: Option<u64>,
    pub pump_meter_id: Option<String>,
    pub pump_calibration_valid_through_tick: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterCycleDisposition {
    BalancedForSyntheticCycle,
    UnknownReviewRequired,
    EvidenceInconsistent,
    DuplicateExactReplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterCycleReason {
    InvalidCycleIdentity,
    InvalidTimeSupport,
    LockProfileCurrentnessUnknown,
    LockProfileExpired,
    CycleStale,
    LockIdentityMismatch,
    LockProfileRevisionMismatch,
    MissingWaterBoundaryMeasurement,
    StorageCapacityExceeded,
    WaterBalanceOutsideTolerance,
    EvidenceUnknown,
    EvidenceDisputed,
    EvidenceRevoked,
    PumpEnergyMissing,
    PumpMeterIdentityMissing,
    PumpCalibrationUnknown,
    PumpCalibrationExpired,
    DuplicateCycleIdChangedRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterCycleResult {
    pub disposition: WaterCycleDisposition,
    pub reasons: Vec<WaterCycleReason>,
    pub residual_ml: Option<i128>,
    pub next_cursor: WaterCycleCursor,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WaterCycleCursor {
    accepted: BTreeMap<String, (WaterCycleRecord, i128)>,
}

/// Reconcile one supplied water-cycle control volume.
///
/// Basin/chamber internal transfers are not separate external flows: total system
/// storage at open/close already accounts for them and prevents double counting.
pub fn evaluate_water_cycle(
    lock: &LockProfile,
    record: &WaterCycleRecord,
    cursor: &WaterCycleCursor,
    evaluation_tick: u64,
    maximum_cycle_age_ticks: u64,
) -> WaterCycleResult {
    let mut next_cursor = cursor.clone();
    let mut reasons = Vec::new();
    let mut inconsistent = false;
    let mut unknown = false;

    if !nonempty(&record.cycle_id) || !nonempty(&record.evidence_id) {
        reasons.push(WaterCycleReason::InvalidCycleIdentity);
        inconsistent = true;
    }
    if !record.support.is_valid() {
        reasons.push(WaterCycleReason::InvalidTimeSupport);
        inconsistent = true;
    }
    if record.lock_id != lock.lock_id {
        reasons.push(WaterCycleReason::LockIdentityMismatch);
        inconsistent = true;
    }
    if record.lock_revision != lock.revision {
        reasons.push(WaterCycleReason::LockProfileRevisionMismatch);
        inconsistent = true;
    }

    if let Some((previous, previous_residual)) = cursor.accepted.get(&record.cycle_id) {
        if previous == record {
            return WaterCycleResult {
                disposition: WaterCycleDisposition::DuplicateExactReplay,
                reasons: Vec::new(),
                residual_ml: Some(*previous_residual),
                next_cursor,
            };
        }
        reasons.push(WaterCycleReason::DuplicateCycleIdChangedRecord);
        inconsistent = true;
    }

    if record.support.end_tick > evaluation_tick {
        reasons.push(WaterCycleReason::InvalidTimeSupport);
        inconsistent = true;
    } else if evaluation_tick - record.support.end_tick > maximum_cycle_age_ticks {
        reasons.push(WaterCycleReason::CycleStale);
        unknown = true;
    }

    let mut lock_profile_usable = true;
    match lock.valid_through_tick {
        None => {
            reasons.push(WaterCycleReason::LockProfileCurrentnessUnknown);
            lock_profile_usable = false;
            unknown = true;
        }
        Some(expiry) if expiry < evaluation_tick => {
            reasons.push(WaterCycleReason::LockProfileExpired);
            lock_profile_usable = false;
            unknown = true;
        }
        Some(_) => {}
    }

    let values = [
        record.opening_system_storage_ml,
        record.inflow_from_upper_pound_ml,
        record.inflow_from_lower_pound_ml,
        record.outflow_to_upper_pound_ml,
        record.outflow_to_lower_pound_ml,
        record.declared_losses_ml,
        record.closing_system_storage_ml,
    ];
    if values.iter().any(Option::is_none) {
        reasons.push(WaterCycleReason::MissingWaterBoundaryMeasurement);
        unknown = true;
    }

    let record_evidence_usable = match record.evidence_state {
        EvidenceState::Current => true,
        EvidenceState::Unknown => {
            reasons.push(WaterCycleReason::EvidenceUnknown);
            unknown = true;
            false
        }
        EvidenceState::Disputed => {
            reasons.push(WaterCycleReason::EvidenceDisputed);
            unknown = true;
            false
        }
        EvidenceState::Revoked => {
            reasons.push(WaterCycleReason::EvidenceRevoked);
            inconsistent = true;
            false
        }
    };

    if lock_profile_usable && record_evidence_usable {
        if record.opening_system_storage_ml.is_some_and(|v| v > lock.total_storage_capacity_ml)
            || record.closing_system_storage_ml.is_some_and(|v| v > lock.total_storage_capacity_ml)
        {
            reasons.push(WaterCycleReason::StorageCapacityExceeded);
            inconsistent = true;
        }
    }

    if record.pump_required {
        if record.pump_energy_wh.is_none() {
            reasons.push(WaterCycleReason::PumpEnergyMissing);
            unknown = true;
        }
        if record.pump_meter_id.as_deref().is_none_or(|id| !nonempty(id)) {
            reasons.push(WaterCycleReason::PumpMeterIdentityMissing);
            unknown = true;
        }
        match record.pump_calibration_valid_through_tick {
            None => {
                reasons.push(WaterCycleReason::PumpCalibrationUnknown);
                unknown = true;
            }
            Some(expiry) if expiry < evaluation_tick => {
                reasons.push(WaterCycleReason::PumpCalibrationExpired);
                unknown = true;
            }
            Some(_) => {}
        }
    }

    let residual = match values {
        [Some(opening), Some(upper_in), Some(lower_in), Some(upper_out),
         Some(lower_out), Some(losses), Some(closing)] => {
            Some(
                i128::from(upper_in) + i128::from(lower_in) + i128::from(opening)
                    - i128::from(upper_out) - i128::from(lower_out)
                    - i128::from(losses) - i128::from(closing)
            )
        }
        _ => None,
    };

    if lock_profile_usable && record_evidence_usable {
        if let Some(value) = residual {
            if value.unsigned_abs() > u128::from(lock.water_balance_tolerance_ml) {
                reasons.push(WaterCycleReason::WaterBalanceOutsideTolerance);
                inconsistent = true;
            }
        }
    }

    let disposition = if inconsistent {
        WaterCycleDisposition::EvidenceInconsistent
    } else if unknown {
        WaterCycleDisposition::UnknownReviewRequired
    } else {
        WaterCycleDisposition::BalancedForSyntheticCycle
    };

    if disposition == WaterCycleDisposition::BalancedForSyntheticCycle {
        if let Some(value) = residual {
            next_cursor.accepted.insert(record.cycle_id.clone(), (record.clone(), value));
        }
    }

    WaterCycleResult {
        disposition,
        reasons,
        residual_ml: residual,
        next_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock() -> LockProfile {
        LockProfile {
            lock_id: "lock-01".into(),
            revision: "lock-r4".into(),
            usable_length_mm: 120_000,
            usable_beam_mm: 12_000,
            required_end_clearance_mm: 2_000,
            required_side_clearance_mm: 500,
            required_under_keel_clearance_mm: 300,
            datum_id: "local-chart-datum".into(),
            total_storage_capacity_ml: 50_000_000_000,
            water_balance_tolerance_ml: 100,
            valid_through_tick: Some(1_000),
        }
    }

    fn vessel() -> VesselMeasurement {
        VesselMeasurement {
            vessel_id: "barge-44".into(),
            vessel_revision: "barge-r2".into(),
            measurement_id: "vessel-survey-12".into(),
            length_mm: 100_000,
            beam_mm: 10_000,
            draft_mm: 2_000,
            observed_at_tick: 95,
            calibration_valid_through_tick: Some(500),
            evidence_state: EvidenceState::Current,
        }
    }

    fn water() -> WaterLevelObservation {
        WaterLevelObservation {
            lock_id: "lock-01".into(),
            lock_revision: "lock-r4".into(),
            observation_id: "depth-survey-18".into(),
            available_depth_mm: 2_500,
            datum_id: "local-chart-datum".into(),
            observed_at_tick: 98,
            calibration_valid_through_tick: Some(500),
            evidence_state: EvidenceState::Current,
        }
    }

    fn passage() -> PassageEvaluation {
        evaluate_passage(
            &lock(), &vessel(), &water(),
            LockAssetStatus::Available, OperatorApproval::Approved, 100, 10
        )
    }

    fn cycle() -> WaterCycleRecord {
        WaterCycleRecord {
            cycle_id: "cycle-01".into(),
            lock_id: "lock-01".into(),
            lock_revision: "lock-r4".into(),
            support: TimeInterval { start_tick: 100, end_tick: 110 },
            evidence_id: "cycle-evidence-01".into(),
            evidence_state: EvidenceState::Current,
            opening_system_storage_ml: Some(1_000_000),
            inflow_from_upper_pound_ml: Some(500_000),
            inflow_from_lower_pound_ml: Some(0),
            outflow_to_upper_pound_ml: Some(0),
            outflow_to_lower_pound_ml: Some(0),
            declared_losses_ml: Some(0),
            closing_system_storage_ml: Some(1_500_000),
            pump_required: false,
            pump_energy_wh: None,
            pump_meter_id: None,
            pump_calibration_valid_through_tick: None,
        }
    }

    fn cycle_eval(record: &WaterCycleRecord, cursor: &WaterCycleCursor) -> WaterCycleResult {
        evaluate_water_cycle(&lock(), record, cursor, 120, 20)
    }

    #[test]
    fn compatible_vessel_passage_is_eligible() {
        assert_eq!(passage().disposition, PassageDisposition::EligibleForSyntheticPassage);
    }

    #[test]
    fn exact_clearance_boundary_is_eligible() {
        let mut v = vessel();
        v.length_mm = 118_000;
        v.beam_mm = 11_500;
        v.draft_mm = 2_200;
        assert_eq!(
            evaluate_passage(&lock(), &v, &water(), LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::EligibleForSyntheticPassage
        );
    }

    #[test]
    fn vessel_too_long_is_blocked() {
        let mut v = vessel();
        v.length_mm = 119_000;
        assert_eq!(
            evaluate_passage(&lock(), &v, &water(), LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn vessel_too_wide_is_blocked() {
        let mut v = vessel();
        v.beam_mm = 11_501;
        assert_eq!(
            evaluate_passage(&lock(), &v, &water(), LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn insufficient_under_keel_clearance_is_blocked() {
        let mut w = water();
        w.available_depth_mm = 2_299;
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &w, LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn stale_vessel_measurement_requires_review() {
        let mut v = vessel();
        v.observed_at_tick = 80;
        assert_eq!(
            evaluate_passage(&lock(), &v, &water(), LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn stale_water_depth_requires_review() {
        let mut w = water();
        w.observed_at_tick = 80;
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &w, LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn datum_mismatch_is_inconsistent() {
        let mut w = water();
        w.datum_id = "different-datum".into();
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &w, LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn observation_for_another_lock_is_inconsistent() {
        let mut w = water();
        w.lock_id = "lock-02".into();
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &w, LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn missing_calibration_requires_review() {
        let mut v = vessel();
        v.calibration_valid_through_tick = None;
        assert_eq!(
            evaluate_passage(&lock(), &v, &water(), LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn revoked_water_evidence_is_inconsistent() {
        let mut w = water();
        w.evidence_state = EvidenceState::Revoked;
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &w, LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn pending_operator_approval_requires_review() {
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &water(), LockAssetStatus::Available,
                OperatorApproval::Pending, 100, 10).disposition,
            PassageDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn declined_operator_approval_blocks_passage() {
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &water(), LockAssetStatus::Available,
                OperatorApproval::Declined, 100, 10).disposition,
            PassageDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn lock_outage_blocks_passage() {
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &water(), LockAssetStatus::Outage,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn unknown_lock_status_requires_review() {
        assert_eq!(
            evaluate_passage(&lock(), &vessel(), &water(), LockAssetStatus::Unknown,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn future_vessel_measurement_is_inconsistent() {
        let mut v = vessel();
        v.observed_at_tick = 101;
        assert_eq!(
            evaluate_passage(&lock(), &v, &water(), LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn expired_lock_profile_requires_review() {
        let mut l = lock();
        l.valid_through_tick = Some(99);
        assert_eq!(
            evaluate_passage(&l, &vessel(), &water(), LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn stale_vessel_and_depth_evidence_cannot_be_used_to_block_or_allow() {
        let mut v = vessel();
        let mut w = water();
        v.observed_at_tick = 80;
        w.observed_at_tick = 80;
        v.length_mm = 500_000;
        w.available_depth_mm = 0;
        let result = evaluate_passage(
            &lock(), &v, &w, LockAssetStatus::Available, OperatorApproval::Approved, 100, 10
        );
        assert_eq!(result.disposition, PassageDisposition::UnknownReviewRequired);
        assert!(!result.reasons.contains(&PassageReason::LengthClearanceExceeded));
        assert!(!result.reasons.contains(&PassageReason::UnderKeelClearanceExceeded));
    }

    #[test]
    fn stale_lock_profile_does_not_authorize_dimension_comparison() {
        let mut l = lock();
        l.valid_through_tick = Some(99);
        let mut v = vessel();
        v.length_mm = 500_000;
        let result = evaluate_passage(
            &l, &v, &water(), LockAssetStatus::Available, OperatorApproval::Approved, 100, 10
        );
        assert_eq!(result.disposition, PassageDisposition::UnknownReviewRequired);
        assert!(!result.reasons.contains(&PassageReason::LengthClearanceExceeded));
    }

    #[test]
    fn water_cycle_balances_chamber_and_basin_storage() {
        let r = cycle_eval(&cycle(), &WaterCycleCursor::default());
        assert_eq!(r.disposition, WaterCycleDisposition::BalancedForSyntheticCycle);
        assert_eq!(r.residual_ml, Some(0));
    }

    #[test]
    fn storage_change_accounts_for_water_saving_basin_reuse() {
        let mut record = cycle();
        record.opening_system_storage_ml = Some(10_000);
        record.inflow_from_upper_pound_ml = Some(0);
        record.inflow_from_lower_pound_ml = Some(50_000);
        record.closing_system_storage_ml = Some(60_000);
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::BalancedForSyntheticCycle
        );
    }

    #[test]
    fn lower_pound_inflow_and_upper_return_are_conserved() {
        let mut record = cycle();
        record.opening_system_storage_ml = Some(1_000);
        record.inflow_from_upper_pound_ml = Some(0);
        record.inflow_from_lower_pound_ml = Some(100);
        record.outflow_to_upper_pound_ml = Some(100);
        record.closing_system_storage_ml = Some(1_000);
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).residual_ml,
            Some(0)
        );
    }

    #[test]
    fn declared_losses_are_part_of_water_balance() {
        let mut record = cycle();
        record.opening_system_storage_ml = Some(1_000);
        record.inflow_from_upper_pound_ml = Some(100);
        record.closing_system_storage_ml = Some(1_050);
        record.declared_losses_ml = Some(50);
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).residual_ml,
            Some(0)
        );
    }

    #[test]
    fn balance_residual_within_tolerance_is_accepted() {
        let mut record = cycle();
        record.inflow_from_upper_pound_ml = Some(500_050);
        let r = cycle_eval(&record, &WaterCycleCursor::default());
        assert_eq!(r.disposition, WaterCycleDisposition::BalancedForSyntheticCycle);
        assert_eq!(r.residual_ml, Some(50));
    }

    #[test]
    fn balance_residual_outside_tolerance_is_inconsistent() {
        let mut record = cycle();
        record.inflow_from_upper_pound_ml = Some(500_101);
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn missing_water_boundary_is_unknown_not_zero() {
        let mut record = cycle();
        record.inflow_from_lower_pound_ml = None;
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn water_storage_above_lock_profile_capacity_is_inconsistent() {
        let mut record = cycle();
        record.closing_system_storage_ml = Some(50_000_000_001);
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn revoked_cycle_evidence_is_inconsistent() {
        let mut record = cycle();
        record.evidence_state = EvidenceState::Revoked;
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn disputed_cycle_evidence_requires_review() {
        let mut record = cycle();
        record.evidence_state = EvidenceState::Disputed;
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn pump_energy_required_but_missing_is_unknown() {
        let mut record = cycle();
        record.pump_required = true;
        record.pump_meter_id = Some("meter-01".into());
        record.pump_calibration_valid_through_tick = Some(500);
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn pump_energy_without_meter_identity_is_unknown() {
        let mut record = cycle();
        record.pump_required = true;
        record.pump_energy_wh = Some(200);
        record.pump_calibration_valid_through_tick = Some(500);
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn expired_pump_calibration_requires_review() {
        let mut record = cycle();
        record.pump_required = true;
        record.pump_energy_wh = Some(200);
        record.pump_meter_id = Some("meter-01".into());
        record.pump_calibration_valid_through_tick = Some(119);
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn invalid_cycle_time_support_is_inconsistent() {
        let mut record = cycle();
        record.support.end_tick = record.support.start_tick;
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn cycle_for_different_lock_revision_is_inconsistent() {
        let mut record = cycle();
        record.lock_revision = "old-revision".into();
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn cycle_ending_after_evaluation_time_is_inconsistent() {
        let mut record = cycle();
        record.support.end_tick = 130;
        assert_eq!(
            evaluate_water_cycle(&lock(), &record, &WaterCycleCursor::default(), 120, 20).disposition,
            WaterCycleDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn exact_cycle_replay_is_idempotent() {
        let record = cycle();
        let first = cycle_eval(&record, &WaterCycleCursor::default());
        let replay = cycle_eval(&record, &first.next_cursor);
        assert_eq!(replay.disposition, WaterCycleDisposition::DuplicateExactReplay);
        assert_eq!(replay.residual_ml, first.residual_ml);
        assert_eq!(replay.next_cursor, first.next_cursor);
    }

    #[test]
    fn stale_cycle_evidence_requires_review() {
        let mut record = cycle();
        record.support = TimeInterval { start_tick: 50, end_tick: 60 };
        assert_eq!(
            evaluate_water_cycle(&lock(), &record, &WaterCycleCursor::default(), 120, 20).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn unknown_cycle_evidence_does_not_turn_residual_into_a_definitive_failure() {
        let mut record = cycle();
        record.inflow_from_upper_pound_ml = Some(10_000);
        record.evidence_state = EvidenceState::Unknown;
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn expired_lock_profile_requires_cycle_review() {
        let mut l = lock();
        l.valid_through_tick = Some(119);
        assert_eq!(
            evaluate_water_cycle(&l, &cycle(), &WaterCycleCursor::default(), 120, 20).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn invalid_lock_profile_capacity_is_not_used_when_cycle_evidence_is_unknown() {
        let mut record = cycle();
        record.closing_system_storage_ml = Some(60_000_000_000);
        record.evidence_state = EvidenceState::Unknown;
        assert_eq!(
            cycle_eval(&record, &WaterCycleCursor::default()).disposition,
            WaterCycleDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn changed_record_with_same_cycle_id_is_inconsistent() {
        let record = cycle();
        let first = cycle_eval(&record, &WaterCycleCursor::default());
        let mut changed = record;
        changed.inflow_from_upper_pound_ml = Some(500_001);
        assert_eq!(
            cycle_eval(&changed, &first.next_cursor).disposition,
            WaterCycleDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn invalid_lock_profile_identity_is_inconsistent() {
        let mut l = lock();
        l.lock_id.clear();
        assert_eq!(
            evaluate_passage(&l, &vessel(), &water(), LockAssetStatus::Available,
                OperatorApproval::Approved, 100, 10).disposition,
            PassageDisposition::EvidenceInconsistent
        );
    }
}
