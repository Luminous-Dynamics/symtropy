// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! First-failure, right-censored component lifetime ledger.
//! Kaplan-Meier summaries are descriptive within one declared test group only.

use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process::ExitCode};

const HEADER: [&str; 16] = [
    "test_group_id",
    "evidence_class",
    "bearer",
    "component_family",
    "hardware_sha256",
    "site_id",
    "environment_stratum",
    "test_regime",
    "exposure_unit",
    "measurement_sha256",
    "failure_criterion_sha256",
    "asset_serial",
    "exposure_value",
    "outcome",
    "failure_mode",
    "valid_until_us",
];

const MAX_COMPONENTS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Group {
    test_group_id: String,
    evidence_class: String,
    bearer: String,
    component_family: String,
    hardware_sha256: String,
    site_id: String,
    environment_stratum: String,
    test_regime: String,
    exposure_unit: String,
    measurement_sha256: String,
    failure_criterion_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Observation {
    asset_serial: String,
    exposure_value: u64,
    failed: bool,
    failure_mode: String,
    valid_until_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Cohort {
    group: Group,
    observations: Vec<Observation>,
}

#[derive(Clone, Debug, PartialEq)]
struct SurvivalPoint {
    exposure_value: u64,
    at_risk: usize,
    failures: usize,
    survival: f64,
    greenwood_sum: Option<f64>,
    lower_95_log_log: Option<f64>,
    upper_95_log_log: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
struct Evaluation {
    component_count: usize,
    failure_count: usize,
    right_censored_count: usize,
    failure_modes: BTreeMap<String, usize>,
    curve: Vec<SurvivalPoint>,
    median_failure_exposure: Option<u64>,
    minimum_valid_until_us: u64,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{field}: empty or untrimmed integer"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{field}: expected non-negative integer"))
}

fn allowed_evidence(value: &str) -> bool {
    matches!(
        value,
        "SYNTHETIC_KNOWN_TRUTH" | "BENCH_MEASURED" | "FIELD_MEASURED"
    )
}

fn allowed_bearer(value: &str) -> bool {
    matches!(
        value,
        "CONDUCTIVE"
            | "IPT"
            | "CPT"
            | "BATTERY_EXCHANGE"
            | "POWER_LINE"
            | "RF_MICROWAVE"
            | "OPTICAL"
    )
}

fn allowed_regime(value: &str) -> bool {
    matches!(value, "BENCH" | "FIELD" | "ACCELERATED")
}

fn allowed_unit(value: &str) -> bool {
    matches!(
        value,
        "MATING_CYCLES"
            | "THERMAL_CYCLES"
            | "POWER_ON_HOURS"
            | "FLIGHT_HOURS"
            | "CALENDAR_HOURS"
            | "ENERGY_THROUGHPUT_MWH"
    )
}

fn parse_cohort(input: &str) -> Result<Cohort, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(vec![format!(
            "header must contain exactly {} columns",
            HEADER.len()
        )]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut group: Option<Group> = None;
    let mut serials = BTreeSet::new();
    let mut observations = Vec::new();
    let mut errors = Vec::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != HEADER.len() {
            errors.push(format!(
                "line {line_no}: expected {} columns, found {}",
                HEADER.len(),
                fields.len()
            ));
            continue;
        }

        let row_metadata: Vec<String> =
            fields[..11].iter().map(|field| (*field).to_string()).collect();
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!(
                    "line {line_no}: test-group metadata differs from first row"
                ));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            let candidate = Group {
                test_group_id: fields[0].to_string(),
                evidence_class: fields[1].to_string(),
                bearer: fields[2].to_string(),
                component_family: fields[3].to_string(),
                hardware_sha256: fields[4].to_string(),
                site_id: fields[5].to_string(),
                environment_stratum: fields[6].to_string(),
                test_regime: fields[7].to_string(),
                exposure_unit: fields[8].to_string(),
                measurement_sha256: fields[9].to_string(),
                failure_criterion_sha256: fields[10].to_string(),
            };
            if !valid_id(&candidate.test_group_id)
                || !allowed_evidence(&candidate.evidence_class)
                || !allowed_bearer(&candidate.bearer)
                || !valid_id(&candidate.component_family)
                || !digest_valid(&candidate.hardware_sha256)
                || !valid_id(&candidate.site_id)
                || !valid_id(&candidate.environment_stratum)
                || !allowed_regime(&candidate.test_regime)
                || !allowed_unit(&candidate.exposure_unit)
                || !digest_valid(&candidate.measurement_sha256)
                || !digest_valid(&candidate.failure_criterion_sha256)
            {
                errors.push(format!(
                    "line {line_no}: invalid test group, evidence class, bearer, regime, unit or digest"
                ));
                continue;
            }
            if candidate.test_regime == "FIELD"
                && candidate.evidence_class == "BENCH_MEASURED"
            {
                errors.push(format!(
                    "line {line_no}: FIELD regime cannot use BENCH_MEASURED evidence"
                ));
                continue;
            }
            if matches!(candidate.test_regime.as_str(), "BENCH" | "ACCELERATED")
                && candidate.evidence_class == "FIELD_MEASURED"
            {
                errors.push(format!(
                    "line {line_no}: BENCH/ACCELERATED regime cannot use FIELD_MEASURED evidence"
                ));
                continue;
            }
            group = Some(candidate);
        }

        let asset_serial = fields[11];
        if !valid_id(asset_serial) {
            errors.push(format!("line {line_no}: invalid asset_serial"));
            continue;
        }
        if !serials.insert(asset_serial.to_string()) {
            errors.push(format!(
                "line {line_no}: duplicate physical asset_serial {asset_serial}"
            ));
            continue;
        }

        let exposure_value = match parse_u64(fields[12], "exposure_value") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        if exposure_value == 0 {
            errors.push(format!("line {line_no}: exposure_value must be positive"));
            continue;
        }

        let failed = match fields[13] {
            "FAILED" => true,
            "RIGHT_CENSORED" => false,
            other => {
                errors.push(format!(
                    "line {line_no}: outcome must be FAILED or RIGHT_CENSORED, found {other:?}"
                ));
                continue;
            }
        };
        let failure_mode = fields[14];
        if failed {
            if !valid_id(failure_mode) || failure_mode == "NONE" {
                errors.push(format!(
                    "line {line_no}: FAILED requires a predeclared failure_mode"
                ));
                continue;
            }
        } else if failure_mode != "NONE" {
            errors.push(format!(
                "line {line_no}: RIGHT_CENSORED requires failure_mode NONE"
            ));
            continue;
        }

        let valid_until_us = match parse_u64(fields[15], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };

        observations.push(Observation {
            asset_serial: asset_serial.to_string(),
            exposure_value,
            failed,
            failure_mode: failure_mode.to_string(),
            valid_until_us,
        });
        if observations.len() > MAX_COMPONENTS {
            errors.push(format!("cohort exceeds {MAX_COMPONENTS} physical components"));
            break;
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    let Some(group) = group else {
        return Err(vec!["cohort has no component observations".to_string()]);
    };
    if observations.is_empty() {
        return Err(vec!["cohort has no component observations".to_string()]);
    }
    observations.sort_by(|left, right| {
        left.exposure_value
            .cmp(&right.exposure_value)
            .then(left.asset_serial.cmp(&right.asset_serial))
    });

    Ok(Cohort { group, observations })
}

