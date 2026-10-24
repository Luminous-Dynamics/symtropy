//! Deterministic stress-case registry for lighting optimization research.
//!
//! These cases are declared synthetic interventions, not calibrated probability
//! distributions, standards thresholds, or evidence about real-world prevalence.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ResearchTier {
    TaskPlaneOptics,
    AdaptiveControl,
    OffGridResilience,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Split {
    Development,
    HeldOut,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Perturbation {
    FixtureTranslationMm {
        dx: f64,
        dy: f64,
    },
    FixtureHeightScale {
        scale: f64,
    },
    UniformReflectanceDelta {
        delta: f64,
    },
    PowerInputScale {
        scale: f64,
    },
    WallReflectanceAsymmetry {
        north_delta: f64,
        south_delta: f64,
    },
    OccluderPreset {
        preset: &'static str,
    },
    CoupledGeometryReflectance {
        dx_mm: f64,
        height_scale: f64,
        reflectance_delta: f64,
    },
    AmbientSensorBias {
        bias_fraction: f64,
    },
    AmbientSensorDropout {
        dropout_fraction: f64,
    },
    ControlLatencySeconds {
        latency_s: f64,
    },
    StaleSensorAgeSeconds {
        age_s: f64,
    },
    BatteryStateOfCharge {
        fraction: f64,
    },
    GridOutageDurationSeconds {
        duration_s: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StressScenario {
    pub id: &'static str,
    pub tier: ResearchTier,
    pub family: &'static str,
    pub split: Split,
    pub perturbation: Perturbation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorpusError {
    EmptyCorpus,
    EmptyId,
    DuplicateId,
    EmptyFamily,
    FamilyLeaksAcrossSplits,
    ParameterOutOfBounds,
    UnsupportedOccluderPreset,
}

fn scenario(
    id: &'static str,
    tier: ResearchTier,
    family: &'static str,
    split: Split,
    perturbation: Perturbation,
) -> StressScenario {
    StressScenario {
        id,
        tier,
        family,
        split,
        perturbation,
    }
}

/// Fixed, reviewable interventions; values are synthetic stress points, not
/// inferred from a site or a calibrated population distribution.
pub fn scenarios() -> Vec<StressScenario> {
    use Perturbation::*;
    use ResearchTier::*;
    use Split::*;

    vec![
        scenario(
            "OPT-GEO-NEG-100",
            TaskPlaneOptics,
            "fixture_translation",
            Development,
            FixtureTranslationMm {
                dx: -100.0,
                dy: 0.0,
            },
        ),
        scenario(
            "OPT-GEO-POS-100",
            TaskPlaneOptics,
            "fixture_translation",
            Development,
            FixtureTranslationMm {
                dx: 100.0,
                dy: 0.0,
            },
        ),
        scenario(
            "OPT-GEO-NEG-200",
            TaskPlaneOptics,
            "fixture_translation",
            Development,
            FixtureTranslationMm {
                dx: 0.0,
                dy: -200.0,
            },
        ),
        scenario(
            "OPT-GEO-POS-200",
            TaskPlaneOptics,
            "fixture_translation",
            Development,
            FixtureTranslationMm {
                dx: 0.0,
                dy: 200.0,
            },
        ),
        scenario(
            "OPT-HEIGHT-080",
            TaskPlaneOptics,
            "fixture_height",
            Development,
            FixtureHeightScale { scale: 0.8 },
        ),
        scenario(
            "OPT-HEIGHT-120",
            TaskPlaneOptics,
            "fixture_height",
            Development,
            FixtureHeightScale { scale: 1.2 },
        ),
        scenario(
            "OPT-REFLECT-LOW",
            TaskPlaneOptics,
            "uniform_reflectance",
            Development,
            UniformReflectanceDelta { delta: -0.1 },
        ),
        scenario(
            "OPT-REFLECT-HIGH",
            TaskPlaneOptics,
            "uniform_reflectance",
            Development,
            UniformReflectanceDelta { delta: 0.1 },
        ),
        scenario(
            "OPT-POWER-090",
            TaskPlaneOptics,
            "power_input",
            Development,
            PowerInputScale { scale: 0.9 },
        ),
        scenario(
            "OPT-POWER-110",
            TaskPlaneOptics,
            "power_input",
            Development,
            PowerInputScale { scale: 1.1 },
        ),
        scenario(
            "OPT-ASYM-A",
            TaskPlaneOptics,
            "asymmetric_reflectance",
            HeldOut,
            WallReflectanceAsymmetry {
                north_delta: 0.15,
                south_delta: -0.1,
            },
        ),
        scenario(
            "OPT-ASYM-B",
            TaskPlaneOptics,
            "asymmetric_reflectance",
            HeldOut,
            WallReflectanceAsymmetry {
                north_delta: -0.1,
                south_delta: 0.15,
            },
        ),
        scenario(
            "OPT-OCCLUSION-A",
            TaskPlaneOptics,
            "partial_occlusion",
            HeldOut,
            OccluderPreset {
                preset: "small_offset",
            },
        ),
        scenario(
            "OPT-OCCLUSION-B",
            TaskPlaneOptics,
            "partial_occlusion",
            HeldOut,
            OccluderPreset {
                preset: "edge_intersection",
            },
        ),
        scenario(
            "OPT-COUPLED-A",
            TaskPlaneOptics,
            "coupled_geometry_reflectance",
            HeldOut,
            CoupledGeometryReflectance {
                dx_mm: -120.0,
                height_scale: 1.15,
                reflectance_delta: -0.08,
            },
        ),
        scenario(
            "OPT-COUPLED-B",
            TaskPlaneOptics,
            "coupled_geometry_reflectance",
            HeldOut,
            CoupledGeometryReflectance {
                dx_mm: 120.0,
                height_scale: 0.85,
                reflectance_delta: 0.08,
            },
        ),
        scenario(
            "CTL-BIAS-NEG",
            AdaptiveControl,
            "ambient_sensor_bias",
            Development,
            AmbientSensorBias {
                bias_fraction: -0.1,
            },
        ),
        scenario(
            "CTL-BIAS-POS",
            AdaptiveControl,
            "ambient_sensor_bias",
            Development,
            AmbientSensorBias {
                bias_fraction: 0.1,
            },
        ),
        scenario(
            "CTL-DROPOUT-005",
            AdaptiveControl,
            "ambient_sensor_dropout",
            Development,
            AmbientSensorDropout {
                dropout_fraction: 0.05,
            },
        ),
        scenario(
            "CTL-DROPOUT-020",
            AdaptiveControl,
            "ambient_sensor_dropout",
            Development,
            AmbientSensorDropout {
                dropout_fraction: 0.2,
            },
        ),
        scenario(
            "CTL-LATENCY-250",
            AdaptiveControl,
            "control_latency",
            HeldOut,
            ControlLatencySeconds { latency_s: 0.25 },
        ),
        scenario(
            "CTL-STALE-030",
            AdaptiveControl,
            "stale_sensor_measurement",
            HeldOut,
            StaleSensorAgeSeconds { age_s: 30.0 },
        ),
        scenario(
            "CTL-STALE-120",
            AdaptiveControl,
            "stale_sensor_measurement",
            HeldOut,
            StaleSensorAgeSeconds { age_s: 120.0 },
        ),
        scenario(
            "GRID-LOW-SOC-020",
            OffGridResilience,
            "battery_state_of_charge",
            Development,
            BatteryStateOfCharge { fraction: 0.2 },
        ),
        scenario(
            "GRID-LOW-SOC-080",
            OffGridResilience,
            "battery_state_of_charge",
            Development,
            BatteryStateOfCharge { fraction: 0.8 },
        ),
        scenario(
            "GRID-OUTAGE-030",
            OffGridResilience,
            "grid_outage_duration",
            HeldOut,
            GridOutageDurationSeconds { duration_s: 30.0 },
        ),
        scenario(
            "GRID-OUTAGE-300",
            OffGridResilience,
            "grid_outage_duration",
            HeldOut,
            GridOutageDurationSeconds { duration_s: 300.0 },
        ),
    ]
}

fn valid_perturbation(perturbation: Perturbation) -> Result<(), CorpusError> {
    let in_range = match perturbation {
        Perturbation::FixtureTranslationMm { dx, dy } => {
            dx.is_finite() && dy.is_finite() && dx.abs() <= 250.0 && dy.abs() <= 250.0
        }
        Perturbation::FixtureHeightScale { scale } => {
            scale.is_finite() && (0.5..=1.5).contains(&scale)
        }
        Perturbation::UniformReflectanceDelta { delta } => {
            delta.is_finite() && (-0.2..=0.2).contains(&delta)
        }
        Perturbation::PowerInputScale { scale } => {
            scale.is_finite() && (0.7..=1.3).contains(&scale)
        }
        Perturbation::WallReflectanceAsymmetry {
            north_delta,
            south_delta,
        } => {
            north_delta.is_finite()
                && south_delta.is_finite()
                && (-0.2..=0.2).contains(&north_delta)
                && (-0.2..=0.2).contains(&south_delta)
        }
        Perturbation::OccluderPreset { preset } => {
            if preset != "small_offset" && preset != "edge_intersection" {
                return Err(CorpusError::UnsupportedOccluderPreset);
            }
            true
        }
        Perturbation::CoupledGeometryReflectance {
            dx_mm,
            height_scale,
            reflectance_delta,
        } => {
            dx_mm.is_finite()
                && dx_mm.abs() <= 250.0
                && height_scale.is_finite()
                && (0.5..=1.5).contains(&height_scale)
                && reflectance_delta.is_finite()
                && (-0.2..=0.2).contains(&reflectance_delta)
        }
        Perturbation::AmbientSensorBias { bias_fraction } => {
            bias_fraction.is_finite() && bias_fraction.abs() <= 0.3
        }
        Perturbation::AmbientSensorDropout { dropout_fraction } => {
            dropout_fraction.is_finite() && (0.0..=0.5).contains(&dropout_fraction)
        }
        Perturbation::ControlLatencySeconds { latency_s } => {
            latency_s.is_finite() && (0.0..=2.0).contains(&latency_s)
        }
        Perturbation::StaleSensorAgeSeconds { age_s } => {
            age_s.is_finite() && (0.0..=300.0).contains(&age_s)
        }
        Perturbation::BatteryStateOfCharge { fraction } => {
            fraction.is_finite() && (0.0..=1.0).contains(&fraction)
        }
        Perturbation::GridOutageDurationSeconds { duration_s } => {
            duration_s.is_finite() && (0.0..=3600.0).contains(&duration_s)
        }
    };
    if in_range {
        Ok(())
    } else {
        Err(CorpusError::ParameterOutOfBounds)
    }
}

/// Enforce identity uniqueness, parameter bounds, and family-level split
/// isolation within each research tier. This prevents the same intervention
/// family from leaking between development and held-out evaluation.
pub fn validate_corpus(cases: &[StressScenario]) -> Result<(), CorpusError> {
    if cases.is_empty() {
        return Err(CorpusError::EmptyCorpus);
    }
    let mut ids = BTreeSet::new();
    let mut family_split = BTreeMap::new();
    for case in cases {
        if case.id.trim().is_empty() {
            return Err(CorpusError::EmptyId);
        }
        if !ids.insert(case.id) {
            return Err(CorpusError::DuplicateId);
        }
        if case.family.trim().is_empty() {
            return Err(CorpusError::EmptyFamily);
        }
        match family_split.get(&(case.tier, case.family)) {
            Some(split) if *split != case.split => {
                return Err(CorpusError::FamilyLeaksAcrossSplits);
            }
            _ => {
                family_split.insert((case.tier, case.family), case.split);
            }
        }
        valid_perturbation(case.perturbation)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_corpus_is_valid_and_has_held_out_families() {
        let cases = scenarios();
        assert_eq!(cases.len(), 27);
        assert_eq!(validate_corpus(&cases), Ok(()));
        assert!(cases.iter().any(|case| case.split == Split::HeldOut));
        assert!(cases
            .iter()
            .any(|case| case.tier == ResearchTier::OffGridResilience));
    }

    #[test]
    fn family_level_split_leakage_is_rejected() {
        let cases = [
            StressScenario {
                id: "DEV-GEO",
                tier: ResearchTier::TaskPlaneOptics,
                family: "geometry",
                split: Split::Development,
                perturbation: Perturbation::FixtureTranslationMm {
                    dx: 10.0,
                    dy: 0.0,
                },
            },
            StressScenario {
                id: "TEST-GEO",
                tier: ResearchTier::TaskPlaneOptics,
                family: "geometry",
                split: Split::HeldOut,
                perturbation: Perturbation::FixtureTranslationMm {
                    dx: 20.0,
                    dy: 0.0,
                },
            },
        ];
        assert_eq!(
            validate_corpus(&cases),
            Err(CorpusError::FamilyLeaksAcrossSplits)
        );
    }

    #[test]
    fn duplicate_scenario_ids_are_rejected() {
        let mut cases = scenarios();
        let duplicate = cases[0];
        cases.push(duplicate);
        assert_eq!(validate_corpus(&cases), Err(CorpusError::DuplicateId));
    }

    #[test]
    fn out_of_range_perturbation_is_rejected() {
        let case = StressScenario {
            id: "BAD-REFLECTANCE",
            tier: ResearchTier::TaskPlaneOptics,
            family: "bad_reflectance",
            split: Split::Development,
            perturbation: Perturbation::UniformReflectanceDelta { delta: 0.8 },
        };
        assert_eq!(
            validate_corpus(&[case]),
            Err(CorpusError::ParameterOutOfBounds)
        );
    }

    #[test]
    fn unknown_occluder_is_rejected() {
        let case = StressScenario {
            id: "BAD-OCCLUDER",
            tier: ResearchTier::TaskPlaneOptics,
            family: "occlusion",
            split: Split::HeldOut,
            perturbation: Perturbation::OccluderPreset {
                preset: "unspecified",
            },
        };
        assert_eq!(
            validate_corpus(&[case]),
            Err(CorpusError::UnsupportedOccluderPreset)
        );
    }
}
