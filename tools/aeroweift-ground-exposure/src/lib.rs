//! Evidence-qualified, time-indexed ground-exposure aggregation for AeroWeft.
//!
//! The evaluator consumes source-qualified population, impact-distribution and
//! consequence-model packages. It does not construct maps or estimate physical
//! distributions. It rejects missing spatial coverage, binds every input to one
//! grid/CRS/time/model scope, and uses conservative bounds:
//! - expected exposed persons are summed from supplied joint-exposure bounds;
//! - probability of at least one person impacted uses max(cell lower bounds) and
//!   the union bound min(1, sum(cell upper bounds) + residual impact mass).
//!
//! No cell-independence assumption or multiplication of unrelated mean values is made.
//! A positive output is only admissibility for declared exposure review.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// A half-open analysis time interval on the declared time basis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeWindow {
    /// Inclusive start time.
    pub start: i64,
    /// Exclusive end time.
    pub end: i64,
}

impl TimeWindow {
    fn is_valid(self) -> bool {
        self.start >= 0 && self.start < self.end
    }
}

/// Spatial region represented by one source-owned geometry artifact.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GroundZone {
    /// Footprint of the authorized operational volume.
    OperationalVolume,
    /// Footprint of the off-nominal contingency volume.
    ContingencyVolume,
    /// Ground-risk buffer surrounding the contingency-volume footprint.
    GroundRiskBuffer,
    /// Adjacent area included in the assessment profile.
    AdjacentArea,
}

/// Exact geometry definition for one ground zone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZoneGeometry {
    /// Zone type.
    pub zone: GroundZone,
    /// Exact geometry artifact digest; bytes are not authenticated here.
    pub geometry_sha256: String,
    /// Cell IDs intersecting this zone under the frozen rasterization policy.
    pub cell_ids: Vec<String>,
}

/// Model and data identity shared by one flight/contingency exposure assessment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExposureScope {
    /// Pseudonymous aircraft identifier.
    pub aircraft_id: String,
    /// Individual flight identifier.
    pub flight_id: String,
    /// Contingency-case identifier from Track W.
    pub contingency_id: String,
    /// Aircraft/model artifact digest.
    pub aircraft_model_sha256: String,
    /// Track T model-validation package digest.
    pub validation_package_sha256: String,
    /// Ground population dataset digest.
    pub population_dataset_sha256: String,
    /// Impact-location distribution model digest.
    pub impact_model_sha256: String,
    /// Consequence/severity model digest.
    pub severity_model_sha256: String,
    /// Exact cell-grid/rasterization artifact digest.
    pub cell_grid_sha256: String,
    /// Exact coordinate reference system definition digest.
    pub crs_sha256: String,
    /// Time-scale identity; this crate does not convert clocks.
    pub time_scale_id: String,
    /// Explicit population classification semantics (e.g. resident, worker, visitor).
    pub population_definition_sha256: String,
}

impl ExposureScope {
    fn is_complete(&self) -> bool {
        !self.aircraft_id.trim().is_empty()
            && !self.flight_id.trim().is_empty()
            && !self.contingency_id.trim().is_empty()
            && is_sha256(&self.aircraft_model_sha256)
            && is_sha256(&self.validation_package_sha256)
            && is_sha256(&self.population_dataset_sha256)
            && is_sha256(&self.impact_model_sha256)
            && is_sha256(&self.severity_model_sha256)
            && is_sha256(&self.cell_grid_sha256)
            && is_sha256(&self.crs_sha256)
            && !self.time_scale_id.trim().is_empty()
            && is_sha256(&self.population_definition_sha256)
    }
}

/// Applicability of the exact model/domain stack to the assessed contingency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Applicability {
    /// Track T/U owners admit the exact model/domain for this use.
    InDomain,
    /// The case is outside the declared model/domain envelope.
    OutOfDomain,
    /// Model/domain applicability is unresolved.
    Unknown,
}

/// Owner-provided qualification state of a source package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceState {
    /// Source package has been qualified by its responsible owner.
    Qualified,
    /// Source package was rejected, revoked or invalidated by its owner.
    Rejected,
    /// Source qualification is absent or unresolved.
    Unknown,
}