fn evaluate(cohort: &Cohort) -> Evaluation {
    let failure_count = cohort.observations.iter().filter(|item| item.failed).count();
    let right_censored_count = cohort.observations.len() - failure_count;
    let mut failure_modes = BTreeMap::new();

    for item in &cohort.observations {
        if item.failed {
            *failure_modes.entry(item.failure_mode.clone()).or_insert(0) += 1;
        }
    }

    let failure_times: BTreeSet<u64> = cohort
        .observations
        .iter()
        .filter(|item| item.failed)
        .map(|item| item.exposure_value)
        .collect();

    let mut survival = 1.0f64;
    let mut greenwood_sum = 0.0f64;
    let mut greenwood_defined = true;
    let mut curve = Vec::new();
    let mut median_failure_exposure = None;

    for exposure_value in failure_times {
        // All observations at t (failures and censoring) are at risk immediately
        // before failures at t are applied. Censoring at tied t is then removed.
        let at_risk = cohort
            .observations
            .iter()
            .filter(|item| item.exposure_value >= exposure_value)
            .count();
        let failures = cohort
            .observations
            .iter()
            .filter(|item| item.failed && item.exposure_value == exposure_value)
            .count();

        if at_risk == 0 || failures > at_risk {
            // Parser and cohort invariants make this unreachable.
            continue;
        }

        survival *= (at_risk - failures) as f64 / at_risk as f64;

        // Greenwood's cumulative sum is sum_j d_j / (n_j * (n_j - d_j)).
        // If every remaining component fails at one event time, the denominator
        // is zero and a finite Greenwood variance / log-log interval is undefined.
        if failures == at_risk {
            greenwood_defined = false;
        } else if greenwood_defined {
            greenwood_sum += failures as f64
                / (at_risk as f64 * (at_risk - failures) as f64);
        }

        let (point_greenwood, lower_95_log_log, upper_95_log_log) =
            if greenwood_defined && survival > 0.0 && survival < 1.0 {
                let log_neg_log_survival = (-survival.ln()).ln();
                let standard_error = greenwood_sum.sqrt() / survival.ln().abs();
                let z_975 = 1.959_963_984_540_054_f64;
                let lower = (-(log_neg_log_survival + z_975 * standard_error).exp()).exp();
                let upper = (-(log_neg_log_survival - z_975 * standard_error).exp()).exp();
                (Some(greenwood_sum), Some(lower), Some(upper))
            } else {
                (None, None, None)
            };

        curve.push(SurvivalPoint {
            exposure_value,
            at_risk,
            failures,
            survival,
            greenwood_sum: point_greenwood,
            lower_95_log_log,
            upper_95_log_log,
        });
        if median_failure_exposure.is_none() && survival <= 0.5 {
            median_failure_exposure = Some(exposure_value);
        }
    }

    let minimum_valid_until_us = cohort
        .observations
        .iter()
        .map(|item| item.valid_until_us)
        .min()
        .unwrap_or(0);

    Evaluation {
        component_count: cohort.observations.len(),
        failure_count,
        right_censored_count,
        failure_modes,
        curve,
        median_failure_exposure,
        minimum_valid_until_us,
    }
}

