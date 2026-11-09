// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Synthetic anti-leakage contract for aerospace SHM benchmark partitions.
// This validates split metadata only; it does not admit or score any dataset.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const FIXTURE: &[u8] = include_bytes!("../../data/benchmark-split-contract-v0.1.json");
const CLAIM_CEILING: &str =
    "synthetic_partition_integrity_only_no_dataset_or_model_performance_claim";
const REQUIRED_SPLITS: [&str; 3] = ["train", "validation", "test"];

type AuditResult<T> = Result<T, String>;

#[derive(Clone, Debug, Deserialize)]
struct PurgePolicy {
    feature_window_seconds: f64,
    preprocessing_memory_seconds: f64,
    minimum_purge_gap_seconds: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct SampleRecord {
    sample_id: String,
    split: String,
    dataset_id: String,
    source_record_id: String,
    acquisition_window_id: String,
    specimen_id: String,
    campaign_id: String,
    configuration_id: String,
    thermal_regime_id: String,
    installation_generation: String,
    interval_start_s: Option<f64>,
    interval_end_s: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct Track {
    track_id: String,
    objective: String,
    split_mode: String,
    holdout_axis: Option<String>,
    required_splits: Vec<String>,
    purge_policy: Option<PurgePolicy>,
    records: Vec<SampleRecord>,
}

#[derive(Clone, Debug, Deserialize)]
struct Policy {
    required_splits: Vec<String>,
    every_sample_has_unique_sample_id: bool,
    every_sample_has_unique_source_record_id_within_track: bool,
    every_sample_has_unique_acquisition_window_id_within_track: bool,
    held_out_group_must_be_disjoint_across_splits: bool,
    group_holdout_requires_one_nonempty_holdout_axis: bool,
    temporal_forward_requires_fixed_article_and_installation: bool,
    temporal_forward_requires_purge_gap_at_least_feature_window_plus_preprocessing_memory: bool,
    no_random_window_level_split_claim: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    schema_version: String,
    contract_id: String,
    claim_ceiling: String,
    policy: Policy,
    tracks: Vec<Track>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct TrackResult {
    track_id: String,
    split_mode: String,
    holdout_axis: Option<String>,
    sample_count: usize,
    split_counts: Vec<(String, usize)>,
    leakage_checks: Vec<String>,
    status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    contract_id: String,
    source_sha256: String,
    source_bytes: usize,
    track_count: usize,
    sample_record_count: usize,
    tracks: Vec<TrackResult>,
    claim_ceiling: String,
}

fn fail<T>(message: impl Into<String>) -> AuditResult<T> {
    Err(message.into())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn has_required_splits(splits: &[String]) -> bool {
    splits.len() == REQUIRED_SPLITS.len()
        && splits
            .iter()
            .zip(REQUIRED_SPLITS)
            .all(|(actual, expected)| actual == expected)
}

fn holdout_value<'a>(record: &'a SampleRecord, axis: &str) -> Option<&'a str> {
    match axis {
        "specimen_id" => Some(&record.specimen_id),
        "thermal_regime_id" => Some(&record.thermal_regime_id),
        "installation_generation" => Some(&record.installation_generation),
        "campaign_id" => Some(&record.campaign_id),
        "configuration_id" => Some(&record.configuration_id),
        _ => None,
    }
}

fn validate_policy(policy: &Policy) -> AuditResult<()> {
    if !has_required_splits(&policy.required_splits)
        || !policy.every_sample_has_unique_sample_id
        || !policy.every_sample_has_unique_source_record_id_within_track
        || !policy.every_sample_has_unique_acquisition_window_id_within_track
        || !policy.held_out_group_must_be_disjoint_across_splits
        || !policy.group_holdout_requires_one_nonempty_holdout_axis
        || !policy.temporal_forward_requires_fixed_article_and_installation
        || !policy.temporal_forward_requires_purge_gap_at_least_feature_window_plus_preprocessing_memory
        || !policy.no_random_window_level_split_claim
    {
        return fail("split policy was weakened or changed");
    }
    Ok(())
}

fn validate_group_holdout(track: &Track) -> AuditResult<Vec<String>> {
    let axis_specification = track
        .holdout_axis
        .as_deref()
        .ok_or_else(|| "group holdout requires a declared holdout axis".to_owned())?;
    let axes: Vec<&str> = axis_specification.split('+').collect();
    let mut unique_axes = BTreeSet::new();
    if axes.is_empty()
        || axes.iter().any(|axis| {
            ![
                "specimen_id",
                "thermal_regime_id",
                "installation_generation",
                "campaign_id",
                "configuration_id",
            ]
            .contains(axis)
                || !unique_axes.insert(*axis)
        })
    {
        return fail("unsupported, empty or duplicate group holdout axis");
    }
    if track.purge_policy.is_some() {
        return fail("group-holdout track must not carry a temporal purge policy");
    }

    let mut leakage_checks = vec![
        "SAMPLE_ID_UNIQUE".to_owned(),
        "SOURCE_RECORD_ID_UNIQUE".to_owned(),
        "ACQUISITION_WINDOW_ID_UNIQUE".to_owned(),
    ];
    for axis in axes {
        let mut values_by_split = BTreeMap::<&str, BTreeSet<&str>>::new();
        for record in &track.records {
            let value = holdout_value(record, axis)
                .ok_or_else(|| "holdout axis has no record value".to_owned())?;
            if !nonempty(value) {
                return fail("holdout axis values must be non-empty");
            }
            values_by_split
                .entry(record.split.as_str())
                .or_default()
                .insert(value);
        }
        for left_index in 0..REQUIRED_SPLITS.len() {
            for right_index in (left_index + 1)..REQUIRED_SPLITS.len() {
                let left = values_by_split
                    .get(REQUIRED_SPLITS[left_index])
                    .ok_or_else(|| "required split missing from group-holdout track".to_owned())?;
                let right = values_by_split
                    .get(REQUIRED_SPLITS[right_index])
                    .ok_or_else(|| "required split missing from group-holdout track".to_owned())?;
                if !left.is_disjoint(right) {
                    return fail(format!(
                        "holdout leakage on axis {axis} between {} and {}",
                        REQUIRED_SPLITS[left_index],
                        REQUIRED_SPLITS[right_index]
                    ));
                }
            }
        }
        leakage_checks.push(format!("GROUP_DISJOINT:{axis}"));
    }
    Ok(leakage_checks)
}

fn validate_temporal_forward(track: &Track) -> AuditResult<Vec<String>> {
    if track.holdout_axis.is_some() {
        return fail("purged forward track must not declare a group holdout axis");
    }
    let purge = track
        .purge_policy
        .as_ref()
        .ok_or_else(|| "temporal forward track requires a purge policy".to_owned())?;
    let values = [
        purge.feature_window_seconds,
        purge.preprocessing_memory_seconds,
        purge.minimum_purge_gap_seconds,
    ];
    if values.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return fail("temporal purge policy values must be finite and non-negative");
    }
    let required = purge.feature_window_seconds + purge.preprocessing_memory_seconds;
    if purge.minimum_purge_gap_seconds + 1.0e-9 < required {
        return fail("minimum purge gap is shorter than feature window plus preprocessing memory");
    }

    let first = &track.records[0];
    for record in &track.records {
        if record.specimen_id != first.specimen_id
            || record.campaign_id != first.campaign_id
            || record.configuration_id != first.configuration_id
            || record.installation_generation != first.installation_generation
        {
            return fail("temporal-forward track must hold article, campaign, configuration and installation fixed");
        }
        if record.interval_start_s.is_none() || record.interval_end_s.is_none() {
            return fail("temporal-forward records require both interval endpoints");
        }
        let start = record.interval_start_s.unwrap();
        let end = record.interval_end_s.unwrap();
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
            return fail("temporal interval must be finite, non-negative and increasing");
        }
    }