/// Shared bitemporal and spatial identity of one owner-produced evidence package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceHeader {
    /// Source/issuer namespace.
    pub source_id: String,
    /// Exact manifest digest string.
    pub manifest_sha256: String,
    /// Cell-grid digest used by this package.
    pub cell_grid_sha256: String,
    /// Coordinate reference system digest used by this package.
    pub crs_sha256: String,
    /// Earliest effective time covered by this package.
    pub effective_from: i64,
    /// Exclusive end of effective-time coverage.
    pub effective_until: i64,
    /// Time this package was recorded/available in the evidence system.
    pub recorded_at: i64,
    /// Owner's evidence qualification state.
    pub evidence_state: EvidenceState,
    /// Cells explicitly covered by this package.
    pub covered_cell_ids: Vec<String>,
}

/// Population-density/occupancy evidence for one declared time and spatial scope.
#[derive(Clone, Debug, PartialEq)]
pub struct PopulationSnapshot {
    /// Shared source header.
    pub header: EvidenceHeader,
    /// Exact population dataset digest.
    pub dataset_sha256: String,
    /// Population definition digest (who/what counts as an occupant).
    pub population_definition_sha256: String,
    /// Maximum cell area represented by this snapshot, square metres.
    pub maximum_cell_area_m2: f64,
    /// Whether source coverage is complete across its declared covered cells.
    pub coverage_complete: bool,
}

/// Impact-location probability distribution over the frozen analysis grid.
#[derive(Clone, Debug, PartialEq)]
pub struct ImpactDistribution {
    /// Shared source header.
    pub header: EvidenceHeader,
    /// Exact impact model digest.
    pub model_sha256: String,
    /// Exact contingency/scenario model digest.
    pub scenario_sha256: String,
    /// Lower bound on probability mass outside all supplied assessment cells.
    pub residual_mass_lower: f64,
    /// Upper bound on probability mass outside all supplied assessment cells.
    pub residual_mass_upper: f64,
    /// Whether the impact model's spatial coverage is complete for its declared support.
    pub coverage_complete: bool,
}

/// Consequence/severity model evidence for the exact aircraft and use.
#[derive(Clone, Debug, PartialEq)]
pub struct SeverityModel {
    /// Shared source header.
    pub header: EvidenceHeader,
    /// Exact severity/consequence model digest.
    pub model_sha256: String,
    /// Exact aircraft configuration/model digest to which it applies.
    pub aircraft_model_sha256: String,
    /// Whether the consequence model covers the full declared assessment window/domain.
    pub coverage_complete: bool,
}

/// Bounds supplied by an independent cell-level joint exposure evaluator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellExposureBounds {
    /// Lower bound on the number of people occupying this cell in the analysis window.
    pub population_lower: f64,
    /// Upper bound on the number of people occupying this cell in the analysis window.
    pub population_upper: f64,
    /// Lower bound on impact-location probability mass in this cell.
    pub impact_mass_lower: f64,
    /// Upper bound on impact-location probability mass in this cell.
    pub impact_mass_upper: f64,
    /// Lower bound on probability that at least one person in this cell is impacted.
    pub person_impact_probability_lower: f64,
    /// Upper bound on probability that at least one person in this cell is impacted.
    pub person_impact_probability_upper: f64,
    /// Lower bound on expected persons exposed in this cell, from a joint model.
    pub joint_exposed_persons_lower: f64,
    /// Upper bound on expected persons exposed in this cell, from a joint model.
    pub joint_exposed_persons_upper: f64,
    /// Upper bound on consequence units per impacted person in this cell.
    pub consequence_units_per_person_upper: f64,
}

/// One spatial-cell evidence record.
#[derive(Clone, Debug, PartialEq)]
pub struct CellExposureEvidence {
    /// Stable cell identifier in the exact grid manifest.
    pub cell_id: String,
    /// Cell area, square metres.
    pub cell_area_m2: f64,
    /// Bounds produced by the independent joint exposure evaluator.
    pub bounds: CellExposureBounds,
}

