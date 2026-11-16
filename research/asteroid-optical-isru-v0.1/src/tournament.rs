//! Evidence-bounded matched-campaign comparison for optical ISRU candidates.
//!
//! This module validates campaign identity, independently declared product
//! acceptance fields, energy meter-plane compatibility and uncertainty
//! intervals. It does not calculate physical process performance, validate an
//! assay, simulate thermal/material response, or authorize an operation.

use crate::MaterialBalance;
use serde::Deserialize;

const INTERVAL_EPSILON: f64 = 1.0e-12;
const SECONDS_PER_DAY: f64 = 86_400.0;
const SECONDS_PER_YEAR: f64 = 31_557_600.0; // Julian year (365.25 days)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparabilityTrack {
    ProcessIsolation,
    WholeSystem,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CampaignIdentity {
    pub campaign_id: String,
    pub comparability_track: ComparabilityTrack,
    pub target_body: String,
    pub site_frame_epoch_id: String,
    pub feedstock_batch_sha256: String,
    pub environment_sha256: String,
    pub product_contract_sha256: String,
    /// Shared metrology protocol; candidate-specific calibration artifacts
    /// remain attached to each run and are not treated as one common device.
    pub measurement_protocol_sha256: String,
    pub campaign_duration_seconds: u64,
}

impl CampaignIdentity {
    pub fn validate(&self) -> Result<(), TournamentError> {
        for (field, value) in [
            ("campaign_id", self.campaign_id.as_str()),
            ("target_body", self.target_body.as_str()),
            ("site_frame_epoch_id", self.site_frame_epoch_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(TournamentError::MissingIdentity(field));
            }
        }

        for (field, digest) in [
            ("feedstock_batch_sha256", self.feedstock_batch_sha256.as_str()),
            ("environment_sha256", self.environment_sha256.as_str()),
            ("product_contract_sha256", self.product_contract_sha256.as_str()),
            (
                "measurement_protocol_sha256",
                self.measurement_protocol_sha256.as_str(),
            ),
        ] {
            if !is_sha256_digest(digest) {
                return Err(TournamentError::InvalidDigest(field));
            }
        }

        if self.campaign_duration_seconds == 0 {
            return Err(TournamentError::InvalidDuration);
        }
        Ok(())
    }
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MetricInterval {
    pub lower: f64,
    pub upper: f64,
}

impl MetricInterval {
    pub fn validate_nonnegative(&self, field: &'static str) -> Result<(), TournamentError> {
        if !self.lower.is_finite() || !self.upper.is_finite() {
            return Err(TournamentError::NonFinite(field));
        }
        if self.lower < 0.0 || self.upper < self.lower {
            return Err(TournamentError::InvalidInterval(field));
        }
        Ok(())
    }

