//! Domain-limit diagnostics for an empirical two-dimensional section drag polar.
//!
//! This module fits Cd_section = a + b * Cl^2 independently for each measured roughness zone.
//! The fitted b is an empirical section coefficient; it is NOT an aircraft induced-drag factor.
//! Nothing in this module certifies a 3D aircraft model or defines a safe operating envelope.

use std::collections::{BTreeMap, HashSet};

use crate::reference_data::ForceObservation;

pub const SECTION_POLAR_MODEL_VERSION: &str = "section-polar-domain-audit/0.1.0";
pub const NASA_TMR_SOURCE_REVISION: &str = "20a39f549dbff988a6accc421ac84dafc4487695";
pub const NASA_TMR_FORCE_SHA256_EXPECTED: &str =
    "78cd2f6aa4968e80f44cbf6c96f699bd9c6e45681d958ec528a10cf72ed23357";
pub const CALIBRATION_MAX_ABS_ALPHA_DEG: f64 = 8.5;
pub const INTERMEDIATE_MAX_ABS_ALPHA_DEG: f64 = 16.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ErrorMetrics {
    pub sample_count: usize,
    pub mean_absolute_error: f64,
    pub root_mean_square_error: f64,
    pub maximum_absolute_error: f64,
    /// Mean observed-minus-predicted Cd; positive means the fit underpredicts drag.
    pub mean_signed_error: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZonePolarReport {
    pub zone: String,
    pub observation_count: usize,
    pub calibration: ErrorMetrics,
    pub intermediate_holdout: ErrorMetrics,
    pub high_angle_extrapolation: ErrorMetrics,
    pub fitted_cd0_section: f64,
    pub fitted_cd2_section_coefficient: f64,
    pub minimum_source_line: usize,
    pub maximum_source_line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionPolarProvenance {
    pub dataset_id: String,
    pub source_revision: String,
    /// Expected digest from the source manifest. The caller must independently hash bytes.
    pub declared_source_sha256: String,
    pub source_condition: String,
    pub evidence_level: String,
}

impl SectionPolarProvenance {
    pub fn nasa_tmr_ladson() -> Self {
        Self {
            dataset_id: "NASA-TMR-NACA0012-LADSON-FORCES".to_owned(),
            source_revision: NASA_TMR_SOURCE_REVISION.to_owned(),
            declared_source_sha256: NASA_TMR_FORCE_SHA256_EXPECTED.to_owned(),
            source_condition:
                "Ladson NASA TM 4074 (1988); Re=6 million; M=0.15; transition tripped"
                    .to_owned(),
            evidence_level: "reference_data_diagnostic_not_3d_validation".to_owned(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("dataset_id", self.dataset_id.as_str()),
            ("source_revision", self.source_revision.as_str()),
            ("source_condition", self.source_condition.as_str()),
            ("evidence_level", self.evidence_level.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("section-polar provenance {name} must not be empty"));
            }
        }
        if self.declared_source_sha256.len() != 64
            || !self
                .declared_source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(
                "section-polar provenance declared_source_sha256 must be 64 hex characters"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionPolarReport {
    pub model_version: &'static str,
    pub source_dataset_id: String,
    pub source_revision: String,
    /// Declared digest only. The caller must independently hash the raw file.
    pub declared_source_sha256: String,
    pub source_condition: String,
    pub evidence_level: String,
    pub calibration_split: &'static str,
    pub intermediate_split: &'static str,
    pub high_angle_split: &'static str,
    pub zones: Vec<ZonePolarReport>,
}

#[derive(Debug, Clone, Copy)]
struct Fit {
    intercept: f64,
    slope: f64,
}

impl Fit {
    fn predict(self, cl: f64) -> f64 {
        self.intercept + self.slope * cl * cl
    }
}

/// Fit a separate empirical section drag polar for every measured zone.
///
/// The function never pools different roughness zones. All split boundaries are fixed constants
/// so repeated runs cannot silently tune the test set.
pub fn audit_section_polar(
    observations: &[ForceObservation],
    provenance: SectionPolarProvenance,
) -> Result<SectionPolarReport, String> {
    provenance.validate()?;
    if observations.is_empty() {
        return Err("section-polar audit requires at least one observation".to_owned());
    }

    let mut zones = BTreeMap::<String, Vec<ForceObservation>>::new();
    let mut source_lines = HashSet::<usize>::new();
    for observation in observations {
        if observation.zone.trim().is_empty() {
            return Err(format!(
                "source line {}: zone must not be empty",
                observation.source_line
            ));
        }
        if !observation.alpha_deg.is_finite()
            || !observation.cl.is_finite()
            || !observation.cd.is_finite()
            || observation.cd < 0.0
        {
            return Err(format!(
                "source line {}: alpha/Cl/Cd must be finite and Cd non-negative",
                observation.source_line
            ));
        }
        if observation.source_line == 0 {
            return Err("source line numbers must be one-based".to_owned());
        }
        if !source_lines.insert(observation.source_line) {
            return Err(format!(
                "duplicate source line {} across the source file",
                observation.source_line
            ));
        }
        zones
            .entry(observation.zone.clone())
            .or_default()
            .push(observation.clone());
    }

    let mut reports = Vec::with_capacity(zones.len());
    for (zone, rows) in zones {
        let calibration: Vec<&ForceObservation> = rows
            .iter()
            .filter(|row| row.alpha_deg.abs() <= CALIBRATION_MAX_ABS_ALPHA_DEG)
            .collect();
        let intermediate: Vec<&ForceObservation> = rows
            .iter()
            .filter(|row| {
                row.alpha_deg.abs() > CALIBRATION_MAX_ABS_ALPHA_DEG
                    && row.alpha_deg.abs() <= INTERMEDIATE_MAX_ABS_ALPHA_DEG
            })
            .collect();
        let high_angle: Vec<&ForceObservation> = rows
            .iter()
            .filter(|row| row.alpha_deg.abs() > INTERMEDIATE_MAX_ABS_ALPHA_DEG)
            .collect();

        if calibration.len() < 3 {
            return Err(format!(
                "zone {zone}: at least 3 calibration observations are required; got {}",
                calibration.len()
            ));
        }
        if intermediate.is_empty() {
            return Err(format!(
                "zone {zone}: intermediate holdout partition is empty"
            ));
        }
        if high_angle.is_empty() {
            return Err(format!(
                "zone {zone}: high-angle extrapolation partition is empty"
            ));
        }

        let fit = fit_polar(&calibration, &zone)?;
        let calibration_metrics = calculate_metrics(&calibration, fit);
        let intermediate_metrics = calculate_metrics(&intermediate, fit);
        let high_angle_metrics = calculate_metrics(&high_angle, fit);
        let minimum_source_line = rows
            .iter()
            .map(|row| row.source_line)
            .min()
            .expect("non-empty zone");
        let maximum_source_line = rows
            .iter()
            .map(|row| row.source_line)
            .max()
            .expect("non-empty zone");

        reports.push(ZonePolarReport {
            zone,
            observation_count: rows.len(),
            calibration: calibration_metrics,
            intermediate_holdout: intermediate_metrics,
            high_angle_extrapolation: high_angle_metrics,
            fitted_cd0_section: fit.intercept,
            fitted_cd2_section_coefficient: fit.slope,
            minimum_source_line,
            maximum_source_line,
        });
    }

    Ok(SectionPolarReport {
        model_version: SECTION_POLAR_MODEL_VERSION,
        source_dataset_id: provenance.dataset_id,
        source_revision: provenance.source_revision,
        declared_source_sha256: provenance.declared_source_sha256,
        source_condition: provenance.source_condition,
        evidence_level: provenance.evidence_level,
        calibration_split: "|alpha_deg| <= 8.5",
        intermediate_split: "8.5 < |alpha_deg| <= 16.5",
        high_angle_split: "|alpha_deg| > 16.5",
        zones: reports,
    })
}

fn fit_polar(rows: &[&ForceObservation], zone: &str) -> Result<Fit, String> {
    let count = rows.len() as f64;
    let mean_x = rows.iter().map(|row| row.cl * row.cl).sum::<f64>() / count;
    let mean_y = rows.iter().map(|row| row.cd).sum::<f64>() / count;
    let covariance = rows
        .iter()
        .map(|row| (row.cl * row.cl - mean_x) * (row.cd - mean_y))
        .sum::<f64>();
    let variance = rows
        .iter()
        .map(|row| (row.cl * row.cl - mean_x).powi(2))
        .sum::<f64>();

    if !variance.is_finite() || variance <= 1e-18 {
        return Err(format!(
            "zone {zone}: calibration Cl^2 values are singular; cannot fit section polar"
        ));
    }
    let slope = covariance / variance;
    let intercept = mean_y - slope * mean_x;
    if !slope.is_finite() || !intercept.is_finite() {
        return Err(format!("zone {zone}: section-polar fit is non-finite"));
    }
    Ok(Fit { intercept, slope })
}

fn calculate_metrics(rows: &[&ForceObservation], fit: Fit) -> ErrorMetrics {
    let errors: Vec<f64> = rows
        .iter()
        .map(|row| row.cd - fit.predict(row.cl))
        .collect();
    let count = errors.len() as f64;
    let sum_abs = errors.iter().map(|error| error.abs()).sum::<f64>();
    let sum_square = errors.iter().map(|error| error.powi(2)).sum::<f64>();
    let sum_signed = errors.iter().sum::<f64>();
    ErrorMetrics {
        sample_count: errors.len(),
        mean_absolute_error: sum_abs / count,
        root_mean_square_error: (sum_square / count).sqrt(),
        maximum_absolute_error: errors
            .iter()
            .map(|error| error.abs())
            .fold(0.0, f64::max),
        mean_signed_error: sum_signed / count,
    }
}

pub const SECTION_POLAR_CSV_HEADER: &str =
    "model_version,dataset_id,declared_source_sha256,source_revision,source_condition,evidence_level,zone,observation_count,calibration_split,calibration_n,fit_cd0_section,fit_cd2_section_coefficient,calibration_mae,calibration_rmse,calibration_max_abs_error,calibration_mean_signed_error,intermediate_split,intermediate_n,intermediate_mae,intermediate_rmse,intermediate_max_abs_error,intermediate_mean_signed_error,high_angle_split,high_angle_n,high_angle_mae,high_angle_rmse,high_angle_max_abs_error,high_angle_mean_signed_error,min_source_line,max_source_line";

pub fn report_to_csv(report: &SectionPolarReport) -> String {
    let mut output = format!("{SECTION_POLAR_CSV_HEADER}\n");
    for zone in &report.zones {
        let c = zone.calibration;
        let m = zone.intermediate_holdout;
        let h = zone.high_angle_extrapolation;
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{},{},{:.12},{:.12},{:.12},{:.12},{},{},{:.12},{:.12},{:.12},{:.12},{},{}\n",
            report.model_version,
            report.source_dataset_id,
            report.declared_source_sha256,
            report.source_revision,
            csv_escape(report.source_condition),
            report.evidence_level,
            csv_escape(&zone.zone),
            zone.observation_count,
            report.calibration_split,
            c.sample_count,
            zone.fitted_cd0_section,
            zone.fitted_cd2_section_coefficient,
            c.mean_absolute_error,
            c.root_mean_square_error,
            c.maximum_absolute_error,
            c.mean_signed_error,
            report.intermediate_split,
            m.sample_count,
            m.mean_absolute_error,
            m.root_mean_square_error,
            m.maximum_absolute_error,
            m.mean_signed_error,
            report.high_angle_split,
            h.sample_count,
            h.mean_absolute_error,
            h.root_mean_square_error,
            h.maximum_absolute_error,
            h.mean_signed_error,
            zone.minimum_source_line,
            zone.maximum_source_line
        ));
    }
    output
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_provenance() -> SectionPolarProvenance {
        SectionPolarProvenance {
            dataset_id: "synthetic-section-polar-fixture".to_owned(),
            source_revision: "synthetic-fixture".to_owned(),
            declared_source_sha256: "0".repeat(64),
            source_condition: "synthetic unit-test observations".to_owned(),
            evidence_level: "synthetic_fixture_not_validation".to_owned(),
        }
    }

    fn observation(zone: &str, line: usize, alpha: f64, cl: f64, cd: f64) -> ForceObservation {
        ForceObservation {
            zone: zone.to_owned(),
            alpha_deg: alpha,
            cl,
            cd,
            source_line: line,
        }
    }

    fn synthetic_zone(zone: &str, offset: f64) -> Vec<ForceObservation> {
        let calibration = [
            (-8.0, -0.8),
            (-4.0, -0.4),
            (0.0, 0.0),
            (4.0, 0.4),
            (8.0, 0.8),
        ];
        let intermediate = [(10.0, 0.9), (12.0, 1.0)];
        let high_angle = [(17.0, 1.1), (18.0, 0.7)];
        let mut rows = Vec::new();
        // Test fixtures represent distinct zones in one source file, so line numbers stay global.
        let mut line = if zone == "A" { 1 } else { 101 };
        for (alpha, cl) in calibration
            .into_iter()
            .chain(intermediate)
            .chain(high_angle)
        {
            rows.push(observation(
                zone,
                line,
                alpha,
                cl,
                0.01 + 0.02 * cl * cl + offset,
            ));
            line += 1;
        }
        rows
    }

    #[test]
    fn recovers_known_empirical_section_polar_without_pooling_zones() {
        let mut rows = synthetic_zone("A", 0.0);
        rows.extend(synthetic_zone("B", 0.003));
        let report = audit_section_polar(&rows, synthetic_provenance()).expect("valid synthetic zones");
        assert_eq!(report.zones.len(), 2);
        assert_eq!(report.zones[0].zone, "A");
        assert_eq!(report.zones[1].zone, "B");
        for zone in &report.zones {
            let expected_cd0 = 0.01 + if zone.zone == "A" { 0.0 } else { 0.003 };
            assert!((zone.fitted_cd0_section - expected_cd0).abs() < 1e-12);
            assert!((zone.fitted_cd2_section_coefficient - 0.02).abs() < 1e-12);
            assert!(zone.calibration.root_mean_square_error < 1e-12);
        }
    }

    #[test]
    fn missing_partitions_fail_closed() {
        let only_low = vec![
            observation("A", 1, -2.0, -0.2, 0.01),
            observation("A", 2, 0.0, 0.0, 0.01),
            observation("A", 3, 2.0, 0.2, 0.011),
        ];
        assert!(audit_section_polar(&only_low, synthetic_provenance()).is_err());
        assert!(audit_section_polar(&[], synthetic_provenance()).is_err());
    }

    #[test]
    fn singular_fit_fails_closed() {
        let rows = vec![
            observation("A", 1, -8.0, 0.2, 0.01),
            observation("A", 2, -4.0, 0.2, 0.01),
            observation("A", 3, 0.0, 0.2, 0.01),
            observation("A", 4, 10.0, 0.3, 0.02),
            observation("A", 5, 17.0, 0.4, 0.03),
        ];
        assert!(audit_section_polar(&rows, synthetic_provenance()).is_err());
    }

    #[test]
    fn duplicate_source_line_and_nonfinite_values_are_rejected() {
        let mut duplicate = synthetic_zone("A", 0.0);
        duplicate[1].source_line = duplicate[0].source_line;
        assert!(audit_section_polar(&duplicate, synthetic_provenance()).is_err());

        let mut duplicate_across_zones = synthetic_zone("A", 0.0);
        let mut other_zone = synthetic_zone("B", 0.0);
        other_zone[0].source_line = duplicate_across_zones[0].source_line;
        duplicate_across_zones.extend(other_zone);
        assert!(
            audit_section_polar(&duplicate_across_zones, synthetic_provenance()).is_err()
        );

        let mut nonfinite = synthetic_zone("A", 0.0);
        nonfinite[0].cd = f64::NAN;
        assert!(audit_section_polar(&nonfinite, synthetic_provenance()).is_err());
    }

    #[test]
    fn heldout_perturbations_cannot_change_calibration_fit() {
        let mut rows = synthetic_zone("A", 0.0);
        let baseline = audit_section_polar(&rows, synthetic_provenance()).expect("baseline report");
        let high_angle_index = rows
            .iter()
            .position(|row| row.alpha_deg > INTERMEDIATE_MAX_ABS_ALPHA_DEG)
            .expect("high angle row");
        rows[high_angle_index].cd = 0.9;
        let changed = audit_section_polar(&rows, synthetic_provenance()).expect("changed holdout");
        assert_eq!(
            baseline.zones[0].fitted_cd0_section,
            changed.zones[0].fitted_cd0_section
        );
        assert_eq!(
            baseline.zones[0].fitted_cd2_section_coefficient,
            changed.zones[0].fitted_cd2_section_coefficient
        );
        assert_ne!(
            baseline.zones[0].high_angle_extrapolation.root_mean_square_error,
            changed.zones[0].high_angle_extrapolation.root_mean_square_error
        );
    }

    #[test]
    fn invalid_provenance_is_rejected() {
        let rows = synthetic_zone("A", 0.0);
        let mut provenance = synthetic_provenance();
        provenance.declared_source_sha256 = "not-a-digest".to_owned();
        assert!(audit_section_polar(&rows, provenance).is_err());
    }

    #[test]
    fn deterministic_csv_keeps_the_2d_evidence_boundary_explicit() {
        let rows = synthetic_zone("A", 0.0);
        let report = audit_section_polar(&rows, synthetic_provenance()).expect("report");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert!(first.contains("synthetic_fixture_not_validation"));
        assert!(first.contains("declared_source_sha256"));
        assert!(first.contains("synthetic-section-polar-fixture"));
    }
}