fn run(path: &str) -> Result<(), String> {
    let input =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let cohort = parse_cohort(&input).map_err(|errors| errors.join("\n"))?;
    let result = evaluate(&cohort);

    println!("test_group_id={}", cohort.group.test_group_id);
    println!("evidence_class={}", cohort.group.evidence_class);
    println!("bearer={}", cohort.group.bearer);
    println!("component_family={}", cohort.group.component_family);
    println!("hardware_sha256={}", cohort.group.hardware_sha256);
    println!("site_id={}", cohort.group.site_id);
    println!("environment_stratum={}", cohort.group.environment_stratum);
    println!("test_regime={}", cohort.group.test_regime);
    println!("exposure_unit={}", cohort.group.exposure_unit);
    println!("component_count={}", result.component_count);
    println!("failure_count={}", result.failure_count);
    println!("right_censored_count={}", result.right_censored_count);
    println!("minimum_valid_until_us={}", result.minimum_valid_until_us);
    match result.median_failure_exposure {
        Some(value) => println!("median_failure_exposure={value}"),
        None => println!("median_failure_exposure=NOT_REACHED"),
    }
    println!(
        "failure_modes={}",
        result
            .failure_modes
            .iter()
            .map(|(mode, count)| format!("{mode}:{count}"))
            .collect::<Vec<_>>()
            .join(";")
    );
    for point in &result.curve {
        let greenwood = point
            .greenwood_sum
            .map(|value| format!("{value:.6}"))
            .unwrap_or_else(|| "UNDEFINED_BOUNDARY".to_string());
        let lower = point
            .lower_95_log_log
            .map(|value| format!("{value:.6}"))
            .unwrap_or_else(|| "UNDEFINED_BOUNDARY".to_string());
        let upper = point
            .upper_95_log_log
            .map(|value| format!("{value:.6}"))
            .unwrap_or_else(|| "UNDEFINED_BOUNDARY".to_string());
        println!(
            "survival_at_{}={:.6};at_risk={};failures={};greenwood_sum={};pointwise_95_loglog=[{},{}]",
            point.exposure_value, point.survival, point.at_risk, point.failures, greenwood, lower, upper
        );
    }
    println!("censoring_assumption=NON_INFORMATIVE_WITHIN_DECLARED_TEST_GROUP_NOT_VERIFIED");
    println!("component_independence=DECLARED_PHYSICAL_ASSET_SERIAL_NOT_VERIFIED");
    println!("accelerated_to_field_extrapolation=NOT_PERFORMED");
    println!("confidence_intervals=GREENWOOD_POINTWISE_95_PERCENT_LOG_LOG_APPROXIMATION");
    println!("operational_permit=false");
    println!("authority=NONE_DESCRIPTIVE_LIFECYCLE_EVIDENCE_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: wpt-lifecycle-evidence <cohort.tsv>");
        return ExitCode::from(2);
    }
    match run(&args[1]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("REJECTED_WPT_LIFECYCLE_COHORT {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../research/wpt-lifecycle-evidence-v0/synthetic-contact-cohort.tsv");

    fn parsed(input: &str) -> Cohort {
        parse_cohort(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    #[test]
    fn kaplan_meier_curve_matches_hand_enumerated_failure_and_censoring_counts() {
        let cohort = parsed(FIXTURE);
        let result = evaluate(&cohort);
        assert_eq!(result.component_count, 5);
        assert_eq!(result.failure_count, 3);
        assert_eq!(result.right_censored_count, 2);
        assert_eq!(result.curve.len(), 2);
        assert_eq!(result.curve[0].exposure_value, 10);
        assert_eq!(result.curve[0].at_risk, 5);
        assert_eq!(result.curve[0].failures, 1);
        assert!((result.curve[0].survival - 0.8).abs() < 1e-12);
        assert_eq!(result.curve[1].exposure_value, 20);
        assert_eq!(result.curve[1].at_risk, 3);
        assert_eq!(result.curve[1].failures, 2);
        assert!((result.curve[1].survival - (4.0 / 15.0)).abs() < 1e-12);
        assert_eq!(result.median_failure_exposure, Some(20));
        assert_eq!(result.failure_modes.get("CONTACT_RESISTANCE_DRIFT"), Some(&2));
        assert_eq!(result.failure_modes.get("CONTACT_DROPOUT"), Some(&1));
    }

    #[test]
    fn greenwood_variance_and_pointwise_log_log_bounds_are_reported() {
        let result = evaluate(&parsed(FIXTURE));
        assert!((result.curve[0].greenwood_sum.unwrap() - 0.05).abs() < 1e-12);
        assert!((result.curve[1].greenwood_sum.unwrap() - (0.05 + 2.0 / 3.0)).abs() < 1e-12);
        for point in &result.curve {
            let lower = point.lower_95_log_log.expect("finite survival lower bound");
            let upper = point.upper_95_log_log.expect("finite survival upper bound");
            assert!(lower >= 0.0 && lower <= point.survival);
            assert!(point.survival <= upper && upper <= 1.0);
        }
    }

    #[test]
    fn complete_failure_boundary_has_no_finite_greenwood_interval() {
        let first_row = FIXTURE.lines().nth(1).expect("first component");
        let single = format!("{}\n{}\n", FIXTURE.lines().next().unwrap(), first_row);
        let result = evaluate(&parsed(&single));
        assert_eq!(result.curve.len(), 1);
        assert_eq!(result.curve[0].survival, 0.0);
        assert_eq!(result.curve[0].greenwood_sum, None);
        assert_eq!(result.curve[0].lower_95_log_log, None);
        assert_eq!(result.curve[0].upper_95_log_log, None);
    }

    #[test]
    fn all_right_censored_cohort_has_no_failure_curve_and_no_median() {
        let censored = FIXTURE
            .replace("\tFAILED\tCONTACT_RESISTANCE_DRIFT", "\tRIGHT_CENSORED\tNONE")
            .replace("\tFAILED\tCONTACT_DROPOUT", "\tRIGHT_CENSORED\tNONE");
        let result = evaluate(&parsed(&censored));
        assert_eq!(result.failure_count, 0);
        assert_eq!(result.right_censored_count, 5);
        assert!(result.curve.is_empty());
        assert_eq!(result.median_failure_exposure, None);
    }

    #[test]
    fn failure_and_censor_at_same_exposure_use_declared_tie_rule() {
        let tied = FIXTURE.replace("\tASSET_B\t15\tRIGHT_CENSORED", "\tASSET_B\t20\tRIGHT_CENSORED");
        let result = evaluate(&parsed(&tied));
        assert_eq!(result.curve[1].at_risk, 4);
        assert_eq!(result.curve[1].failures, 2);
        assert!((result.curve[1].survival - 0.4).abs() < 1e-12);
    }

    #[test]
    fn row_permutation_preserves_curve_and_summary() {
        let mut rows: Vec<&str> = FIXTURE.lines().collect();
        let header = rows.remove(0);
        rows.reverse();
        let permuted = format!("{header}\n{}\n", rows.join("\n"));
        assert_eq!(evaluate(&parsed(FIXTURE)), evaluate(&parsed(&permuted)));
    }

    #[test]
    fn duplicate_physical_asset_serials_are_rejected() {
        let duplicate = format!(
            "{FIXTURE}{}",
            FIXTURE.lines().nth(1).expect("first asset row")
        );
        assert!(parse_cohort(&duplicate).is_err());
    }

    #[test]
    fn accelerated_and_field_test_regimes_cannot_be_pooled() {
        let mixed = FIXTURE.replacen("\tBENCH\tMATING_CYCLES", "\tFIELD\tMATING_CYCLES", 1);
        assert!(parse_cohort(&mixed).is_err());
    }

    #[test]
    fn hardware_revision_mismatch_is_rejected() {
        let mismatch = FIXTURE.replacen(
            "1111111111111111111111111111111111111111111111111111111111111111",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            1,
        );
        assert!(parse_cohort(&mismatch).is_err());
    }

    #[test]
    fn censored_record_cannot_carry_a_failure_mode() {
        let malformed = FIXTURE.replace("\tRIGHT_CENSORED\tNONE", "\tRIGHT_CENSORED\tCONTACT_DROPOUT");
        assert!(parse_cohort(&malformed).is_err());
    }

    #[test]
    fn failed_record_requires_a_failure_mode() {
        let malformed = FIXTURE.replace("\tFAILED\tCONTACT_DROPOUT", "\tFAILED\tNONE");
        assert!(parse_cohort(&malformed).is_err());
    }

    #[test]
    fn exposure_units_cannot_be_mixed() {
        let mixed = FIXTURE.replacen("MATING_CYCLES", "POWER_ON_HOURS", 1);
        assert!(parse_cohort(&mixed).is_err());
    }

    #[test]
    fn field_measurements_cannot_claim_accelerated_regime() {
        let invalid = FIXTURE.replace("SYNTHETIC_KNOWN_TRUTH", "FIELD_MEASURED");
        assert!(parse_cohort(&invalid).is_err());
    }

    #[test]
    fn zero_exposure_observations_are_rejected() {
        let zero = FIXTURE.replace("\tASSET_A\t10\tFAILED", "\tASSET_A\t0\tFAILED");
        assert!(parse_cohort(&zero).is_err());
    }
}
