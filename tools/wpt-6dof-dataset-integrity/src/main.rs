// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integrity checks for 6-DOF WPT experiment manifests.
//! Payload and calibration artifact bytes are deliberately not fetched here.

use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process::ExitCode};

const HEADER: [&str; 40] = [
    "dataset_id",
    "protocol_version",
    "evidence_class",
    "sampling_unit",
    "holdout_axis",
    "max_time_skew_ns",
    "claim_ceiling",
    "sample_id",
    "episode_id",
    "independent_unit_id",
    "split",
    "receiver_serial",
    "receiver_hardware_sha256",
    "transmitter_hardware_sha256",
    "site_id",
    "environment_sha256",
    "trajectory_id",
    "modality",
    "field_configuration_sha256",
    "coordinate_frame_sha256",
    "ground_truth_contract_sha256",
    "ground_truth_receipt_id",
    "ground_truth_receipt_sha256",
    "sensor_calibration_sha256",
    "measurement_contract_sha256",
    "observation_payload_id",
    "observation_payload_sha256",
    "pose_covariance_receipt_sha256",
    "ground_truth_time_ns",
    "observation_time_ns",
    "valid_until_ns",
    "position_x_um",
    "position_y_um",
    "position_z_um",
    "quaternion_w_q1e9",
    "quaternion_x_q1e9",
    "quaternion_y_q1e9",
    "quaternion_z_q1e9",
    "energy_receipt_id",
    "energy_receipt_sha256",
];