/// Frozen exposure assessment request and required spatial coverage.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundExposureCase {
    /// Stable assessment identity.
    pub assessment_id: String,
    /// Exact aircraft/flight/contingency/model/data scope.
    pub scope: ExposureScope,
    /// Model/domain applicability from Track T/U.
    pub applicability: Applicability,
    /// Effective-time interval being assessed.
    pub analysis_window: TimeWindow,
    /// Knowledge-time cutoff: only source packages recorded by this time may be used.
    pub known_at: i64,
    /// Required operational/contingency/buffer/adjacent zone geometries.
    pub required_zones: Vec<ZoneGeometry>,
}

/// Frozen aggregation and coverage policy.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundExposurePolicy {
    /// Exact policy manifest digest.
    pub policy_sha256: String,
    /// Maximum permitted cell area, square metres.
    pub maximum_cell_area_m2: f64,
    /// Maximum permissible unrepresented impact-location mass.
    pub maximum_residual_impact_mass: f64,
    /// Whether all four named ground zones are mandatory.
    pub require_all_ground_zones: bool,
}

/// Overall result status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExposureStatus {
    /// Inputs satisfy the frozen structural and conservative aggregation policy.
    QualifiedForDeclaredExposureReview,
    /// Spatial/time/source/model evidence is incomplete or unresolved.
    Indeterminate,
    /// The requested aircraft/model/domain is outside its qualified envelope.
    OutOfDomain,
}

/// Typed reason that the input package is structurally malformed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExposureError {
    /// Policy is incomplete or invalid.
    InvalidPolicy,
    /// Case identity, time window or scope is invalid.
    InvalidCase,
    /// Zone identity, geometry or cell list is invalid.
    InvalidZoneGeometry,
    /// Duplicate zone role.
    DuplicateZone,
    /// Duplicate cell ID in the evidence package.
    DuplicateCell,
    /// A cell is not part of the required geometry union.
    UnexpectedCell,
    /// A required cell is absent from cell evidence.
    MissingCell,
    /// Source header, digest or time metadata is malformed.
    InvalidEvidenceHeader,
    /// Source manifest is bound to a different model, dataset, grid or CRS.
    EvidenceScopeMismatch,
    /// Numeric bounds are non-finite, out of range or internally inconsistent.
    InvalidCellBounds,
    /// Impact-location distribution mass bounds cannot contain total probability one.
    InvalidImpactMass,
}

/// Conservative aggregate exposure and consequence bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExposureBounds {
    /// Lower bound on expected exposed persons over the declared space/time scope.
    pub expected_exposed_persons_lower: f64,
    /// Upper bound on expected exposed persons over the declared space/time scope.
    pub expected_exposed_persons_upper: f64,
    /// Lower bound on probability that at least one person is impacted.
    pub probability_at_least_one_impacted_lower: f64,
    /// Conservative upper bound using the union bound plus unrepresented impact mass.
    pub probability_at_least_one_impacted_upper: f64,
    /// Upper bound on aggregated consequence units under the supplied per-person bounds.
    pub expected_consequence_units_upper: f64,
    /// Total population lower bound over unique cells.
    pub population_lower: f64,
    /// Total population upper bound over unique cells.
    pub population_upper: f64,
    /// Lower bound on total impact-location mass represented by cells plus residual.
    pub total_impact_mass_lower: f64,
    /// Upper bound on total impact-location mass represented by cells plus residual.
    pub total_impact_mass_upper: f64,
}

/// Complete replay identity for an exposure assessment.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundExposureResult {
    /// Assessment identity.
    pub assessment_id: String,
    /// Overall qualification status.
    pub status: ExposureStatus,
    /// Exact effective-time window assessed.
    pub analysis_window: TimeWindow,
    /// Exact knowledge-time cutoff.
    pub known_at: i64,
    /// Number of unique spatial cells included.
    pub cell_count: usize,
    /// Number of cells represented by each required zone, in canonical order.
    pub zone_cell_counts: Vec<(GroundZone, usize)>,
    /// Conservative aggregated bounds, preserved even for indeterminate results.
    pub bounds: ExposureBounds,
    /// Exact policy digest.
    pub policy_sha256: String,
    /// Exact source manifest digests used.
    pub source_manifest_sha256: [String; 3],
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn validate_policy(policy: &GroundExposurePolicy) -> Result<(), ExposureError> {
    if !is_sha256(&policy.policy_sha256)
        || !valid_nonnegative(policy.maximum_cell_area_m2)
        || policy.maximum_cell_area_m2 <= 0.0
        || !valid_probability(policy.maximum_residual_impact_mass)
    {
        return Err(ExposureError::InvalidPolicy);
    }
    Ok(())
}

