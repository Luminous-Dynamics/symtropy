#![forbid(unsafe_code)]

//! Installation-grouped transfer qualification for in-situ torque sensors.
//!
//! This M1 reference calibrates an additive interval expansion using the
//! maximum expansion miss over each independent calibration installation.
//! It then evaluates whole-installation coverage on disjoint test hardware.
//! Angle, temperature, speed and torque quadrant are retained as strata.
//!
//! This is a synthetic/statistical reference, not a sensor compensation
//! algorithm, traceability verifier, physical sensor result or safety case.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TorqueDirection {
    ForwardMotoring,
    ForwardRegenerating,
    ReverseMotoring,
    ReverseRegenerating,
    StaticHold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceEvidenceClass {
    IndependentPhysical,
    SourceAuthorPhysical,
    VendorPhysical,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    Calibrated,
    InsufficientCalibrationInstallations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationDisposition {
    Covered,
    Uncovered,
    OutOfDomain,
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorDomain {
    pub joint_angle_rad: NumericInterval,
    pub temperature_c: NumericInterval,
    pub maximum_absolute_speed_rad_s: f64,
    pub supported_directions: Vec<TorqueDirection>,
    /// Strictly increasing edges covering the complete declared angle domain.
    pub angle_sector_edges_rad: Vec<f64>,
    /// Strictly increasing edges covering the complete declared temperature domain.
    pub temperature_band_edges_c: Vec<f64>,
    /// Strictly increasing edges covering [0, maximum_absolute_speed_rad_s].
    pub speed_band_edges_rad_s: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferPlan {
    pub model_digest: String,
    pub feature_schema_digest: String,
    pub training_split_digest: String,
    pub calibration_split_digest: String,
    pub test_split_digest: String,
    pub system_boundary_digest: String,
    pub sensor_family_digest: String,
    pub reference_method_digest: String,
    pub domain_digest: String,
    pub target_miscoverage_alpha: f64,
    pub reference_evidence_class: ReferenceEvidenceClass,
    pub training_installation_ids: Vec<String>,
    pub training_sensor_serials: Vec<String>,
    pub training_flexspline_serials: Vec<String>,
    pub training_reducer_serials: Vec<String>,
    pub domain: SensorDomain,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorObservation {
    pub sample_id: String,
    pub joint_angle_rad: f64,
    pub temperature_c: f64,
    pub angular_speed_rad_s: f64,
    pub direction: TorqueDirection,
    /// Raw sensor's uncorrected indicated-torque interval, N m.
    pub raw_torque_nm: NumericInterval,
    /// Compensation model's indicated-torque interval, N m.
    pub compensated_torque_nm: NumericInterval,
    /// Independent external reference-transducer interval, N m.
    pub reference_torque_nm: NumericInterval,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallationGroup {
    pub installation_id: String,
    pub sensor_serial: String,
    pub flexspline_serial: String,
    pub reducer_serial: String,
    pub installation_digest: String,
    pub sensor_assembly_digest: String,
    pub calibration_digest: String,
    pub reference_calibration_digest: String,
    pub reference_evidence_class: ReferenceEvidenceClass,
    pub reference_source_reference: String,
    pub reference_method: String,
    pub reference_raw_trace_digest: String,
    pub sensor_raw_trace_digest: String,
    /// Receipt digest from the synchronized torque/sensor metrology chain.
    pub synchronized_metrology_receipt_digest: String,
    pub observations: Vec<SensorObservation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferDataset {
    pub split_digest: String,
    pub model_digest: String,
    pub feature_schema_digest: String,
    pub system_boundary_digest: String,
    pub sensor_family_digest: String,
    pub reference_method_digest: String,
    pub domain_digest: String,
    pub installations: Vec<InstallationGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallationCalibrationScore {
    pub installation_id: String,
    pub sensor_serial: String,
    pub maximum_required_expansion_nm: f64,
    pub reference_source_reference: String,
    pub reference_raw_trace_digest: String,
    pub reference_calibration_digest: String,
    pub synchronized_metrology_receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallationAssessment {
    pub installation_id: String,
    pub sensor_serial: String,
    pub flexspline_serial: String,
    pub reducer_serial: String,
    pub disposition: InstallationDisposition,
    pub observation_count: usize,
    pub maximum_raw_interval_miss_nm: Option<f64>,
    pub maximum_compensated_interval_miss_nm: Option<f64>,
    pub maximum_calibrated_interval_miss_nm: Option<f64>,
    pub reference_source_reference: String,
    pub reference_raw_trace_digest: String,
    pub calibration_digest: String,
    pub synchronized_metrology_receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConditionStratum {
    pub angle_sector: usize,
    pub temperature_band: usize,
    pub speed_band: usize,
    pub direction: TorqueDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StratumMetrics {
    pub stratum: ConditionStratum,
    pub sample_count: usize,
    pub raw_mean_centre_error_nm: f64,
    pub compensated_mean_centre_error_nm: f64,
    pub calibrated_interval_covered_count: usize,
    pub calibrated_interval_coverage: f64,
    pub compensated_mean_interval_miss_nm: f64,
    pub calibrated_mean_interval_miss_nm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferReport {
    pub status: TransferStatus,
    pub model_digest: String,
    pub feature_schema_digest: String,
    pub training_split_digest: String,
    pub calibration_split_digest: String,
    pub test_split_digest: String,
    pub system_boundary_digest: String,
    pub sensor_family_digest: String,
    pub reference_method_digest: String,
    pub domain_digest: String,
    pub target_miscoverage_alpha: f64,
    pub calibration_installation_count: usize,
    pub conformal_order_statistic_rank: usize,
    pub correction_q_nm: Option<f64>,
    pub test_installation_count: usize,
    pub in_domain_test_installation_count: usize,
    pub covered_test_installation_count: usize,
    pub uncovered_test_installation_count: usize,
    pub out_of_domain_test_installation_count: usize,
    pub empirical_installation_coverage: Option<f64>,
    pub wilson_95_lower: Option<f64>,
    pub wilson_95_upper: Option<f64>,
    pub mean_raw_centre_error_nm: Option<f64>,
    pub mean_compensated_centre_error_nm: Option<f64>,
    pub mean_calibrated_interval_miss_nm: Option<f64>,
    pub mean_raw_interval_width_nm: Option<f64>,
    pub mean_compensated_interval_width_nm: Option<f64>,
    pub mean_calibrated_interval_width_nm: Option<f64>,
    pub raw_improvement_nm: Option<f64>,
    pub calibration_scores: Vec<InstallationCalibrationScore>,
    pub installations: Vec<InstallationAssessment>,
    pub strata: Vec<StratumMetrics>,
    pub interpretation: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite_interval(value: NumericInterval) -> bool {
    value.lower.is_finite() && value.upper.is_finite() && value.lower <= value.upper
}

fn interval_expansion_required(
    estimate: NumericInterval,
    reference: NumericInterval,
) -> f64 {
    (estimate.lower - reference.lower)
        .max(reference.upper - estimate.upper)
        .max(0.0)
}

fn interval_miss(estimate: NumericInterval, reference: NumericInterval) -> f64 {
    (estimate.lower - reference.upper)
        .max(reference.lower - estimate.upper)
        .max(0.0)
}

fn centre(interval: NumericInterval) -> f64 {
    interval.lower + (interval.upper - interval.lower) * 0.5
}

fn fully_covers(estimate: NumericInterval, reference: NumericInterval) -> bool {
    estimate.lower <= reference.lower && estimate.upper >= reference.upper
}

fn wilson_95(successes: usize, trials: usize) -> Option<(f64, f64)> {
    if trials == 0 {
        return None;
    }
    const Z: f64 = 1.959_963_984_540_054;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = Z * Z;
    let denominator = 1.0 + z2 / n;
    let midpoint = (p + z2 / (2.0 * n)) / denominator;
    let half = Z * ((p * (1.0 - p) / n) + (z2 / (4.0 * n * n))).sqrt() / denominator;
    Some(((midpoint - half).max(0.0), (midpoint + half).min(1.0)))
}

fn valid_edges(edges: &[f64], lower: f64, upper: f64, label: &str) -> Result<(), String> {
    if edges.len() < 2 {
        return Err(format!("{label} requires at least two bin edges"));
    }
    if edges.iter().any(|value| !value.is_finite()) {
        return Err(format!("{label} edges must be finite"));
    }
    if edges[0] != lower || *edges.last().unwrap() != upper {
        return Err(format!("{label} edges must exactly cover the declared domain"));
    }
    for pair in edges.windows(2) {
        if pair[1] <= pair[0] {
            return Err(format!("{label} edges must be strictly increasing"));
        }
    }
    Ok(())
}

fn validate_plan(plan: &TransferPlan) -> Result<(), String> {
    for (label, value) in [
        ("model digest", plan.model_digest.as_str()),
        ("feature-schema digest", plan.feature_schema_digest.as_str()),
        ("training split digest", plan.training_split_digest.as_str()),
        ("calibration split digest", plan.calibration_split_digest.as_str()),
        ("test split digest", plan.test_split_digest.as_str()),
        ("system-boundary digest", plan.system_boundary_digest.as_str()),
        ("sensor-family digest", plan.sensor_family_digest.as_str()),
        ("reference-method digest", plan.reference_method_digest.as_str()),
        ("domain digest", plan.domain_digest.as_str()),
    ] {
        if !nonempty(value) {
            return Err(format!("{label} is required"));
        }
    }

    let splits = [
        plan.training_split_digest.as_str(),
        plan.calibration_split_digest.as_str(),
        plan.test_split_digest.as_str(),
    ];
    if splits[0] == splits[1] || splits[0] == splits[2] || splits[1] == splits[2] {
        return Err("training, calibration and test split digests must be distinct".to_owned());
    }
    if !plan.target_miscoverage_alpha.is_finite()
        || !(0.0..1.0).contains(&plan.target_miscoverage_alpha)
    {
        return Err("target miscoverage alpha must be finite and strictly between zero and one".to_owned());
    }
    if plan.reference_evidence_class != ReferenceEvidenceClass::IndependentPhysical {
        return Err("external torque-reference evidence must be independently measured".to_owned());
    }

    let domain = &plan.domain;
    if !finite_interval(domain.joint_angle_rad)
        || !finite_interval(domain.temperature_c)
        || !domain.maximum_absolute_speed_rad_s.is_finite()
        || domain.maximum_absolute_speed_rad_s <= 0.0
    {
        return Err("sensor operating domain must contain finite ordered ranges and positive speed limit".to_owned());
    }
    valid_edges(
        &domain.angle_sector_edges_rad,
        domain.joint_angle_rad.lower,
        domain.joint_angle_rad.upper,
        "angle-sector",
    )?;
    valid_edges(
        &domain.temperature_band_edges_c,
        domain.temperature_c.lower,
        domain.temperature_c.upper,
        "temperature-band",
    )?;
    valid_edges(
        &domain.speed_band_edges_rad_s,
        0.0,
        domain.maximum_absolute_speed_rad_s,
        "speed-band",
    )?;
    if domain.supported_directions.is_empty() {
        return Err("at least one torque direction must be supported".to_owned());
    }
    let mut directions = BTreeSet::new();
    if domain.supported_directions.iter().any(|direction| !directions.insert(*direction)) {
        return Err("supported torque directions must be unique".to_owned());
    }

    if plan.training_installation_ids.is_empty()
        || plan.training_sensor_serials.is_empty()
        || plan.training_flexspline_serials.is_empty()
        || plan.training_reducer_serials.is_empty()
    {
        return Err("training installation, sensor, flexspline and reducer identities must be explicitly enumerated".to_owned());
    }

    let mut training_installations = BTreeSet::new();
    for id in &plan.training_installation_ids {
        if !nonempty(id) || !training_installations.insert(id.as_str()) {
            return Err("training installation IDs must be nonempty and unique".to_owned());
        }
    }
    let mut training_serials = BTreeSet::new();
    for id in &plan.training_sensor_serials {
        if !nonempty(id) || !training_serials.insert(id.as_str()) {
            return Err("training sensor serials must be nonempty and unique".to_owned());
        }
    }
    let mut training_flexsplines = BTreeSet::new();
    for id in &plan.training_flexspline_serials {
        if !nonempty(id) || !training_flexsplines.insert(id.as_str()) {
            return Err("training flexspline serials must be nonempty and unique".to_owned());
        }
    }
    let mut training_reducers = BTreeSet::new();
    for id in &plan.training_reducer_serials {
        if !nonempty(id) || !training_reducers.insert(id.as_str()) {
            return Err("training reducer serials must be nonempty and unique".to_owned());
        }
    }
    Ok(())
}

fn direction_is_consistent(sample: &SensorObservation) -> bool {
    let torque = sample.reference_torque_nm;
    let speed = sample.angular_speed_rad_s;
    match sample.direction {
        TorqueDirection::ForwardMotoring => torque.lower > 0.0 && speed > 0.0,
        TorqueDirection::ForwardRegenerating => torque.upper < 0.0 && speed > 0.0,
        TorqueDirection::ReverseMotoring => torque.upper < 0.0 && speed < 0.0,
        TorqueDirection::ReverseRegenerating => torque.lower > 0.0 && speed < 0.0,
        TorqueDirection::StaticHold => speed == 0.0,
    }
}

fn condition_is_in_domain(plan: &TransferPlan, sample: &SensorObservation) -> bool {
    let domain = &plan.domain;
    sample.joint_angle_rad >= domain.joint_angle_rad.lower
        && sample.joint_angle_rad <= domain.joint_angle_rad.upper
        && sample.temperature_c >= domain.temperature_c.lower
        && sample.temperature_c <= domain.temperature_c.upper
        && sample.angular_speed_rad_s.abs() <= domain.maximum_absolute_speed_rad_s
        && domain.supported_directions.contains(&sample.direction)
        && direction_is_consistent(sample)
}

fn bin_index(edges: &[f64], value: f64) -> Option<usize> {
    if !value.is_finite() || value < edges[0] || value > *edges.last().unwrap() {
        return None;
    }
    if value == *edges.last().unwrap() {
        return Some(edges.len() - 2);
    }
    let upper = edges.partition_point(|edge| *edge <= value);
    upper.checked_sub(1)
}

fn condition_stratum(plan: &TransferPlan, sample: &SensorObservation) -> Option<ConditionStratum> {
    Some(ConditionStratum {
        angle_sector: bin_index(&plan.domain.angle_sector_edges_rad, sample.joint_angle_rad)?,
        temperature_band: bin_index(&plan.domain.temperature_band_edges_c, sample.temperature_c)?,
        speed_band: bin_index(
            &plan.domain.speed_band_edges_rad_s,
            sample.angular_speed_rad_s.abs(),
        )?,
        direction: sample.direction,
    })
}

fn validate_dataset(
    dataset: &TransferDataset,
    expected_split: &str,
    plan: &TransferPlan,
    is_calibration: bool,
    installation_ids: &mut BTreeSet<String>,
    sensor_serials: &mut BTreeSet<String>,
    flexspline_serials: &mut BTreeSet<String>,
    reducer_serials: &mut BTreeSet<String>,
    installation_digests: &mut BTreeSet<String>,
    sensor_assembly_digests: &mut BTreeSet<String>,
    sample_ids: &mut BTreeSet<String>,
) -> Result<usize, String> {
    if dataset.split_digest != expected_split {
        return Err("dataset split digest does not match frozen plan".to_owned());
    }
    if dataset.model_digest != plan.model_digest
        || dataset.feature_schema_digest != plan.feature_schema_digest
        || dataset.system_boundary_digest != plan.system_boundary_digest
        || dataset.sensor_family_digest != plan.sensor_family_digest
        || dataset.reference_method_digest != plan.reference_method_digest
        || dataset.domain_digest != plan.domain_digest
    {
        return Err("dataset model, feature, boundary, sensor-family, reference-method or domain digest mismatch".to_owned());
    }
    if dataset.installations.is_empty() {
        return Err("calibration and test datasets must contain installations".to_owned());
    }

    let training_installations: BTreeSet<&str> =
        plan.training_installation_ids.iter().map(String::as_str).collect();
    let training_serials: BTreeSet<&str> =
        plan.training_sensor_serials.iter().map(String::as_str).collect();
    let training_flexsplines: BTreeSet<&str> =
        plan.training_flexspline_serials.iter().map(String::as_str).collect();
    let training_reducers: BTreeSet<&str> =
        plan.training_reducer_serials.iter().map(String::as_str).collect();
    let mut sample_count = None;

    for installation in &dataset.installations {
        for (label, value) in [
            ("installation ID", installation.installation_id.as_str()),
            ("sensor serial", installation.sensor_serial.as_str()),
            ("flexspline serial", installation.flexspline_serial.as_str()),
            ("reducer serial", installation.reducer_serial.as_str()),
            ("installation digest", installation.installation_digest.as_str()),
            ("sensor assembly digest", installation.sensor_assembly_digest.as_str()),
            ("sensor calibration digest", installation.calibration_digest.as_str()),
            ("reference calibration digest", installation.reference_calibration_digest.as_str()),
            ("reference source", installation.reference_source_reference.as_str()),
            ("reference method", installation.reference_method.as_str()),
            ("reference raw-trace digest", installation.reference_raw_trace_digest.as_str()),
            ("sensor raw-trace digest", installation.sensor_raw_trace_digest.as_str()),
            ("synchronized metrology receipt digest", installation.synchronized_metrology_receipt_digest.as_str()),
        ] {
            if !nonempty(value) {
                return Err(format!("{label} is required"));
            }
        }
        if installation.reference_evidence_class != plan.reference_evidence_class {
            return Err("installation reference evidence class differs from frozen plan".to_owned());
        }
        if training_installations.contains(installation.installation_id.as_str()) {
            return Err(format!(
                "installation {} overlaps training split",
                installation.installation_id
            ));
        }
        if training_serials.contains(installation.sensor_serial.as_str()) {
            return Err(format!(
                "sensor serial {} overlaps training split",
                installation.sensor_serial
            ));
        }
        if training_flexsplines.contains(installation.flexspline_serial.as_str()) {
            return Err(format!(
                "flexspline serial {} overlaps training split",
                installation.flexspline_serial
            ));
        }
        if training_reducers.contains(installation.reducer_serial.as_str()) {
            return Err(format!(
                "reducer serial {} overlaps training split",
                installation.reducer_serial
            ));
        }
        if !installation_ids.insert(installation.installation_id.clone()) {
            return Err(format!(
                "installation leakage or duplicate installation ID: {}",
                installation.installation_id
            ));
        }
        if !sensor_serials.insert(installation.sensor_serial.clone()) {
            return Err(format!(
                "sensor serial leakage or duplicate sensor serial: {}",
                installation.sensor_serial
            ));
        }
        if !flexspline_serials.insert(installation.flexspline_serial.clone()) {
            return Err(format!(
                "flexspline serial leakage or duplicate flexspline serial: {}",
                installation.flexspline_serial
            ));
        }
        if !reducer_serials.insert(installation.reducer_serial.clone()) {
            return Err(format!(
                "reducer serial leakage or duplicate reducer serial: {}",
                installation.reducer_serial
            ));
        }
        if !installation_digests.insert(installation.installation_digest.clone()) {
            return Err("installation digest leakage or duplication".to_owned());
        }
        if !sensor_assembly_digests.insert(installation.sensor_assembly_digest.clone()) {
            return Err("sensor assembly digest leakage or duplication".to_owned());
        }
        if installation.observations.is_empty() {
            return Err(format!(
                "installation {} contains no observations",
                installation.installation_id
            ));
        }
        match sample_count {
            None => sample_count = Some(installation.observations.len()),
            Some(expected) if expected != installation.observations.len() => {
                return Err("all installation groups must use the same frozen trajectory sample count".to_owned());
            }
            Some(_) => {}
        }

        for (index, sample) in installation.observations.iter().enumerate() {
            if !nonempty(&sample.sample_id) {
                return Err("sample ID must be nonempty".to_owned());
            }
            if !sample_ids.insert(sample.sample_id.clone()) {
                return Err(format!("duplicate sample ID: {}", sample.sample_id));
            }
            if !sample.joint_angle_rad.is_finite()
                || !sample.temperature_c.is_finite()
                || !sample.angular_speed_rad_s.is_finite()
                || !finite_interval(sample.raw_torque_nm)
                || !finite_interval(sample.compensated_torque_nm)
                || !finite_interval(sample.reference_torque_nm)
            {
                return Err(format!(
                    "installation {} observation {index} contains non-finite or inverted data",
                    installation.installation_id
                ));
            }
            if !direction_is_consistent(sample) {
                return Err(format!(
                    "installation {} observation {index} direction conflicts with reference torque/speed signs",
                    installation.installation_id
                ));
            }
            if is_calibration && !condition_is_in_domain(plan, sample) {
                return Err(format!(
                    "calibration installation {} contains an out-of-domain sample",
                    installation.installation_id
                ));
            }
        }
    }
    Ok(sample_count.unwrap_or(0))
}

#[derive(Debug, Clone, Default)]
struct MutableStratum {
    sample_count: usize,
    raw_error_sum: f64,
    compensated_error_sum: f64,
    compensated_miss_sum: f64,
    calibrated_miss_sum: f64,
    covered_count: usize,
}

impl MutableStratum {
    fn add(
        &mut self,
        raw_error: f64,
        compensated_error: f64,
        compensated_miss: f64,
        calibrated_miss: f64,
        covered: bool,
    ) {
        self.sample_count += 1;
        self.raw_error_sum += raw_error;
        self.compensated_error_sum += compensated_error;
        self.compensated_miss_sum += compensated_miss;
        self.calibrated_miss_sum += calibrated_miss;
        if covered {
            self.covered_count += 1;
        }
    }

    fn freeze(self, stratum: ConditionStratum) -> StratumMetrics {
        let count = self.sample_count as f64;
        StratumMetrics {
            stratum,
            sample_count: self.sample_count,
            raw_mean_centre_error_nm: self.raw_error_sum / count,
            compensated_mean_centre_error_nm: self.compensated_error_sum / count,
            calibrated_interval_covered_count: self.covered_count,
            calibrated_interval_coverage: self.covered_count as f64 / count,
            compensated_mean_interval_miss_nm: self.compensated_miss_sum / count,
            calibrated_mean_interval_miss_nm: self.calibrated_miss_sum / count,
        }
    }
}

fn make_empty_report(
    plan: &TransferPlan,
    calibration_count: usize,
    rank: usize,
    scores: Vec<InstallationCalibrationScore>,
    test: &TransferDataset,
) -> TransferReport {
    TransferReport {
        status: TransferStatus::InsufficientCalibrationInstallations,
        model_digest: plan.model_digest.clone(),
        feature_schema_digest: plan.feature_schema_digest.clone(),
        training_split_digest: plan.training_split_digest.clone(),
        calibration_split_digest: plan.calibration_split_digest.clone(),
        test_split_digest: plan.test_split_digest.clone(),
        system_boundary_digest: plan.system_boundary_digest.clone(),
        sensor_family_digest: plan.sensor_family_digest.clone(),
        reference_method_digest: plan.reference_method_digest.clone(),
        domain_digest: plan.domain_digest.clone(),
        target_miscoverage_alpha: plan.target_miscoverage_alpha,
        calibration_installation_count: calibration_count,
        conformal_order_statistic_rank: rank,
        correction_q_nm: None,
        test_installation_count: test.installations.len(),
        in_domain_test_installation_count: 0,
        covered_test_installation_count: 0,
        uncovered_test_installation_count: 0,
        out_of_domain_test_installation_count: 0,
        empirical_installation_coverage: None,
        wilson_95_lower: None,
        wilson_95_upper: None,
        mean_raw_centre_error_nm: None,
        mean_compensated_centre_error_nm: None,
        mean_calibrated_interval_miss_nm: None,
        mean_raw_interval_width_nm: None,
        mean_compensated_interval_width_nm: None,
        mean_calibrated_interval_width_nm: None,
        raw_improvement_nm: None,
        calibration_scores: scores,
        installations: test.installations.iter().map(|installation| InstallationAssessment {
            installation_id: installation.installation_id.clone(),
            sensor_serial: installation.sensor_serial.clone(),
            flexspline_serial: installation.flexspline_serial.clone(),
            reducer_serial: installation.reducer_serial.clone(),
            disposition: InstallationDisposition::NotEvaluated,
            observation_count: installation.observations.len(),
            maximum_raw_interval_miss_nm: None,
            maximum_compensated_interval_miss_nm: None,
            maximum_calibrated_interval_miss_nm: None,
            reference_source_reference: installation.reference_source_reference.clone(),
            reference_raw_trace_digest: installation.reference_raw_trace_digest.clone(),
            calibration_digest: installation.calibration_digest.clone(),
        }).collect(),
        strata: Vec::new(),
        interpretation: "Insufficient independent calibration installations for the requested group-level conformal rank. No calibration correction or test-coverage claim is issued.".to_owned(),
    }
}

/// Fits one additive torque-interval expansion from the maximum required
/// expansion over each independent calibration installation, then evaluates
/// whole-installation coverage on a disjoint test set.
pub fn evaluate_sensor_transfer(
    plan: &TransferPlan,
    calibration: &TransferDataset,
    test: &TransferDataset,
) -> Result<TransferReport, String> {
    validate_plan(plan)?;
    if calibration.split_digest == test.split_digest {
        return Err("calibration and test split digests must differ".to_owned());
    }

    let mut installation_ids = BTreeSet::new();
    let mut sensor_serials = BTreeSet::new();
    let mut flexspline_serials = BTreeSet::new();
    let mut reducer_serials = BTreeSet::new();
    let mut installation_digests = BTreeSet::new();
    let mut sensor_assembly_digests = BTreeSet::new();
    let mut sample_ids = BTreeSet::new();
    let calibration_samples = validate_dataset(
        calibration,
        &plan.calibration_split_digest,
        plan,
        true,
        &mut installation_ids,
        &mut sensor_serials,
        &mut flexspline_serials,
        &mut reducer_serials,
        &mut installation_digests,
        &mut sensor_assembly_digests,
        &mut sample_ids,
    )?;
    let test_samples = validate_dataset(
        test,
        &plan.test_split_digest,
        plan,
        false,
        &mut installation_ids,
        &mut sensor_serials,
        &mut flexspline_serials,
        &mut reducer_serials,
        &mut installation_digests,
        &mut sensor_assembly_digests,
        &mut sample_ids,
    )?;
    if calibration_samples != test_samples {
        return Err("calibration and test installations must use the same frozen trajectory sample count".to_owned());
    }

    let mut calibration_scores: Vec<InstallationCalibrationScore> = calibration
        .installations
        .iter()
        .map(|installation| {
            let maximum = installation
                .observations
                .iter()
                .map(|sample| {
                    interval_expansion_required(
                        sample.compensated_torque_nm,
                        sample.reference_torque_nm,
                    )
                })
                .fold(0.0_f64, f64::max);
            InstallationCalibrationScore {
                installation_id: installation.installation_id.clone(),
                sensor_serial: installation.sensor_serial.clone(),
                maximum_required_expansion_nm: maximum,
                reference_source_reference: installation.reference_source_reference.clone(),
                reference_raw_trace_digest: installation.reference_raw_trace_digest.clone(),
                reference_calibration_digest: installation.reference_calibration_digest.clone(),
                synchronized_metrology_receipt_digest: installation.synchronized_metrology_receipt_digest.clone(),
            }
        })
        .collect();
    calibration_scores.sort_by(|left, right| {
        left.maximum_required_expansion_nm
            .total_cmp(&right.maximum_required_expansion_nm)
            .then_with(|| left.installation_id.cmp(&right.installation_id))
    });

    let n = calibration_scores.len();
    let rank = (((n + 1) as f64) * (1.0 - plan.target_miscoverage_alpha)).ceil() as usize;
    let Some(q) = calibration_scores
        .get(rank.saturating_sub(1))
        .filter(|_| rank > 0)
        .map(|score| score.maximum_required_expansion_nm)
    else {
        return Ok(make_empty_report(plan, n, rank, calibration_scores, test));
    };

    let mut installations = Vec::with_capacity(test.installations.len());
    let mut covered_installations = 0usize;
    let mut uncovered_installations = 0usize;
    let mut ood_installations = 0usize;
    let mut in_domain_installations = 0usize;
    let mut raw_error_sum = 0.0;
    let mut compensated_error_sum = 0.0;
    let mut calibrated_interval_miss_sum = 0.0;
    let mut raw_width_sum = 0.0;
    let mut compensated_width_sum = 0.0;
    let mut calibrated_width_sum = 0.0;
    let mut in_domain_observation_count = 0usize;
    let mut strata: BTreeMap<ConditionStratum, MutableStratum> = BTreeMap::new();

    for installation in &test.installations {
        if installation
            .observations
            .iter()
            .any(|sample| !condition_is_in_domain(plan, sample))
        {
            ood_installations += 1;
            installations.push(InstallationAssessment {
                installation_id: installation.installation_id.clone(),
                sensor_serial: installation.sensor_serial.clone(),
                flexspline_serial: installation.flexspline_serial.clone(),
                reducer_serial: installation.reducer_serial.clone(),
                disposition: InstallationDisposition::OutOfDomain,
                observation_count: installation.observations.len(),
                maximum_raw_interval_miss_nm: None,
                maximum_compensated_interval_miss_nm: None,
                maximum_calibrated_interval_miss_nm: None,
                reference_source_reference: installation.reference_source_reference.clone(),
                reference_raw_trace_digest: installation.reference_raw_trace_digest.clone(),
                calibration_digest: installation.calibration_digest.clone(),
                synchronized_metrology_receipt_digest: installation.synchronized_metrology_receipt_digest.clone(),
            });
            continue;
        }

        in_domain_installations += 1;
        let mut group_covered = true;
        let mut maximum_raw_miss = 0.0_f64;
        let mut maximum_compensated_miss = 0.0_f64;
        let mut maximum_calibrated_miss = 0.0_f64;

        for sample in &installation.observations {
            let calibrated = NumericInterval {
                lower: sample.compensated_torque_nm.lower - q,
                upper: sample.compensated_torque_nm.upper + q,
            };
            let raw_miss = interval_expansion_required(
                sample.raw_torque_nm,
                sample.reference_torque_nm,
            );
            let compensated_miss = interval_expansion_required(
                sample.compensated_torque_nm,
                sample.reference_torque_nm,
            );
            let calibrated_miss = interval_expansion_required(
                calibrated,
                sample.reference_torque_nm,
            );
            let covered = fully_covers(calibrated, sample.reference_torque_nm);
            if !covered {
                group_covered = false;
            }
            maximum_raw_miss = maximum_raw_miss.max(raw_miss);
            maximum_compensated_miss = maximum_compensated_miss.max(compensated_miss);
            maximum_calibrated_miss = maximum_calibrated_miss.max(calibrated_miss);

            let raw_error = (centre(sample.raw_torque_nm) - centre(sample.reference_torque_nm)).abs();
            let compensated_error =
                (centre(sample.compensated_torque_nm) - centre(sample.reference_torque_nm)).abs();
            raw_error_sum += raw_error;
            compensated_error_sum += compensated_error;
            calibrated_interval_miss_sum += interval_miss(calibrated, sample.reference_torque_nm);
            raw_width_sum += sample.raw_torque_nm.upper - sample.raw_torque_nm.lower;
            compensated_width_sum +=
                sample.compensated_torque_nm.upper - sample.compensated_torque_nm.lower;
            calibrated_width_sum += calibrated.upper - calibrated.lower;
            in_domain_observation_count += 1;

            let key = condition_stratum(plan, sample)
                .ok_or_else(|| "in-domain sample could not be assigned to a declared stratum".to_owned())?;
            strata.entry(key).or_default().add(
                raw_error,
                compensated_error,
                interval_miss(sample.compensated_torque_nm, sample.reference_torque_nm),
                interval_miss(calibrated, sample.reference_torque_nm),
                covered,
            );
        }

        let disposition = if group_covered {
            covered_installations += 1;
            InstallationDisposition::Covered
        } else {
            uncovered_installations += 1;
            InstallationDisposition::Uncovered
        };
        installations.push(InstallationAssessment {
            installation_id: installation.installation_id.clone(),
            sensor_serial: installation.sensor_serial.clone(),
            flexspline_serial: installation.flexspline_serial.clone(),
            reducer_serial: installation.reducer_serial.clone(),
            disposition,
            observation_count: installation.observations.len(),
            maximum_raw_interval_miss_nm: Some(maximum_raw_miss),
            maximum_compensated_interval_miss_nm: Some(maximum_compensated_miss),
            maximum_calibrated_interval_miss_nm: Some(maximum_calibrated_miss),
            reference_source_reference: installation.reference_source_reference.clone(),
            reference_raw_trace_digest: installation.reference_raw_trace_digest.clone(),
            calibration_digest: installation.calibration_digest.clone(),
            synchronized_metrology_receipt_digest: installation.synchronized_metrology_receipt_digest.clone(),
        });
    }

    let mean = |sum: f64| {
        if in_domain_observation_count == 0 {
            None
        } else {
            Some(sum / in_domain_observation_count as f64)
        }
    };
    let coverage = if in_domain_installations == 0 {
        None
    } else {
        Some(covered_installations as f64 / in_domain_installations as f64)
    };
    let (wilson_95_lower, wilson_95_upper) =
        wilson_95(covered_installations, in_domain_installations)
            .map_or((None, None), |(lower, upper)| (Some(lower), Some(upper)));

    let strata = strata
        .into_iter()
        .map(|(key, value)| value.freeze(key))
        .collect();
    let mean_raw = mean(raw_error_sum);
    let mean_compensated = mean(compensated_error_sum);
    let raw_improvement = mean_raw.zip(mean_compensated).map(|(raw, compensated)| raw - compensated);

    Ok(TransferReport {
        status: TransferStatus::Calibrated,
        model_digest: plan.model_digest.clone(),
        feature_schema_digest: plan.feature_schema_digest.clone(),
        training_split_digest: plan.training_split_digest.clone(),
        calibration_split_digest: plan.calibration_split_digest.clone(),
        test_split_digest: plan.test_split_digest.clone(),
        system_boundary_digest: plan.system_boundary_digest.clone(),
        sensor_family_digest: plan.sensor_family_digest.clone(),
        reference_method_digest: plan.reference_method_digest.clone(),
        domain_digest: plan.domain_digest.clone(),
        target_miscoverage_alpha: plan.target_miscoverage_alpha,
        calibration_installation_count: n,
        conformal_order_statistic_rank: rank,
        correction_q_nm: Some(q),
        test_installation_count: test.installations.len(),
        in_domain_test_installation_count: in_domain_installations,
        covered_test_installation_count: covered_installations,
        uncovered_test_installation_count: uncovered_installations,
        out_of_domain_test_installation_count: ood_installations,
        empirical_installation_coverage: coverage,
        wilson_95_lower,
        wilson_95_upper,
        mean_raw_centre_error_nm: mean_raw,
        mean_compensated_centre_error_nm: mean_compensated,
        mean_calibrated_interval_miss_nm: mean(calibrated_interval_miss_sum),
        mean_raw_interval_width_nm: mean(raw_width_sum),
        mean_compensated_interval_width_nm: mean(compensated_width_sum),
        mean_calibrated_interval_width_nm: mean(calibrated_width_sum),
        raw_improvement_nm: raw_improvement,
        calibration_scores,
        installations,
        strata,
        interpretation: "Installation-level split-conformal expansion under exchangeability of independent physical installations, a fixed compensation model and frozen condition domain. Test group coverage is marginal at the installation unit; stratum metrics are descriptive and do not establish conditional coverage. Raw versus compensated centre errors are descriptive per-observation summaries.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "compensation-model-v1";
    const FEATURE: &str = "mems-grid-features-v1";
    const TRAIN: &str = "training-installations-v1";
    const CAL: &str = "calibration-installations-v1";
    const TEST: &str = "heldout-installations-v1";
    const BOUNDARY: &str = "installed-harmonic-joint-boundary-v1";
    const SENSOR_FAMILY: &str = "flexspline-mems-grid-family-v1";
    const REFERENCE: &str = "external-torque-transducer-method-v1";
    const DOMAIN: &str = "angle-temp-speed-quadrant-domain-v1";

    fn plan(alpha: f64) -> TransferPlan {
        TransferPlan {
            model_digest: MODEL.to_owned(),
            feature_schema_digest: FEATURE.to_owned(),
            training_split_digest: TRAIN.to_owned(),
            calibration_split_digest: CAL.to_owned(),
            test_split_digest: TEST.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            sensor_family_digest: SENSOR_FAMILY.to_owned(),
            reference_method_digest: REFERENCE.to_owned(),
            domain_digest: DOMAIN.to_owned(),
            target_miscoverage_alpha: alpha,
            reference_evidence_class: ReferenceEvidenceClass::IndependentPhysical,
            training_installation_ids: vec!["train-installation-1".to_owned()],
            training_sensor_serials: vec!["train-sensor-1".to_owned()],
            training_flexspline_serials: vec!["train-flex-1".to_owned()],
            training_reducer_serials: vec!["train-reducer-1".to_owned()],
            domain: SensorDomain {
                joint_angle_rad: NumericInterval { lower: -3.0, upper: 3.0 },
                temperature_c: NumericInterval { lower: 10.0, upper: 70.0 },
                maximum_absolute_speed_rad_s: 10.0,
                supported_directions: vec![
                    TorqueDirection::ForwardMotoring,
                    TorqueDirection::ForwardRegenerating,
                    TorqueDirection::ReverseMotoring,
                    TorqueDirection::ReverseRegenerating,
                    TorqueDirection::StaticHold,
                ],
                angle_sector_edges_rad: vec![-3.0, -1.0, 1.0, 3.0],
                temperature_band_edges_c: vec![10.0, 30.0, 50.0, 70.0],
                speed_band_edges_rad_s: vec![0.0, 2.0, 6.0, 10.0],
            },
        }
    }

    fn sample(
        id: &str,
        angle: f64,
        temp: f64,
        speed: f64,
        direction: TorqueDirection,
        reference: NumericInterval,
        raw: NumericInterval,
        compensated: NumericInterval,
    ) -> SensorObservation {
        SensorObservation {
            sample_id: id.to_owned(),
            joint_angle_rad: angle,
            temperature_c: temp,
            angular_speed_rad_s: speed,
            direction,
            raw_torque_nm: raw,
            compensated_torque_nm: compensated,
            reference_torque_nm: reference,
        }
    }

    fn installation(
        id: &str,
        sensor: &str,
        observations: Vec<SensorObservation>,
    ) -> InstallationGroup {
        InstallationGroup {
            installation_id: id.to_owned(),
            sensor_serial: sensor.to_owned(),
            flexspline_serial: format!("flex-{id}"),
            reducer_serial: format!("reducer-{id}"),
            installation_digest: format!("installation-digest-{id}"),
            sensor_assembly_digest: format!("sensor-assembly-{id}"),
            calibration_digest: format!("sensor-calibration-{id}"),
            reference_calibration_digest: format!("reference-calibration-{id}"),
            reference_evidence_class: ReferenceEvidenceClass::IndependentPhysical,
            reference_source_reference: format!("https://example.invalid/reference/{id}"),
            reference_method: "Synthetic independent external torque reference fixture".to_owned(),
            reference_raw_trace_digest: format!("reference-trace-{id}"),
            sensor_raw_trace_digest: format!("sensor-trace-{id}"),
            synchronized_metrology_receipt_digest: format!("synced-metrology-receipt-{id}"),
            observations,
        }
    }

    fn motoring_sample(id: &str, reference: f64, compensated_error: f64) -> SensorObservation {
        sample(
            id,
            0.0,
            25.0,
            3.0,
            TorqueDirection::ForwardMotoring,
            NumericInterval { lower: reference - 0.1, upper: reference + 0.1 },
            NumericInterval { lower: reference + 1.5, upper: reference + 1.7 },
            NumericInterval {
                lower: reference + compensated_error - 0.1,
                upper: reference + compensated_error + 0.1,
            },
        )
    }

    fn calibration_dataset(groups: Vec<InstallationGroup>) -> TransferDataset {
        TransferDataset {
            split_digest: CAL.to_owned(),
            model_digest: MODEL.to_owned(),
            feature_schema_digest: FEATURE.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            sensor_family_digest: SENSOR_FAMILY.to_owned(),
            reference_method_digest: REFERENCE.to_owned(),
            domain_digest: DOMAIN.to_owned(),
            installations: groups,
        }
    }

    fn test_dataset(groups: Vec<InstallationGroup>) -> TransferDataset {
        TransferDataset {
            split_digest: TEST.to_owned(),
            model_digest: MODEL.to_owned(),
            feature_schema_digest: FEATURE.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            sensor_family_digest: SENSOR_FAMILY.to_owned(),
            reference_method_digest: REFERENCE.to_owned(),
            domain_digest: DOMAIN.to_owned(),
            installations: groups,
        }
    }

    fn known_calibration() -> TransferDataset {
        calibration_dataset(vec![
            installation("cal-a", "sensor-cal-a", vec![
                motoring_sample("cal-a-1", 10.0, 0.0),
                motoring_sample("cal-a-2", 20.0, 0.1),
            ]),
            installation("cal-b", "sensor-cal-b", vec![
                motoring_sample("cal-b-1", 15.0, 0.2),
                motoring_sample("cal-b-2", 25.0, 0.3),
            ]),
            installation("cal-c", "sensor-cal-c", vec![
                motoring_sample("cal-c-1", 30.0, 0.4),
                motoring_sample("cal-c-2", 40.0, 0.5),
            ]),
        ])
    }

    fn known_test() -> TransferDataset {
        test_dataset(vec![
            installation("test-a", "sensor-test-a", vec![
                motoring_sample("test-a-1", 10.0, 0.2),
                motoring_sample("test-a-2", 20.0, 0.3),
            ]),
            installation("test-b", "sensor-test-b", vec![
                motoring_sample("test-b-1", 15.0, 0.8),
                motoring_sample("test-b-2", 25.0, 0.9),
            ]),
            installation("test-ood", "sensor-test-ood", vec![
                sample(
                    "test-ood-1", 4.0, 25.0, 3.0, TorqueDirection::ForwardMotoring,
                    NumericInterval { lower: 10.0, upper: 10.1 },
                    NumericInterval { lower: 11.0, upper: 11.1 },
                    NumericInterval { lower: 10.0, upper: 10.1 },
                ),
                motoring_sample("test-ood-2", 20.0, 0.0),
            ]),
        ])
    }

    fn assessment<'a>(report: &'a TransferReport, id: &str) -> &'a InstallationAssessment {
        report.installations.iter().find(|item| item.installation_id == id).unwrap()
    }

    #[test]
    fn sensor_transfer_001_grouped_calibration_rank_uses_one_score_per_installation() {
        let report = evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert_eq!(report.status, TransferStatus::Calibrated);
        assert_eq!(report.calibration_installation_count, 3);
        assert_eq!(report.conformal_order_statistic_rank, 3);
        // Each installation contributes one score: 0.1, 0.3 and 0.5 N m.
        assert_eq!(
            report.calibration_scores.iter().map(|score| score.maximum_required_expansion_nm).collect::<Vec<_>>(),
            vec![0.1, 0.3, 0.5]
        );
        assert_eq!(report.correction_q_nm, Some(0.5));
    }

    #[test]
    fn sensor_transfer_002_heldout_coverage_is_installation_grouped_and_ood_abstains() {
        let report = evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert_eq!(report.test_installation_count, 3);
        assert_eq!(report.in_domain_test_installation_count, 2);
        assert_eq!(report.covered_test_installation_count, 1);
        assert_eq!(report.uncovered_test_installation_count, 1);
        assert_eq!(report.out_of_domain_test_installation_count, 1);
        assert_eq!(report.empirical_installation_coverage, Some(0.5));
        assert_eq!(assessment(&report, "test-a").disposition, InstallationDisposition::Covered);
        assert_eq!(assessment(&report, "test-b").disposition, InstallationDisposition::Uncovered);
        assert_eq!(assessment(&report, "test-ood").disposition, InstallationDisposition::OutOfDomain);
    }

    #[test]
    fn sensor_transfer_003_raw_vs_compensated_error_is_reported_separately() {
        let report = evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert!(report.mean_raw_centre_error_nm.unwrap() > report.mean_compensated_centre_error_nm.unwrap());
        assert!(report.raw_improvement_nm.unwrap() > 0.0);
    }

    #[test]
    fn sensor_transfer_004_miscoverage_rank_beyond_n_returns_insufficient() {
        let report = evaluate_sensor_transfer(&plan(0.05), &known_calibration(), &known_test()).unwrap();
        assert_eq!(report.status, TransferStatus::InsufficientCalibrationInstallations);
        assert_eq!(report.conformal_order_statistic_rank, 4);
        assert_eq!(report.correction_q_nm, None);
        assert!(report.empirical_installation_coverage.is_none());
        assert!(report.installations.iter().all(|item| item.disposition == InstallationDisposition::NotEvaluated));
    }

    #[test]
    fn sensor_transfer_005_training_lineage_inventory_cannot_be_empty() {
        let mut invalid = plan(0.25);
        invalid.training_flexspline_serials.clear();
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_006_training_calibration_test_splits_must_be_distinct() {
        let mut invalid = plan(0.25);
        invalid.test_split_digest = invalid.calibration_split_digest.clone();
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_007_installation_leakage_between_calibration_and_test_rejects() {
        let calibration = known_calibration();
        let mut test = known_test();
        test.installations[0].installation_id = calibration.installations[0].installation_id.clone();
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &test).is_err());
    }

    #[test]
    fn sensor_transfer_008_sensor_serial_leakage_between_splits_rejects() {
        let calibration = known_calibration();
        let mut test = known_test();
        test.installations[0].sensor_serial = calibration.installations[0].sensor_serial.clone();
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &test).is_err());
    }

    #[test]
    fn sensor_transfer_009_training_installation_or_sensor_cannot_leak() {
        let mut invalid = plan(0.25);
        invalid.training_installation_ids.push("cal-a".to_owned());
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());

        let mut invalid = plan(0.25);
        invalid.training_sensor_serials.push("sensor-cal-a".to_owned());
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_010_duplicate_sample_ids_reject() {
        let mut calibration = known_calibration();
        calibration.installations[1].observations[0].sample_id =
            calibration.installations[0].observations[0].sample_id.clone();
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_011_calibration_must_be_in_declared_condition_domain() {
        let mut calibration = known_calibration();
        calibration.installations[0].observations[0].temperature_c = 100.0;
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_012_any_ood_test_sample_abstains_whole_installation() {
        let mut test = known_test();
        test.installations[0].observations[1].joint_angle_rad = 3.5;
        let report = evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).unwrap();
        assert_eq!(assessment(&report, "test-a").disposition, InstallationDisposition::OutOfDomain);
        assert_eq!(report.out_of_domain_test_installation_count, 2);
    }

    #[test]
    fn sensor_transfer_013_direction_and_reference_quadrant_must_agree() {
        let mut calibration = known_calibration();
        calibration.installations[0].observations[0].direction = TorqueDirection::ForwardRegenerating;
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_014_only_independent_external_reference_is_accepted() {
        let mut calibration = known_calibration();
        calibration.installations[0].reference_evidence_class = ReferenceEvidenceClass::VendorPhysical;
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &known_test()).is_err());

        let mut invalid = plan(0.25);
        invalid.reference_evidence_class = ReferenceEvidenceClass::SourceAuthorPhysical;
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_015_model_feature_boundary_sensor_and_domain_digest_mismatch_rejects() {
        let mut test = known_test();
        test.model_digest = "different-model".to_owned();
        assert!(evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).is_err());

        let mut test = known_test();
        test.sensor_family_digest = "different-sensor-family".to_owned();
        assert!(evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).is_err());
    }

    #[test]
    fn sensor_transfer_016_wrong_reference_method_or_boundary_rejects() {
        let mut test = known_test();
        test.reference_method_digest = "other-reference-method".to_owned();
        assert!(evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).is_err());

        let mut test = known_test();
        test.system_boundary_digest = "external-only-boundary".to_owned();
        assert!(evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).is_err());
    }

    #[test]
    fn sensor_transfer_017_inverted_or_nonfinite_intervals_reject() {
        let mut test = known_test();
        test.installations[0].observations[0].compensated_torque_nm =
            NumericInterval { lower: 2.0, upper: 1.0 };
        assert!(evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).is_err());

        let mut test = known_test();
        test.installations[0].observations[0].temperature_c = f64::NAN;
        assert!(evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).is_err());
    }

    #[test]
    fn sensor_transfer_018_invalid_domain_edges_or_alpha_reject() {
        let mut invalid = plan(0.0);
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());

        invalid = plan(0.25);
        invalid.domain.angle_sector_edges_rad[1] = -4.0;
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_019_calibration_and_test_groups_need_same_frozen_trajectory_length() {
        let mut test = known_test();
        test.installations[0].observations.pop();
        assert!(evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).is_err());
    }

    #[test]
    fn sensor_transfer_020_stratified_reports_cover_angle_temperature_speed_and_direction() {
        let report = evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert!(!report.strata.is_empty());
        assert!(report.strata.iter().all(|stratum| stratum.sample_count > 0));
        let key = &report.strata[0].stratum;
        assert_eq!(key.direction, TorqueDirection::ForwardMotoring);
    }

    #[test]
    fn sensor_transfer_021_empty_in_domain_test_cohort_has_no_fake_zero_coverage() {
        let mut test = known_test();
        for installation in &mut test.installations {
            for observation in &mut installation.observations {
                observation.temperature_c = 100.0;
            }
        }
        let report = evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &test).unwrap();
        assert_eq!(report.empirical_installation_coverage, None);
        assert_eq!(report.wilson_95_lower, None);
        assert_eq!(report.mean_compensated_centre_error_nm, None);
    }

    #[test]
    fn sensor_transfer_022_wilson_interval_contains_observed_installation_coverage() {
        let report = evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert!(report.wilson_95_lower.unwrap() <= 0.5);
        assert!(report.wilson_95_upper.unwrap() >= 0.5);
        assert!(report.wilson_95_lower.unwrap() >= 0.0);
        assert!(report.wilson_95_upper.unwrap() <= 1.0);
    }

    #[test]
    fn sensor_transfer_023_evidence_receipts_survive_in_installation_results() {
        let report = evaluate_sensor_transfer(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert_eq!(
            report.installations[0].reference_source_reference,
            "https://example.invalid/reference/test-a"
        );
        assert_eq!(
            report.installations[0].reference_raw_trace_digest,
            "reference-trace-test-a"
        );
        assert_eq!(
            report.installations[0].calibration_digest,
            "sensor-calibration-test-a"
        );
        assert_eq!(
            report.installations[0].synchronized_metrology_receipt_digest,
            "synced-metrology-receipt-test-a"
        );
    }

    #[test]
    fn sensor_transfer_024_static_hold_and_reverse_quadrants_are_supported() {
        let mut calibration = known_calibration();
        let mut hold = calibration.installations[0].observations[0].clone();
        hold.sample_id = "cal-a-static".to_owned();
        hold.angular_speed_rad_s = 0.0;
        hold.direction = TorqueDirection::StaticHold;
        calibration.installations[0].observations[0] = hold;

        let reverse_motoring = &mut calibration.installations[1].observations[0];
        reverse_motoring.sample_id = "cal-b-reverse-motoring".to_owned();
        reverse_motoring.angular_speed_rad_s = -3.0;
        reverse_motoring.direction = TorqueDirection::ReverseMotoring;
        reverse_motoring.reference_torque_nm = NumericInterval { lower: -15.1, upper: -14.9 };
        reverse_motoring.raw_torque_nm = NumericInterval { lower: -13.6, upper: -13.4 };
        reverse_motoring.compensated_torque_nm = NumericInterval { lower: -14.9, upper: -14.7 };

        let reverse_regenerating = &mut calibration.installations[2].observations[0];
        reverse_regenerating.sample_id = "cal-c-reverse-regenerating".to_owned();
        reverse_regenerating.angular_speed_rad_s = -3.0;
        reverse_regenerating.direction = TorqueDirection::ReverseRegenerating;
        reverse_regenerating.reference_torque_nm = NumericInterval { lower: 29.9, upper: 30.1 };
        reverse_regenerating.raw_torque_nm = NumericInterval { lower: 31.5, upper: 31.7 };
        reverse_regenerating.compensated_torque_nm = NumericInterval { lower: 30.1, upper: 30.3 };

        let report = evaluate_sensor_transfer(&plan(0.25), &calibration, &known_test()).unwrap();
        assert_eq!(report.status, TransferStatus::Calibrated);
    }

    #[test]
    fn sensor_transfer_025_training_flexspline_and_reducer_serials_cannot_leak() {
        let mut invalid = plan(0.25);
        invalid.training_flexspline_serials.push("flex-cal-a".to_owned());
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());

        let mut invalid = plan(0.25);
        invalid.training_reducer_serials.push("reducer-cal-a".to_owned());
        assert!(evaluate_sensor_transfer(&invalid, &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_026_shared_reducer_or_flexspline_across_splits_rejects() {
        let calibration = known_calibration();
        let mut test = known_test();
        test.installations[0].reducer_serial = calibration.installations[0].reducer_serial.clone();
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &test).is_err());

        let calibration = known_calibration();
        let mut test = known_test();
        test.installations[0].flexspline_serial = calibration.installations[0].flexspline_serial.clone();
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &test).is_err());
    }

    #[test]
    fn sensor_transfer_027_synchronized_metrology_receipt_is_required() {
        let mut calibration = known_calibration();
        calibration.installations[0].synchronized_metrology_receipt_digest.clear();
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn sensor_transfer_028_duplicate_installation_and_sensor_identities_reject() {
        let mut calibration = known_calibration();
        calibration.installations[1].sensor_serial =
            calibration.installations[0].sensor_serial.clone();
        assert!(evaluate_sensor_transfer(&plan(0.25), &calibration, &known_test()).is_err());
    }
}
