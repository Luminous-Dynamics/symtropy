//! Minimal, deterministic accounting and geometry checks for the
//! ASTEROID-OPTICAL-ISRU-001 benchmark.
//!
//! This crate deliberately does not simulate thermal spalling, chemistry,
//! particle capture dynamics, orbital motion, or hardware safety. It checks
//! declared boundaries and closed-form screening relationships only.

use serde::Deserialize;
use std::collections::BTreeSet;

pub const SOLAR_ANGULAR_DIAMETER_AT_1_AU_RAD: f64 = 9.30e-3;
pub const AU_REFERENCE_SOLAR_FLUX_W_M2: f64 = 1361.0;
pub const NUMERIC_EPSILON: f64 = 1.0e-9;

#[derive(Debug, Clone, PartialEq)]
pub enum AuditError {
    NonFinite(&'static str),
    Negative(&'static str),
    InvalidRange(&'static str),
    StageOutOfOrder(&'static str),
    EnergyOverdraw(&'static str),
    MassImbalance { residual_kg: f64, tolerance_kg: f64 },
    ProductExceedsCapturedGas,
    ProductPurityBelowContract,
    ProductAssayMissing,
    DuplicateSourceId(String),
    DuplicateCaseId(String),
    UnknownSource(String),
    UnsupportedSchema(String),
    MissingClaimCeiling,
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinite(field) => write!(f, "{field} must be finite"),
            Self::Negative(field) => write!(f, "{field} must be non-negative"),
            Self::InvalidRange(field) => write!(f, "{field} is outside its valid range"),
            Self::StageOutOfOrder(field) => {
                write!(f, "energy stage {field} is present after a missing stage")
            }
            Self::EnergyOverdraw(field) => {
                write!(f, "energy stage {field} exceeds its immediately preceding stage")
            }
            Self::MassImbalance {
                residual_kg,
                tolerance_kg,
            } => write!(
                f,
                "mass balance residual {residual_kg} kg exceeds tolerance {tolerance_kg} kg"
            ),
            Self::ProductExceedsCapturedGas => {
                write!(f, "accepted product exceeds captured raw gas mass")
            }
            Self::ProductPurityBelowContract => {
                write!(f, "product purity is below the declared acceptance threshold")
            }
            Self::ProductAssayMissing => {
                write!(f, "independent product assay is required for acceptance")
            }
            Self::DuplicateSourceId(id) => write!(f, "duplicate source id: {id}"),
            Self::DuplicateCaseId(id) => write!(f, "duplicate case id: {id}"),
            Self::UnknownSource(id) => write!(f, "case references unknown source: {id}"),
            Self::UnsupportedSchema(version) => {
                write!(f, "unsupported corpus schema version: {version}")
            }
            Self::MissingClaimCeiling => write!(f, "corpus must declare its claim ceiling"),
        }
    }
}

impl std::error::Error for AuditError {}

fn require_finite_nonnegative(value: f64, field: &'static str) -> Result<(), AuditError> {
    if !value.is_finite() {
        return Err(AuditError::NonFinite(field));
    }
    if value < 0.0 {
        return Err(AuditError::Negative(field));
    }
    Ok(())
}

fn require_finite_positive(value: f64, field: &'static str) -> Result<(), AuditError> {
    require_finite_nonnegative(value, field)?;
    if value <= 0.0 {
        return Err(AuditError::InvalidRange(field));
    }
    Ok(())
}

/// Inverse-square solar-flux screening from a declared reference flux at 1 AU.
/// This is incident solar flux before site horizon, atmosphere, collector,
/// tracking, degradation, or optical losses.
pub fn solar_flux_at_au(
    reference_flux_w_m2: f64,
    heliocentric_distance_au: f64,
) -> Result<f64, AuditError> {
    require_finite_positive(reference_flux_w_m2, "reference_flux_w_m2")?;
    require_finite_positive(heliocentric_distance_au, "heliocentric_distance_au")?;
    Ok(reference_flux_w_m2 / heliocentric_distance_au.powi(2))
}

/// Geometric diameter of the solar image at a focal distance / mirror-to-target
/// range. The Sun's angular diameter is approximately 9.30 mrad at 1 AU and
/// scales inversely with heliocentric distance.
///
/// This is not a full concentrator model. Optical aberration, mirror shape,
/// pointing, thermal deformation, obscuration and nonuniform focal irradiance
/// are deliberately outside this screening relationship.
pub fn solar_image_diameter_m(
    range_m: f64,
    heliocentric_distance_au: f64,
) -> Result<f64, AuditError> {
    require_finite_positive(range_m, "range_m")?;
    require_finite_positive(heliocentric_distance_au, "heliocentric_distance_au")?;
    let solar_angular_diameter =
        SOLAR_ANGULAR_DIAMETER_AT_1_AU_RAD / heliocentric_distance_au;
    Ok(range_m * solar_angular_diameter)
}

/// Ideal geometric concentration ratio for a circular collector focused onto
/// the solar image. This is collector area divided by solar-image area and
/// assumes all intercepted energy reaches that image uniformly with no optical
/// losses. It is an upper-bound screen, not a flux-profile or thermal model.
pub fn ideal_solar_concentration_ratio(
    collector_diameter_m: f64,
    focal_distance_m: f64,
    heliocentric_distance_au: f64,
) -> Result<f64, AuditError> {
    require_finite_positive(collector_diameter_m, "collector_diameter_m")?;
    let image_diameter =
        solar_image_diameter_m(focal_distance_m, heliocentric_distance_au)?;
    Ok((collector_diameter_m / image_diameter).powi(2))
}

/// Ideal circular-aperture Airy first-null diameter in the far field:
/// 2.44 * wavelength * range / aperture.
///
/// Excludes beam quality (M²), pointing jitter, truncation, aberration,
/// turbulence/plasma effects, receiver tracking and finite receiver size.
/// It is a diffraction screen, not a complete laser link budget.
pub fn ideal_laser_airy_diameter_m(
    wavelength_m: f64,
    range_m: f64,
    aperture_m: f64,
) -> Result<f64, AuditError> {
    require_finite_positive(wavelength_m, "wavelength_m")?;
    require_finite_positive(range_m, "range_m")?;
    require_finite_positive(aperture_m, "aperture_m")?;
    Ok(2.44 * wavelength_m * range_m / aperture_m)
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceRecord {
    pub id: String,
    pub title: String,
    pub url: String,
    pub evidence_class: String,
    pub caveat: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaseRecord {
    pub id: String,
    pub evidence_class: String,
    pub expected_disposition: String,
    pub note: String,
    pub source_id: Option<String>,
    pub conditions: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkCorpus {
    pub schema_version: String,
    pub claim_ceiling: String,
    pub source_records: Vec<SourceRecord>,
    pub cases: Vec<CaseRecord>,
}

/// Validate the stable identities and cross-references in the frozen JSON
/// corpus. This says nothing about the truth of cited physical claims.
pub fn validate_corpus(corpus: &BenchmarkCorpus) -> Result<(), AuditError> {
    if corpus.schema_version != "0.1" {
        return Err(AuditError::UnsupportedSchema(
            corpus.schema_version.clone(),
        ));
    }
    if corpus.claim_ceiling.trim().is_empty() {
        return Err(AuditError::MissingClaimCeiling);
    }

    let mut source_ids = BTreeSet::new();
    for source in &corpus.source_records {
        if !source_ids.insert(source.id.as_str()) {
            return Err(AuditError::DuplicateSourceId(source.id.clone()));
        }
        if source.id.trim().is_empty()
            || source.title.trim().is_empty()
            || source.url.trim().is_empty()
            || source.evidence_class.trim().is_empty()
            || source.caveat.trim().is_empty()
        {
            return Err(AuditError::InvalidRange("source_record_required_field"));
        }
    }

    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.id.as_str()) {
            return Err(AuditError::DuplicateCaseId(case.id.clone()));
        }
        if case.id.trim().is_empty()
            || case.evidence_class.trim().is_empty()
            || case.expected_disposition.trim().is_empty()
            || case.note.trim().is_empty()
            || !case.conditions.is_object()
        {
            return Err(AuditError::InvalidRange("case_record_required_field"));
        }
        if let Some(source_id) = &case.source_id {
            if !source_ids.contains(source_id.as_str()) {
                return Err(AuditError::UnknownSource(source_id.clone()));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceEnergyBoundary {
    SolarIncidentAtCollector,
    DcElectricalAtTransmitter,
}

/// Energy is kept in kWh at explicit measurement planes. A missing stage
/// terminates the chain; later stages cannot be silently inferred.
#[derive(Debug, Clone, Deserialize)]
pub struct EnergyChain {
    pub source_boundary: SourceEnergyBoundary,
    pub source_input_kwh: f64,
    pub optical_output_kwh: Option<f64>,
    pub receiver_incident_kwh: Option<f64>,
    pub receiver_absorbed_kwh: Option<f64>,
    pub useful_process_kwh: Option<f64>,
}

impl EnergyChain {
    pub fn validate(&self) -> Result<(), AuditError> {
        require_finite_nonnegative(self.source_input_kwh, "source_input_kwh")?;
        let stages = [
            ("optical_output_kwh", self.optical_output_kwh),
            ("receiver_incident_kwh", self.receiver_incident_kwh),
            ("receiver_absorbed_kwh", self.receiver_absorbed_kwh),
            ("useful_process_kwh", self.useful_process_kwh),
        ];

        let mut previous = self.source_input_kwh;
        let mut missing_seen = false;
        for (name, stage) in stages {
            match stage {
                Some(value) => {
                    if missing_seen {
                        return Err(AuditError::StageOutOfOrder(name));
                    }
                    require_finite_nonnegative(value, name)?;
                    if value > previous + NUMERIC_EPSILON {
                        return Err(AuditError::EnergyOverdraw(name));
                    }
                    previous = value;
                }
                None => missing_seen = true,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MaterialBalance {
    pub feedstock_input_kg: f64,
    pub solid_residue_kg: f64,
    pub captured_solids_kg: f64,
    pub escaped_solids_kg: f64,
    pub captured_raw_gas_kg: f64,
    pub escaped_gas_kg: f64,
    pub other_accounted_mass_kg: f64,
}

impl MaterialBalance {
    pub fn residual_kg(&self) -> Result<f64, AuditError> {
        let values = [
            ("feedstock_input_kg", self.feedstock_input_kg),
            ("solid_residue_kg", self.solid_residue_kg),
            ("captured_solids_kg", self.captured_solids_kg),
            ("escaped_solids_kg", self.escaped_solids_kg),
            ("captured_raw_gas_kg", self.captured_raw_gas_kg),
            ("escaped_gas_kg", self.escaped_gas_kg),
            ("other_accounted_mass_kg", self.other_accounted_mass_kg),
        ];
        for (name, value) in values {
            require_finite_nonnegative(value, name)?;
        }

        let accounted = self.solid_residue_kg
            + self.captured_solids_kg
            + self.escaped_solids_kg
            + self.captured_raw_gas_kg
            + self.escaped_gas_kg
            + self.other_accounted_mass_kg;
        Ok(self.feedstock_input_kg - accounted)
    }

    pub fn validate(&self, tolerance_kg: f64) -> Result<(), AuditError> {
        require_finite_nonnegative(tolerance_kg, "tolerance_kg")?;
        let residual = self.residual_kg()?;
        if residual.abs() > tolerance_kg {
            return Err(AuditError::MassImbalance {
                residual_kg: residual,
                tolerance_kg,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductAcceptance {
    pub captured_raw_gas_kg: f64,
    pub purified_product_kg: f64,
    pub measured_purity_fraction: f64,
    pub minimum_purity_fraction: f64,
    pub independent_assay_present: bool,
}

impl ProductAcceptance {
    /// Accepted product is independently re-derived from declared product and
    /// assay fields. Captured gas is not automatically accepted product.
    pub fn accepted_product_kg(&self) -> Result<f64, AuditError> {
        require_finite_nonnegative(self.captured_raw_gas_kg, "captured_raw_gas_kg")?;
        require_finite_nonnegative(self.purified_product_kg, "purified_product_kg")?;
        if !self.measured_purity_fraction.is_finite()
            || !self.minimum_purity_fraction.is_finite()
        {
            return Err(AuditError::NonFinite("purity_fraction"));
        }
        if !(0.0..=1.0).contains(&self.measured_purity_fraction)
            || !(0.0..=1.0).contains(&self.minimum_purity_fraction)
        {
            return Err(AuditError::InvalidRange("purity_fraction"));
        }
        if self.purified_product_kg > self.captured_raw_gas_kg + NUMERIC_EPSILON {
            return Err(AuditError::ProductExceedsCapturedGas);
        }
        if self.measured_purity_fraction < self.minimum_purity_fraction {
            return Err(AuditError::ProductPurityBelowContract);
        }
        if !self.independent_assay_present {
            return Err(AuditError::ProductAssayMissing);
        }
        Ok(self.purified_product_kg)
    }
}

/// Returns product kg per kWh measured at the source boundary.
/// The boundary enum is returned alongside the ratio so unlike source
/// boundaries cannot be silently compared.
pub fn accepted_product_yield(
    product: &ProductAcceptance,
    energy: &EnergyChain,
) -> Result<(f64, SourceEnergyBoundary), AuditError> {
    energy.validate()?;
    let accepted_kg = product.accepted_product_kg()?;
    require_finite_positive(energy.source_input_kwh, "source_input_kwh")?;
    Ok((
        accepted_kg / energy.source_input_kwh,
        energy.source_boundary,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_energy() -> EnergyChain {
        EnergyChain {
            source_boundary: SourceEnergyBoundary::DcElectricalAtTransmitter,
            source_input_kwh: 10.0,
            optical_output_kwh: Some(4.0),
            receiver_incident_kwh: Some(3.0),
            receiver_absorbed_kwh: Some(2.5),
            useful_process_kwh: Some(2.0),
        }
    }

    fn valid_mass() -> MaterialBalance {
        MaterialBalance {
            feedstock_input_kg: 10.0,
            solid_residue_kg: 7.0,
            captured_solids_kg: 1.0,
            escaped_solids_kg: 0.5,
            captured_raw_gas_kg: 1.0,
            escaped_gas_kg: 0.4,
            other_accounted_mass_kg: 0.1,
        }
    }

    fn valid_product() -> ProductAcceptance {
        ProductAcceptance {
            captured_raw_gas_kg: 1.0,
            purified_product_kg: 0.7,
            measured_purity_fraction: 0.995,
            minimum_purity_fraction: 0.99,
            independent_assay_present: true,
        }
    }

    #[test]
    fn inverse_square_flux_is_reproduced_at_two_au() {
        let flux = solar_flux_at_au(1361.0, 2.0).unwrap();
        assert!((flux - 340.25).abs() < 1.0e-9);
    }

    #[test]
    fn solar_image_scales_with_mirror_to_target_range() {
        let spot = solar_image_diameter_m(100_000.0, 1.0).unwrap();
        assert!((spot - 930.0).abs() < 1.0e-9);
    }

    #[test]
    fn solar_image_shrinks_with_heliocentric_distance() {
        let spot = solar_image_diameter_m(100_000.0, 2.0).unwrap();
        assert!((spot - 465.0).abs() < 1.0e-9);
        let belt_spot = solar_image_diameter_m(100_000.0, 2.7).unwrap();
        assert!((belt_spot - (930.0 / 2.7)).abs() < 1.0e-9);
    }

    #[test]
    fn ideal_solar_concentration_depends_on_aperture_and_standoff() {
        let near = ideal_solar_concentration_ratio(10.0, 100.0, 1.0).unwrap();
        let far = ideal_solar_concentration_ratio(10.0, 1_000.0, 1.0).unwrap();
        assert!((near - (10.0_f64 / 0.93).powi(2)).abs() < 1.0e-9);
        assert!((far - (10.0_f64 / 9.3).powi(2)).abs() < 1.0e-9);
        assert!(near > far * 99.0);
    }

    #[test]
    fn ideal_laser_diffraction_screen_matches_hand_calculation() {
        let diameter =
            ideal_laser_airy_diameter_m(1.064e-6, 1_000.0, 2.0).unwrap();
        assert!((diameter - 0.00129808).abs() < 1.0e-7);
    }

    #[test]
    fn rejects_zero_or_negative_heliocentric_distance() {
        assert!(solar_flux_at_au(1361.0, 0.0).is_err());
        assert!(solar_flux_at_au(1361.0, -1.0).is_err());
    }

    #[test]
    fn rejects_non_finite_geometry() {
        assert!(solar_image_diameter_m(f64::NAN, 1.0).is_err());
        assert!(solar_image_diameter_m(100.0, 0.0).is_err());
        assert!(ideal_laser_airy_diameter_m(1.0e-6, f64::INFINITY, 1.0).is_err());
    }

    #[test]
    fn valid_energy_chain_closes_as_monotonic_meter_planes() {
        assert!(valid_energy().validate().is_ok());
    }

    #[test]
    fn rejects_energy_stage_larger_than_previous_boundary() {
        let mut chain = valid_energy();
        chain.receiver_incident_kwh = Some(4.5);
        assert_eq!(
            chain.validate().unwrap_err(),
            AuditError::EnergyOverdraw("receiver_incident_kwh")
        );
    }

    #[test]
    fn rejects_energy_stage_after_unknown_intermediate() {
        let mut chain = valid_energy();
        chain.receiver_incident_kwh = None;
        assert_eq!(
            chain.validate().unwrap_err(),
            AuditError::StageOutOfOrder("receiver_absorbed_kwh")
        );
    }

    #[test]
    fn rejects_negative_energy() {
        let mut chain = valid_energy();
        chain.optical_output_kwh = Some(-1.0);
        assert!(chain.validate().is_err());
    }

    #[test]
    fn mass_balance_closes_for_hand_calculated_fixture() {
        assert!(valid_mass().validate(1.0e-9).is_ok());
    }

    #[test]
    fn mass_balance_rejects_missing_or_unaccounted_mass() {
        let mut balance = valid_mass();
        balance.escaped_solids_kg = 0.0;
        assert!(matches!(
            balance.validate(1.0e-6),
            Err(AuditError::MassImbalance { .. })
        ));
    }

    #[test]
    fn accepted_product_requires_independent_assay() {
        let mut product = valid_product();
        product.independent_assay_present = false;
        assert_eq!(
            product.accepted_product_kg().unwrap_err(),
            AuditError::ProductAssayMissing
        );
    }

    #[test]
    fn captured_raw_gas_is_not_automatically_product() {
        let product = valid_product();
        assert_eq!(product.accepted_product_kg().unwrap(), 0.7);
        assert!(product.accepted_product_kg().unwrap() < product.captured_raw_gas_kg);
    }

    #[test]
    fn rejects_product_larger_than_captured_stream() {
        let mut product = valid_product();
        product.purified_product_kg = 1.1;
        assert_eq!(
            product.accepted_product_kg().unwrap_err(),
            AuditError::ProductExceedsCapturedGas
        );
    }

    #[test]
    fn rejects_product_below_purity_contract() {
        let mut product = valid_product();
        product.measured_purity_fraction = 0.90;
        assert_eq!(
            product.accepted_product_kg().unwrap_err(),
            AuditError::ProductPurityBelowContract
        );
    }

    #[test]
    fn yield_reports_declared_source_boundary() {
        let (yield_kg_per_kwh, boundary) =
            accepted_product_yield(&valid_product(), &valid_energy()).unwrap();
        assert!((yield_kg_per_kwh - 0.07).abs() < 1.0e-12);
        assert_eq!(boundary, SourceEnergyBoundary::DcElectricalAtTransmitter);
    }

    #[test]
    fn corpus_has_unique_ids_and_valid_source_references() {
        let corpus: BenchmarkCorpus =
            serde_json::from_str(include_str!("../fixtures/cases-v0.1.json")).unwrap();
        assert!(validate_corpus(&corpus).is_ok());
    }

    fn valid_corpus() -> BenchmarkCorpus {
        serde_json::from_str(include_str!("../fixtures/cases-v0.1.json")).unwrap()
    }

    #[test]
    fn rejects_duplicate_source_ids() {
        let mut corpus = valid_corpus();
        corpus.source_records.push(corpus.source_records[0].clone());
        assert!(matches!(
            validate_corpus(&corpus),
            Err(AuditError::DuplicateSourceId(_))
        ));
    }

    #[test]
    fn rejects_duplicate_case_ids() {
        let mut corpus = valid_corpus();
        corpus.cases.push(corpus.cases[0].clone());
        assert!(matches!(
            validate_corpus(&corpus),
            Err(AuditError::DuplicateCaseId(_))
        ));
    }

    #[test]
    fn rejects_dangling_source_reference() {
        let mut corpus = valid_corpus();
        corpus.cases[0].source_id = Some("unknown-source".to_string());
        assert_eq!(
            validate_corpus(&corpus).unwrap_err(),
            AuditError::UnknownSource("unknown-source".to_string())
        );
    }

    #[test]
    fn rejects_missing_claim_ceiling() {
        let mut corpus = valid_corpus();
        corpus.claim_ceiling.clear();
        assert_eq!(
            validate_corpus(&corpus).unwrap_err(),
            AuditError::MissingClaimCeiling
        );
    }
}
