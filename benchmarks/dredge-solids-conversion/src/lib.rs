//! Evidence-bound fixed-point conversion from wet slurry measurements to dry solids.
//! The output is a bounded observation only; it never creates a ledger transfer.

use std::collections::BTreeMap;

const PRODUCT_UNITS_PER_GRAM: u128 = 1_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeInterval {
    pub start_tick: u64,
    pub end_tick: u64,
}

impl TimeInterval {
    fn valid(self) -> bool {
        self.start_tick < self.end_tick
    }

    fn contains(self, tick: u64) -> bool {
        self.valid() && tick >= self.start_tick && tick < self.end_tick
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
pub enum WetVolumeUnit {
    Microlitre,
    Millilitre,
    CubicMetre,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityUnit {
    MilliKilogramPerCubicMetre,
    KilogramPerCubicMetre,
    GramPerLitre,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractionBasis {
    DrySolidsMassFraction,
    SolidsVolumeFraction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionManifest {
    pub conversion_id: String,
    pub plant_id: String,
    pub telemetry_event_id: String,
    pub telemetry_profile_revision: String,
    pub material_lot_id: String,
    pub activity_id: String,
    pub spatial_scope_id: String,
    pub support: TimeInterval,
    pub algorithm_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WetVolumeMeasurement {
    pub measurement_id: String,
    pub telemetry_event_id: String,
    pub telemetry_profile_revision: String,
    pub plant_id: String,
    pub material_lot_id: String,
    pub spatial_scope_id: String,
    pub support: TimeInterval,
    pub lower_microlitres: u64,
    pub upper_microlitres: u64,
    pub unit: WetVolumeUnit,
    pub instrument_id: String,
    pub calibration_valid_through_tick: Option<u64>,
    pub observed_at_tick: u64,
    pub evidence_state: EvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WetDensityMeasurement {
    pub measurement_id: String,
    pub plant_id: String,
    pub material_lot_id: String,
    pub spatial_scope_id: String,
    pub support: TimeInterval,
    pub lower_milli_kg_per_m3: u64,
    pub upper_milli_kg_per_m3: u64,
    pub unit: DensityUnit,
    pub instrument_id: String,
    pub calibration_valid_through_tick: Option<u64>,
    pub observed_at_tick: u64,
    pub evidence_state: EvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolidsFractionSample {
    pub sample_id: String,
    pub laboratory_id: String,
    pub method_revision: String,
    pub material_lot_id: String,
    pub spatial_scope_id: String,
    pub sampled_at_tick: u64,
    pub valid_through_tick: Option<u64>,
    pub lower_permille: u16,
    pub upper_permille: u16,
    pub basis: FractionBasis,
    pub representative_for_declared_support: bool,
    pub evidence_state: EvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionRequest {
    pub manifest: ConversionManifest,
    pub wet_volume: WetVolumeMeasurement,
    pub wet_density: WetDensityMeasurement,
    pub solids_fraction: SolidsFractionSample,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DryMassIntervalGrams {
    pub lower_grams: u128,
    pub upper_grams: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionDisposition {
    AcceptedAsBoundedSyntheticObservation,
    UnknownReviewRequired,
    EvidenceInconsistent,
    UnsupportedMeasurementBasis,
    DuplicateExactReplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionReason {
    InvalidManifestIdentity,
    InvalidTimeSupport,
    MissingMeasurementIdentity,
    MeasurementPlantMismatch,
    MeasurementLotMismatch,
    MeasurementSpatialScopeMismatch,
    MeasurementTimeSupportMismatch,
    TelemetryEventMismatch,
    TelemetryProfileMismatch,
    UnsupportedWetVolumeUnit,
    UnsupportedDensityUnit,
    UnsupportedFractionBasis,
    InvalidMeasurementInterval,
    InvalidDensityRange,
    InvalidFractionRange,
    FractionOutsidePhysicalRange,
    VolumeCalibrationUnknown,
    VolumeCalibrationExpired,
    DensityCalibrationUnknown,
    DensityCalibrationExpired,
    FractionValidityUnknown,
    FractionEvidenceExpired,
    VolumeMeasurementFromFuture,
    DensityMeasurementFromFuture,
    VolumeMeasurementOutsideSupport,
    DensityMeasurementOutsideSupport,
    FractionSampleOutsideSupport,
    VolumeEvidenceUnknown,
    DensityEvidenceUnknown,
    FractionEvidenceUnknown,
    VolumeEvidenceDisputed,
    DensityEvidenceDisputed,
    FractionEvidenceDisputed,
    VolumeEvidenceRevoked,
    DensityEvidenceRevoked,
    FractionEvidenceRevoked,
    FractionNotDeclaredRepresentative,
    ArithmeticOverflow,
    DuplicateConversionIdChangedInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConversionCursor {
    accepted: BTreeMap<String, (ConversionRequest, DryMassIntervalGrams)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionResult {
    pub disposition: ConversionDisposition,
    pub reasons: Vec<ConversionReason>,
    pub dry_mass_interval: Option<DryMassIntervalGrams>,
    pub next_cursor: ConversionCursor,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn check_evidence(
    state: EvidenceState,
    unknown_reason: ConversionReason,
    disputed_reason: ConversionReason,
    revoked_reason: ConversionReason,
    reasons: &mut Vec<ConversionReason>,
) -> (bool, bool) {
    match state {
        EvidenceState::Current => (false, false),
        EvidenceState::Unknown => {
            reasons.push(unknown_reason);
            (false, true)
        }
        EvidenceState::Disputed => {
            reasons.push(disputed_reason);
            (false, true)
        }
        EvidenceState::Revoked => {
            reasons.push(revoked_reason);
            (true, false)
        }
    }
}

fn validate_scope(request: &ConversionRequest, reasons: &mut Vec<ConversionReason>) -> bool {
    let manifest = &request.manifest;
    let volume = &request.wet_volume;
    let density = &request.wet_density;
    let fraction = &request.solids_fraction;
    let mut inconsistent = false;

    if !nonempty(&manifest.conversion_id)
        || !nonempty(&manifest.plant_id)
        || !nonempty(&manifest.telemetry_event_id)
        || !nonempty(&manifest.telemetry_profile_revision)
        || !nonempty(&manifest.material_lot_id)
        || !nonempty(&manifest.activity_id)
        || !nonempty(&manifest.spatial_scope_id)
        || !nonempty(&manifest.algorithm_revision)
    {
        reasons.push(ConversionReason::InvalidManifestIdentity);
        inconsistent = true;
    }
    if !manifest.support.valid() {
        reasons.push(ConversionReason::InvalidTimeSupport);
        inconsistent = true;
    }
    if !nonempty(&volume.measurement_id)
        || !nonempty(&volume.instrument_id)
        || !nonempty(&density.measurement_id)
        || !nonempty(&density.instrument_id)
        || !nonempty(&fraction.sample_id)
        || !nonempty(&fraction.laboratory_id)
        || !nonempty(&fraction.method_revision)
    {
        reasons.push(ConversionReason::MissingMeasurementIdentity);
        inconsistent = true;
    }

    for (plant, lot, spatial, support) in [
        (
            &volume.plant_id,
            &volume.material_lot_id,
            &volume.spatial_scope_id,
            volume.support,
        ),
        (
            &density.plant_id,
            &density.material_lot_id,
            &density.spatial_scope_id,
            density.support,
        ),
    ] {
        if plant != &manifest.plant_id {
            reasons.push(ConversionReason::MeasurementPlantMismatch);
            inconsistent = true;
        }
        if lot != &manifest.material_lot_id {
            reasons.push(ConversionReason::MeasurementLotMismatch);
            inconsistent = true;
        }
        if spatial != &manifest.spatial_scope_id {
            reasons.push(ConversionReason::MeasurementSpatialScopeMismatch);
            inconsistent = true;
        }
        if support != manifest.support {
            reasons.push(ConversionReason::MeasurementTimeSupportMismatch);
            inconsistent = true;
        }
    }

    if volume.telemetry_event_id != manifest.telemetry_event_id {
        reasons.push(ConversionReason::TelemetryEventMismatch);
        inconsistent = true;
    }
    if volume.telemetry_profile_revision != manifest.telemetry_profile_revision {
        reasons.push(ConversionReason::TelemetryProfileMismatch);
        inconsistent = true;
    }
    if fraction.material_lot_id != manifest.material_lot_id {
        reasons.push(ConversionReason::MeasurementLotMismatch);
        inconsistent = true;
    }
    if fraction.spatial_scope_id != manifest.spatial_scope_id {
        reasons.push(ConversionReason::MeasurementSpatialScopeMismatch);
        inconsistent = true;
    }
    if !manifest.support.contains(volume.observed_at_tick) {
        reasons.push(ConversionReason::VolumeMeasurementOutsideSupport);
        inconsistent = true;
    }
    if !manifest.support.contains(density.observed_at_tick) {
        reasons.push(ConversionReason::DensityMeasurementOutsideSupport);
        inconsistent = true;
    }
    if !manifest.support.contains(fraction.sampled_at_tick) {
        reasons.push(ConversionReason::FractionSampleOutsideSupport);
        inconsistent = true;
    }
    inconsistent
}

fn product(a: u64, b: u64, c: u16) -> Result<u128, ConversionReason> {
    u128::from(a)
        .checked_mul(u128::from(b))
        .and_then(|value| value.checked_mul(u128::from(c)))
        .ok_or(ConversionReason::ArithmeticOverflow)
}

fn derive_interval(
    request: &ConversionRequest,
) -> Result<DryMassIntervalGrams, ConversionReason> {
    let low = product(
        request.wet_volume.lower_microlitres,
        request.wet_density.lower_milli_kg_per_m3,
        request.solids_fraction.lower_permille,
    )?;
    let high = product(
        request.wet_volume.upper_microlitres,
        request.wet_density.upper_milli_kg_per_m3,
        request.solids_fraction.upper_permille,
    )?;

    let lower_grams = low / PRODUCT_UNITS_PER_GRAM;
    let quotient = high / PRODUCT_UNITS_PER_GRAM;
    let upper_grams = quotient
        .checked_add(if high % PRODUCT_UNITS_PER_GRAM == 0 { 0 } else { 1 })
        .ok_or(ConversionReason::ArithmeticOverflow)?;

    Ok(DryMassIntervalGrams {
        lower_grams,
        upper_grams,
    })
}

/// Produce a bounded dry-mass observation. No ledger transfer is created.
#[must_use]
pub fn evaluate_conversion(
    request: &ConversionRequest,
    cursor: &ConversionCursor,
    evaluation_tick: u64,
    maximum_measurement_age_ticks: u64,
) -> ConversionResult {
    let mut next_cursor = cursor.clone();
    let mut reasons = Vec::new();
    let mut inconsistent = validate_scope(request, &mut reasons);
    let mut unknown = false;
    let mut unsupported = false;

    if let Some((prior, prior_result)) = cursor.accepted.get(&request.manifest.conversion_id) {
        if prior == request {
            return ConversionResult {
                disposition: ConversionDisposition::DuplicateExactReplay,
                reasons: Vec::new(),
                dry_mass_interval: Some(*prior_result),
                next_cursor,
            };
        }
        reasons.push(ConversionReason::DuplicateConversionIdChangedInput);
        inconsistent = true;
    }

    if request.wet_volume.unit != WetVolumeUnit::Microlitre {
        reasons.push(ConversionReason::UnsupportedWetVolumeUnit);
        unsupported = true;
    }
    if request.wet_density.unit != DensityUnit::MilliKilogramPerCubicMetre {
        reasons.push(ConversionReason::UnsupportedDensityUnit);
        unsupported = true;
    }
    if request.solids_fraction.basis != FractionBasis::DrySolidsMassFraction {
        reasons.push(ConversionReason::UnsupportedFractionBasis);
        unsupported = true;
    }

    if request.wet_volume.lower_microlitres > request.wet_volume.upper_microlitres {
        reasons.push(ConversionReason::InvalidMeasurementInterval);
        inconsistent = true;
    }
    if request.wet_density.upper_milli_kg_per_m3 == 0
        || request.wet_density.lower_milli_kg_per_m3 > request.wet_density.upper_milli_kg_per_m3
    {
        reasons.push(ConversionReason::InvalidDensityRange);
        inconsistent = true;
    }
    if request.solids_fraction.lower_permille > request.solids_fraction.upper_permille {
        reasons.push(ConversionReason::InvalidFractionRange);
        inconsistent = true;
    }
    if request.solids_fraction.upper_permille > 1_000 {
        reasons.push(ConversionReason::FractionOutsidePhysicalRange);
        inconsistent = true;
    }

    match request.wet_volume.calibration_valid_through_tick {
        None => {
            reasons.push(ConversionReason::VolumeCalibrationUnknown);
            unknown = true;
        }
        Some(expiry) if expiry < evaluation_tick => {
            reasons.push(ConversionReason::VolumeCalibrationExpired);
            unknown = true;
        }
        Some(_) => {}
    }
    match request.wet_density.calibration_valid_through_tick {
        None => {
            reasons.push(ConversionReason::DensityCalibrationUnknown);
            unknown = true;
        }
        Some(expiry) if expiry < evaluation_tick => {
            reasons.push(ConversionReason::DensityCalibrationExpired);
            unknown = true;
        }
        Some(_) => {}
    }
    match request.solids_fraction.valid_through_tick {
        None => {
            reasons.push(ConversionReason::FractionValidityUnknown);
            unknown = true;
        }
        Some(expiry) if expiry < evaluation_tick => {
            reasons.push(ConversionReason::FractionEvidenceExpired);
            unknown = true;
        }
        Some(_) => {}
    }

    if request.wet_volume.observed_at_tick > evaluation_tick {
        reasons.push(ConversionReason::VolumeMeasurementFromFuture);
        inconsistent = true;
    } else if evaluation_tick - request.wet_volume.observed_at_tick > maximum_measurement_age_ticks {
        reasons.push(ConversionReason::VolumeEvidenceUnknown);
        unknown = true;
    }
    if request.wet_density.observed_at_tick > evaluation_tick {
        reasons.push(ConversionReason::DensityMeasurementFromFuture);
        inconsistent = true;
    } else if evaluation_tick - request.wet_density.observed_at_tick > maximum_measurement_age_ticks {
        reasons.push(ConversionReason::DensityEvidenceUnknown);
        unknown = true;
    }
    if !request.solids_fraction.representative_for_declared_support {
        reasons.push(ConversionReason::FractionNotDeclaredRepresentative);
        unknown = true;
    }

    for (bad, review) in [
        check_evidence(
            request.wet_volume.evidence_state,
            ConversionReason::VolumeEvidenceUnknown,
            ConversionReason::VolumeEvidenceDisputed,
            ConversionReason::VolumeEvidenceRevoked,
            &mut reasons,
        ),
        check_evidence(
            request.wet_density.evidence_state,
            ConversionReason::DensityEvidenceUnknown,
            ConversionReason::DensityEvidenceDisputed,
            ConversionReason::DensityEvidenceRevoked,
            &mut reasons,
        ),
        check_evidence(
            request.solids_fraction.evidence_state,
            ConversionReason::FractionEvidenceUnknown,
            ConversionReason::FractionEvidenceDisputed,
            ConversionReason::FractionEvidenceRevoked,
            &mut reasons,
        ),
    ] {
        inconsistent |= bad;
        unknown |= review;
    }

    let mass_interval = if !inconsistent && !unsupported && !unknown {
        match derive_interval(request) {
            Ok(value) => Some(value),
            Err(reason) => {
                reasons.push(reason);
                inconsistent = true;
                None
            }
        }
    } else {
        None
    };

    let disposition = if inconsistent {
        ConversionDisposition::EvidenceInconsistent
    } else if unsupported {
        ConversionDisposition::UnsupportedMeasurementBasis
    } else if unknown {
        ConversionDisposition::UnknownReviewRequired
    } else {
        ConversionDisposition::AcceptedAsBoundedSyntheticObservation
    };

    if disposition == ConversionDisposition::AcceptedAsBoundedSyntheticObservation {
        if let Some(result) = mass_interval {
            next_cursor.accepted.insert(
                request.manifest.conversion_id.clone(),
                (request.clone(), result),
            );
        }
    }

    ConversionResult {
        disposition,
        reasons,
        dry_mass_interval: mass_interval,
        next_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn support() -> TimeInterval {
        TimeInterval { start_tick: 100, end_tick: 200 }
    }

    fn request() -> ConversionRequest {
        ConversionRequest {
            manifest: ConversionManifest {
                conversion_id: "conversion-001".into(),
                plant_id: "pipeline-plant-01".into(),
                telemetry_event_id: "telemetry-event-77".into(),
                telemetry_profile_revision: "pipeline-standard-r3".into(),
                material_lot_id: "lot-sand-04".into(),
                activity_id: "dredge-campaign-09".into(),
                spatial_scope_id: "inlet-cell-east".into(),
                support: support(),
                algorithm_revision: "dry-mass-v0.1".into(),
            },
            wet_volume: WetVolumeMeasurement {
                measurement_id: "wet-volume-77".into(),
                telemetry_event_id: "telemetry-event-77".into(),
                telemetry_profile_revision: "pipeline-standard-r3".into(),
                plant_id: "pipeline-plant-01".into(),
                material_lot_id: "lot-sand-04".into(),
                spatial_scope_id: "inlet-cell-east".into(),
                support: support(),
                lower_microlitres: 1_000_000_000,
                upper_microlitres: 1_000_000_000,
                unit: WetVolumeUnit::Microlitre,
                instrument_id: "flow-totalizer-01".into(),
                calibration_valid_through_tick: Some(500),
                observed_at_tick: 150,
                evidence_state: EvidenceState::Current,
            },
            wet_density: WetDensityMeasurement {
                measurement_id: "density-77".into(),
                plant_id: "pipeline-plant-01".into(),
                material_lot_id: "lot-sand-04".into(),
                spatial_scope_id: "inlet-cell-east".into(),
                support: support(),
                lower_milli_kg_per_m3: 1_200_000,
                upper_milli_kg_per_m3: 1_200_000,
                unit: DensityUnit::MilliKilogramPerCubicMetre,
                instrument_id: "density-meter-02".into(),
                calibration_valid_through_tick: Some(500),
                observed_at_tick: 150,
                evidence_state: EvidenceState::Current,
            },
            solids_fraction: SolidsFractionSample {
                sample_id: "lab-sample-77".into(),
                laboratory_id: "lab-01".into(),
                method_revision: "dry-solids-method-r3".into(),
                material_lot_id: "lot-sand-04".into(),
                spatial_scope_id: "inlet-cell-east".into(),
                sampled_at_tick: 150,
                valid_through_tick: Some(500),
                lower_permille: 250,
                upper_permille: 250,
                basis: FractionBasis::DrySolidsMassFraction,
                representative_for_declared_support: true,
                evidence_state: EvidenceState::Current,
            },
        }
    }

    fn eval(req: &ConversionRequest, cursor: &ConversionCursor) -> ConversionResult {
        evaluate_conversion(req, cursor, 160, 10)
    }

    #[test]
    fn exact_fixed_point_calculation_returns_three_hundred_kilograms() {
        let result = eval(&request(), &ConversionCursor::default());
        assert_eq!(result.disposition, ConversionDisposition::AcceptedAsBoundedSyntheticObservation);
        assert_eq!(result.dry_mass_interval, Some(DryMassIntervalGrams {
            lower_grams: 300_000, upper_grams: 300_000
        }));
    }

    #[test]
    fn interval_uncertainty_is_propagated_conservatively() {
        let mut req = request();
        req.wet_volume.lower_microlitres = 900_000_000;
        req.wet_volume.upper_microlitres = 1_100_000_000;
        req.wet_density.lower_milli_kg_per_m3 = 1_100_000;
        req.wet_density.upper_milli_kg_per_m3 = 1_300_000;
        req.solids_fraction.lower_permille = 200;
        req.solids_fraction.upper_permille = 300;
        assert_eq!(eval(&req, &ConversionCursor::default()).dry_mass_interval,
            Some(DryMassIntervalGrams { lower_grams: 198_000, upper_grams: 429_000 }));
    }

    #[test]
    fn upper_mass_rounds_outward() {
        let mut req = request();
        req.wet_volume.upper_microlitres += 1;
        let result = eval(&req, &ConversionCursor::default());
        assert_eq!(result.dry_mass_interval.unwrap().upper_grams, 300_001);
    }

    #[test]
    fn zero_solids_fraction_is_a_valid_zero_mass_observation() {
        let mut req = request();
        req.solids_fraction.lower_permille = 0;
        req.solids_fraction.upper_permille = 0;
        assert_eq!(eval(&req, &ConversionCursor::default()).dry_mass_interval,
            Some(DryMassIntervalGrams { lower_grams: 0, upper_grams: 0 }));
    }

    #[test]
    fn fraction_above_one_thousand_permille_is_inconsistent() {
        let mut req = request();
        req.solids_fraction.upper_permille = 1_001;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn zero_density_range_is_inconsistent() {
        let mut req = request();
        req.wet_density.lower_milli_kg_per_m3 = 0;
        req.wet_density.upper_milli_kg_per_m3 = 0;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn density_interval_with_zero_lower_bound_is_conservative() {
        let mut req = request();
        req.wet_density.lower_milli_kg_per_m3 = 0;
        let result = eval(&req, &ConversionCursor::default());
        assert_eq!(result.disposition, ConversionDisposition::AcceptedAsBoundedSyntheticObservation);
        assert_eq!(result.dry_mass_interval.unwrap().lower_grams, 0);
    }

    #[test]
    fn reversed_volume_interval_is_inconsistent() {
        let mut req = request();
        req.wet_volume.lower_microlitres = 101;
        req.wet_volume.upper_microlitres = 100;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn unsupported_volume_unit_does_not_convert() {
        let mut req = request();
        req.wet_volume.unit = WetVolumeUnit::CubicMetre;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnsupportedMeasurementBasis);
    }

    #[test]
    fn unsupported_density_unit_does_not_convert() {
        let mut req = request();
        req.wet_density.unit = DensityUnit::GramPerLitre;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnsupportedMeasurementBasis);
    }

    #[test]
    fn solids_volume_fraction_is_not_treated_as_mass_fraction() {
        let mut req = request();
        req.solids_fraction.basis = FractionBasis::SolidsVolumeFraction;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnsupportedMeasurementBasis);
    }

    #[test]
    fn missing_density_evidence_does_not_create_mass() {
        let mut req = request();
        req.wet_density.evidence_state = EvidenceState::Unknown;
        let result = eval(&req, &ConversionCursor::default());
        assert_eq!(result.disposition, ConversionDisposition::UnknownReviewRequired);
        assert!(result.dry_mass_interval.is_none());
    }

    #[test]
    fn missing_fraction_evidence_does_not_create_mass() {
        let mut req = request();
        req.solids_fraction.evidence_state = EvidenceState::Unknown;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnknownReviewRequired);
    }

    #[test]
    fn lot_mismatch_is_inconsistent() {
        let mut req = request();
        req.solids_fraction.material_lot_id = "other-lot".into();
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn plant_mismatch_is_inconsistent() {
        let mut req = request();
        req.wet_density.plant_id = "other-plant".into();
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn spatial_scope_mismatch_is_inconsistent() {
        let mut req = request();
        req.wet_density.spatial_scope_id = "other-cell".into();
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn support_mismatch_is_inconsistent() {
        let mut req = request();
        req.wet_density.support.end_tick = 190;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn sample_outside_support_is_inconsistent() {
        let mut req = request();
        req.solids_fraction.sampled_at_tick = 99;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn nonrepresentative_sample_requires_review() {
        let mut req = request();
        req.solids_fraction.representative_for_declared_support = false;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnknownReviewRequired);
    }

    #[test]
    fn expired_volume_calibration_requires_review() {
        let mut req = request();
        req.wet_volume.calibration_valid_through_tick = Some(159);
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnknownReviewRequired);
    }

    #[test]
    fn unknown_density_calibration_requires_review() {
        let mut req = request();
        req.wet_density.calibration_valid_through_tick = None;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnknownReviewRequired);
    }

    #[test]
    fn expired_fraction_sample_requires_review() {
        let mut req = request();
        req.solids_fraction.valid_through_tick = Some(159);
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnknownReviewRequired);
    }

    #[test]
    fn future_measurement_is_inconsistent() {
        let mut req = request();
        req.wet_volume.observed_at_tick = 161;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn revoked_lab_evidence_is_inconsistent() {
        let mut req = request();
        req.solids_fraction.evidence_state = EvidenceState::Revoked;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn disputed_density_requires_review() {
        let mut req = request();
        req.wet_density.evidence_state = EvidenceState::Disputed;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::UnknownReviewRequired);
    }

    #[test]
    fn exact_duplicate_replay_is_idempotent() {
        let req = request();
        let first = eval(&req, &ConversionCursor::default());
        let replay = eval(&req, &first.next_cursor);
        assert_eq!(replay.disposition, ConversionDisposition::DuplicateExactReplay);
        assert_eq!(replay.dry_mass_interval, first.dry_mass_interval);
        assert_eq!(replay.next_cursor, first.next_cursor);
    }

    #[test]
    fn duplicate_conversion_id_with_changed_input_is_inconsistent() {
        let req = request();
        let first = eval(&req, &ConversionCursor::default());
        let mut changed = req;
        changed.wet_volume.upper_microlitres += 1;
        assert_eq!(eval(&changed, &first.next_cursor).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn multiplication_overflow_is_rejected_without_output() {
        let mut req = request();
        req.wet_volume.upper_microlitres = u64::MAX;
        req.wet_density.upper_milli_kg_per_m3 = u64::MAX;
        req.solids_fraction.upper_permille = 1_000;
        let result = eval(&req, &ConversionCursor::default());
        assert_eq!(result.disposition, ConversionDisposition::EvidenceInconsistent);
        assert!(result.dry_mass_interval.is_none());
    }

    #[test]
    fn invalid_support_is_inconsistent() {
        let mut req = request();
        req.manifest.support.end_tick = req.manifest.support.start_tick;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn telemetry_profile_revision_mismatch_is_inconsistent() {
        let mut req = request();
        req.wet_volume.telemetry_profile_revision = "stale-profile".into();
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn volume_observation_outside_support_is_inconsistent() {
        let mut req = request();
        req.wet_volume.observed_at_tick = 99;
        assert_eq!(eval(&req, &ConversionCursor::default()).disposition,
            ConversionDisposition::EvidenceInconsistent);
    }

    #[test]
    fn bounded_observation_does_not_change_ledger_state() {
        let result = eval(&request(), &ConversionCursor::default());
        assert!(result.dry_mass_interval.is_some());
        // The public interface has no ledger mutation or transfer creation.
        assert_eq!(result.next_cursor.accepted.len(), 1);
    }
}
