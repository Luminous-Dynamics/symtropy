//! Quality-gated wall-surface coverage and return-reserve reference for AUV missions.
//!
//! The evaluator consumes already-derived sonar footprint observations in a
//! declared wall-local frame. It is not a sonar ray tracer, navigation system,
//! condition assessor, maintenance recommender, battery model or actuation
//! authority.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const MAX_SURFACE_CELLS: usize = 1_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceIdentity {
    pub mission_id: String,
    pub surface_id: String,
    pub surface_revision: String,
    pub frame_id: String,
    pub frame_revision: u64,
    pub sensor_id: String,
    pub sensor_revision: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub calibration_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceProfile {
    pub identity: SurfaceIdentity,
    pub length_u_m: f64,
    pub height_v_m: f64,
    pub cell_size_u_m: f64,
    pub cell_size_v_m: f64,
    pub min_sonar_range_m: f64,
    pub max_sonar_range_m: f64,
    pub min_return_strength_db: f64,
    pub max_resolution_m: f64,
    pub max_incidence_angle_rad: f64,
    pub target_standoff_m: f64,
    pub standoff_tolerance_m: f64,
    pub max_standoff_uncertainty_m: f64,
    pub max_navigation_uncertainty_m: f64,
    pub max_footprint_uncertainty_m: f64,
    pub max_heading_error_rad: f64,
    pub minimum_independent_passes: usize,
    pub minimum_view_angle_separation_rad: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceObservation {
    pub observation_id: String,
    pub ping_id: String,
    pub pass_id: String,
    pub evidence_ref: String,
    pub mission_id: String,
    pub surface_id: String,
    pub surface_revision: String,
    pub frame_id: String,
    pub frame_revision: u64,
    pub sensor_id: String,
    pub sensor_revision: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub calibration_digest: String,
    /// Acquisition time on the declared mission timebase, in seconds.
    pub acquired_at_s: f64,
    /// Axis-aligned footprint center in the declared wall-local (u, v) frame.
    pub center_u_m: f64,
    pub center_v_m: f64,
    pub half_width_u_m: f64,
    pub half_height_v_m: f64,
    pub footprint_uncertainty_m: f64,
    pub navigation_uncertainty_m: f64,
    pub sonar_range_m: f64,
    pub range_uncertainty_m: f64,
    pub resolution_m: f64,
    /// Source-specific return strength; profile threshold must use same sensor semantics.
    pub return_strength_db: f64,
    pub incidence_angle_rad: f64,
    pub standoff_m: f64,
    pub standoff_uncertainty_m: f64,
    pub heading_error_rad: f64,
    /// Wall-relative viewing angle in radians.
    pub view_angle_rad: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameDisposition {
    Qualified,
    QualityRejected,
    InvalidEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameAssessment {
    pub observation_id: String,
    pub ping_id: String,
    pub pass_id: String,
    pub acquired_at_s: f64,
    pub disposition: FrameDisposition,
    pub covered_cells: Vec<(usize, usize)>,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellDisposition {
    Qualified,
    InsufficientIndependentViews,
    Unobserved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellAssessment {
    pub row: usize,
    pub column: usize,
    pub u_min_m: f64,
    pub u_max_m: f64,
    pub v_min_m: f64,
    pub v_max_m: f64,
    pub distinct_passes: usize,
    pub qualified_passes: usize,
    pub disposition: CellDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceDisposition {
    Qualified,
    Incomplete,
    InvalidEvidence,
    Inconsistent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoverageReport {
    pub disposition: SurfaceDisposition,
    pub rows: usize,
    pub columns: usize,
    pub expected_cells: usize,
    pub qualified_cells: usize,
    pub incomplete_cells: usize,
    pub invalid_frames: usize,
    pub quality_rejected_frames: usize,
    /// Acquisition-time order, with observation ID as deterministic tie-break.
    pub frames: Vec<FrameAssessment>,
    /// Row-major surface-cell order.
    pub cells: Vec<CellAssessment>,
    pub errors: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReserveDisposition {
    Sufficient,
    Insufficient,
    Unknown,
    InvalidEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReturnReserveSnapshot {
    pub mission_id: String,
    pub energy_model_id: String,
    pub energy_model_revision: String,
    pub evidence_ref: String,
    pub initial_energy_j: f64,
    pub consumed_energy_j: f64,
    /// Conservative planning estimate supplied by a separately qualified estimator.
    pub estimated_return_energy_j: Option<f64>,
    pub return_energy_uncertainty_j: f64,
    pub required_reserve_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReserveAssessment {
    pub disposition: ReserveDisposition,
    pub remaining_energy_j: Option<f64>,
    pub conservative_return_requirement_j: Option<f64>,
    pub required_reserve_j: Option<f64>,
    pub margin_j: Option<f64>,
    pub reason: Option<String>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn dimensions(profile: &SurfaceProfile) -> Result<(usize, usize), String> {
    if !profile.length_u_m.is_finite()
        || profile.length_u_m <= 0.0
        || !profile.height_v_m.is_finite()
        || profile.height_v_m <= 0.0
        || !profile.cell_size_u_m.is_finite()
        || profile.cell_size_u_m <= 0.0
        || !profile.cell_size_v_m.is_finite()
        || profile.cell_size_v_m <= 0.0
    {
        return Err("surface dimensions and cell sizes must be finite and positive".to_owned());
    }
    let columns_f = (profile.length_u_m / profile.cell_size_u_m).ceil();
    let rows_f = (profile.height_v_m / profile.cell_size_v_m).ceil();
    if !columns_f.is_finite()
        || !rows_f.is_finite()
        || columns_f < 1.0
        || rows_f < 1.0
        || columns_f > MAX_SURFACE_CELLS as f64
        || rows_f > MAX_SURFACE_CELLS as f64
    {
        return Err("surface grid dimensions exceed supported limits".to_owned());
    }
    let columns = columns_f as usize;
    let rows = rows_f as usize;
    if rows
        .checked_mul(columns)
        .is_none_or(|count| count > MAX_SURFACE_CELLS)
    {
        return Err("surface grid contains too many cells".to_owned());
    }
    Ok((rows, columns))
}

fn validate_profile(profile: &SurfaceProfile) -> Result<(usize, usize), String> {
    let identity = &profile.identity;
    if !nonempty(&identity.mission_id)
        || !nonempty(&identity.surface_id)
        || !nonempty(&identity.surface_revision)
        || !nonempty(&identity.frame_id)
        || !nonempty(&identity.sensor_id)
        || !nonempty(&identity.sensor_revision)
        || !nonempty(&identity.calibration_id)
        || !nonempty(&identity.calibration_revision)
    {
        return Err(
            "surface, mission, frame, sensor or calibration identity is incomplete".to_owned(),
        );
    }
    if !valid_digest(&identity.calibration_digest) {
        return Err("calibration digest must be lowercase SHA-256 hex".to_owned());
    }
    let (rows, columns) = dimensions(profile)?;
    if !profile.min_sonar_range_m.is_finite()
        || profile.min_sonar_range_m < 0.0
        || !profile.max_sonar_range_m.is_finite()
        || profile.max_sonar_range_m <= profile.min_sonar_range_m
        || !profile.min_return_strength_db.is_finite()
        || !profile.max_resolution_m.is_finite()
        || profile.max_resolution_m <= 0.0
        || !profile.max_incidence_angle_rad.is_finite()
        || profile.max_incidence_angle_rad < 0.0
        || profile.max_incidence_angle_rad > std::f64::consts::FRAC_PI_2
        || !profile.target_standoff_m.is_finite()
        || profile.target_standoff_m <= 0.0
        || !profile.standoff_tolerance_m.is_finite()
        || profile.standoff_tolerance_m <= 0.0
        || !finite_nonnegative(profile.max_standoff_uncertainty_m)
        || !finite_nonnegative(profile.max_navigation_uncertainty_m)
        || !finite_nonnegative(profile.max_footprint_uncertainty_m)
        || !profile.max_heading_error_rad.is_finite()
        || profile.max_heading_error_rad < 0.0
        || profile.max_heading_error_rad > std::f64::consts::PI
        || profile.minimum_independent_passes == 0
        || !profile.minimum_view_angle_separation_rad.is_finite()
        || profile.minimum_view_angle_separation_rad < 0.0
        || profile.minimum_view_angle_separation_rad > std::f64::consts::PI
    {
        return Err("sensor profile contains invalid limits or coverage requirements".to_owned());
    }
    Ok((rows, columns))
}

fn validate_observation_identity(
    profile: &SurfaceProfile,
    observation: &SurfaceObservation,
) -> Result<(), String> {
    let expected = &profile.identity;
    if !nonempty(&observation.observation_id)
        || !nonempty(&observation.ping_id)
        || !nonempty(&observation.pass_id)
        || !nonempty(&observation.evidence_ref)
    {
        return Err("observation, ping, pass or evidence identity is missing".to_owned());
    }
    if observation.mission_id != expected.mission_id
        || observation.surface_id != expected.surface_id
        || observation.surface_revision != expected.surface_revision
    {
        return Err("mission/surface identity or revision does not match profile".to_owned());
    }
    if observation.frame_id != expected.frame_id
        || observation.frame_revision != expected.frame_revision
    {
        return Err("wall-local frame identity or revision does not match profile".to_owned());
    }
    if observation.sensor_id != expected.sensor_id
        || observation.sensor_revision != expected.sensor_revision
    {
        return Err("sonar sensor identity or revision does not match profile".to_owned());
    }
    if observation.calibration_id != expected.calibration_id
        || observation.calibration_revision != expected.calibration_revision
        || observation.calibration_digest != expected.calibration_digest
    {
        return Err("sonar calibration identity/revision/digest does not match profile".to_owned());
    }
    Ok(())
}

fn validate_observation_values(observation: &SurfaceObservation) -> Result<(), String> {
    let values = [
        observation.acquired_at_s,
        observation.center_u_m,
        observation.center_v_m,
        observation.half_width_u_m,
        observation.half_height_v_m,
        observation.footprint_uncertainty_m,
        observation.navigation_uncertainty_m,
        observation.sonar_range_m,
        observation.range_uncertainty_m,
        observation.resolution_m,
        observation.return_strength_db,
        observation.incidence_angle_rad,
        observation.standoff_m,
        observation.standoff_uncertainty_m,
        observation.heading_error_rad,
        observation.view_angle_rad,
    ];
    if !values.iter().all(|value| value.is_finite()) {
        return Err("observation contains a non-finite measurement".to_owned());
    }
    if observation.acquired_at_s < 0.0
        || observation.half_width_u_m <= 0.0
        || observation.half_height_v_m <= 0.0
        || !finite_nonnegative(observation.footprint_uncertainty_m)
        || !finite_nonnegative(observation.navigation_uncertainty_m)
        || observation.sonar_range_m <= 0.0
        || !finite_nonnegative(observation.range_uncertainty_m)
        || observation.resolution_m <= 0.0
        || observation.incidence_angle_rad < 0.0
        || observation.standoff_m <= 0.0
        || !finite_nonnegative(observation.standoff_uncertainty_m)
    {
        return Err(
            "observation contains an invalid range, footprint, time or uncertainty".to_owned(),
        );
    }
    Ok(())
}

fn quality_rejection(profile: &SurfaceProfile, observation: &SurfaceObservation) -> Option<String> {
    if observation.navigation_uncertainty_m > profile.max_navigation_uncertainty_m {
        return Some("navigation uncertainty exceeds profile".to_owned());
    }
    if observation.footprint_uncertainty_m > profile.max_footprint_uncertainty_m {
        return Some("footprint uncertainty exceeds profile".to_owned());
    }
    if observation.standoff_uncertainty_m > profile.max_standoff_uncertainty_m {
        return Some("standoff uncertainty exceeds profile".to_owned());
    }
    if observation.sonar_range_m - observation.range_uncertainty_m < profile.min_sonar_range_m
        || observation.sonar_range_m + observation.range_uncertainty_m > profile.max_sonar_range_m
    {
        return Some("sonar range plus uncertainty is outside profile".to_owned());
    }
    if observation.return_strength_db < profile.min_return_strength_db {
        return Some("sonar return strength is below source-specific threshold".to_owned());
    }
    if observation.resolution_m > profile.max_resolution_m {
        return Some("sonar resolution is coarser than profile maximum".to_owned());
    }
    if observation.incidence_angle_rad > profile.max_incidence_angle_rad {
        return Some("sonar incidence angle exceeds profile".to_owned());
    }
    if (observation.standoff_m - profile.target_standoff_m).abs()
        + observation.standoff_uncertainty_m
        > profile.standoff_tolerance_m
    {
        return Some("standoff error plus uncertainty exceeds profile tolerance".to_owned());
    }
    if observation.heading_error_rad.abs() > profile.max_heading_error_rad {
        return Some("wall-relative heading error exceeds profile".to_owned());
    }
    None
}

fn angular_separation(left: f64, right: f64) -> f64 {
    let difference = (left - right).abs().rem_euclid(std::f64::consts::TAU);
    difference.min(std::f64::consts::TAU - difference)
}

fn conservative_footprint(observation: &SurfaceObservation) -> Option<(f64, f64, f64, f64)> {
    let shrink = observation.navigation_uncertainty_m + observation.footprint_uncertainty_m;
    if !shrink.is_finite() {
        return None;
    }
    let half_u = observation.half_width_u_m - shrink;
    let half_v = observation.half_height_v_m - shrink;
    if half_u <= 0.0 || half_v <= 0.0 {
        return None;
    }
    Some((
        observation.center_u_m - half_u,
        observation.center_u_m + half_u,
        observation.center_v_m - half_v,
        observation.center_v_m + half_v,
    ))
}

fn cell_bounds(profile: &SurfaceProfile, row: usize, column: usize) -> (f64, f64, f64, f64) {
    let u_min = column as f64 * profile.cell_size_u_m;
    let u_max = (u_min + profile.cell_size_u_m).min(profile.length_u_m);
    let v_min = row as f64 * profile.cell_size_v_m;
    let v_max = (v_min + profile.cell_size_v_m).min(profile.height_v_m);
    (u_min, u_max, v_min, v_max)
}

fn covered_cells(
    profile: &SurfaceProfile,
    rows: usize,
    columns: usize,
    observation: &SurfaceObservation,
) -> Vec<(usize, usize)> {
    let Some((u_min, u_max, v_min, v_max)) = conservative_footprint(observation) else {
        return Vec::new();
    };
    let mut cells = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            let (cell_u_min, cell_u_max, cell_v_min, cell_v_max) =
                cell_bounds(profile, row, column);
            if cell_u_min >= u_min
                && cell_u_max <= u_max
                && cell_v_min >= v_min
                && cell_v_max <= v_max
            {
                cells.push((row, column));
            }
        }
    }
    cells
}

fn qualified_pass_count(profile: &SurfaceProfile, views: &BTreeMap<String, f64>) -> usize {
    let mut selected_angles = Vec::new();
    for angle in views.values() {
        if selected_angles.iter().all(|selected| {
            angular_separation(*selected, *angle) >= profile.minimum_view_angle_separation_rad
        }) {
            selected_angles.push(*angle);
        }
    }
    selected_angles.len()
}

/// Evaluate sonar frames against the frozen planar wall-surface profile.
///
/// The output is coverage evidence only. It does not classify defects or
/// accept the asset condition.
pub fn evaluate_coverage(
    profile: &SurfaceProfile,
    observations: &[SurfaceObservation],
) -> CoverageReport {
    let (rows, columns) = match validate_profile(profile) {
        Ok(value) => value,
        Err(error) => {
            return CoverageReport {
                disposition: SurfaceDisposition::InvalidEvidence,
                rows: 0,
                columns: 0,
                expected_cells: 0,
                qualified_cells: 0,
                incomplete_cells: 0,
                invalid_frames: 0,
                quality_rejected_frames: 0,
                frames: Vec::new(),
                cells: Vec::new(),
                errors: vec![error],
            };
        }
    };

    let mut errors = Vec::new();
    let mut observation_ids = BTreeSet::new();
    let mut ping_ids = BTreeSet::new();
    let mut frame_assessments = Vec::with_capacity(observations.len());
    let mut pass_views_by_cell = BTreeMap::<(usize, usize), BTreeMap<String, f64>>::new();
    let mut invalid_frames = 0;
    let mut quality_rejected_frames = 0;

    for observation in observations {
        let identity_result = validate_observation_identity(profile, observation);
        let value_result = validate_observation_values(observation);
        if !observation_ids.insert(observation.observation_id.as_str()) {
            errors.push(format!(
                "duplicate observation ID: {}",
                observation.observation_id
            ));
            continue;
        }
        if !ping_ids.insert(observation.ping_id.as_str()) {
            errors.push(format!(
                "duplicate/replayed ping ID: {}",
                observation.ping_id
            ));
            continue;
        }

        let invalid_reason = identity_result.err().or_else(|| value_result.err());
        if let Some(reason) = invalid_reason {
            invalid_frames += 1;
            frame_assessments.push(FrameAssessment {
                observation_id: observation.observation_id.clone(),
                ping_id: observation.ping_id.clone(),
                pass_id: observation.pass_id.clone(),
                acquired_at_s: observation.acquired_at_s,
                disposition: FrameDisposition::InvalidEvidence,
                covered_cells: Vec::new(),
                reason: Some(reason),
            });
            continue;
        }
        if let Some(reason) = quality_rejection(profile, observation) {
            quality_rejected_frames += 1;
            frame_assessments.push(FrameAssessment {
                observation_id: observation.observation_id.clone(),
                ping_id: observation.ping_id.clone(),
                pass_id: observation.pass_id.clone(),
                acquired_at_s: observation.acquired_at_s,
                disposition: FrameDisposition::QualityRejected,
                covered_cells: Vec::new(),
                reason: Some(reason),
            });
            continue;
        }

        let cells = covered_cells(profile, rows, columns, observation);
        if !cells.is_empty() {
            for cell in &cells {
                let views = pass_views_by_cell.entry(*cell).or_default();
                views
                    .entry(observation.pass_id.clone())
                    .or_insert(observation.view_angle_rad);
            }
        }
        frame_assessments.push(FrameAssessment {
            observation_id: observation.observation_id.clone(),
            ping_id: observation.ping_id.clone(),
            pass_id: observation.pass_id.clone(),
            acquired_at_s: observation.acquired_at_s,
            disposition: FrameDisposition::Qualified,
            covered_cells: cells,
            reason: None,
        });
    }

    frame_assessments.sort_by(|left, right| {
        left.acquired_at_s
            .total_cmp(&right.acquired_at_s)
            .then_with(|| left.observation_id.cmp(&right.observation_id))
    });

    let mut cells = Vec::with_capacity(rows * columns);
    let mut qualified_cells = 0;
    let mut incomplete_cells = 0;
    for row in 0..rows {
        for column in 0..columns {
            let (u_min_m, u_max_m, v_min_m, v_max_m) = cell_bounds(profile, row, column);
            let views = pass_views_by_cell
                .get(&(row, column))
                .cloned()
                .unwrap_or_default();
            let distinct_passes = views.len();
            let qualified_passes = qualified_pass_count(profile, &views);
            let disposition = if qualified_passes >= profile.minimum_independent_passes {
                qualified_cells += 1;
                CellDisposition::Qualified
            } else if distinct_passes == 0 {
                incomplete_cells += 1;
                CellDisposition::Unobserved
            } else {
                incomplete_cells += 1;
                CellDisposition::InsufficientIndependentViews
            };
            cells.push(CellAssessment {
                row,
                column,
                u_min_m,
                u_max_m,
                v_min_m,
                v_max_m,
                distinct_passes,
                qualified_passes,
                disposition,
            });
        }
    }

    let disposition = if !errors.is_empty() {
        SurfaceDisposition::Inconsistent
    } else if invalid_frames > 0 {
        SurfaceDisposition::InvalidEvidence
    } else if incomplete_cells > 0 {
        SurfaceDisposition::Incomplete
    } else {
        SurfaceDisposition::Qualified
    };

    CoverageReport {
        disposition,
        rows,
        columns,
        expected_cells: rows * columns,
        qualified_cells,
        incomplete_cells,
        invalid_frames,
        quality_rejected_frames,
        frames: frame_assessments,
        cells,
        errors,
    }
}

/// Assess an energy-envelope estimate without claiming physical battery truth
/// or guaranteeing a return trajectory.
pub fn assess_return_reserve(snapshot: &ReturnReserveSnapshot) -> ReserveAssessment {
    let invalid = if !nonempty(&snapshot.mission_id)
        || !nonempty(&snapshot.energy_model_id)
        || !nonempty(&snapshot.energy_model_revision)
        || !nonempty(&snapshot.evidence_ref)
    {
        Some("mission, energy-model or evidence identity is incomplete".to_owned())
    } else if !snapshot.initial_energy_j.is_finite()
        || snapshot.initial_energy_j <= 0.0
        || !finite_nonnegative(snapshot.consumed_energy_j)
        || snapshot.consumed_energy_j > snapshot.initial_energy_j
        || !finite_nonnegative(snapshot.return_energy_uncertainty_j)
        || !snapshot.required_reserve_fraction.is_finite()
        || !(0.0..=1.0).contains(&snapshot.required_reserve_fraction)
        || snapshot
            .estimated_return_energy_j
            .is_some_and(|value| !finite_nonnegative(value))
    {
        Some("energy envelope contains invalid or inconsistent numeric values".to_owned())
    } else {
        None
    };
    if let Some(reason) = invalid {
        return ReserveAssessment {
            disposition: ReserveDisposition::InvalidEvidence,
            remaining_energy_j: None,
            conservative_return_requirement_j: None,
            required_reserve_j: None,
            margin_j: None,
            reason: Some(reason),
        };
    }

    let remaining = snapshot.initial_energy_j - snapshot.consumed_energy_j;
    let reserve = snapshot.initial_energy_j * snapshot.required_reserve_fraction;
    let Some(estimated_return) = snapshot.estimated_return_energy_j else {
        return ReserveAssessment {
            disposition: ReserveDisposition::Unknown,
            remaining_energy_j: Some(remaining),
            conservative_return_requirement_j: None,
            required_reserve_j: Some(reserve),
            margin_j: None,
            reason: Some("return-energy estimate is unavailable".to_owned()),
        };
    };
    let conservative_return = estimated_return + snapshot.return_energy_uncertainty_j;
    let margin = remaining - conservative_return - reserve;
    let disposition = if margin >= 0.0 {
        ReserveDisposition::Sufficient
    } else {
        ReserveDisposition::Insufficient
    };
    ReserveAssessment {
        disposition,
        remaining_energy_j: Some(remaining),
        conservative_return_requirement_j: Some(conservative_return),
        required_reserve_j: Some(reserve),
        margin_j: Some(margin),
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn profile() -> SurfaceProfile {
        SurfaceProfile {
            identity: SurfaceIdentity {
                mission_id: "mission:wall-survey-01".to_owned(),
                surface_id: "asset:wall-panel-01".to_owned(),
                surface_revision: "wall-r3".to_owned(),
                frame_id: "wall-local:uv".to_owned(),
                frame_revision: 7,
                sensor_id: "sonar:msis-01".to_owned(),
                sensor_revision: "sensor-r2".to_owned(),
                calibration_id: "cal:sonar-01".to_owned(),
                calibration_revision: "cal-r4".to_owned(),
                calibration_digest: DIGEST_A.to_owned(),
            },
            length_u_m: 2.0,
            height_v_m: 2.0,
            cell_size_u_m: 1.0,
            cell_size_v_m: 1.0,
            min_sonar_range_m: 0.3,
            max_sonar_range_m: 12.0,
            min_return_strength_db: 22.0,
            max_resolution_m: 0.10,
            max_incidence_angle_rad: 0.70,
            target_standoff_m: 1.5,
            standoff_tolerance_m: 0.15,
            max_standoff_uncertainty_m: 0.05,
            max_navigation_uncertainty_m: 0.10,
            max_footprint_uncertainty_m: 0.10,
            max_heading_error_rad: 0.20,
            minimum_independent_passes: 2,
            minimum_view_angle_separation_rad: 0.40,
        }
    }

    fn frame(pass: &str, ping: &str, angle: f64, time: f64) -> SurfaceObservation {
        let identity = profile().identity;
        SurfaceObservation {
            observation_id: format!("observation-{ping}"),
            ping_id: ping.to_owned(),
            pass_id: pass.to_owned(),
            evidence_ref: format!("synthetic-evidence-{ping}"),
            mission_id: identity.mission_id,
            surface_id: identity.surface_id,
            surface_revision: identity.surface_revision,
            frame_id: identity.frame_id,
            frame_revision: identity.frame_revision,
            sensor_id: identity.sensor_id,
            sensor_revision: identity.sensor_revision,
            calibration_id: identity.calibration_id,
            calibration_revision: identity.calibration_revision,
            calibration_digest: identity.calibration_digest,
            acquired_at_s: time,
            center_u_m: 1.0,
            center_v_m: 1.0,
            half_width_u_m: 1.0,
            half_height_v_m: 1.0,
            footprint_uncertainty_m: 0.0,
            navigation_uncertainty_m: 0.0,
            sonar_range_m: 8.0,
            range_uncertainty_m: 0.1,
            resolution_m: 0.05,
            return_strength_db: 30.0,
            incidence_angle_rad: 0.2,
            standoff_m: 1.5,
            standoff_uncertainty_m: 0.02,
            heading_error_rad: 0.02,
            view_angle_rad: angle,
        }
    }

    fn two_view_frames() -> Vec<SurfaceObservation> {
        vec![
            frame("track-A", "ping-A", 0.0, 1.0),
            frame("track-B", "ping-B", 0.8, 2.0),
        ]
    }

    fn reserve_snapshot() -> ReturnReserveSnapshot {
        ReturnReserveSnapshot {
            mission_id: "mission:wall-survey-01".to_owned(),
            energy_model_id: "auv-energy-model".to_owned(),
            energy_model_revision: "energy-r2".to_owned(),
            evidence_ref: "synthetic-energy-receipt".to_owned(),
            initial_energy_j: 1000.0,
            consumed_energy_j: 400.0,
            estimated_return_energy_j: Some(300.0),
            return_energy_uncertainty_j: 50.0,
            required_reserve_fraction: 0.10,
        }
    }

    #[test]
    fn two_independent_qualified_passes_cover_every_cell() {
        let result = evaluate_coverage(&profile(), &two_view_frames());
        assert_eq!(result.disposition, SurfaceDisposition::Qualified);
        assert_eq!(result.expected_cells, 4);
        assert_eq!(result.qualified_cells, 4);
        assert!(
            result
                .cells
                .iter()
                .all(|cell| cell.disposition == CellDisposition::Qualified)
        );
    }

    #[test]
    fn one_pass_does_not_satisfy_independent_view_requirement() {
        let result = evaluate_coverage(&profile(), &[frame("track-A", "ping-A", 0.0, 1.0)]);
        assert_eq!(result.disposition, SurfaceDisposition::Incomplete);
        assert_eq!(result.qualified_cells, 0);
        assert!(
            result
                .cells
                .iter()
                .all(|cell| { cell.disposition == CellDisposition::InsufficientIndependentViews })
        );
    }

    #[test]
    fn multiple_frames_on_one_trackline_do_not_count_as_independent_passes() {
        let observations = vec![
            frame("track-A", "ping-A", 0.0, 1.0),
            frame("track-A", "ping-B", 0.8, 2.0),
        ];
        let result = evaluate_coverage(&profile(), &observations);
        assert_eq!(result.disposition, SurfaceDisposition::Incomplete);
        assert!(result.cells.iter().all(|cell| cell.distinct_passes == 1));
    }

    #[test]
    fn insufficient_view_angle_separation_does_not_qualify_a_cell() {
        let observations = vec![
            frame("track-A", "ping-A", 0.0, 1.0),
            frame("track-B", "ping-B", 0.2, 2.0),
        ];
        let result = evaluate_coverage(&profile(), &observations);
        assert_eq!(result.disposition, SurfaceDisposition::Incomplete);
        assert!(result.cells.iter().all(|cell| cell.qualified_passes == 1));
    }

    #[test]
    fn partial_footprint_overlap_does_not_count_as_full_cell_coverage() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.center_u_m = 0.5;
        observation.half_width_u_m = 0.5;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(result.frames[0].covered_cells, vec![(0, 0), (1, 0)]);
        assert_eq!(result.qualified_cells, 0);
    }

    #[test]
    fn footprint_uncertainty_shrinks_the_admitted_coverage_region() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.footprint_uncertainty_m = 0.2;
        observation.navigation_uncertainty_m = 0.2;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert!(result.frames[0].covered_cells.is_empty());
    }

    #[test]
    fn excessive_navigation_uncertainty_rejects_frame() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.navigation_uncertainty_m = 0.2;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(
            result.frames[0].disposition,
            FrameDisposition::QualityRejected
        );
        assert!(
            result.frames[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("navigation uncertainty")
        );
    }

    #[test]
    fn sonar_range_outside_profile_rejects_frame() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.sonar_range_m = 12.0;
        observation.range_uncertainty_m = 0.2;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(
            result.frames[0].disposition,
            FrameDisposition::QualityRejected
        );
    }

    #[test]
    fn low_echo_return_rejects_frame() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.return_strength_db = 21.9;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(
            result.frames[0].disposition,
            FrameDisposition::QualityRejected
        );
    }

    #[test]
    fn coarse_sonar_resolution_rejects_frame() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.resolution_m = 0.11;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(
            result.frames[0].disposition,
            FrameDisposition::QualityRejected
        );
    }

    #[test]
    fn excessive_incidence_angle_rejects_frame() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.incidence_angle_rad = 0.71;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(
            result.frames[0].disposition,
            FrameDisposition::QualityRejected
        );
    }

    #[test]
    fn standoff_error_plus_uncertainty_rejects_frame() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.standoff_m = 1.62;
        observation.standoff_uncertainty_m = 0.04;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(
            result.frames[0].disposition,
            FrameDisposition::QualityRejected
        );
    }

    #[test]
    fn excessive_wall_heading_error_rejects_frame() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.heading_error_rad = 0.21;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(
            result.frames[0].disposition,
            FrameDisposition::QualityRejected
        );
    }

    #[test]
    fn duplicate_ping_id_is_inconsistent_and_never_double_counts() {
        let mut observations = two_view_frames();
        observations[1].ping_id = observations[0].ping_id.clone();
        let result = evaluate_coverage(&profile(), &observations);
        assert_eq!(result.disposition, SurfaceDisposition::Inconsistent);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn duplicate_observation_id_is_inconsistent() {
        let mut observations = two_view_frames();
        observations[1].observation_id = observations[0].observation_id.clone();
        let result = evaluate_coverage(&profile(), &observations);
        assert_eq!(result.disposition, SurfaceDisposition::Inconsistent);
    }

    #[test]
    fn stale_surface_revision_is_invalid_evidence() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.surface_revision = "wall-old".to_owned();
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(result.disposition, SurfaceDisposition::InvalidEvidence);
        assert_eq!(result.invalid_frames, 1);
    }

    #[test]
    fn stale_calibration_digest_is_invalid_evidence() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.calibration_digest =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(result.disposition, SurfaceDisposition::InvalidEvidence);
    }

    #[test]
    fn malformed_numeric_observation_is_invalid_evidence() {
        let mut observation = frame("track-A", "ping-A", 0.0, 1.0);
        observation.center_u_m = f64::NAN;
        let result = evaluate_coverage(&profile(), &[observation]);
        assert_eq!(result.disposition, SurfaceDisposition::InvalidEvidence);
    }

    #[test]
    fn observation_permutation_preserves_report_and_chronology() {
        let observations = two_view_frames();
        let mut reversed = observations.clone();
        reversed.reverse();
        let first = evaluate_coverage(&profile(), &observations);
        let second = evaluate_coverage(&profile(), &reversed);
        assert_eq!(first, second);
        assert!(first.frames[0].acquired_at_s <= first.frames[1].acquired_at_s);
    }

    #[test]
    fn return_reserve_sufficient_when_energy_covers_return_and_reserve() {
        let result = assess_return_reserve(&reserve_snapshot());
        assert_eq!(result.disposition, ReserveDisposition::Sufficient);
        assert_eq!(result.remaining_energy_j, Some(600.0));
        assert_eq!(result.required_reserve_j, Some(100.0));
        assert_eq!(result.margin_j, Some(150.0));
    }

    #[test]
    fn return_reserve_insufficient_when_conservative_requirement_exceeds_remaining() {
        let mut snapshot = reserve_snapshot();
        snapshot.estimated_return_energy_j = Some(550.0);
        let result = assess_return_reserve(&snapshot);
        assert_eq!(result.disposition, ReserveDisposition::Insufficient);
        assert!(result.margin_j.unwrap() < 0.0);
    }

    #[test]
    fn missing_return_energy_estimate_is_unknown_not_sufficient() {
        let mut snapshot = reserve_snapshot();
        snapshot.estimated_return_energy_j = None;
        let result = assess_return_reserve(&snapshot);
        assert_eq!(result.disposition, ReserveDisposition::Unknown);
        assert!(result.margin_j.is_none());
    }

    #[test]
    fn inconsistent_energy_values_are_invalid_evidence() {
        let mut snapshot = reserve_snapshot();
        snapshot.consumed_energy_j = 1100.0;
        let result = assess_return_reserve(&snapshot);
        assert_eq!(result.disposition, ReserveDisposition::InvalidEvidence);
    }

    #[test]
    fn reserve_fraction_above_one_is_invalid() {
        let mut snapshot = reserve_snapshot();
        snapshot.required_reserve_fraction = 1.1;
        let result = assess_return_reserve(&snapshot);
        assert_eq!(result.disposition, ReserveDisposition::InvalidEvidence);
    }
}