    let interval = |split: &str| -> AuditResult<(f64, f64)> {
        let selected: Vec<&SampleRecord> = track
            .records
            .iter()
            .filter(|record| record.split == split)
            .collect();
        if selected.is_empty() {
            return fail(format!("temporal-forward split {split} is empty"));
        }
        let start = selected
            .iter()
            .map(|record| record.interval_start_s.unwrap())
            .fold(f64::INFINITY, f64::min);
        let end = selected
            .iter()
            .map(|record| record.interval_end_s.unwrap())
            .fold(f64::NEG_INFINITY, f64::max);
        Ok((start, end))
    };

    let train = interval("train")?;
    let validation = interval("validation")?;
    let test = interval("test")?;
    let train_to_validation = validation.0 - train.1;
    let validation_to_test = test.0 - validation.1;
    if train_to_validation + 1.0e-9 < purge.minimum_purge_gap_seconds
        || validation_to_test + 1.0e-9 < purge.minimum_purge_gap_seconds
    {
        return fail("forward partitions overlap the declared preprocessing/feature purge horizon");
    }
    Ok(vec![
        "SAMPLE_ID_UNIQUE".to_owned(),
        "SOURCE_RECORD_ID_UNIQUE".to_owned(),
        "ACQUISITION_WINDOW_ID_UNIQUE".to_owned(),
        "FIXED_ARTICLE_CONFIGURATION_INSTALLATION".to_owned(),
        "FORWARD_TIME_ORDER".to_owned(),
        "PURGE_GAP_GE_FEATURE_PLUS_PREPROCESSING_MEMORY".to_owned(),
    ])
}