    pub fn validate_fraction(&self, field: &'static str) -> Result<(), TournamentError> {
        self.validate_nonnegative(field)?;
        if self.upper > 1.0 {
            return Err(TournamentError::InvalidFraction(field));
        }
        Ok(())
    }
}

fn interval_ratio(
    numerator: MetricInterval,
    denominator: MetricInterval,
    field: &'static str,
) -> Result<MetricInterval, TournamentError> {
    numerator.validate_nonnegative(field)?;
    denominator.validate_nonnegative(field)?;
    if denominator.lower <= 0.0 {
        return Err(TournamentError::NonPositiveDenominator(field));
    }

    let result = MetricInterval {
        lower: numerator.lower / denominator.upper,
        upper: numerator.upper / denominator.lower,
    };
    result.validate_nonnegative(field)?;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductStreamKind {
    Gas,
    Solid,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProductAcceptanceEvidence {
    pub stream_kind: ProductStreamKind,
    pub captured_raw_mass_kg: MetricInterval,
    pub accepted_product_mass_kg: MetricInterval,
    pub measured_purity_fraction: MetricInterval,
    pub minimum_purity_fraction: f64,
    pub independent_assay_id: Option<String>,
    pub independent_assay_issuer_id: Option<String>,
    pub independent_assay_record_sha256: Option<String>,
}

impl ProductAcceptanceEvidence {
    pub fn validate(&self) -> Result<(), TournamentError> {
        self.captured_raw_mass_kg
            .validate_nonnegative("captured_raw_mass_kg")?;
        self.accepted_product_mass_kg
            .validate_nonnegative("accepted_product_mass_kg")?;
        self.measured_purity_fraction
            .validate_fraction("measured_purity_fraction")?;

        if !self.minimum_purity_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_purity_fraction)
        {
            return Err(TournamentError::InvalidFraction(
                "minimum_purity_fraction",
            ));
        }
        if self.accepted_product_mass_kg.upper
            > self.captured_raw_mass_kg.lower + INTERVAL_EPSILON
        {
            return Err(TournamentError::ProductExceedsCapturedStream);
        }
        if self.accepted_product_mass_kg.upper <= 0.0 {
            return Err(TournamentError::NoAcceptedProduct);
        }
        if self.measured_purity_fraction.lower + INTERVAL_EPSILON
            < self.minimum_purity_fraction
        {
            return Err(TournamentError::PurityBelowContract);
        }
        if !self
            .independent_assay_id
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty())
            || !self
                .independent_assay_issuer_id
                .as_deref()
                .is_some_and(|id| !id.trim().is_empty())
        {
            return Err(TournamentError::IndependentAssayMissing);
        }
        match self.independent_assay_record_sha256.as_deref() {
            Some(digest) if is_sha256_digest(digest) => Ok(()),
            _ => Err(TournamentError::IndependentAssayRecordMissing),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnergyForm {
    SolarRadiant,
    Electrical,
    Thermal,
    Chemical,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EnergyEvidence {
    pub boundary_id: String,
    pub energy_form: EnergyForm,
    pub input_kwh: MetricInterval,
}

impl EnergyEvidence {
    pub fn validate(&self) -> Result<(), TournamentError> {
        if self.boundary_id.trim().is_empty() {
            return Err(TournamentError::MissingIdentity(
                "source_energy_boundary_id",
            ));
        }
        self.input_kwh.validate_nonnegative("source_input_kwh")?;
        if self.input_kwh.lower <= 0.0 {
            return Err(TournamentError::NonPositiveDenominator(
                "source_input_kwh",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CandidateRun {
    pub candidate_id: String,
    pub campaign: CampaignIdentity,
    /// Candidate-specific article identity. Whole-system comparisons permit
    /// different articles and architectures.
    pub process_article_id: String,
    pub process_article_revision: String,
    /// Process-isolation comparisons require the same declared test fixture.
    /// Whole-system campaigns may leave this absent.
    pub shared_test_fixture_id: Option<String>,
    /// Digest of this candidate's full configuration, not a shared campaign
    /// identity. Different architectures are expected to differ.
    pub architecture_config_sha256: String,
    pub operator_entity_id: String,
    /// Candidate-specific calibration evidence identity and declared status.
    /// Authenticity/currentness must be independently resolved by the evidence
    /// authority; these fields alone do not authenticate a calibration.
    pub calibration_record_sha256: String,
    pub calibration_status: CalibrationStatus,
    pub product: ProductAcceptanceEvidence,
    /// The existing resource-to-product mass ledger is reused and independently
    /// rechecked here before any captured stream receives product credit.
    pub material_balance: MaterialBalance,
    pub mass_balance_tolerance_kg: f64,
    pub source_energy: Option<EnergyEvidence>,
    /// Total deployed plus declared replacement/spares mass within the frozen
    /// lifecycle boundary. Unknown mass must not be represented as zero.
    pub lifecycle_system_mass_kg: MetricInterval,
    pub availability_fraction: MetricInterval,
}

impl CandidateRun {
    pub fn validate(&self) -> Result<(), TournamentError> {
        if self.candidate_id.trim().is_empty() {
            return Err(TournamentError::MissingIdentity("candidate_id"));
        }
        if self.process_article_id.trim().is_empty() {
            return Err(TournamentError::MissingIdentity("process_article_id"));
        }
        if self.process_article_revision.trim().is_empty() {
            return Err(TournamentError::MissingIdentity("process_article_revision"));
        }
        if !is_sha256_digest(&self.architecture_config_sha256) {
            return Err(TournamentError::InvalidDigest("architecture_config_sha256"));
        }
        if self.operator_entity_id.trim().is_empty() {
            return Err(TournamentError::MissingIdentity("operator_entity_id"));
        }
        if !is_sha256_digest(&self.calibration_record_sha256) {
            return Err(TournamentError::InvalidDigest("calibration_record_sha256"));
        }
        if self.calibration_status != CalibrationStatus::Current {
            return Err(TournamentError::CalibrationNotCurrent);
        }
        if self
            .product
            .independent_assay_issuer_id
            .as_deref()
            .is_some_and(|issuer| issuer == self.operator_entity_id)
        {
            return Err(TournamentError::AssayIssuerNotIndependent);
        }
        if self.campaign.comparability_track == ComparabilityTrack::ProcessIsolation {
            match &self.shared_test_fixture_id {
                Some(id) if !id.trim().is_empty() => {}
                _ => return Err(TournamentError::MissingIdentity("shared_test_fixture_id")),
            }
        }
        self.campaign.validate()?;
        self.product.validate()?;
        self.material_balance
            .validate(self.mass_balance_tolerance_kg)
            .map_err(|_| TournamentError::MaterialBalanceInvalid)?;
        let captured_stream_mass_kg = match self.product.stream_kind {
            ProductStreamKind::Gas => self.material_balance.captured_raw_gas_kg,
            ProductStreamKind::Solid => self.material_balance.captured_solids_kg,
        };
        if self.product.captured_raw_mass_kg.upper
            > captured_stream_mass_kg + INTERVAL_EPSILON
        {
            return Err(TournamentError::CapturedStreamExceedsMaterialBalance);
        }
        self.lifecycle_system_mass_kg
            .validate_nonnegative("lifecycle_system_mass_kg")?;
        if self.lifecycle_system_mass_kg.lower <= 0.0 {
            return Err(TournamentError::NonPositiveDenominator(
                "lifecycle_system_mass_kg",
            ));
        }
        self.availability_fraction
            .validate_fraction("availability_fraction")?;
        if let Some(energy) = &self.source_energy {
            energy.validate()?;
        }
        Ok(())
    }

    pub fn metrics(&self) -> Result<CandidateMetrics, TournamentError> {
        self.validate()?;

        let duration_seconds = self.campaign.campaign_duration_seconds as f64;
        let duration_days = duration_seconds / SECONDS_PER_DAY;
        let duration_years = duration_seconds / SECONDS_PER_YEAR;
        if !duration_days.is_finite()
            || !duration_years.is_finite()
            || duration_days <= 0.0
            || duration_years <= 0.0
        {
            return Err(TournamentError::InvalidDuration);
        }

        let product = self.product.accepted_product_mass_kg;
        let accepted_kg_per_day = interval_ratio(
            product,
            MetricInterval {
                lower: duration_days,
                upper: duration_days,
            },
            "accepted_kg_per_day",
        )?;

        let system_mass_for_campaign = MetricInterval {
            lower: self.lifecycle_system_mass_kg.lower * duration_years,
            upper: self.lifecycle_system_mass_kg.upper * duration_years,
        };
        let accepted_kg_per_system_kg_year = interval_ratio(
            product,
            system_mass_for_campaign,
            "accepted_kg_per_system_kg_year",
        )?;

        let accepted_kg_per_source_kwh = self
            .source_energy
            .as_ref()
            .map(|energy| {
                interval_ratio(
                    product,
                    energy.input_kwh,
                    "accepted_kg_per_source_kwh",
                )
            })
            .transpose()?;

        Ok(CandidateMetrics {
            accepted_kg_per_day,
            accepted_kg_per_source_kwh,
            accepted_kg_per_system_kg_year,
            availability_fraction: self.availability_fraction,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateMetrics {
    pub accepted_kg_per_day: MetricInterval,
    pub accepted_kg_per_source_kwh: Option<MetricInterval>,
    pub accepted_kg_per_system_kg_year: MetricInterval,
    pub availability_fraction: MetricInterval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairComparison {
    A_Robustly_Dominates,
    B_Robustly_Dominates,
    Incomparable,
    NotComparable(&'static str),
}

fn robustly_dominates(a: &CandidateMetrics, b: &CandidateMetrics) -> bool {
    let Some(a_energy) = a.accepted_kg_per_source_kwh else {
        return false;
    };
    let Some(b_energy) = b.accepted_kg_per_source_kwh else {
        return false;
    };

    let objectives = [
        (a.accepted_kg_per_day, b.accepted_kg_per_day),
        (a_energy, b_energy),
        (
            a.accepted_kg_per_system_kg_year,
            b.accepted_kg_per_system_kg_year,
        ),
        (a.availability_fraction, b.availability_fraction),
    ];

    let no_worse_on_all = objectives.iter().all(|(left, right)| {
        left.lower + INTERVAL_EPSILON >= right.upper
    });
    let strictly_better_on_one = objectives.iter().any(|(left, right)| {
        left.lower > right.upper + INTERVAL_EPSILON
    });

    no_worse_on_all && strictly_better_on_one
}

/// Compare two candidates only if campaign, accepted product stream and source
/// energy meter boundary/form match. Any mismatch remains explicitly
/// NOT_COMPARABLE; it is not silently dropped from the objective vector.
pub fn compare_pair(
    a: &CandidateRun,
    b: &CandidateRun,
) -> Result<PairComparison, TournamentError> {
    a.validate()?;
    b.validate()?;

    if a.candidate_id == b.candidate_id {
        return Err(TournamentError::DuplicateCandidateId);
    }
    if a.campaign != b.campaign {
        return Ok(PairComparison::NotComparable("campaign identity differs"));
    }
    if a.campaign.comparability_track == ComparabilityTrack::ProcessIsolation
        && a.shared_test_fixture_id != b.shared_test_fixture_id
    {
        return Ok(PairComparison::NotComparable(
            "process-isolation test fixture differs",
        ));
    }
    if a.product.stream_kind != b.product.stream_kind {
        return Ok(PairComparison::NotComparable(
            "product stream kind differs",
        ));
    }

    let (Some(a_energy), Some(b_energy)) = (&a.source_energy, &b.source_energy) else {
        return Ok(PairComparison::NotComparable(
            "source energy evidence is missing",
        ));
    };
    if a_energy.boundary_id != b_energy.boundary_id
        || a_energy.energy_form != b_energy.energy_form
    {
        return Ok(PairComparison::NotComparable(
            "source energy boundary or form differs",
        ));
    }

    let a_metrics = a.metrics()?;
    let b_metrics = b.metrics()?;
    if robustly_dominates(&a_metrics, &b_metrics) {
        Ok(PairComparison::A_Robustly_Dominates)
    } else if robustly_dominates(&b_metrics, &a_metrics) {
        Ok(PairComparison::B_Robustly_Dominates)
    } else {
        Ok(PairComparison::Incomparable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationStatus {
    Current,
    Stale,
    Revoked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TournamentError {
    MissingIdentity(&'static str),
    InvalidDigest(&'static str),
    InvalidDuration,
    NonFinite(&'static str),
    InvalidInterval(&'static str),
    InvalidFraction(&'static str),
    NonPositiveDenominator(&'static str),
    ProductExceedsCapturedStream,
    CapturedStreamExceedsMaterialBalance,
    MaterialBalanceInvalid,
    NoAcceptedProduct,
    PurityBelowContract,
    IndependentAssayMissing,
    IndependentAssayRecordMissing,
    AssayIssuerNotIndependent,
    CalibrationNotCurrent,
    DuplicateCandidateId,
}

impl std::fmt::Display for TournamentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingIdentity(field) => write!(f, "{field} is required"),
            Self::InvalidDigest(field) => write!(f, "{field} must be a 64-character hex digest"),
            Self::InvalidDuration => write!(f, "campaign duration must be positive and finite"),
            Self::NonFinite(field) => write!(f, "{field} must be finite"),
            Self::InvalidInterval(field) => {
                write!(f, "{field} must satisfy 0 <= lower <= upper")
            }
            Self::InvalidFraction(field) => {
                write!(f, "{field} must be within the closed interval [0, 1]")
            }
            Self::NonPositiveDenominator(field) => {
                write!(f, "{field} lower bound must be greater than zero")
            }
            Self::ProductExceedsCapturedStream => {
                write!(f, "accepted product upper bound exceeds captured raw stream lower bound")
            }
            Self::CapturedStreamExceedsMaterialBalance => {
                write!(f, "captured product stream exceeds the corresponding closed mass-balance stream")
            }
            Self::MaterialBalanceInvalid => {
                write!(f, "material balance does not close within its declared tolerance")
            }
            Self::NoAcceptedProduct => write!(f, "accepted product must have positive upper bound"),
            Self::PurityBelowContract => {
                write!(f, "conservative measured purity is below the product contract")
            }
            Self::IndependentAssayMissing => {
                write!(f, "an independent product-assay identity and issuer are required")
            }
            Self::IndependentAssayRecordMissing => {
                write!(f, "a valid product-assay record digest is required")
            }
            Self::AssayIssuerNotIndependent => {
                write!(f, "assay issuer must differ from the candidate operator")
            }
            Self::CalibrationNotCurrent => {
                write!(f, "candidate calibration status is not current")
            }
            Self::DuplicateCandidateId => write!(f, "candidate identifiers must be unique"),
        }
    }
}

impl std::error::Error for TournamentError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        ch.to_string().repeat(64)
    }

    fn interval(lower: f64, upper: f64) -> MetricInterval {
        MetricInterval { lower, upper }
    }

    fn campaign() -> CampaignIdentity {
        CampaignIdentity {
            campaign_id: "synthetic-campaign-001".to_string(),
            comparability_track: ComparabilityTrack::WholeSystem,
            target_body: "Moon".to_string(),
            site_frame_epoch_id: "synthetic-site-frame-epoch".to_string(),
            feedstock_batch_sha256: digest('a'),
            environment_sha256: digest('b'),
            product_contract_sha256: digest('c'),
            measurement_protocol_sha256: digest('d'),
            campaign_duration_seconds: 86_400,
        }
    }

    fn run(
        id: &str,
        product_mass: MetricInterval,
        energy_kwh: MetricInterval,
        system_mass: MetricInterval,
        availability: MetricInterval,
    ) -> CandidateRun {
        CandidateRun {
            candidate_id: id.to_string(),
            campaign: campaign(),
            process_article_id: format!("article-{id}"),
            process_article_revision: "rev-1".to_string(),
            shared_test_fixture_id: Some("shared-rig-001".to_string()),
            architecture_config_sha256: digest('e'),
            operator_entity_id: format!("operator-{id}"),
            calibration_record_sha256: digest('f'),
            calibration_status: CalibrationStatus::Current,
            product: ProductAcceptanceEvidence {
                stream_kind: ProductStreamKind::Solid,
                captured_raw_mass_kg: interval(2.2, 2.5),
                accepted_product_mass_kg: product_mass,
                measured_purity_fraction: interval(0.995, 0.999),
                minimum_purity_fraction: 0.99,
                independent_assay_id: Some(format!("assay-{id}")),
                independent_assay_issuer_id: Some("independent-lab-001".to_string()),
                independent_assay_record_sha256: Some(digest('a')),
            },
            material_balance: MaterialBalance {
                feedstock_input_kg: 10.0,
                solid_residue_kg: 5.5,
                captured_solids_kg: 2.5,
                escaped_solids_kg: 0.5,
                captured_raw_gas_kg: 1.0,
                escaped_gas_kg: 0.4,
                other_accounted_mass_kg: 0.1,
            },
            mass_balance_tolerance_kg: 1.0e-9,
            source_energy: Some(EnergyEvidence {
                boundary_id: "solar-incident-at-primary-collector".to_string(),
                energy_form: EnergyForm::SolarRadiant,
                input_kwh: energy_kwh,
            }),
            lifecycle_system_mass_kg: system_mass,
            availability_fraction: availability,
        }
    }

    fn strong_candidate() -> CandidateRun {
        run(
            "candidate-a",
            interval(2.0, 2.1),
            interval(10.0, 11.0),
            interval(100.0, 110.0),
            interval(0.98, 0.99),
        )
    }

    fn weak_candidate() -> CandidateRun {
        run(
            "candidate-b",
            interval(1.0, 1.1),
            interval(12.0, 13.0),
            interval(120.0, 130.0),
            interval(0.90, 0.95),
        )
    }

    #[test]
    fn valid_candidate_passes_contract_validation() {
        assert!(strong_candidate().validate().is_ok());
    }

    #[test]
    fn conservative_intervals_produce_metric_vector() {
        let metrics = strong_candidate().metrics().unwrap();
        assert!((metrics.accepted_kg_per_day.lower - 2.0).abs() < 1.0e-12);
        assert!(metrics.accepted_kg_per_source_kwh.is_some());
        assert!(metrics.accepted_kg_per_system_kg_year.lower > 0.0);
    }

    #[test]
    fn robustly_separated_candidate_dominates_all_objectives() {
        assert_eq!(
            compare_pair(&strong_candidate(), &weak_candidate()).unwrap(),
            PairComparison::A_Robustly_Dominates
        );
    }

    #[test]
    fn overlapping_uncertainty_intervals_remain_incomparable() {
        let a = run(
            "candidate-a",
            interval(1.0, 2.0),
            interval(10.0, 20.0),
            interval(100.0, 200.0),
            interval(0.90, 0.99),
        );
        let b = run(
            "candidate-b",
            interval(1.0, 2.0),
            interval(10.0, 20.0),
            interval(100.0, 200.0),
            interval(0.90, 0.99),
        );
        assert_eq!(
            compare_pair(&a, &b).unwrap(),
            PairComparison::Incomparable
        );
    }

    #[test]
    fn objective_tradeoff_remains_incomparable() {
        let mut high_product = strong_candidate();
        high_product.availability_fraction = interval(0.70, 0.80);
        let mut high_availability = weak_candidate();
        high_availability.availability_fraction = interval(0.95, 0.99);
        assert_eq!(
            compare_pair(&high_product, &high_availability).unwrap(),
            PairComparison::Incomparable
        );
    }

    #[test]
    fn campaign_mismatch_is_not_comparable() {
        let a = strong_candidate();
        let mut b = weak_candidate();
        b.campaign.feedstock_batch_sha256 = digest('e');
        assert_eq!(
            compare_pair(&a, &b).unwrap(),
            PairComparison::NotComparable("campaign identity differs")
        );
    }

    #[test]
    fn whole_system_candidates_may_use_different_process_articles() {
        let a = strong_candidate();
        let b = weak_candidate();
        assert_eq!(
            compare_pair(&a, &b).unwrap(),
            PairComparison::A_Robustly_Dominates
        );
    }

    #[test]
    fn process_isolation_requires_a_shared_test_fixture() {
        let mut a = strong_candidate();
        let mut b = weak_candidate();
        a.campaign.comparability_track = ComparabilityTrack::ProcessIsolation;
        b.campaign.comparability_track = ComparabilityTrack::ProcessIsolation;
        b.shared_test_fixture_id = Some("different-rig".to_string());
        assert_eq!(
            compare_pair(&a, &b).unwrap(),
            PairComparison::NotComparable("process-isolation test fixture differs")
        );
    }

    #[test]
    fn process_isolation_requires_fixture_identity() {
        let mut candidate = strong_candidate();
        candidate.campaign.comparability_track = ComparabilityTrack::ProcessIsolation;
        candidate.shared_test_fixture_id = None;
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::MissingIdentity("shared_test_fixture_id")
        );
    }

    #[test]
    fn source_energy_boundary_mismatch_is_not_comparable() {
        let a = strong_candidate();
        let mut b = weak_candidate();
        b.source_energy.as_mut().unwrap().boundary_id = "dc-bus-at-transmitter".to_string();
        b.source_energy.as_mut().unwrap().energy_form = EnergyForm::Electrical;
        assert_eq!(
            compare_pair(&a, &b).unwrap(),
            PairComparison::NotComparable("source energy boundary or form differs")
        );
    }

    #[test]
    fn missing_source_energy_is_not_comparable_not_zero() {
        let a = strong_candidate();
        let mut b = weak_candidate();
        b.source_energy = None;
        assert_eq!(
            compare_pair(&a, &b).unwrap(),
            PairComparison::NotComparable("source energy evidence is missing")
        );
        assert!(b.validate().is_ok());
    }

    #[test]
    fn gas_and_solid_products_are_not_comparable() {
        let a = strong_candidate();
        let mut b = weak_candidate();
        b.product.stream_kind = ProductStreamKind::Gas;
        b.product.captured_raw_mass_kg = interval(2.2, 2.5);
        b.material_balance.solid_residue_kg = 4.0;
        b.material_balance.captured_raw_gas_kg = 2.5;
        assert_eq!(
            compare_pair(&a, &b).unwrap(),
            PairComparison::NotComparable("product stream kind differs")
        );
    }

    #[test]
    fn missing_independent_assay_is_rejected() {
        let mut candidate = strong_candidate();
        candidate.product.independent_assay_id = None;
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::IndependentAssayMissing
        );
    }

    #[test]
    fn accepted_product_cannot_exceed_captured_stream() {
        let mut candidate = strong_candidate();
        candidate.product.accepted_product_mass_kg = interval(3.1, 3.2);
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::ProductExceedsCapturedStream
        );
    }

    #[test]
    fn purity_interval_must_clear_product_contract() {
        let mut candidate = strong_candidate();
        candidate.product.measured_purity_fraction = interval(0.97, 0.999);
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::PurityBelowContract
        );
    }

    #[test]
    fn non_finite_metric_interval_is_rejected() {
        let mut candidate = strong_candidate();
        candidate.lifecycle_system_mass_kg = interval(100.0, f64::NAN);
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::NonFinite("lifecycle_system_mass_kg")
        );
    }

    #[test]
    fn zero_energy_lower_bound_is_rejected() {
        let mut candidate = strong_candidate();
        candidate.source_energy.as_mut().unwrap().input_kwh = interval(0.0, 10.0);
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::NonPositiveDenominator("source_input_kwh")
        );
    }

    #[test]
    fn zero_lifecycle_mass_lower_bound_is_rejected() {
        let mut candidate = strong_candidate();
        candidate.lifecycle_system_mass_kg = interval(0.0, 110.0);
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::NonPositiveDenominator("lifecycle_system_mass_kg")
        );
    }

    #[test]
    fn availability_above_one_is_rejected() {
        let mut candidate = strong_candidate();
        candidate.availability_fraction = interval(0.98, 1.01);
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::InvalidFraction("availability_fraction")
        );
    }

    #[test]
    fn captured_stream_cannot_exceed_closed_mass_balance_stream() {
        let mut candidate = strong_candidate();
        candidate.product.captured_raw_mass_kg = interval(2.6, 2.7);
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::CapturedStreamExceedsMaterialBalance
        );
    }

    #[test]
    fn unclosed_material_balance_blocks_product_acceptance() {
        let mut candidate = strong_candidate();
        candidate.material_balance.escaped_solids_kg = 0.0;
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::MaterialBalanceInvalid
        );
    }

    #[test]
    fn assay_issuer_must_differ_from_candidate_operator() {
        let mut candidate = strong_candidate();
        candidate.product.independent_assay_issuer_id =
            Some(candidate.operator_entity_id.clone());
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::AssayIssuerNotIndependent
        );
    }

    #[test]
    fn assay_record_digest_is_required() {
        let mut candidate = strong_candidate();
        candidate.product.independent_assay_record_sha256 = None;
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::IndependentAssayRecordMissing
        );
    }

    #[test]
    fn stale_calibration_blocks_candidate() {
        let mut candidate = strong_candidate();
        candidate.calibration_status = CalibrationStatus::Stale;
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::CalibrationNotCurrent
        );
    }

    #[test]
    fn invalid_calibration_digest_is_rejected() {
        let mut candidate = strong_candidate();
        candidate.calibration_record_sha256 = "stale-calibration".to_string();
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::InvalidDigest("calibration_record_sha256")
        );
    }

    #[test]
    fn invalid_digest_is_rejected() {
        let mut candidate = strong_candidate();
        candidate.campaign.feedstock_batch_sha256 = "not-a-digest".to_string();
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::InvalidDigest("feedstock_batch_sha256")
        );
    }

    #[test]
    fn empty_campaign_duration_is_rejected() {
        let mut candidate = strong_candidate();
        candidate.campaign.campaign_duration_seconds = 0;
        assert_eq!(
            candidate.validate().unwrap_err(),
            TournamentError::InvalidDuration
        );
    }

    #[test]
    fn duplicate_candidate_identity_is_rejected() {
        let a = strong_candidate();
        let mut b = weak_candidate();
        b.candidate_id = a.candidate_id.clone();
        assert_eq!(
            compare_pair(&a, &b).unwrap_err(),
            TournamentError::DuplicateCandidateId
        );
    }
}
