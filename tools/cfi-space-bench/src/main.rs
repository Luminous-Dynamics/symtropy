#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::{env, fs, process};

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EPISODES: usize = 100_000;
const MAX_FRAMES_PER_EPISODE: usize = 1_000_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateManifest {
    schema_version: u32,
    benchmark_id: String,
    episodes: Vec<CandidateEpisode>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateEpisode {
    episode_id: String,
    input_bundle_sha256: String,
    frame_budget: usize,
    expected_frame_count: usize,
    first_sequence: u64,
    horizon_us: u64,
    frames: Vec<CandidateFrame>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateFrame {
    frame_id: String,
    sequence: u64,
    timestamp_us: u64,
    modalities: Vec<Modality>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Modality {
    Rgb,
    Event,
    Depth,
    Infrared,
    Lidar,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluatorLedger {
    schema_version: u32,
    benchmark_id: String,
    episodes: Vec<LedgerEpisode>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerEpisode {
    episode_id: String,
    parent_episode_id: String,
    counterfactual_group_id: String,
    variant_kind: VariantKind,
    split: Split,
    independent_unit: IndependentUnit,
    fault_ids: Vec<String>,
    fault_families: Vec<FaultFamily>,
    critical: bool,
    expected_disposition: Disposition,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct IndependentUnit {
    target_id: String,
    session_id: String,
    trajectory_id: String,
    sensor_rig_id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Split {
    Development,
    Calibration,
    Confirmatory,
    BlindHoldout,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum VariantKind {
    Clean,
    SingleFault,
    Interaction,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum FaultFamily {
    RgbLocal,
    EventLocal,
    Timing,
    Extrinsics,
    CommonCause,
    Translation,
    Dynamics,
    Observability,
    Integrity,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    Qualified,
    Abstain,
    OutOfScope,
    Disagreement,
    DependenceUnknown,
    MissingEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PredictionFile {
    schema_version: u32,
    benchmark_id: String,
    predictions: Vec<Prediction>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Prediction {
    episode_id: String,
    input_bundle_sha256: String,
    disposition: Disposition,
    detected_fault_families: Vec<FaultFamily>,
}

#[derive(Debug, Serialize)]
struct Score {
    benchmark_id: String,
    episodes: usize,
    independent_units: usize,
    counterfactual_groups: usize,
    clean_episodes: usize,
    fault_episodes: usize,
    clean_qualified: usize,
    false_abstentions: usize,
    correctly_contained_faults: usize,
    unsafe_accepts: usize,
    exact_disposition_matches: usize,
    false_positive_family_labels: usize,
    family_detection: Vec<FamilyMetric>,
}

#[derive(Debug, Serialize)]
struct FamilyMetric {
    family: FaultFamily,
    episodes: usize,
    detected: usize,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_nonempty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_candidate_manifest(manifest: &CandidateManifest) -> Result<(), String> {
    if manifest.schema_version != 1 {
        return Err(format!(
            "unsupported candidate manifest schema_version {}",
            manifest.schema_version
        ));
    }
    require_nonempty(&manifest.benchmark_id, "benchmark_id")?;
    if manifest.episodes.is_empty() || manifest.episodes.len() > MAX_EPISODES {
        return Err(format!(
            "episode count must be in 1..={MAX_EPISODES}, got {}",
            manifest.episodes.len()
        ));
    }

    let mut episode_ids = BTreeSet::new();
    for episode in &manifest.episodes {
        require_nonempty(&episode.episode_id, "episode_id")?;
        if !episode_ids.insert(&episode.episode_id) {
            return Err(format!("duplicate episode_id {}", episode.episode_id));
        }
        if !is_sha256_hex(&episode.input_bundle_sha256) {
            return Err(format!(
                "episode {} has malformed input_bundle_sha256",
                episode.episode_id
            ));
        }
        if episode.frame_budget == 0
            || episode.expected_frame_count == 0
            || episode.expected_frame_count > episode.frame_budget
            || episode.expected_frame_count != episode.frames.len()
            || episode.frames.len() > MAX_FRAMES_PER_EPISODE
        {
            return Err(format!(
                "episode {} has inconsistent frame budget/count",
                episode.episode_id
            ));
        }
        if episode.horizon_us == 0 {
            return Err(format!("episode {} has zero horizon", episode.episode_id));
        }

        let mut frame_ids = BTreeSet::new();
        let mut first_timestamp = None;
        let mut previous_timestamp = None;
        for (index, frame) in episode.frames.iter().enumerate() {
            require_nonempty(&frame.frame_id, "frame_id")?;
            if !frame_ids.insert(&frame.frame_id) {
                return Err(format!(
                    "episode {} contains duplicate frame_id {}",
                    episode.episode_id, frame.frame_id
                ));
            }
            let offset = u64::try_from(index)
                .map_err(|_| "frame index exceeds u64".to_string())?;
            let expected_sequence = episode
                .first_sequence
                .checked_add(offset)
                .ok_or_else(|| "frame sequence overflow".to_string())?;
            if frame.sequence != expected_sequence {
                return Err(format!(
                    "episode {} frame {} sequence gap: expected {}, got {}",
                    episode.episode_id, frame.frame_id, expected_sequence, frame.sequence
                ));
            }
            if frame.modalities.is_empty() {
                return Err(format!(
                    "episode {} frame {} has no modality",
                    episode.episode_id, frame.frame_id
                ));
            }
            let unique_modalities: BTreeSet<_> = frame.modalities.iter().copied().collect();
            if unique_modalities.len() != frame.modalities.len() {
                return Err(format!(
                    "episode {} frame {} repeats a modality",
                    episode.episode_id, frame.frame_id
                ));
            }

            if let Some(previous) = previous_timestamp {
                if frame.timestamp_us <= previous {
                    return Err(format!(
                        "episode {} timestamps are not strictly increasing",
                        episode.episode_id
                    ));
                }
            } else {
                first_timestamp = Some(frame.timestamp_us);
            }
            previous_timestamp = Some(frame.timestamp_us);
        }

        let start = first_timestamp.ok_or_else(|| {
            format!("episode {} has no first timestamp", episode.episode_id)
        })?;
        let end = previous_timestamp.ok_or_else(|| {
            format!("episode {} has no last timestamp", episode.episode_id)
        })?;
        if end - start > episode.horizon_us {
            return Err(format!(
                "episode {} exceeds its declared fixed horizon",
                episode.episode_id
            ));
        }
    }
    Ok(())
}

fn validate_ledger(ledger: &EvaluatorLedger) -> Result<(), String> {
    if ledger.schema_version != 1 {
        return Err(format!(
            "unsupported evaluator ledger schema_version {}",
            ledger.schema_version
        ));
    }
    require_nonempty(&ledger.benchmark_id, "ledger benchmark_id")?;
    if ledger.episodes.is_empty() || ledger.episodes.len() > MAX_EPISODES {
        return Err(format!(
            "ledger episode count must be in 1..={MAX_EPISODES}, got {}",
            ledger.episodes.len()
        ));
    }

    let mut episode_ids = BTreeSet::new();
    let mut groups: BTreeMap<String, (String, Split, IndependentUnit)> = BTreeMap::new();
    let mut target_splits = BTreeMap::new();
    let mut session_splits = BTreeMap::new();
    let mut trajectory_splits = BTreeMap::new();
    let mut rig_splits = BTreeMap::new();

    for row in &ledger.episodes {
        for (field, value) in [
            ("episode_id", row.episode_id.as_str()),
            ("parent_episode_id", row.parent_episode_id.as_str()),
            ("counterfactual_group_id", row.counterfactual_group_id.as_str()),
            ("target_id", row.independent_unit.target_id.as_str()),
            ("session_id", row.independent_unit.session_id.as_str()),
            ("trajectory_id", row.independent_unit.trajectory_id.as_str()),
            ("sensor_rig_id", row.independent_unit.sensor_rig_id.as_str()),
        ] {
            require_nonempty(value, field)?;
        }
        if !episode_ids.insert(&row.episode_id) {
            return Err(format!("duplicate evaluator episode_id {}", row.episode_id));
        }

        let fault_ids: BTreeSet<_> = row.fault_ids.iter().collect();
        let fault_families: BTreeSet<_> = row.fault_families.iter().collect();
        if fault_ids.len() != row.fault_ids.len()
            || fault_families.len() != row.fault_families.len()
            || row.fault_ids.iter().any(|id| id.trim().is_empty())
        {
            return Err(format!(
                "episode {} has duplicate or empty fault identifiers",
                row.episode_id
            ));
        }

        match row.variant_kind {
            VariantKind::Clean => {
                if !row.fault_ids.is_empty()
                    || !row.fault_families.is_empty()
                    || row.expected_disposition != Disposition::Qualified
                {
                    return Err(format!(
                        "clean episode {} must have no faults and expect qualified",
                        row.episode_id
                    ));
                }
            }
            VariantKind::SingleFault => {
                if row.fault_ids.len() != 1
                    || row.fault_families.len() != 1
                    || row.expected_disposition == Disposition::Qualified
                {
                    return Err(format!(
                        "single-fault episode {} must declare exactly one fault and be non-qualified",
                        row.episode_id
                    ));
                }
            }
            VariantKind::Interaction => {
                if row.fault_ids.len() < 2
                    || row.fault_families.is_empty()
                    || row.expected_disposition == Disposition::Qualified
                {
                    return Err(format!(
                        "interaction episode {} must declare at least two faults and be non-qualified",
                        row.episode_id
                    ));
                }
            }
        }

        if let Some((parent, split, unit)) = groups.get(&row.counterfactual_group_id) {
            if parent != &row.parent_episode_id
                || split != &row.split
                || unit != &row.independent_unit
            {
                return Err(format!(
                    "counterfactual siblings in group {} cross parent, split or independent-unit boundaries",
                    row.counterfactual_group_id
                ));
            }
        } else {
            groups.insert(
                row.counterfactual_group_id.clone(),
                (
                    row.parent_episode_id.clone(),
                    row.split,
                    row.independent_unit.clone(),
                ),
            );
        }

        for (name, value, map) in [
            (
                "target",
                row.independent_unit.target_id.as_str(),
                &mut target_splits,
            ),
            (
                "session",
                row.independent_unit.session_id.as_str(),
                &mut session_splits,
            ),
            (
                "trajectory",
                row.independent_unit.trajectory_id.as_str(),
                &mut trajectory_splits,
            ),
            (
                "sensor rig",
                row.independent_unit.sensor_rig_id.as_str(),
                &mut rig_splits,
            ),
        ] {
            if let Some(existing) = map.insert(value.to_string(), row.split) {
                if existing != row.split {
                    return Err(format!(
                        "{name} {value} leaks across dataset splits"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_and_score(
    manifest: &CandidateManifest,
    ledger: &EvaluatorLedger,
    predictions: &PredictionFile,
) -> Result<Score, String> {
    validate_candidate_manifest(manifest)?;
    validate_ledger(ledger)?;

    if predictions.schema_version != 1 {
        return Err(format!(
            "unsupported predictions schema_version {}",
            predictions.schema_version
        ));
    }
    if manifest.benchmark_id != ledger.benchmark_id
        || manifest.benchmark_id != predictions.benchmark_id
    {
        return Err("benchmark_id mismatch across manifest, ledger and predictions".to_string());
    }
    if predictions.predictions.len() != manifest.episodes.len() {
        return Err("prediction count does not match candidate episode count".to_string());
    }

    let manifest_ids: BTreeSet<_> = manifest
        .episodes
        .iter()
        .map(|episode| episode.episode_id.as_str())
        .collect();
    let ledger_ids: BTreeSet<_> = ledger
        .episodes
        .iter()
        .map(|episode| episode.episode_id.as_str())
        .collect();
    let prediction_ids: BTreeSet<_> = predictions
        .predictions
        .iter()
        .map(|prediction| prediction.episode_id.as_str())
        .collect();
    if manifest_ids != ledger_ids {
        return Err("candidate manifest and evaluator ledger episode identities differ".to_string());
    }
    if manifest_ids != prediction_ids {
        return Err("candidate manifest and prediction episode identities differ".to_string());
    }
    if prediction_ids.len() != predictions.predictions.len() {
        return Err("duplicate prediction episode_id".to_string());
    }

    let candidate_by_id: BTreeMap<_, _> = manifest
        .episodes
        .iter()
        .map(|episode| (episode.episode_id.as_str(), episode))
        .collect();
    let ledger_by_id: BTreeMap<_, _> = ledger
        .episodes
        .iter()
        .map(|episode| (episode.episode_id.as_str(), episode))
        .collect();
    let prediction_by_id: BTreeMap<_, _> = predictions
        .predictions
        .iter()
        .map(|prediction| (prediction.episode_id.as_str(), prediction))
        .collect();

    let mut unsafe_accepts = 0;
    let mut false_abstentions = 0;
    let mut correctly_contained_faults = 0;
    let mut clean_qualified = 0;
    let mut fault_episodes = 0;
    let mut clean_episodes = 0;
    let mut exact_disposition_matches = 0;
    let mut false_positive_family_labels = 0;
    let mut family_counts: BTreeMap<FaultFamily, (usize, usize)> = BTreeMap::new();

    for episode_id in &manifest_ids {
        let episode = candidate_by_id[episode_id];
        let truth = ledger_by_id[episode_id];
        let prediction = prediction_by_id[episode_id];

        if prediction.input_bundle_sha256 != episode.input_bundle_sha256
            || !is_sha256_hex(&prediction.input_bundle_sha256)
        {
            return Err(format!(
                "episode {} prediction is not bound to the candidate input digest",
                episode_id
            ));
        }

        let predicted_families: BTreeSet<_> =
            prediction.detected_fault_families.iter().copied().collect();
        if predicted_families.len() != prediction.detected_fault_families.len() {
            return Err(format!(
                "episode {} repeats a detected fault family",
                episode_id
            ));
        }
        let expected_families: BTreeSet<_> = truth.fault_families.iter().copied().collect();

        for family in &predicted_families {
            if !expected_families.contains(family) {
                false_positive_family_labels += 1;
            }
        }
        for family in &expected_families {
            let counts = family_counts.entry(*family).or_insert((0, 0));
            counts.0 += 1;
            if predicted_families.contains(family) {
                counts.1 += 1;
            }
        }

        let expected_qualified = truth.expected_disposition == Disposition::Qualified;
        let predicted_qualified = prediction.disposition == Disposition::Qualified;
        if truth.expected_disposition == prediction.disposition {
            exact_disposition_matches += 1;
        }
        if expected_qualified {
            clean_episodes += 1;
            if predicted_qualified {
                clean_qualified += 1;
            } else {
                false_abstentions += 1;
            }
        } else {
            fault_episodes += 1;
            if predicted_qualified {
                unsafe_accepts += 1;
            } else {
                correctly_contained_faults += 1;
            }
        }

        // Read the episode identity through the same validated map: this also
        // prevents accidental dead-code removal of the per-episode join.
        if episode.episode_id != *episode_id {
            return Err("internal episode identity join failure".to_string());
        }
    }

    let family_detection = family_counts
        .into_iter()
        .map(|(family, (episodes, detected))| FamilyMetric {
            family,
            episodes,
            detected,
        })
        .collect();

    let independent_units: BTreeSet<_> = ledger
        .episodes
        .iter()
        .map(|row| {
            (
                row.independent_unit.target_id.as_str(),
                row.independent_unit.session_id.as_str(),
                row.independent_unit.trajectory_id.as_str(),
                row.independent_unit.sensor_rig_id.as_str(),
            )
        })
        .collect();

    let groups: BTreeSet<_> = ledger
        .episodes
        .iter()
        .map(|row| row.counterfactual_group_id.as_str())
        .collect();

    Ok(Score {
        benchmark_id: manifest.benchmark_id.clone(),
        episodes: manifest.episodes.len(),
        independent_units: independent_units.len(),
        counterfactual_groups: groups.len(),
        clean_episodes,
        fault_episodes,
        clean_qualified,
        false_abstentions,
        correctly_contained_faults,
        unsafe_accepts,
        exact_disposition_matches,
        false_positive_family_labels,
        family_detection,
    })
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot stat {}: {error}", path.display()))?;
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "input {} exceeds {} bytes",
            path.display(),
            MAX_INPUT_BYTES
        ));
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON in {}: {error}", path.display()))
}

fn run() -> Result<(), String> {
    let mut manifest_path = None;
    let mut ledger_path = None;
    let mut predictions_path = None;
    let mut expected_unsafe = None;

    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for argument {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = Some(value),
            "--ledger" => ledger_path = Some(value),
            "--predictions" => predictions_path = Some(value),
            "--expect-unsafe" => {
                expected_unsafe = Some(
                    value
                        .parse::<usize>()
                        .map_err(|error| format!("invalid --expect-unsafe value: {error}"))?,
                )
            }
            "--help" | "-h" => {
                println!(
                    "cfi-space-bench --manifest FILE --ledger FILE --predictions FILE --expect-unsafe N"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }

    let manifest_path = manifest_path.ok_or_else(|| "missing --manifest".to_string())?;
    let ledger_path = ledger_path.ok_or_else(|| "missing --ledger".to_string())?;
    let predictions_path =
        predictions_path.ok_or_else(|| "missing --predictions".to_string())?;
    let expected_unsafe =
        expected_unsafe.ok_or_else(|| "missing --expect-unsafe".to_string())?;

    let manifest: CandidateManifest = read_json(Path::new(&manifest_path))?;
    let ledger: EvaluatorLedger = read_json(Path::new(&ledger_path))?;
    let predictions: PredictionFile = read_json(Path::new(&predictions_path))?;
    let score = validate_and_score(&manifest, &ledger, &predictions)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&score)
            .map_err(|error| format!("cannot serialize score: {error}"))?
    );
    if score.unsafe_accepts != expected_unsafe {
        return Err(format!(
            "unsafe_accepts mismatch: expected {}, observed {}",
            expected_unsafe, score.unsafe_accepts
        ));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cfi-space-bench: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str =
        include_str!("../../../research/spacecraft/cfi-space/fixtures/synthetic-candidate-manifest-v0.1.json");
    const LEDGER: &str =
        include_str!("../../../research/spacecraft/cfi-space/fixtures/synthetic-evaluator-ledger-v0.1.json");
    const SAFE_PREDICTIONS: &str =
        include_str!("../../../research/spacecraft/cfi-space/fixtures/synthetic-predictions-safe-v0.1.json");
    const UNSAFE_PREDICTIONS: &str =
        include_str!("../../../research/spacecraft/cfi-space/fixtures/synthetic-predictions-unsafe-v0.1.json");

    fn parse<T: serde::de::DeserializeOwned>(source: &str) -> T {
        serde_json::from_str(source).unwrap()
    }

    #[test]
    fn known_answer_safe_fixture_has_no_unsafe_accepts() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);
        let score = validate_and_score(&manifest, &ledger, &predictions).unwrap();

        assert_eq!(score.episodes, 10);
        assert_eq!(score.independent_units, 2);
        assert_eq!(score.counterfactual_groups, 2);
        assert_eq!(score.clean_episodes, 2);
        assert_eq!(score.fault_episodes, 8);
        assert_eq!(score.unsafe_accepts, 0);
        assert_eq!(score.false_abstentions, 0);
        assert_eq!(score.correctly_contained_faults, 8);
        assert_eq!(score.exact_disposition_matches, 10);
    }

    #[test]
    fn unsafe_common_cause_acceptance_is_counted() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(UNSAFE_PREDICTIONS);
        let score = validate_and_score(&manifest, &ledger, &predictions).unwrap();

        assert_eq!(score.unsafe_accepts, 1);
        assert_eq!(score.false_abstentions, 0);
        assert_eq!(score.exact_disposition_matches, 9);
    }

    #[test]
    fn frame_sequence_gap_is_rejected() {
        let mut manifest: CandidateManifest = parse(MANIFEST);
        manifest.episodes[0].frames[1].sequence = 2;
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(validate_and_score(&manifest, &ledger, &predictions).is_err());
    }

    #[test]
    fn counterfactual_split_leakage_is_rejected() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let mut ledger: EvaluatorLedger = parse(LEDGER);
        ledger.episodes[1].split = Split::Calibration;
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(validate_and_score(&manifest, &ledger, &predictions).is_err());
    }

    #[test]
    fn prediction_digest_substitution_is_rejected() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let mut predictions: PredictionFile = parse(SAFE_PREDICTIONS);
        predictions.predictions[0].input_bundle_sha256 = "f".repeat(64);

        assert!(validate_and_score(&manifest, &ledger, &predictions).is_err());
    }

    #[test]
    fn duplicate_prediction_identity_is_rejected() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let mut predictions: PredictionFile = parse(SAFE_PREDICTIONS);
        predictions.predictions[1].episode_id = predictions.predictions[0].episode_id.clone();

        assert!(validate_and_score(&manifest, &ledger, &predictions).is_err());
    }
}