fn validate_track(track: &Track) -> AuditResult<TrackResult> {
    if !nonempty(&track.track_id) || !nonempty(&track.objective) {
        return fail("track identity and objective must be non-empty");
    }
    if !has_required_splits(&track.required_splits) {
        return fail(format!("{}: required split set changed", track.track_id));
    }
    if track.records.len() < 3 {
        return fail(format!("{}: at least one record per split is required", track.track_id));
    }
    let mut sample_ids = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    let mut window_ids = BTreeSet::new();
    let mut counts = BTreeMap::<String, usize>::new();
    for record in &track.records {
        if !nonempty(&record.sample_id)
            || !nonempty(&record.dataset_id)
            || !nonempty(&record.source_record_id)
            || !nonempty(&record.acquisition_window_id)
            || !nonempty(&record.specimen_id)
            || !nonempty(&record.campaign_id)
            || !nonempty(&record.configuration_id)
            || !nonempty(&record.thermal_regime_id)
            || !nonempty(&record.installation_generation)
        {
            return fail(format!("{}: record contains an empty identity field", track.track_id));
        }
        if !REQUIRED_SPLITS.contains(&record.split.as_str()) {
            return fail(format!("{}: unsupported split label", track.track_id));
        }
        if !sample_ids.insert(record.sample_id.as_str())
            || !source_ids.insert(record.source_record_id.as_str())
            || !window_ids.insert(record.acquisition_window_id.as_str())
        {
            return fail(format!("{}: duplicate sample, source record or acquisition window", track.track_id));
        }
        *counts.entry(record.split.clone()).or_default() += 1;
    }
    if REQUIRED_SPLITS
        .iter()
        .any(|split| !counts.contains_key(*split))
    {
        return fail(format!("{}: every required split must be populated", track.track_id));
    }

    let leakage_checks = match track.split_mode.as_str() {
        "group_holdout" => validate_group_holdout(track)?,
        "purged_forward_chaining" => validate_temporal_forward(track)?,
        _ => return fail(format!("{}: unsupported split mode", track.track_id)),
    };
    Ok(TrackResult {
        track_id: track.track_id.clone(),
        split_mode: track.split_mode.clone(),
        holdout_axis: track.holdout_axis.clone(),
        sample_count: track.records.len(),
        split_counts: REQUIRED_SPLITS
            .iter()
            .filter_map(|split| {
                counts
                    .get(*split)
                    .map(|count| ((*split).to_owned(), *count))
            })
            .collect(),
        leakage_checks,
        status: "PASS_SYNTHETIC_PARTITION_INTEGRITY".to_owned(),
    })
}

fn validate_fixture(fixture: &Fixture) -> AuditResult<()> {
    if fixture.schema_version != "aero-shm-benchmark-split-contract/0.1.0"
        || !nonempty(&fixture.contract_id)
        || fixture.claim_ceiling != CLAIM_CEILING
    {
        return fail("unsupported split-contract schema, identity or claim ceiling");
    }
    validate_policy(&fixture.policy)?;
    if fixture.tracks.len() < 3 || fixture.tracks.len() > 32 {
        return fail("split contract must contain 3..=32 tracks");
    }
    let mut ids = BTreeSet::new();
    for track in &fixture.tracks {
        if !ids.insert(track.track_id.as_str()) {
            return fail("duplicate track_id");
        }
        validate_track(track)?;
    }
    Ok(())
}