const MAX_SAMPLES: usize = 2_000_000;
const QUATERNION_SCALE: i128 = 1_000_000_000;
const QUATERNION_NORM_TOLERANCE: i128 = 1_000_000_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct DatasetMeta {
    dataset_id: String,
    evidence_class: String,
    sampling_unit: String,
    holdout_axis: String,
    max_time_skew_ns: u64,
    claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EpisodeIdentity {
    independent_unit_id: String,
    split: String,
    receiver_serial: String,
    receiver_hardware_sha256: String,
    transmitter_hardware_sha256: String,
    site_id: String,
    environment_sha256: String,
    trajectory_id: String,
    modality: String,
    coordinate_frame_sha256: String,
    ground_truth_contract_sha256: String,
    ground_truth_receipt_id: String,
    ground_truth_receipt_sha256: String,
    sensor_calibration_sha256: String,
    measurement_contract_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Sample {
    sample_id: String,
    episode_id: String,
    observation_payload_id: String,
    field_configuration_sha256: String,
    observation_payload_sha256: String,
    pose_covariance_receipt_sha256: String,
    ground_truth_time_ns: u64,
    observation_time_ns: u64,
    valid_until_ns: u64,
    position_um: [i64; 3],
    quaternion_q1e9: [i64; 4],
    energy_receipt_id: String,
    energy_receipt_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Dataset {
    meta: DatasetMeta,
    samples: Vec<Sample>,
    episode_count: usize,
    independent_unit_count: usize,
    split_counts: BTreeMap<String, usize>,
    holdout_entity_counts: BTreeMap<String, usize>,
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
        return Err(format!("{field}: empty or untrimmed unsigned integer"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{field}: expected non-negative integer"))
}

fn parse_i64(value: &str, field: &str) -> Result<i64, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{field}: empty or untrimmed signed integer"));
    }
    value
        .parse::<i64>()
        .map_err(|_| format!("{field}: expected signed integer"))
}

fn valid_split(value: &str) -> bool {
    matches!(value, "CALIBRATION" | "VALIDATION" | "TEST")
}

fn valid_modality(value: &str) -> bool {
    matches!(
        value,
        "IPT" | "CPT" | "RF_MICROWAVE" | "OPTICAL" | "CONDUCTIVE" | "POWER_LINE"
    )
}

fn holdout_value(axis: &str, row: &[&str]) -> Option<&str> {
    match axis {
        "EPISODE" => Some(row[8]),
        "RECEIVER_SERIAL" => Some(row[11]),
        "RECEIVER_HARDWARE_REVISION" => Some(row[12]),
        "TRANSMITTER_HARDWARE_REVISION" => Some(row[13]),
        "SITE" => Some(row[14]),
        "ENVIRONMENT" => Some(row[15]),
        "TRAJECTORY" => Some(row[16]),
        _ => None,
    }
}

fn quaternion_is_unit(q: [i64; 4]) -> bool {
    let norm_squared: i128 = q
        .iter()
        .map(|value| i128::from(*value) * i128::from(*value))
        .sum();
    (norm_squared - QUATERNION_SCALE * QUATERNION_SCALE).abs()
        <= QUATERNION_NORM_TOLERANCE
}

fn parse_dataset(input: &str) -> Result<Dataset, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["dataset input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(vec![format!(
            "dataset header must contain exactly {} columns",
            HEADER.len()
        )]);
    }

    let mut meta_values: Option<Vec<String>> = None;
    let mut meta: Option<DatasetMeta> = None;
    let mut episode_identities = BTreeMap::<String, EpisodeIdentity>::new();
    let mut episode_splits = BTreeMap::<String, String>::new();
    let mut independent_unit_splits = BTreeMap::<String, String>::new();
    let mut holdout_splits = BTreeMap::<String, String>::new();
    let mut samples = Vec::new();
    let mut sample_ids = BTreeSet::new();
    let mut observation_payload_ids = BTreeSet::new();
    let mut split_counts = BTreeMap::<String, usize>::new();
    let mut holdout_entity_counts = BTreeMap::<String, usize>::new();
    let mut errors = Vec::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("line {line_no}: blank rows are not allowed"));
            continue;
        }
        let row: Vec<&str> = line.split('\t').collect();
        if row.len() != HEADER.len() {
            errors.push(format!(
                "line {line_no}: expected {} fields, found {}",
                HEADER.len(),
                row.len()
            ));
            continue;
        }

        let row_meta: Vec<String> = row[..7].iter().map(|value| (*value).to_string()).collect();
        if let Some(first) = &meta_values {
            if *first != row_meta {
                errors.push(format!("line {line_no}: dataset-level metadata differs from first row"));
                continue;
            }
        } else {
            meta_values = Some(row_meta);
            let protocol_version = row[1];
            let max_time_skew_ns = match parse_u64(row[5], "max_time_skew_ns") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(format!("line {line_no}: {error}"));
                    continue;
                }
            };
            if !valid_id(row[0])
                || protocol_version != "1"
                || !matches!(
                    row[2],
                    "SYNTHETIC_KNOWN_TRUTH" | "BENCH_MEASURED" | "FIELD_MEASURED"
                )
                || row[3] != "EPISODE"
                || !matches!(
                    row[4],
                    "EPISODE"
                        | "RECEIVER_SERIAL"
                        | "RECEIVER_HARDWARE_REVISION"
                        | "TRANSMITTER_HARDWARE_REVISION"
                        | "SITE"
                        | "ENVIRONMENT"
                        | "TRAJECTORY"
                )
                || row[6].trim().is_empty()
                || row[6].len() > 1024
                || row[6].trim() != row[6]
            {
                errors.push(format!(
                    "line {line_no}: invalid dataset ID, version, evidence class, sampling unit, holdout axis or claim ceiling"
                ));
                continue;
            }
            meta = Some(DatasetMeta {
                dataset_id: row[0].to_string(),
                evidence_class: row[2].to_string(),
                sampling_unit: row[3].to_string(),
                holdout_axis: row[4].to_string(),
                max_time_skew_ns,
                claim_ceiling: row[6].to_string(),
            });
        }
        let dataset_meta = meta.as_ref().expect("metadata initialized");

        let sample_id = row[7];
        let episode_id = row[8];
        let independent_unit_id = row[9];
        let split = row[10];
        let receiver_serial = row[11];
        let receiver_hardware_sha256 = row[12];
        let transmitter_hardware_sha256 = row[13];
        let site_id = row[14];
        let environment_sha256 = row[15];
        let trajectory_id = row[16];
        let modality = row[17];
        let field_configuration_sha256 = row[18];
        let coordinate_frame_sha256 = row[19];
        let ground_truth_contract_sha256 = row[20];
        let ground_truth_receipt_id = row[21];
        let ground_truth_receipt_sha256 = row[22];
        let sensor_calibration_sha256 = row[23];
        let measurement_contract_sha256 = row[24];
        let observation_payload_id = row[25];
        let observation_payload_sha256 = row[26];
        let pose_covariance_receipt_sha256 = row[27];

        if !valid_id(sample_id)
            || !valid_id(episode_id)
            || !valid_id(independent_unit_id)
            || !valid_split(split)
            || !valid_id(receiver_serial)
            || !digest_valid(receiver_hardware_sha256)
            || !digest_valid(transmitter_hardware_sha256)
            || !valid_id(site_id)
            || !digest_valid(environment_sha256)
            || !valid_id(trajectory_id)
            || !valid_modality(modality)
            || !digest_valid(field_configuration_sha256)
            || !digest_valid(coordinate_frame_sha256)
            || !digest_valid(ground_truth_contract_sha256)
            || !valid_id(ground_truth_receipt_id)
            || !digest_valid(ground_truth_receipt_sha256)
            || !digest_valid(sensor_calibration_sha256)
            || !digest_valid(measurement_contract_sha256)
            || !valid_id(observation_payload_id)
            || !digest_valid(observation_payload_sha256)
            || !digest_valid(pose_covariance_receipt_sha256)
        {
            errors.push(format!("line {line_no}: invalid sample identifiers or evidence digests"));
            continue;
        }
        if !sample_ids.insert(sample_id.to_string()) {
            errors.push(format!("line {line_no}: duplicate sample_id {sample_id}"));
            continue;
        }
        if !observation_payload_ids.insert(observation_payload_id.to_string()) {
            errors.push(format!(
                "line {line_no}: duplicate observation_payload_id {observation_payload_id}"
            ));
            continue;
        }

        let ground_truth_time_ns = match parse_u64(row[28], "ground_truth_time_ns") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        let observation_time_ns = match parse_u64(row[29], "observation_time_ns") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        let valid_until_ns = match parse_u64(row[30], "valid_until_ns") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        let time_skew_ns = ground_truth_time_ns.abs_diff(observation_time_ns);
        if time_skew_ns > dataset_meta.max_time_skew_ns {
            errors.push(format!(
                "line {line_no}: ground-truth/observation time skew {time_skew_ns} exceeds {} ns",
                dataset_meta.max_time_skew_ns
            ));
            continue;
        }
        if valid_until_ns < ground_truth_time_ns.max(observation_time_ns) {
            errors.push(format!("line {line_no}: sample evidence expired before its measurement time"));
            continue;
        }

        let position_um = [
            parse_i64(row[31], "position_x_um"),
            parse_i64(row[32], "position_y_um"),
            parse_i64(row[33], "position_z_um"),
        ];
        let position_um = match position_um.into_iter().collect::<Result<Vec<_>, _>>() {
            Ok(values) => [values[0], values[1], values[2]],
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        let quaternion = [
            parse_i64(row[34], "quaternion_w_q1e9"),
            parse_i64(row[35], "quaternion_x_q1e9"),
            parse_i64(row[36], "quaternion_y_q1e9"),
            parse_i64(row[37], "quaternion_z_q1e9"),
        ];
        let quaternion_q1e9 = match quaternion.into_iter().collect::<Result<Vec<_>, _>>() {
            Ok(values) => [values[0], values[1], values[2], values[3]],
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        if !quaternion_is_unit(quaternion_q1e9) {
            errors.push(format!("line {line_no}: quaternion norm is outside v0 tolerance"));
            continue;
        }

        let energy_receipt_id = row[38];
        let energy_receipt_sha256 = row[39];
        if energy_receipt_id == "NONE" {
            if energy_receipt_sha256 != "NONE" {
                errors.push(format!(
                    "line {line_no}: energy receipt NONE requires energy digest NONE"
                ));
                continue;
            }
        } else if !valid_id(energy_receipt_id) || !digest_valid(energy_receipt_sha256) {
            errors.push(format!("line {line_no}: invalid optional energy receipt reference"));
            continue;
        }

        let identity = EpisodeIdentity {
            independent_unit_id: independent_unit_id.to_string(),
            split: split.to_string(),
            receiver_serial: receiver_serial.to_string(),
            receiver_hardware_sha256: receiver_hardware_sha256.to_string(),
            transmitter_hardware_sha256: transmitter_hardware_sha256.to_string(),
            site_id: site_id.to_string(),
            environment_sha256: environment_sha256.to_string(),
            trajectory_id: trajectory_id.to_string(),
            modality: modality.to_string(),
            coordinate_frame_sha256: coordinate_frame_sha256.to_string(),
            ground_truth_contract_sha256: ground_truth_contract_sha256.to_string(),
            ground_truth_receipt_id: ground_truth_receipt_id.to_string(),
            ground_truth_receipt_sha256: ground_truth_receipt_sha256.to_string(),
            sensor_calibration_sha256: sensor_calibration_sha256.to_string(),
            measurement_contract_sha256: measurement_contract_sha256.to_string(),
        };
        if let Some(existing) = episode_identities.get(episode_id) {
            if existing != &identity {
                errors.push(format!(
                    "line {line_no}: episode {episode_id} changes split, asset, modality, frame or calibration identity"
                ));
                continue;
            }
        } else {
            episode_identities.insert(episode_id.to_string(), identity);
        }

        for (map, key, label) in [
            (&mut episode_splits, episode_id.to_string(), "episode"),
            (
                &mut independent_unit_splits,
                independent_unit_id.to_string(),
                "independent unit",
            ),
        ] {
            if let Some(existing_split) = map.get(&key) {
                if existing_split != split {
                    errors.push(format!(
                        "line {line_no}: {label} {key} leaks across data splits"
                    ));
                }
            } else {
                map.insert(key, split.to_string());
            }
        }

        let entity = holdout_value(dataset_meta.holdout_axis.as_str(), &row)
            .expect("dataset parser validates holdout axis")
            .to_string();
        if let Some(existing_split) = holdout_splits.get(&entity) {
            if existing_split != split {
                errors.push(format!(
                    "line {line_no}: holdout entity {entity} for axis {} leaks across data splits",
                    dataset_meta.holdout_axis
                ));
            }
        } else {
            holdout_splits.insert(entity.clone(), split.to_string());
        }

        samples.push(Sample {
            sample_id: sample_id.to_string(),
            episode_id: episode_id.to_string(),
            observation_payload_id: observation_payload_id.to_string(),
            field_configuration_sha256: field_configuration_sha256.to_string(),
            observation_payload_sha256: observation_payload_sha256.to_string(),
            pose_covariance_receipt_sha256: pose_covariance_receipt_sha256.to_string(),
            ground_truth_time_ns,
            observation_time_ns,
            valid_until_ns,
            position_um,
            quaternion_q1e9,
            energy_receipt_id: energy_receipt_id.to_string(),
            energy_receipt_sha256: energy_receipt_sha256.to_string(),
        });
        *split_counts.entry(split.to_string()).or_insert(0) += 1;
        *holdout_entity_counts.entry(split.to_string()).or_insert(0) += 0;
        holdout_entity_counts.insert(
            split.to_string(),
            holdout_splits
                .iter()
                .filter(|(_, entity_split)| *entity_split == split)
                .count(),
        );

        if samples.len() > MAX_SAMPLES {
            errors.push(format!("dataset exceeds {MAX_SAMPLES} pose-observation samples"));
            break;
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    let Some(meta) = meta else {
        return Err(vec!["dataset contains no sample rows".to_string()]);
    };
    if samples.is_empty() {
        return Err(vec!["dataset contains no sample rows".to_string()]);
    }
    for split in ["CALIBRATION", "VALIDATION", "TEST"] {
        if !split_counts.contains_key(split) {
            return Err(vec![format!("dataset is missing required {split} partition")]);
        }
    }

    samples.sort_by(|left, right| {
        left.episode_id
            .cmp(&right.episode_id)
            .then(left.ground_truth_time_ns.cmp(&right.ground_truth_time_ns))
            .then(left.sample_id.cmp(&right.sample_id))
    });

    Ok(Dataset {
        meta,
        samples,
        episode_count: episode_splits.len(),
        independent_unit_count: independent_unit_splits.len(),
        split_counts,
        holdout_entity_counts,
    })
}

fn run(path: &str) -> Result<(), String> {
    let input =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let dataset = parse_dataset(&input).map_err(|errors| errors.join("\n"))?;

    println!("dataset_id={}", dataset.meta.dataset_id);
    println!("evidence_class={}", dataset.meta.evidence_class);
    println!("sampling_unit={}", dataset.meta.sampling_unit);
    println!("holdout_axis={}", dataset.meta.holdout_axis);
    println!("sample_count={}", dataset.samples.len());
    println!("episode_count={}", dataset.episode_count);
    println!("independent_unit_count={}", dataset.independent_unit_count);
    println!("max_time_skew_ns={}", dataset.meta.max_time_skew_ns);
    println!(
        "calibration_samples={}",
        dataset.split_counts.get("CALIBRATION").copied().unwrap_or(0)
    );
    println!(
        "validation_samples={}",
        dataset.split_counts.get("VALIDATION").copied().unwrap_or(0)
    );
    println!(
        "test_samples={}",
        dataset.split_counts.get("TEST").copied().unwrap_or(0)
    );
    println!(
        "holdout_entities_by_split={}",
        dataset
            .holdout_entity_counts
            .iter()
            .map(|(split, count)| format!("{split}:{count}"))
            .collect::<Vec<_>>()
            .join(";")
    );
    println!("artifact_bytes_verified=false");
    println!("ground_truth_calibration_verified=false");
    println!("independence_assumption=DECLARED_NOT_PROVEN");
    println!("continuous_6dof_coverage=NOT_ESTABLISHED");
    println!("operational_permit=false");
    println!("authority=NONE_DATASET_MANIFEST_INTEGRITY_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: wpt-6dof-dataset-integrity <manifest.tsv>");
        return ExitCode::from(2);
    }
    match run(&args[1]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("REJECTED_WPT_6DOF_DATASET {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../research/wpt-6dof-physical-observability-v0/synthetic-episode-manifest.tsv");

    fn parsed(input: &str) -> Dataset {
        parse_dataset(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    #[test]
    fn known_synthetic_manifest_has_three_disjoint_holdout_partitions() {
        let dataset = parsed(FIXTURE);
        assert_eq!(dataset.meta.holdout_axis, "RECEIVER_SERIAL");
        assert_eq!(dataset.samples.len(), 6);
        assert_eq!(dataset.episode_count, 3);
        assert_eq!(dataset.independent_unit_count, 3);
        assert_eq!(dataset.split_counts["CALIBRATION"], 2);
        assert_eq!(dataset.split_counts["VALIDATION"], 2);
        assert_eq!(dataset.split_counts["TEST"], 2);
        assert_eq!(dataset.holdout_entity_counts["CALIBRATION"], 1);
        assert_eq!(dataset.holdout_entity_counts["VALIDATION"], 1);
        assert_eq!(dataset.holdout_entity_counts["TEST"], 1);
    }

    #[test]
    fn row_permutation_preserves_the_validated_dataset() {
        let mut rows: Vec<&str> = FIXTURE.lines().collect();
        let header = rows.remove(0);
        rows.reverse();
        let permuted = format!("{header}\n{}\n", rows.join("\n"));
        assert_eq!(parsed(FIXTURE), parsed(&permuted));
    }

    #[test]
    fn receiver_serial_cannot_cross_calibration_and_test() {
        let leaked = FIXTURE.replace(
            "RX_TEST_01",
            "RX_CAL_01",
        );
        assert!(parse_dataset(&leaked).is_err());
    }

    #[test]
    fn one_episode_cannot_be_split_across_partitions() {
        let leaked = FIXTURE.replace(
            "SAMPLE_C1_B\tEP_CAL_01\tUNIT_CAL_01\tCALIBRATION",
            "SAMPLE_C1_B\tEP_CAL_01\tUNIT_CAL_01\tTEST",
        );
        assert!(parse_dataset(&leaked).is_err());
    }

    #[test]
    fn independent_unit_cannot_cross_partitions() {
        let leaked = FIXTURE.replace(
            "EP_TEST_01\tUNIT_TEST_01\tTEST",
            "EP_TEST_01\tUNIT_CAL_01\tTEST",
        );
        assert!(parse_dataset(&leaked).is_err());
    }

    #[test]
    fn invalid_quaternion_norm_is_rejected() {
        let invalid = FIXTURE.replace(
            "\t1000000000\t0\t0\t0\tNONE\tNONE",
            "\t0\t0\t0\t0\tNONE\tNONE",
        );
        assert!(parse_dataset(&invalid).is_err());
    }

    #[test]
    fn excessive_timestamp_skew_is_rejected() {
        let invalid = FIXTURE.replace(
            "\t1000000\t1000500\t",
            "\t1000000\t3000000\t",
        );
        assert!(parse_dataset(&invalid).is_err());
    }

    #[test]
    fn validity_horizon_must_cover_ground_truth_and_observation_times() {
        let invalid = FIXTURE.replace(
            "\t1000000\t1000500\t11000000",
            "\t1000000\t1000500\t1000400",
        );
        assert!(parse_dataset(&invalid).is_err());
    }

    #[test]
    fn coordinate_frame_cannot_change_within_episode() {
        let invalid = FIXTURE.replacen(
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "9999999999999999999999999999999999999999999999999999999999999999",
            1,
        );
        assert!(parse_dataset(&invalid).is_err());
    }

    #[test]
    fn hardware_revision_holdout_axis_blocks_revision_leakage() {
        let input = FIXTURE.replacen(
            "RECEIVER_SERIAL",
            "RECEIVER_HARDWARE_REVISION",
            1,
        );
        assert!(parse_dataset(&input).is_ok());
        let leaked = input.replace(
            "RX_TEST_01\tcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "RX_TEST_01\taaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert!(parse_dataset(&leaked).is_err());
    }

    #[test]
    fn duplicate_sample_and_payload_ids_are_rejected() {
        let duplicate_sample = format!(
            "{FIXTURE}{}",
            FIXTURE.lines().nth(1).expect("first sample")
        );
        assert!(parse_dataset(&duplicate_sample).is_err());

        let duplicate_payload = FIXTURE.replacen(
            "OBS_C1_B",
            "OBS_C1_A",
            1,
        );
        assert!(parse_dataset(&duplicate_payload).is_err());
    }

    #[test]
    fn invalid_energy_receipt_pairing_is_rejected() {
        let invalid = FIXTURE.replacen(
            "\tNONE\tNONE",
            "\tENERGY_RECEIPT_ONLY\tNONE",
            1,
        );
        assert!(parse_dataset(&invalid).is_err());
    }

    #[test]
    fn unsupported_sampling_unit_and_holdout_axis_are_rejected() {
        let frame_sampling = FIXTURE.replace("EPISODE", "FRAME");
        assert!(parse_dataset(&frame_sampling).is_err());

        let wildcard_axis = FIXTURE.replace("RECEIVER_SERIAL", "ANY");
        assert!(parse_dataset(&wildcard_axis).is_err());
    }

    #[test]
    fn invalid_digest_and_unknown_modality_are_rejected() {
        let invalid_digest = FIXTURE.replacen(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "not-a-digest",
            1,
        );
        assert!(parse_dataset(&invalid_digest).is_err());

        let unknown_modality = FIXTURE.replace("\tIPT\t", "\tUNKNOWN\t");
        assert!(parse_dataset(&unknown_modality).is_err());
    }
}
