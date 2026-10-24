use lighting_benchmark::stress_matrix::{scenarios, validate_corpus, Perturbation};

fn tier_name(tier: lighting_benchmark::stress_matrix::ResearchTier) -> &'static str {
    use lighting_benchmark::stress_matrix::ResearchTier::*;
    match tier {
        TaskPlaneOptics => "task_plane_optics",
        AdaptiveControl => "adaptive_control",
        OffGridResilience => "off_grid_resilience",
    }
}

fn split_name(split: lighting_benchmark::stress_matrix::Split) -> &'static str {
    use lighting_benchmark::stress_matrix::Split::*;
    match split {
        Development => "development",
        HeldOut => "held_out",
    }
}

fn perturbation_record(value: Perturbation) -> String {
    match value {
        Perturbation::FixtureTranslationMm { dx, dy } => {
            format!("fixture_translation_mm;dx={dx};dy={dy}")
        }
        Perturbation::FixtureHeightScale { scale } => {
            format!("fixture_height_scale;scale={scale}")
        }
        Perturbation::UniformReflectanceDelta { delta } => {
            format!("uniform_reflectance_delta;delta={delta}")
        }
        Perturbation::PowerInputScale { scale } => {
            format!("power_input_scale;scale={scale}")
        }
        Perturbation::WallReflectanceAsymmetry {
            north_delta,
            south_delta,
        } => format!("wall_asymmetry;north={north_delta};south={south_delta}"),
        Perturbation::OccluderPreset { preset } => {
            format!("occluder_preset;preset={preset}")
        }
        Perturbation::CoupledGeometryReflectance {
            dx_mm,
            height_scale,
            reflectance_delta,
        } => format!("coupled;dx_mm={dx_mm};height={height_scale};reflectance={reflectance_delta}"),
        Perturbation::AmbientSensorBias { bias_fraction } => {
            format!("ambient_sensor_bias;bias={bias_fraction}")
        }
        Perturbation::AmbientSensorDropout { dropout_fraction } => {
            format!("ambient_sensor_dropout;dropout={dropout_fraction}")
        }
        Perturbation::ControlLatencySeconds { latency_s } => {
            format!("control_latency_seconds;latency={latency_s}")
        }
        Perturbation::StaleSensorAgeSeconds { age_s } => {
            format!("stale_sensor_age_seconds;age={age_s}")
        }
        Perturbation::BatteryStateOfCharge { fraction } => {
            format!("battery_soc;fraction={fraction}")
        }
        Perturbation::GridOutageDurationSeconds { duration_s } => {
            format!("grid_outage_duration;duration={duration_s}")
        }
    }
}

fn main() {
    let cases = scenarios();
    if let Err(error) = validate_corpus(&cases) {
        eprintln!("lighting stress corpus rejected: {error:?}");
        std::process::exit(2);
    }

    eprintln!("SYNTHETIC STRESS CASES ONLY — not calibrated distributions or performance results");
    println!("schema_version,case_id,tier,family,split,perturbation");
    for case in &cases {
        let perturbation = perturbation_record(case.perturbation).replace(',', ";");
        println!(
            "0.1.0,{},{},{},{},{}",
            case.id,
            tier_name(case.tier),
            case.family,
            split_name(case.split),
            perturbation
        );
    }
    eprintln!("validated_cases={}", cases.len());
}