fn validate_case(case: &GroundExposureCase) -> Result<(), ExposureError> {
    if case.assessment_id.trim().is_empty()
        || !case.scope.is_complete()
        || !case.analysis_window.is_valid()
        || case.known_at < 0
        || case.required_zones.is_empty()
    {
        return Err(ExposureError::InvalidCase);
    }
    Ok(())
}

fn validate_zone_geometries(
    zones: &[ZoneGeometry],
    require_all: bool,
) -> Result<(BTreeSet<String>, Vec<(GroundZone, usize)>), ExposureError> {
    let mut roles = BTreeSet::new();
    let mut expected_cells = BTreeSet::new();
    let mut counts = Vec::new();

    for zone in zones {
        if !is_sha256(&zone.geometry_sha256)
            || zone.cell_ids.is_empty()
            || zone.cell_ids.iter().any(|cell| cell.trim().is_empty())
        {
            return Err(ExposureError::InvalidZoneGeometry);
        }
        if !roles.insert(zone.zone) {
            return Err(ExposureError::DuplicateZone);
        }
        let mut local = BTreeSet::new();
        for cell_id in &zone.cell_ids {
            if !local.insert(cell_id.as_str()) {
                return Err(ExposureError::InvalidZoneGeometry);
            }
            expected_cells.insert(cell_id.clone());
        }
        counts.push((zone.zone, local.len()));
    }

    if require_all {
        let required = [
            GroundZone::OperationalVolume,
            GroundZone::ContingencyVolume,
            GroundZone::GroundRiskBuffer,
            GroundZone::AdjacentArea,
        ];
        if required.iter().any(|role| !roles.contains(role)) {
            return Err(ExposureError::InvalidZoneGeometry);
        }
    }
    counts.sort_by_key(|item| item.0);
    Ok((expected_cells, counts))
}

fn validate_header(
    header: &EvidenceHeader,
    scope: &ExposureScope,
    window: TimeWindow,
    known_at: i64,
) -> Result<bool, ExposureError> {
    if header.source_id.trim().is_empty()
        || !is_sha256(&header.manifest_sha256)
        || !is_sha256(&header.cell_grid_sha256)
        || !is_sha256(&header.crs_sha256)
        || header.effective_from < 0
        || header.effective_from > header.effective_until
        || header.recorded_at < 0
        || header.covered_cell_ids.is_empty()
    {
        return Err(ExposureError::InvalidEvidenceHeader);
    }
    if header.cell_grid_sha256 != scope.cell_grid_sha256
        || header.crs_sha256 != scope.crs_sha256
    {
        return Err(ExposureError::EvidenceScopeMismatch);
    }
    let mut ids = BTreeSet::new();
    if header.covered_cell_ids.iter().any(|id| id.trim().is_empty() || !ids.insert(id.as_str())) {
        return Err(ExposureError::InvalidEvidenceHeader);
    }
    Ok(header.evidence_state == EvidenceState::Qualified
        && header.effective_from <= window.start
        && header.effective_until >= window.end
        && header.recorded_at <= known_at)
}

