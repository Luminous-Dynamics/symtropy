use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const MAX_MM: u64 = 1_000_000;
type Check<T> = Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    units: Units,
    scenarios: Vec<Scenario>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Units { length: String, volume: String, uncertainty: String }
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    evidence_class: String,
    accounting_tolerance_ppm: u32,
    geometry_tolerance_ppm: u32,
    log: LogGeometry,
    machine: MachineConfig,
    observation: ObservedRun,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogGeometry {
    log_id: String,
    species_code: String,
    species_evidence_state: EvidenceState,
    length_mm: u64,
    gross_volume_mm3: u64,
    gross_volume_method: String,
    source: SourceMetadata,
    geometry_source: SourceMetadata,
    stations: Vec<SectionStation>,
    defects: Vec<Defect>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceMetadata {
    source_id: String,
    license: String,
    measured_at: String,
    method: String,
    instrument_id: Option<String>,
    calibration_status: CalibrationStatus,
    uncertainty_ppm: u32,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionStation {
    axial_position_mm: u64,
    major_diameter_mm: u64,
    minor_diameter_mm: u64,
    center_x_mm: i64,
    center_y_mm: i64,
    ellipse_rotation_mdeg: u32,
    uncertainty_ppm: u32,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Defect {
    defect_id: String,
    kind: DefectKind,
    axial_start_mm: u64,
    axial_end_mm: u64,
    circumferential_angle_mdeg: Option<u32>,
    radial_depth_mm: Option<u64>,
    evidence_state: EvidenceState,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MachineConfig {
    machine_id: String,
    kerf_mm: u64,
    sawing_variation_mm: u64,
    target_thickness_mm: u64,
    target_width_mm: u64,
    green_allowance_mm: u64,
    grade_rule_id: String,
    grade_rule_version: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedRun {
    status: ObservationStatus,
    observed_at: String,
    measurement_uncertainty_ppm: u32,
    boards: Vec<BoardObservation>,
    byproducts: Byproducts,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardObservation {
    board_id: String,
    length_mm: u64,
    width_mm: u64,
    thickness_mm: u64,
    grade_label: Option<String>,
    grade_evidence_state: EvidenceState,
    intended_application: IntendedApplication,
    inspection_pass: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Byproducts {
    sawdust_volume_mm3: u64,
    chips_volume_mm3: u64,
    bark_volume_mm3: u64,
    slabs_and_trim_volume_mm3: u64,
    rejects_volume_mm3: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceState { Qualified, Observed, Estimated, Missing, Unknown, Synthetic }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CalibrationStatus { Current, Expired, NotApplicable, Unknown }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DefectKind { Knot, Crack, Rot, BarkInclusion, Sweep, Crook, Other }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ObservationStatus { Complete, Partial, Unknown }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum IntendedApplication { Structural, NonStructural, Unknown }

#[derive(Debug, Serialize)]
struct Receipt {
    result: &'static str,
    validator_version: &'static str,
    suite_id: String,
    suite_version: String,
    claim_ceiling: String,
    scenario_count: usize,
    corpus_sha256: String,
    source_commit: String,
    geometry_metrics: AggregateGeometryMetrics,
    scenarios: Vec<ScenarioSummary>,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
struct ScenarioSummary {
    scenario_id: String,
    log_id: String,
    evidence_class: String,
    length_mm: u64,
    section_station_count: usize,
    defect_count: usize,
    gross_log_volume_mm3: u64,
    species_evidence_state: EvidenceState,
    reference_source_id: String,
    reference_source_license: String,
    reference_source_method: String,
    reference_source_measured_at: String,
    reference_source_calibration_status: CalibrationStatus,
    reference_source_uncertainty_ppm: u32,
    sawing_variation_mm: u64,
    defects: Vec<DefectSummary>,
    geometry_volume_estimate_mm3: u128,
    absolute_geometry_delta_mm3: u128,
    geometry_tolerance_mm3: u128,
    absolute_geometry_percentage_error_ppm: u64,
    geometry_target_met: bool,
    geometry_source_id: String,
    geometry_source_license: String,
    geometry_source_method: String,
    geometry_source_measured_at: String,
    geometry_source_calibration_status: CalibrationStatus,
    geometry_source_uncertainty_ppm: u32,
    observed_board_count: usize,
    observed_board_volume_mm3: u128,
    known_byproduct_volume_mm3: u128,
    known_output_volume_mm3: u128,
    absolute_accounting_delta_mm3: Option<u128>,
    accounting_status: &'static str,
    grade_rule_id: String,
    grade_rule_version: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct AggregateGeometryMetrics {
    evaluated_log_count: usize,
    mean_absolute_percentage_error_ppm: u64,
    median_absolute_percentage_error_ppm: u64,
    max_absolute_percentage_error_ppm: u64,
    volume_weighted_absolute_percentage_error_ppm: u64,
    within_declared_target_count: usize,
    within_declared_target_fraction_ppm: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct DefectSummary {
    defect_id: String,
    kind: DefectKind,
    axial_start_mm: u64,
    axial_end_mm: u64,
    evidence_state: EvidenceState,
}

fn percent_error_ppm(delta: u128, reference: u128) -> Check<u64> {
    if reference == 0 {
        return Err("reference volume must be positive".to_owned());
    }
    let scaled = delta.checked_mul(1_000_000)
        .ok_or_else(|| "percentage error overflow".to_owned())?;
    let rounded = scaled.checked_add(reference / 2)
        .ok_or_else(|| "percentage error rounding overflow".to_owned())? / reference;
    u64::try_from(rounded).map_err(|_| "percentage error does not fit u64".to_owned())
}

fn nonempty(s: &str, field: &str, context: &str) -> Check<()> {
    if s.trim().is_empty() { return Err(format!("{context}: {field} must not be empty")); }
    Ok(())
}
fn valid_date(s: &str, field: &str, context: &str) -> Check<()> {
    let b = s.as_bytes();
    let shape_valid = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shape_valid {
        return Err(format!("{context}: {field} must use a valid YYYY-MM-DD date"));
    }
    let year = s[0..4].parse::<u32>().unwrap_or(0);
    let month = s[5..7].parse::<u32>().unwrap_or(0);
    let day = s[8..10].parse::<u32>().unwrap_or(0);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day {
        return Err(format!("{context}: {field} must use a valid YYYY-MM-DD date"));
    }
    Ok(())
}
fn board_volume(b: &BoardObservation) -> u128 {
    u128::from(b.length_mm) * u128::from(b.width_mm) * u128::from(b.thickness_mm)
}
fn totals(s: &Scenario) -> Check<(u128, u128, u128)> {
    let boards = s.observation.boards.iter().try_fold(0_u128, |sum, b| {
        sum.checked_add(board_volume(b)).ok_or_else(|| format!("{}: board volume overflow", s.scenario_id))
    })?;
    let b = &s.observation.byproducts;
    let byproducts = [b.sawdust_volume_mm3, b.chips_volume_mm3, b.bark_volume_mm3,
        b.slabs_and_trim_volume_mm3, b.rejects_volume_mm3].into_iter().try_fold(0_u128, |sum, v| {
        sum.checked_add(u128::from(v)).ok_or_else(|| format!("{}: by-product volume overflow", s.scenario_id))
    })?;
    let combined = boards.checked_add(byproducts).ok_or_else(|| format!("{}: total volume overflow", s.scenario_id))?;
    Ok((boards, byproducts, combined))
}
fn estimated_geometry_volume_mm3(log: &LogGeometry) -> Check<u128> {
    // V = pi/8 * sum((d_major * d_minor)_i + (d_major * d_minor)_(i+1)) * dx.
    // A scaled integer pi and checked u128 arithmetic make rounding reproducible
    // across targets; this is an elliptical-section estimate, not CT truth.
    const PI_SCALED: u128 = 3_141_592_653_590;
    const PI_SCALE: u128 = 1_000_000_000_000;
    const DENOMINATOR: u128 = 8 * PI_SCALE;

    let mut diameter_integral = 0_u128;
    for pair in log.stations.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        let left_product = u128::from(left.major_diameter_mm)
            .checked_mul(u128::from(left.minor_diameter_mm))
            .ok_or_else(|| format!("{}: left section area product overflow", log.log_id))?;
        let right_product = u128::from(right.major_diameter_mm)
            .checked_mul(u128::from(right.minor_diameter_mm))
            .ok_or_else(|| format!("{}: right section area product overflow", log.log_id))?;
        let paired_area = left_product
            .checked_add(right_product)
            .ok_or_else(|| format!("{}: paired section area overflow", log.log_id))?;
        let dx = u128::from(right.axial_position_mm - left.axial_position_mm);
        let segment = paired_area
            .checked_mul(dx)
            .ok_or_else(|| format!("{}: geometry segment volume overflow", log.log_id))?;
        diameter_integral = diameter_integral
            .checked_add(segment)
            .ok_or_else(|| format!("{}: integrated geometry volume overflow", log.log_id))?;
    }
    let scaled_volume = diameter_integral
        .checked_mul(PI_SCALED)
        .ok_or_else(|| format!("{}: scaled geometry volume overflow", log.log_id))?;
    let rounded = scaled_volume
        .checked_add(DENOMINATOR / 2)
        .ok_or_else(|| format!("{}: geometry rounding overflow", log.log_id))?
        / DENOMINATOR;
    if rounded == 0 || rounded > u128::from(u64::MAX) {
        return Err(format!("{}: geometry-derived volume is outside representable bounds", log.log_id));
    }
    Ok(rounded)
}

fn validate(c: &Corpus) -> Check<()> {
    nonempty(&c.suite_id, "suite_id", "corpus")?;
    nonempty(&c.suite_version, "suite_version", "corpus")?;
    nonempty(&c.claim_ceiling, "claim_ceiling", "corpus")?;
    if c.evidence_class != "synthetic" { return Err("v0.1 accepts only synthetic data".into()); }
    if c.units.length != "mm" || c.units.volume != "mm3" || c.units.uncertainty != "parts_per_million" {
        return Err("units must be mm, mm3, and parts_per_million".into());
    }
    if c.scenarios.is_empty() || c.scenarios.len() > 100 { return Err("scenario count must be 1..=100".into()); }
    let mut scenarios = BTreeSet::new();
    let mut logs = BTreeSet::new();
    for s in &c.scenarios {
        validate_scenario(s)?;
        if !scenarios.insert(s.scenario_id.as_str()) { return Err(format!("duplicate scenario_id {}", s.scenario_id)); }
        if !logs.insert(s.log.log_id.as_str()) { return Err(format!("duplicate log_id {}", s.log.log_id)); }
    }
    Ok(())
}
fn validate_scenario(s: &Scenario) -> Check<()> {
    let ctx = s.scenario_id.as_str();
    nonempty(ctx, "scenario_id", "scenario")?;
    if s.evidence_class != "synthetic" { return Err(format!("{ctx}: v0.1 accepts synthetic scenarios only")); }
    if s.accounting_tolerance_ppm > 1_000_000 { return Err(format!("{ctx}: accounting tolerance exceeds 100%")); }
    if s.geometry_tolerance_ppm > 1_000_000 { return Err(format!("{ctx}: geometry tolerance exceeds 100%")); }
    let l = &s.log;
    nonempty(&l.log_id, "log_id", ctx)?;
    nonempty(&l.species_code, "species_code", ctx)?;
    nonempty(&l.gross_volume_method, "gross_volume_method", ctx)?;
    if l.length_mm == 0 || l.length_mm > MAX_MM || l.gross_volume_mm3 == 0 {
        return Err(format!("{ctx}: invalid log length or gross volume"));
    }
    nonempty(&l.source.source_id, "source.source_id", ctx)?;
    nonempty(&l.source.license, "source.license", ctx)?;
    nonempty(&l.source.method, "source.method", ctx)?;
    valid_date(&l.source.measured_at, "source.measured_at", ctx)?;
    if let Some(id) = &l.source.instrument_id { nonempty(id, "source.instrument_id", ctx)?; }
    if l.source.calibration_status == CalibrationStatus::Current && l.source.instrument_id.is_none() {
        return Err(format!("{ctx}: current calibration requires an instrument_id"));
    }
    if l.source.uncertainty_ppm > 1_000_000 { return Err(format!("{ctx}: source uncertainty exceeds 100%")); }
    nonempty(&l.geometry_source.source_id, "geometry_source.source_id", ctx)?;
    nonempty(&l.geometry_source.license, "geometry_source.license", ctx)?;
    nonempty(&l.geometry_source.method, "geometry_source.method", ctx)?;
    valid_date(&l.geometry_source.measured_at, "geometry_source.measured_at", ctx)?;
    if let Some(id) = &l.geometry_source.instrument_id { nonempty(id, "geometry_source.instrument_id", ctx)?; }
    if l.geometry_source.calibration_status == CalibrationStatus::Current && l.geometry_source.instrument_id.is_none() {
        return Err(format!("{ctx}: current geometry calibration requires an instrument_id"));
    }
    if l.geometry_source.uncertainty_ppm > 1_000_000 { return Err(format!("{ctx}: geometry source uncertainty exceeds 100%")); }
    if l.stations.len() < 2 || l.stations.len() > 1000 { return Err(format!("{ctx}: station count must be 2..=1000")); }
    if l.stations[0].axial_position_mm != 0 || l.stations.last().map(|p| p.axial_position_mm) != Some(l.length_mm) {
        return Err(format!("{ctx}: stations must span from 0 to log length"));
    }
    let mut previous = None;
    let mut max_diameter = 0_u64;
    for p in &l.stations {
        if p.axial_position_mm > l.length_mm || previous.is_some_and(|n| p.axial_position_mm <= n) {
            return Err(format!("{ctx}: station positions must be strictly increasing within the log"));
        }
        previous = Some(p.axial_position_mm);
        if p.major_diameter_mm == 0 || p.minor_diameter_mm == 0 || p.major_diameter_mm > MAX_MM || p.minor_diameter_mm > p.major_diameter_mm {
            return Err(format!("{ctx}: section diameters must be positive with major >= minor"));
        }
        if p.ellipse_rotation_mdeg >= 360_000 || p.uncertainty_ppm > 1_000_000 {
            return Err(format!("{ctx}: invalid section rotation or uncertainty"));
        }
        if p.center_x_mm.unsigned_abs() > MAX_MM || p.center_y_mm.unsigned_abs() > MAX_MM {
            return Err(format!("{ctx}: section center offsets exceed measurement bounds"));
        }
        max_diameter = max_diameter.max(p.major_diameter_mm);
    }
    if l.defects.len() > 1000 { return Err(format!("{ctx}: defect count exceeds 1000")); }
    let mut defect_ids = BTreeSet::new();
    for d in &l.defects {
        nonempty(&d.defect_id, "defect_id", ctx)?;
        if !defect_ids.insert(d.defect_id.as_str()) { return Err(format!("{ctx}: duplicate defect_id {}", d.defect_id)); }
        if d.axial_start_mm >= d.axial_end_mm || d.axial_end_mm > l.length_mm {
            return Err(format!("{ctx}: defect extent must be non-empty and inside the log"));
        }
        if d.circumferential_angle_mdeg.is_some_and(|a| a >= 360_000) {
            return Err(format!("{ctx}: defect angle must be in [0, 360000) millidegrees"));
        }
        if d.radial_depth_mm.is_some_and(|n| n == 0 || n > max_diameter / 2) {
            return Err(format!("{ctx}: radial depth must be positive and no more than the maximum measured radius"));
        }
    }
    let m = &s.machine;
    nonempty(&m.machine_id, "machine_id", ctx)?;
    nonempty(&m.grade_rule_id, "grade_rule_id", ctx)?;
    nonempty(&m.grade_rule_version, "grade_rule_version", ctx)?;
    if m.kerf_mm == 0 || m.kerf_mm > MAX_MM || m.sawing_variation_mm > MAX_MM
        || m.target_thickness_mm == 0 || m.target_width_mm == 0
        || m.target_thickness_mm > MAX_MM || m.target_width_mm > MAX_MM || m.green_allowance_mm > MAX_MM {
        return Err(format!("{ctx}: invalid machine setup dimensions"));
    }
    let o = &s.observation;
    valid_date(&o.observed_at, "observation.observed_at", ctx)?;
    if o.measurement_uncertainty_ppm > 1_000_000 || o.boards.len() > 10_000 {
        return Err(format!("{ctx}: invalid observation uncertainty or too many boards"));
    }
    let mut board_ids = BTreeSet::new();
    for b in &o.boards {
        nonempty(&b.board_id, "board_id", ctx)?;
        if !board_ids.insert(b.board_id.as_str()) { return Err(format!("{ctx}: duplicate board_id {}", b.board_id)); }
        if b.length_mm == 0 || b.width_mm == 0 || b.thickness_mm == 0
            || b.length_mm > l.length_mm || b.width_mm > MAX_MM || b.thickness_mm > MAX_MM {
            return Err(format!("{ctx}: invalid observed board dimensions"));
        }
        if b.grade_label.as_ref().is_some_and(|g| g.trim().is_empty()) {
            return Err(format!("{ctx}: empty grade_label must be omitted"));
        }
        if b.intended_application == IntendedApplication::Structural
            && (b.grade_evidence_state != EvidenceState::Qualified || !b.inspection_pass) {
            return Err(format!("{ctx}: structural board {} lacks qualified grade evidence and passing inspection", b.board_id));
        }
    }
    let (board_vol, byproduct_vol, combined) = totals(s)?;
    let gross = u128::from(l.gross_volume_mm3);
    let tolerance = gross * u128::from(s.accounting_tolerance_ppm) / 1_000_000;
    match o.status {
        ObservationStatus::Complete => {
            if combined.abs_diff(gross) > tolerance {
                return Err(format!("{ctx}: complete observation accounting differs from gross volume by {} mm3 (tolerance {tolerance} mm3)", combined.abs_diff(gross)));
            }
        }
        ObservationStatus::Partial => {
            if combined > gross.saturating_add(tolerance) { return Err(format!("{ctx}: partial outputs exceed gross volume plus tolerance")); }
        }
        ObservationStatus::Unknown => {
            if !o.boards.is_empty() || board_vol != 0 || byproduct_vol != 0 {
                return Err(format!("{ctx}: unknown status must not carry quantitative outputs"));
            }
        }
    }
    Ok(())
}
fn summary(s: &Scenario) -> Check<ScenarioSummary> {
    let (boards, byproducts, combined) = totals(s)?;
    let gross = u128::from(s.log.gross_volume_mm3);
    let delta = (s.observation.status == ObservationStatus::Complete).then(|| combined.abs_diff(gross));
    let geometry_volume = estimated_geometry_volume_mm3(&s.log)?;
    let geometry_delta = geometry_volume.abs_diff(gross);
    let geometry_tolerance = gross * u128::from(s.geometry_tolerance_ppm) / 1_000_000;
    let absolute_geometry_percentage_error_ppm = percent_error_ppm(geometry_delta, gross)?;
    let geometry_target_met = geometry_delta <= geometry_tolerance;
    let status = match s.observation.status {
        ObservationStatus::Complete => "within_tolerance",
        ObservationStatus::Partial => "partial_only",
        ObservationStatus::Unknown => "unknown",
    };
    Ok(ScenarioSummary {
        scenario_id: s.scenario_id.clone(), log_id: s.log.log_id.clone(), evidence_class: s.evidence_class.clone(),
        length_mm: s.log.length_mm, section_station_count: s.log.stations.len(), defect_count: s.log.defects.len(),
        gross_log_volume_mm3: s.log.gross_volume_mm3,
        species_evidence_state: s.log.species_evidence_state,
        reference_source_id: s.log.source.source_id.clone(),
        reference_source_license: s.log.source.license.clone(),
        reference_source_method: s.log.source.method.clone(),
        reference_source_measured_at: s.log.source.measured_at.clone(),
        reference_source_calibration_status: s.log.source.calibration_status,
        reference_source_uncertainty_ppm: s.log.source.uncertainty_ppm,
        sawing_variation_mm: s.machine.sawing_variation_mm,
        defects: s.log.defects.iter().map(|d| DefectSummary {
            defect_id: d.defect_id.clone(), kind: d.kind, axial_start_mm: d.axial_start_mm,
            axial_end_mm: d.axial_end_mm, evidence_state: d.evidence_state,
        }).collect(),
        geometry_volume_estimate_mm3: geometry_volume,
        absolute_geometry_delta_mm3: geometry_delta, geometry_tolerance_mm3: geometry_tolerance,
        absolute_geometry_percentage_error_ppm, geometry_target_met,
        geometry_source_id: s.log.geometry_source.source_id.clone(),
        geometry_source_license: s.log.geometry_source.license.clone(),
        geometry_source_method: s.log.geometry_source.method.clone(),
        geometry_source_measured_at: s.log.geometry_source.measured_at.clone(),
        geometry_source_calibration_status: s.log.geometry_source.calibration_status,
        geometry_source_uncertainty_ppm: s.log.geometry_source.uncertainty_ppm,
        observed_board_count: s.observation.boards.len(),
        observed_board_volume_mm3: boards, known_byproduct_volume_mm3: byproducts, known_output_volume_mm3: combined,
        absolute_accounting_delta_mm3: delta, accounting_status: status,
        grade_rule_id: s.machine.grade_rule_id.clone(), grade_rule_version: s.machine.grade_rule_version.clone(),
    })
}
fn aggregate_geometry_metrics(summaries: &[ScenarioSummary]) -> Check<AggregateGeometryMetrics> {
    if summaries.is_empty() {
        return Err("cannot aggregate an empty geometry benchmark".to_owned());
    }
    let mut errors = summaries.iter()
        .map(|s| s.absolute_geometry_percentage_error_ppm)
        .collect::<Vec<_>>();
    errors.sort_unstable();
    let error_sum = errors.iter().try_fold(0_u128, |sum, value| {
        sum.checked_add(u128::from(*value))
            .ok_or_else(|| "aggregate percentage error overflow".to_owned())
    })?;
    let mean = error_sum / errors.len() as u128;
    let median = if errors.len() % 2 == 1 {
        u128::from(errors[errors.len() / 2])
    } else {
        (u128::from(errors[errors.len() / 2 - 1])
            + u128::from(errors[errors.len() / 2])) / 2
    };
    let total_delta = summaries.iter().try_fold(0_u128, |sum, s| {
        sum.checked_add(s.absolute_geometry_delta_mm3)
            .ok_or_else(|| "aggregate volume error overflow".to_owned())
    })?;
    let total_reference = summaries.iter().try_fold(0_u128, |sum, s| {
        sum.checked_add(u128::from(s.gross_log_volume_mm3))
            .ok_or_else(|| "aggregate reference volume overflow".to_owned())
    })?;
    let weighted = percent_error_ppm(total_delta, total_reference)?;
    let within = summaries.iter().filter(|s| s.geometry_target_met).count();
    let fraction = (within as u128 * 1_000_000 + summaries.len() as u128 / 2)
        / summaries.len() as u128;
    Ok(AggregateGeometryMetrics {
        evaluated_log_count: summaries.len(),
        mean_absolute_percentage_error_ppm: u64::try_from(mean)
            .map_err(|_| "mean percentage error does not fit u64".to_owned())?,
        median_absolute_percentage_error_ppm: u64::try_from(median)
            .map_err(|_| "median percentage error does not fit u64".to_owned())?,
        max_absolute_percentage_error_ppm: *errors.last().expect("non-empty checked above"),
        volume_weighted_absolute_percentage_error_ppm: weighted,
        within_declared_target_count: within,
        within_declared_target_fraction_ppm: u64::try_from(fraction)
            .map_err(|_| "target coverage fraction does not fit u64".to_owned())?,
    })
}

fn execute() -> Check<()> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| "usage: sawlog-benchmark CORPUS.json --source-commit <40-hex-sha>".to_owned())?;
    if args.next().as_deref() != Some("--source-commit") { return Err("expected --source-commit".into()); }
    let commit = args.next().ok_or_else(|| "missing source commit SHA".to_owned())?;
    if args.next().is_some() { return Err("unexpected trailing argument".into()); }
    if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) { return Err("source commit must be 40 hex characters".into()); }
    let bytes = fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let corpus: Corpus = serde_json::from_slice(&bytes).map_err(|e| format!("invalid corpus JSON: {e}"))?;
    validate(&corpus)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let summaries = corpus.scenarios.iter().map(summary).collect::<Check<Vec<_>>>()?;
    let geometry_metrics = aggregate_geometry_metrics(&summaries)?;
    let receipt = Receipt {
        result: "validated", validator_version: VERSION, suite_id: corpus.suite_id.clone(),
        suite_version: corpus.suite_version.clone(), claim_ceiling: corpus.claim_ceiling.clone(),
        scenario_count: summaries.len(), corpus_sha256: digest, source_commit: commit,
        geometry_metrics, scenarios: summaries,
    };
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?);
    Ok(())
}
fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("sawlog-benchmark: {e}"); ExitCode::from(2) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../docs/tech/sawlog-benchmark-corpus-v0.1.json");
    fn corpus() -> Corpus { serde_json::from_str(FIXTURE).expect("fixture parses") }

    #[test]
    fn frozen_corpus_validates_with_both_accounting_states() {
        let c = corpus();
        validate(&c).unwrap();
        assert_eq!(c.scenarios.len(), 3);
        assert_eq!(c.scenarios[0].log.stations.len(), 4);
        assert_eq!(c.scenarios[1].log.defects.len(), 2);
        assert!(!summary(&c.scenarios[2]).unwrap().geometry_target_met);
        assert_eq!(c.scenarios[0].observation.boards.len(), 2);
        assert_eq!(c.scenarios[1].observation.boards.len(), 3);
        assert_eq!(summary(&c.scenarios[0]).unwrap().accounting_status, "within_tolerance");
        assert_eq!(summary(&c.scenarios[1]).unwrap().accounting_status, "partial_only");
    }
    #[test]
    fn impossible_calendar_dates_are_rejected_but_leap_days_are_allowed() {
        assert!(valid_date("2024-02-29", "date", "test").is_ok());
        assert!(valid_date("2026-02-28", "date", "test").is_ok());
        assert!(valid_date("2026-02-30", "date", "test").is_err());
        assert!(valid_date("2025-02-29", "date", "test").is_err());
        assert!(valid_date("2026-13-01", "date", "test").is_err());
    }

    #[test]
    fn unordered_geometry_fails_closed() {
        let mut c = corpus();
        c.scenarios[0].log.stations.swap(1, 2);
        assert!(validate(&c).unwrap_err().contains("strictly increasing"));
    }
    #[test]
    #[test]
    fn current_calibration_requires_an_instrument_identity() {
        let mut c = corpus();
        c.scenarios[0].log.geometry_source.calibration_status = CalibrationStatus::Current;
        c.scenarios[0].log.geometry_source.instrument_id = None;
        assert!(validate(&c).unwrap_err().contains("current geometry calibration requires an instrument_id"));
    }
    #[test]
    fn fixed_point_geometry_integrator_has_frozen_reference_values() {
        let c = corpus();
        let expected = [245_044_227_u128, 280_230_065_u128, 245_044_227_u128];
        for (scenario, expected_volume) in c.scenarios.iter().zip(expected) {
            assert_eq!(
                estimated_geometry_volume_mm3(&scenario.log).unwrap(),
                expected_volume
            );
        }
    }

    #[test]
    fn geometry_error_is_scored_not_discarded_as_invalid_data() {
        let mut c = corpus();
        c.scenarios[1].log.gross_volume_mm3 = 304_000_000;
        validate(&c).expect("valid but inaccurate estimates remain scoreable");
        let scored = summary(&c.scenarios[1]).unwrap();
        assert!(!scored.geometry_target_met);
        assert!(scored.absolute_geometry_percentage_error_ppm > 50_000);
    }
    #[test]
    fn aggregate_reports_median_mean_weighted_error_and_target_coverage() {
        let c = corpus();
        validate(&c).unwrap();
        let summaries = c.scenarios.iter().map(summary).collect::<Check<Vec<_>>>().unwrap();
        let metrics = aggregate_geometry_metrics(&summaries).unwrap();
        assert_eq!(metrics.evaluated_log_count, 3);
        assert_eq!(metrics.within_declared_target_count, 2);
        assert_eq!(metrics.within_declared_target_fraction_ppm, 666_667);
        assert!(metrics.max_absolute_percentage_error_ppm > metrics.median_absolute_percentage_error_ppm);
        assert!(metrics.volume_weighted_absolute_percentage_error_ppm > 0);
    }
    #[test]
    fn complete_ledger_cannot_hide_volume_gap() {
        let mut c = corpus();
        c.scenarios[0].observation.byproducts.slabs_and_trim_volume_mm3 += 10_000_000;
        assert!(validate(&c).unwrap_err().contains("complete observation accounting differs"));
    }
    #[test]
    fn structural_claim_requires_qualified_evidence_and_inspection() {
        let mut c = corpus();
        c.scenarios[1].observation.boards[0].inspection_pass = false;
        assert!(validate(&c).unwrap_err().contains("lacks qualified grade evidence"));
    }
    #[test]
    fn unknown_fields_are_rejected() {
        let mut value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        value["unexpected_claim"] = serde_json::Value::String("not allowed".into());
        assert!(serde_json::from_value::<Corpus>(value).unwrap_err().to_string().contains("unexpected_claim"));
    }
    #[test]
    fn receipt_is_deterministic_and_binds_exact_fixture() {
        let c = corpus();
        validate(&c).unwrap();
        let digest = format!("{:x}", Sha256::digest(FIXTURE.as_bytes()));
        let build = || {
            serde_json::to_string(&Receipt {
                result: "validated", validator_version: VERSION, suite_id: c.suite_id.clone(),
                suite_version: c.suite_version.clone(), claim_ceiling: c.claim_ceiling.clone(),
                scenario_count: 3, corpus_sha256: digest.clone(), source_commit: "a".repeat(40),
                geometry_metrics: aggregate_geometry_metrics(&c.scenarios.iter().map(summary).collect::<Check<Vec<_>>>().unwrap()).unwrap(),
                scenarios: c.scenarios.iter().map(summary).collect::<Check<Vec<_>>>().unwrap(),
            }).unwrap()
        };
        assert_eq!(build(), build());
    }
}