fn evaluate(fixture: &Fixture, source: &[u8]) -> AuditResult<Report> {
    validate_fixture(fixture)?;
    let tracks = fixture
        .tracks
        .iter()
        .map(validate_track)
        .collect::<AuditResult<Vec<_>>>()?;
    Ok(Report {
        schema_version: "aero-shm-benchmark-split-contract-result/0.1.0".to_owned(),
        contract_id: fixture.contract_id.clone(),
        source_sha256: sha256_hex(source),
        source_bytes: source.len(),
        track_count: tracks.len(),
        sample_record_count: tracks.iter().map(|track| track.sample_count).sum(),
        tracks,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Fixture = serde_json::from_slice(FIXTURE)?;
    let report = evaluate(&fixture, FIXTURE).map_err(std::io::Error::other)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_slice(FIXTURE).expect("split-contract fixture parses")
    }

    #[test]
    fn all_four_split_tracks_pass_frozen_contract() {
        let report = evaluate(&fixture(), FIXTURE).unwrap();
        assert_eq!(report.track_count, 5);
        assert_eq!(report.sample_record_count, 17);
        assert!(report.tracks.iter().all(|track| {
            track.status == "PASS_SYNTHETIC_PARTITION_INTEGRITY"
                && track.split_counts.len() == 3
        }));
    }

    #[test]
    fn group_holdout_axis_overlap_is_rejected() {
        let mut data = fixture();
        let track = data.tracks.iter_mut()
            .find(|item| item.track_id == "SPECIMEN_HOLDOUT")
            .unwrap();
        let train_specimen = track.records[0].specimen_id.clone();
        track.records.iter_mut()
            .find(|record| record.split == "test")
            .unwrap()
            .specimen_id = train_specimen;
        assert!(validate_fixture(&data).unwrap_err().contains("holdout leakage"));
    }

    #[test]
    fn compound_holdout_rejects_leakage_on_any_declared_axis() {
        let mut data = fixture();
        let track = data
            .tracks
            .iter_mut()
            .find(|item| item.track_id == "COMPOUND_ARTICLE_CAMPAIGN_HOLDOUT")
            .unwrap();
        let train_configuration = track.records[0].configuration_id.clone();
        track
            .records
            .iter_mut()
            .find(|record| record.split == "test")
            .unwrap()
            .configuration_id = train_configuration;
        assert!(validate_fixture(&data)
            .unwrap_err()
            .contains("holdout leakage on axis configuration_id"));
    }

    #[test]
    fn duplicate_acquisition_window_is_rejected() {
        let mut data = fixture();
        let track = data.tracks.iter_mut()
            .find(|item| item.track_id == "SPECIMEN_HOLDOUT")
            .unwrap();
        track.records[4].acquisition_window_id = track.records[0].acquisition_window_id.clone();
        assert!(validate_fixture(&data).unwrap_err().contains("duplicate sample"));
    }

    #[test]
    fn insufficient_temporal_purge_gap_is_rejected() {
        let mut data = fixture();
        let track = data.tracks.iter_mut()
            .find(|item| item.track_id == "TEMPORAL_FORWARD")
            .unwrap();
        track.records.iter_mut()
            .find(|record| record.split == "validation")
            .unwrap()
            .interval_start_s = Some(4000.0);
        assert!(validate_fixture(&data).unwrap_err().contains("purge horizon"));
    }

    #[test]
    fn temporal_installation_change_is_not_mislabelled_as_forward_only() {
        let mut data = fixture();
        let track = data.tracks.iter_mut()
            .find(|item| item.track_id == "TEMPORAL_FORWARD")
            .unwrap();
        track.records.iter_mut()
            .find(|record| record.split == "test")
            .unwrap()
            .installation_generation = "INSTALL-REPLACED".to_owned();
        assert!(validate_fixture(&data).unwrap_err().contains("hold article"));
    }
}