fn validate_cell_bounds(bounds: CellExposureBounds) -> Result<(), ExposureError> {
    if !valid_nonnegative(bounds.population_lower)
        || !valid_nonnegative(bounds.population_upper)
        || bounds.population_lower > bounds.population_upper
        || !valid_probability(bounds.impact_mass_lower)
        || !valid_probability(bounds.impact_mass_upper)
        || bounds.impact_mass_lower > bounds.impact_mass_upper
        || !valid_probability(bounds.person_impact_probability_lower)
        || !valid_probability(bounds.person_impact_probability_upper)
        || bounds.person_impact_probability_lower > bounds.person_impact_probability_upper
        || !valid_nonnegative(bounds.joint_exposed_persons_lower)
        || !valid_nonnegative(bounds.joint_exposed_persons_upper)
        || bounds.joint_exposed_persons_lower > bounds.joint_exposed_persons_upper
        || bounds.joint_exposed_persons_upper > bounds.population_upper
        || bounds.joint_exposed_persons_lower
            < bounds.person_impact_probability_lower
        || bounds.joint_exposed_persons_upper
            < bounds.person_impact_probability_upper
        || !valid_nonnegative(bounds.consequence_units_per_person_upper)
        || bounds.person_impact_probability_upper > bounds.impact_mass_upper
    {
        return Err(ExposureError::InvalidCellBounds);
    }
    Ok(())
}

/// Evaluate a ground-exposure evidence package against a frozen profile.
///
/// Population, impact, and severity evidence are owner-produced. The evaluator never
/// multiplies their unrelated means: each cell must provide joint-exposure bounds.
/// For the probability of at least one person impacted, the upper bound is the union
/// bound, so no independence assumption is made between cells.
pub fn evaluate_ground_exposure(
    case: &GroundExposureCase,
    population: &PopulationSnapshot,
    impact: &ImpactDistribution,
    severity: &SeverityModel,
    cells: &[CellExposureEvidence],
    policy: &GroundExposurePolicy,
) -> Result<GroundExposureResult, ExposureError> {
    validate_policy(policy)?;
    validate_case(case)?;

    let (expected_cells, zone_counts) =
        validate_zone_geometries(&case.required_zones, policy.require_all_ground_zones)?;

    let population_header_ok = validate_header(
        &population.header,
        &case.scope,
        case.analysis_window,
        case.known_at,
    )?;
    let impact_header_ok = validate_header(
        &impact.header,
        &case.scope,
        case.analysis_window,
        case.known_at,
    )?;
    let severity_header_ok = validate_header(
        &severity.header,
        &case.scope,
        case.analysis_window,
        case.known_at,
    )?;

    if population.dataset_sha256 != case.scope.population_dataset_sha256
        || population.population_definition_sha256 != case.scope.population_definition_sha256
        || impact.model_sha256 != case.scope.impact_model_sha256
        || severity.model_sha256 != case.scope.severity_model_sha256
        || severity.aircraft_model_sha256 != case.scope.aircraft_model_sha256
    {
        return Err(ExposureError::EvidenceScopeMismatch);
    }

    let mut status = if case.applicability == Applicability::OutOfDomain {
        ExposureStatus::OutOfDomain
    } else if case.applicability == Applicability::Unknown
        || !population_header_ok
        || !impact_header_ok
        || !severity_header_ok
        || !population.coverage_complete
        || !impact.coverage_complete
        || !severity.coverage_complete
    {
        ExposureStatus::Indeterminate
    } else {
        ExposureStatus::QualifiedForDeclaredExposureReview
    };

    if !valid_nonnegative(population.maximum_cell_area_m2)
        || population.maximum_cell_area_m2 > policy.maximum_cell_area_m2
    {
        status = ExposureStatus::Indeterminate;
    }

    if population.header.covered_cell_ids.iter().collect::<BTreeSet<_>>()
        != expected_cells.iter().collect::<BTreeSet<_>>()
        || impact.header.covered_cell_ids.iter().collect::<BTreeSet<_>>()
            != expected_cells.iter().collect::<BTreeSet<_>>()
        || severity.header.covered_cell_ids.iter().collect::<BTreeSet<_>>()
            != expected_cells.iter().collect::<BTreeSet<_>>()
    {
        status = ExposureStatus::Indeterminate;
    }

    if !valid_probability(impact.residual_mass_lower)
        || !valid_probability(impact.residual_mass_upper)
        || impact.residual_mass_lower > impact.residual_mass_upper
    {
        return Err(ExposureError::InvalidImpactMass);
    }
    if impact.residual_mass_upper > policy.maximum_residual_impact_mass {
        status = ExposureStatus::Indeterminate;
    }

    let mut by_id: BTreeMap<&str, &CellExposureEvidence> = BTreeMap::new();
    for cell in cells {
        if cell.cell_id.trim().is_empty()
            || !valid_nonnegative(cell.cell_area_m2)
            || cell.cell_area_m2 > policy.maximum_cell_area_m2
        {
            return Err(ExposureError::InvalidCellBounds);
        }
        validate_cell_bounds(cell.bounds)?;
        if !expected_cells.contains(&cell.cell_id) {
            return Err(ExposureError::UnexpectedCell);
        }
        if by_id.insert(cell.cell_id.as_str(), cell).is_some() {
            return Err(ExposureError::DuplicateCell);
        }
    }
    if expected_cells.iter().any(|cell_id| !by_id.contains_key(cell_id.as_str())) {
        status = ExposureStatus::Indeterminate;
    }

    let mut aggregate = ExposureBounds {
        expected_exposed_persons_lower: 0.0,
        expected_exposed_persons_upper: 0.0,
        probability_at_least_one_impacted_lower: 0.0,
        probability_at_least_one_impacted_upper: 0.0,
        expected_consequence_units_upper: 0.0,
        population_lower: 0.0,
        population_upper: 0.0,
        total_impact_mass_lower: impact.residual_mass_lower,
        total_impact_mass_upper: impact.residual_mass_upper,
    };

    for cell_id in &expected_cells {
        let Some(cell) = by_id.get(cell_id.as_str()).copied() else {
            continue;
        };
        let bounds = cell.bounds;
        aggregate.population_lower += bounds.population_lower;
        aggregate.population_upper += bounds.population_upper;
        aggregate.expected_exposed_persons_lower += bounds.joint_exposed_persons_lower;
        aggregate.expected_exposed_persons_upper += bounds.joint_exposed_persons_upper;
        aggregate.probability_at_least_one_impacted_lower = aggregate
            .probability_at_least_one_impacted_lower
            .max(bounds.person_impact_probability_lower);
        aggregate.probability_at_least_one_impacted_upper +=
            bounds.person_impact_probability_upper;
        aggregate.expected_consequence_units_upper += bounds.joint_exposed_persons_upper
            * bounds.consequence_units_per_person_upper;
        aggregate.total_impact_mass_lower += bounds.impact_mass_lower;
        aggregate.total_impact_mass_upper += bounds.impact_mass_upper;
    }

    aggregate.probability_at_least_one_impacted_upper =
        aggregate.probability_at_least_one_impacted_upper.min(1.0);
    if aggregate.total_impact_mass_lower > 1.0
        || aggregate.total_impact_mass_upper < 1.0
    {
        return Err(ExposureError::InvalidImpactMass);
    }
    if aggregate.total_impact_mass_upper - aggregate.total_impact_mass_lower > 1.0 {
        return Err(ExposureError::InvalidImpactMass);
    }

    // Residual impact mass may lie outside all supplied population cells. It is
    // included in the probability upper bound, but prevents a positive qualification
    // when it exceeds the independently frozen policy tolerance.
    aggregate.probability_at_least_one_impacted_upper = (aggregate
        .probability_at_least_one_impacted_upper
        + impact.residual_mass_upper)
        .min(1.0);

    if expected_cells.is_empty() || by_id.is_empty() {
        status = ExposureStatus::Indeterminate;
    }
    let mut source_manifests = [
        population.header.manifest_sha256.clone(),
        impact.header.manifest_sha256.clone(),
        severity.header.manifest_sha256.clone(),
    ];
    source_manifests.sort_unstable();

    Ok(GroundExposureResult {
        assessment_id: case.assessment_id.clone(),
        status,
        analysis_window: case.analysis_window,
        known_at: case.known_at,
        cell_count: by_id.len(),
        zone_cell_counts: zone_counts,
        bounds: aggregate,
        policy_sha256: policy.policy_sha256.clone(),
        source_manifest_sha256: source_manifests,
    })
}
