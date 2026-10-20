use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct TavisFile {
    pub meta: Meta,
    #[serde(default)]
    pub episodes: Vec<Episode>,
    #[serde(default)]
    pub summary: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub checkpoint: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub camera: Option<String>,
    #[serde(default)]
    pub robot: Option<String>,
    #[serde(default)]
    pub suite: Option<String>,
    pub task: String,
    pub eval_mode: String,
    #[serde(default)]
    pub episodes_requested: Option<usize>,
    #[serde(default)]
    pub episode_length_s: Option<f64>,
    #[serde(default)]
    pub robot_ctrl_rate: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Episode {
    pub success: bool,
    #[serde(default)]
    pub length_steps: Option<u64>,
    #[serde(default)]
    pub length_s: Option<f64>,
    #[serde(default)]
    pub wall_clock_s: Option<f64>,
    #[serde(default)]
    pub episode_id: Option<Value>,
    #[serde(default, alias = "episode_seed")]
    pub seed: Option<Value>,
    #[serde(default)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Extra {
    #[serde(default)]
    pub galt_s: Option<f64>,
    #[serde(default)]
    pub galt_reason: Option<String>,
    #[serde(default)]
    pub galt_arm: Option<String>,
    #[serde(default)]
    pub galt_t_head_arrival_s: Option<f64>,
    #[serde(default)]
    pub galt_t_hand_arrival_s: Option<f64>,
    #[serde(default)]
    pub galt_arm_reach_s: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditManifest {
    pub schema_version: u32,
    pub upstream_commit: String,
    pub checkpoint_sha256: String,
    pub assets_manifest_sha256: String,
    pub environment_lock_sha256: String,
    pub invocation: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Proportion {
    pub successes: usize,
    pub total: usize,
    pub rate: Option<f64>,
    pub low_95: Option<f64>,
    pub high_95: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GaltSummary {
    pub valid_n: usize,
    pub mean_s: Option<f64>,
    pub median_s: Option<f64>,
    pub reason_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct CellResult {
    pub run_label: String,
    pub file_name: String,
    pub meta: Meta,
    pub episodes: Vec<Episode>,
    pub errors: Vec<String>,
    pub proportion: Proportion,
    pub galt: GaltSummary,
    pub episode_ids_present: bool,
    pub seeds_present: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AuditStatus {
    pub result_files: usize,
    pub valid_cells: usize,
    pub missing_cells: Vec<String>,
    pub unexpected_cells: Vec<String>,
    pub errors: Vec<String>,
    pub episode_count_mismatches: Vec<String>,
    pub metadata_inconsistencies: Vec<String>,
    pub manifest_present: bool,
    pub manifest_valid: bool,
    pub episode_ids_present: bool,
    pub seeds_present: bool,
    pub pairing_ready: bool,
}

#[derive(Debug, Clone)]
pub struct RunData {
    pub label: String,
    pub dir: PathBuf,
    pub cells: Vec<CellResult>,
    pub status: AuditStatus,
    pub official_pooled_by_mode: BTreeMap<String, Proportion>,
    pub task_macro_by_mode: BTreeMap<String, Option<f64>>,
}

pub fn wilson_proportion(successes: usize, total: usize) -> Proportion {
    if total == 0 {
        return Proportion {
            successes,
            total,
            rate: None,
            low_95: None,
            high_95: None,
        };
    }
    let p = successes as f64 / total as f64;
    let z = 1.959_963_984_540_054_f64;
    let z2 = z * z;
    let denominator = 1.0 + z2 / total as f64;
    let centre = (p + z2 / (2.0 * total as f64)) / denominator;
    let margin = z / denominator
        * (p * (1.0 - p) / total as f64 + z2 / (4.0 * (total as f64).powi(2))).sqrt();
    Proportion {
        successes,
        total,
        rate: Some(p),
        low_95: Some((centre - margin).max(0.0)),
        high_95: Some((centre + margin).min(1.0)),
    }
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

fn summarize_galt(episodes: &[Episode]) -> GaltSummary {
    let successful: Vec<&Episode> = episodes.iter().filter(|episode| episode.success).collect();
    let mut galt_values = Vec::new();
    let mut reason_counts = BTreeMap::new();

    // Match the native TAVIS summary convention: GALT is summarized over
    // successful episodes; reason codes are counted where supplied.
    for episode in successful {
        if let Some(reason) = &episode.extra.galt_reason {
            *reason_counts.entry(reason.clone()).or_insert(0) += 1;
        }
        if let Some(value) = episode.extra.galt_s {
            if value.is_finite() {
                galt_values.push(value);
            }
        }
    }
    let mean_s = if galt_values.is_empty() {
        None
    } else {
        Some(galt_values.iter().sum::<f64>() / galt_values.len() as f64)
    };
    let median_s = median(&mut galt_values);
    GaltSummary {
        valid_n: galt_values.len(),
        mean_s,
        median_s,
        reason_counts,
    }
}

fn value_identity(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn tsv_clean(value: &str) -> String {
    value
        .replace('\t', " ")
        .replace('\r', " ")
        .replace('\n', " ")
}

pub fn validate_manifest(manifest: &AuditManifest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if manifest.schema_version != 1 {
        errors.push(format!(
            "unsupported manifest schema_version {}",
            manifest.schema_version
        ));
    }
    let commit = manifest.upstream_commit.trim();
    if !(commit.len() == 40 || commit.len() == 64) || !commit.chars().all(|c| c.is_ascii_hexdigit())
    {
        errors.push(
            "manifest upstream_commit must be a 40- or 64-character hex revision".to_string(),
        );
    }
    for (name, digest) in [
        ("checkpoint_sha256", &manifest.checkpoint_sha256),
        ("assets_manifest_sha256", &manifest.assets_manifest_sha256),
        ("environment_lock_sha256", &manifest.environment_lock_sha256),
    ] {
        let lower = digest.to_ascii_lowercase();
        if digest.len() != 64 || !lower.chars().all(|c| c.is_ascii_hexdigit()) {
            errors.push(format!(
                "manifest {name} must be a 64-character SHA-256 hex digest"
            ));
        }
    }
    if manifest.invocation.trim().is_empty() {
        errors.push("manifest invocation is empty".to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn expected_key(task: &str, mode: &str) -> String {
    format!("{task}/{mode}")
}

fn meta_signature(
    meta: &Meta,
) -> (
    Option<&str>,
    Option<&str>,
    Option<&str>,
    Option<&str>,
    Option<u64>,
    Option<u64>,
) {
    (
        meta.model.as_deref(),
        meta.camera.as_deref(),
        meta.robot.as_deref(),
        meta.checkpoint.as_deref(),
        meta.episode_length_s.map(f64::to_bits),
        meta.robot_ctrl_rate.map(f64::to_bits),
    )
}

fn parse_cell(
    path: &Path,
    run_label: &str,
    expected_episodes: usize,
) -> Result<CellResult, Vec<String>> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<non-utf8-filename>")
        .to_string();
    let raw = fs::read_to_string(path)
        .map_err(|error| vec![format!("{}: read error: {error}", path.display())])?;
    let data: TavisFile = serde_json::from_str(&raw).map_err(|error| {
        vec![format!(
            "{}: invalid TAVIS result JSON: {error}",
            path.display()
        )]
    })?;

    let mut errors = Vec::new();
    if let Some(error) = &data.error {
        if !error.trim().is_empty() {
            errors.push(format!(
                "{}: official evaluator wrote an error field",
                path.display()
            ));
        }
    }
    if data.summary.is_none() {
        errors.push(format!("{}: missing native summary object", path.display()));
    }
    if data.episodes.is_empty() {
        errors.push(format!("{}: no episode rows", path.display()));
    }
    if data.episodes.len() != expected_episodes {
        errors.push(format!(
            "{}: expected {expected_episodes} episodes, found {}",
            path.display(),
            data.episodes.len()
        ));
    }
    if data
        .meta
        .episodes_requested
        .is_some_and(|requested| requested != expected_episodes)
    {
        errors.push(format!(
            "{}: metadata episodes_requested={:?}, audit expects {expected_episodes}",
            path.display(),
            data.meta.episodes_requested
        ));
    }
    let expected_name = format!("{}_{}.json", data.meta.task, data.meta.eval_mode);
    if file_name != expected_name {
        errors.push(format!(
            "{}: filename does not match metadata; expected {expected_name}",
            path.display()
        ));
    }

    let ids_complete = !data.episodes.is_empty()
        && data
            .episodes
            .iter()
            .all(|episode| episode.episode_id.is_some());
    let unique_ids: BTreeSet<String> = data
        .episodes
        .iter()
        .filter_map(|episode| episode.episode_id.as_ref())
        .map(value_identity)
        .collect();
    if ids_complete && unique_ids.len() != data.episodes.len() {
        errors.push(format!(
            "{}: duplicate episode_id values prevent reliable pairing",
            path.display()
        ));
    }
    let seeds_complete =
        !data.episodes.is_empty() && data.episodes.iter().all(|episode| episode.seed.is_some());
    let unique_seeds: BTreeSet<String> = data
        .episodes
        .iter()
        .filter_map(|episode| episode.seed.as_ref())
        .map(value_identity)
        .collect();
    if seeds_complete && unique_seeds.len() != data.episodes.len() {
        errors.push(format!(
            "{}: duplicate seed values prevent reliable pairing",
            path.display()
        ));
    }

    let successes = data
        .episodes
        .iter()
        .filter(|episode| episode.success)
        .count();
    let proportion = wilson_proportion(successes, data.episodes.len());
    let galt = summarize_galt(&data.episodes);
    let episode_ids_present = ids_complete && unique_ids.len() == data.episodes.len();
    let seeds_present = seeds_complete && unique_seeds.len() == data.episodes.len();

    Ok(CellResult {
        run_label: run_label.to_string(),
        file_name,
        meta: data.meta,
        episodes: data.episodes,
        errors,
        proportion,
        galt,
        episode_ids_present,
        seeds_present,
    })
}

pub fn audit_run(
    label: &str,
    dir: &Path,
    expected_tasks: &[String],
    expected_modes: &[String],
    expected_episodes: usize,
) -> Result<RunData, Vec<String>> {
    if !dir.is_dir() {
        return Err(vec![format!(
            "run directory does not exist: {}",
            dir.display()
        )]);
    }

    let mut status = AuditStatus::default();
    let manifest_path = dir.join("audit-manifest.json");
    if manifest_path.is_file() {
        status.manifest_present = true;
        match fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<AuditManifest>(&raw).ok())
        {
            Some(manifest) => match validate_manifest(&manifest) {
                Ok(()) => status.manifest_valid = true,
                Err(errors) => status.errors.extend(
                    errors
                        .into_iter()
                        .map(|error| format!("{}: {error}", manifest_path.display())),
                ),
            },
            None => status.errors.push(format!(
                "{}: missing or invalid manifest JSON",
                manifest_path.display()
            )),
        }
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)
        .map_err(|error| vec![format!("cannot list {}: {error}", dir.display())])?
    {
        let entry = entry.map_err(|error| vec![format!("cannot read directory entry: {error}")])?;
        let path = entry.path();
        if !path.is_file()
            || path.file_name().and_then(|name| name.to_str()) == Some("audit-manifest.json")
        {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    paths.sort();
    status.result_files = paths.len();

    let expected: BTreeSet<String> = expected_tasks
        .iter()
        .flat_map(|task| {
            expected_modes
                .iter()
                .map(move |mode| expected_key(task, mode))
        })
        .collect();
    let mut found = BTreeSet::new();
    let mut cells = Vec::new();

    for path in paths {
        match parse_cell(&path, label, expected_episodes) {
            Ok(cell) => {
                let key = expected_key(&cell.meta.task, &cell.meta.eval_mode);
                if !found.insert(key.clone()) {
                    status.unexpected_cells.push(format!("duplicate:{key}"));
                    status.errors.push(format!(
                        "{}: duplicate task/mode cell {key}",
                        cell.file_name
                    ));
                    continue;
                }
                if !expected.contains(&key) {
                    status.unexpected_cells.push(key.clone());
                    status.errors.push(format!(
                        "{}: unexpected task/mode cell {key}",
                        path.display()
                    ));
                }
                if !cell.errors.is_empty() {
                    status.errors.extend(cell.errors.iter().cloned());
                    if cell
                        .errors
                        .iter()
                        .any(|error| error.contains("expected ") && error.contains(" episodes"))
                    {
                        status.episode_count_mismatches.push(key.clone());
                    }
                }
                cells.push(cell);
            }
            Err(errors) => status.errors.extend(errors),
        }
    }

    status.missing_cells = expected.difference(&found).cloned().collect();
    if !status.missing_cells.is_empty() {
        status.errors.push(format!(
            "missing expected task/mode cells: {}",
            status.missing_cells.join(", ")
        ));
    }

    let signatures: BTreeSet<_> = cells
        .iter()
        .map(|cell| meta_signature(&cell.meta))
        .collect();
    if signatures.len() > 1 {
        status.metadata_inconsistencies.push(
            "model/camera/robot/checkpoint/control metadata differs across result files"
                .to_string(),
        );
        status
            .errors
            .extend(status.metadata_inconsistencies.clone());
    }

    status.episode_ids_present =
        !cells.is_empty() && cells.iter().all(|cell| cell.episode_ids_present);
    status.seeds_present = !cells.is_empty() && cells.iter().all(|cell| cell.seeds_present);
    // Pairing also requires the *same identifiers* for each corresponding
    // task/mode cell across different runs. audit_run alone cannot assert that.
    status.pairing_ready = status.episode_ids_present && status.seeds_present;

    let mut official_pooled_by_mode = BTreeMap::new();
    let mut task_macro_by_mode = BTreeMap::new();
    for mode in expected_modes {
        let mode_cells: Vec<&CellResult> = cells
            .iter()
            .filter(|cell| cell.meta.eval_mode == *mode)
            .collect();
        let total_success: usize = mode_cells
            .iter()
            .map(|cell| cell.proportion.successes)
            .sum();
        let total_n: usize = mode_cells.iter().map(|cell| cell.proportion.total).sum();
        official_pooled_by_mode.insert(mode.clone(), wilson_proportion(total_success, total_n));

        let rates: Vec<f64> = expected_tasks
            .iter()
            .filter_map(|task| {
                mode_cells
                    .iter()
                    .find(|cell| cell.meta.task == *task)
                    .and_then(|cell| cell.proportion.rate)
            })
            .collect();
        task_macro_by_mode.insert(
            mode.clone(),
            if rates.len() == expected_tasks.len() && !rates.is_empty() {
                Some(rates.iter().sum::<f64>() / rates.len() as f64)
            } else {
                None
            },
        );
    }

    status.valid_cells = cells.iter().filter(|cell| cell.errors.is_empty()).count();

    Ok(RunData {
        label: label.to_string(),
        dir: dir.to_path_buf(),
        cells,
        status,
        official_pooled_by_mode,
        task_macro_by_mode,
    })
}

pub fn cross_run_pairing_ready(runs: &[RunData]) -> bool {
    if runs.len() < 2
        || runs.iter().any(|run| {
            !run.status.episode_ids_present
                || !run.status.seeds_present
                || !run.status.missing_cells.is_empty()
                || !run.status.unexpected_cells.is_empty()
                || !run.status.errors.is_empty()
        })
    {
        return false;
    }

    let key = |cell: &CellResult| expected_key(&cell.meta.task, &cell.meta.eval_mode);
    let mut canonical: Option<BTreeMap<String, BTreeSet<(String, String)>>> = None;

    for run in runs {
        let mut signatures: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
        for cell in &run.cells {
            let mut values = BTreeSet::new();
            for episode in &cell.episodes {
                let (Some(id), Some(seed)) = (&episode.episode_id, &episode.seed) else {
                    return false;
                };
                values.insert((value_identity(id), value_identity(seed)));
            }
            signatures.insert(key(cell), values);
        }
        if let Some(expected) = &canonical {
            if expected != &signatures {
                return false;
            }
        } else {
            canonical = Some(signatures);
        }
    }
    true
}

pub fn fmt_opt_pct(value: Option<f64>) -> String {
    value
        .map(|rate| format!("{:.4}", rate))
        .unwrap_or_else(|| "NA".to_string())
}

pub fn tsv(value: &str) -> String {
    tsv_clean(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_interval_is_bounded_and_symmetric_at_half() {
        let result = wilson_proportion(50, 100);
        assert_eq!(result.rate, Some(0.5));
        assert!(result.low_95.unwrap() < 0.5);
        assert!(result.high_95.unwrap() > 0.5);
        assert!((result.high_95.unwrap() - 0.5 - (0.5 - result.low_95.unwrap())).abs() < 1e-12);
    }

    #[test]
    fn zero_episodes_produce_unknown_rates_not_zero() {
        let result = wilson_proportion(0, 0);
        assert_eq!(result.rate, None);
        assert_eq!(result.low_95, None);
        assert_eq!(result.high_95, None);
    }

    #[test]
    fn manifest_rejects_invalid_hashes_and_versions() {
        let invalid = AuditManifest {
            schema_version: 2,
            upstream_commit: "abc".into(),
            checkpoint_sha256: "short".into(),
            assets_manifest_sha256: "g".repeat(64),
            environment_lock_sha256: "0".repeat(64),
            invocation: "".into(),
        };
        let errors = validate_manifest(&invalid).unwrap_err();
        assert!(errors.len() >= 4);
    }

    #[test]
    fn native_pooled_and_task_macro_can_differ() {
        let a = wilson_proportion(9, 10);
        let b = wilson_proportion(1, 10);
        let pooled = wilson_proportion(a.successes + b.successes, a.total + b.total)
            .rate
            .unwrap();
        let macro_mean = (a.rate.unwrap() + b.rate.unwrap()) / 2.0;
        assert!((pooled - macro_mean).abs() < 1e-12);
        let imbalanced_a = wilson_proportion(90, 100);
        let imbalanced_b = wilson_proportion(1, 2);
        let pooled_imbalanced = wilson_proportion(
            imbalanced_a.successes + imbalanced_b.successes,
            imbalanced_a.total + imbalanced_b.total,
        )
        .rate
        .unwrap();
        let macro_imbalanced = (imbalanced_a.rate.unwrap() + imbalanced_b.rate.unwrap()) / 2.0;
        assert!((pooled_imbalanced - macro_imbalanced).abs() > 0.05);
    }

    #[test]
    fn tsv_sanitization_drops_newlines_and_tabs() {
        assert_eq!(tsv("a\tb\nc"), "a b c");
    }

    fn fixture_dir(label: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after UNIX epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tavis-result-audit-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create fixture directory");
        path
    }

    fn write_fixture_run(
        dir: &Path,
        tasks_and_modes: &[(&str, &str, usize)],
        include_ids: bool,
        identity_salt: &str,
    ) {
        fs::create_dir_all(dir).expect("create run directory");
        for (task, mode, successes) in tasks_and_modes {
            let episodes: Vec<Value> = (0..2)
                .map(|index| {
                    let identity = format!("{identity_salt}-{task}-{mode}-{index}");
                    serde_json::json!({
                        "success": index < *successes,
                        "length_steps": 10,
                        "length_s": 0.5,
                        "wall_clock_s": 1.0,
                        "episode_id": if include_ids { Some(Value::String(identity.clone())) } else { None },
                        "seed": if include_ids { Some(Value::String(identity)) } else { None },
                        "extra": {
                            "galt_s": 0.75,
                            "galt_reason": "ok",
                            "galt_arm": "right"
                        }
                    })
                })
                .collect();
            let result = serde_json::json!({
                "meta": {
                    "timestamp": "2026-10-20T00:00:00",
                    "checkpoint": "/fixture/checkpoint",
                    "model": "pi0",
                    "camera": "headcam",
                    "robot": "gr1t2",
                    "suite": "tavis-head",
                    "task": task,
                    "eval_mode": mode,
                    "episodes_requested": 2,
                    "episode_length_s": 20.0,
                    "robot_ctrl_rate": 20.0
                },
                "episodes": episodes,
                "summary": {
                    "success_rate": *successes as f64 / 2.0,
                    "n_episodes": 2,
                    "n_successful": successes
                }
            });
            let path = dir.join(format!("{task}_{mode}.json"));
            fs::write(path, serde_json::to_vec_pretty(&result).unwrap())
                .expect("write fixture result");
        }
    }

    fn expected_fixture_tasks() -> Vec<String> {
        vec!["alpha".to_string(), "beta".to_string()]
    }

    fn one_mode() -> Vec<String> {
        vec!["id".to_string()]
    }

    #[test]
    fn audit_run_flags_missing_task_mode_cells_and_never_silently_completes_them() {
        let dir = fixture_dir("missing-cell");
        write_fixture_run(&dir, &[("alpha", "id", 1)], true, "same");

        let run = audit_run("smoke", &dir, &expected_fixture_tasks(), &one_mode(), 2).unwrap();

        assert_eq!(run.status.missing_cells, vec!["beta/id".to_string()]);
        assert!(!run.status.errors.is_empty());
        assert_eq!(run.cells.len(), 1);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pairing_is_unavailable_when_official_rows_have_no_ids_or_seeds() {
        let dir_a = fixture_dir("no-ids-a");
        let dir_b = fixture_dir("no-ids-b");
        let cells = [("alpha", "id", 1), ("beta", "id", 1)];
        write_fixture_run(&dir_a, &cells, false, "a");
        write_fixture_run(&dir_b, &cells, false, "a");

        let tasks = expected_fixture_tasks();
        let modes = one_mode();
        let a = audit_run("headcam", &dir_a, &tasks, &modes, 2).unwrap();
        let b = audit_run("fixedcam", &dir_b, &tasks, &modes, 2).unwrap();

        assert!(!a.status.episode_ids_present);
        assert!(!a.status.seeds_present);
        assert!(!cross_run_pairing_ready(&[a, b]));

        fs::remove_dir_all(dir_a).unwrap();
        fs::remove_dir_all(dir_b).unwrap();
    }

    #[test]
    fn pairing_requires_equal_unique_seed_and_episode_id_sets_across_runs() {
        let dir_a = fixture_dir("paired-a");
        let dir_b = fixture_dir("paired-b");
        let cells = [("alpha", "id", 1), ("beta", "id", 1)];
        write_fixture_run(&dir_a, &cells, true, "shared-seed");
        write_fixture_run(&dir_b, &cells, true, "different-seed");

        let tasks = expected_fixture_tasks();
        let modes = one_mode();
        let a = audit_run("controller-a", &dir_a, &tasks, &modes, 2).unwrap();
        let b = audit_run("controller-b", &dir_b, &tasks, &modes, 2).unwrap();

        assert!(a.status.pairing_ready);
        assert!(b.status.pairing_ready);
        assert!(!cross_run_pairing_ready(&[a, b]));

        fs::remove_dir_all(dir_a).unwrap();
        fs::remove_dir_all(dir_b).unwrap();
    }

    #[test]
    fn pairing_is_ready_only_when_all_cell_ids_and_seeds_match() {
        let dir_a = fixture_dir("same-pair-a");
        let dir_b = fixture_dir("same-pair-b");
        let cells = [("alpha", "id", 2), ("beta", "id", 0)];
        write_fixture_run(&dir_a, &cells, true, "shared");
        write_fixture_run(&dir_b, &cells, true, "shared");

        let tasks = expected_fixture_tasks();
        let modes = one_mode();
        let a = audit_run("headcam", &dir_a, &tasks, &modes, 2).unwrap();
        let b = audit_run("fixedcam", &dir_b, &tasks, &modes, 2).unwrap();

        assert!(cross_run_pairing_ready(&[a, b]));

        fs::remove_dir_all(dir_a).unwrap();
        fs::remove_dir_all(dir_b).unwrap();
    }

    #[test]
    fn duplicate_episode_ids_and_seeds_are_rejected_for_pairing() {
        let dir = fixture_dir("duplicate-identifiers");
        let mut result = serde_json::json!({
            "meta": {
                "timestamp": "2026-10-20T00:00:00",
                "checkpoint": "/fixture/checkpoint",
                "model": "pi0",
                "camera": "headcam",
                "robot": "gr1t2",
                "suite": "tavis-head",
                "task": "alpha",
                "eval_mode": "id",
                "episodes_requested": 2,
                "episode_length_s": 20.0,
                "robot_ctrl_rate": 20.0
            },
            "episodes": [
                {"success": true, "episode_id": "duplicate", "seed": "duplicate", "extra": {}},
                {"success": false, "episode_id": "duplicate", "seed": "duplicate", "extra": {}}
            ],
            "summary": {"success_rate": 0.5, "n_episodes": 2, "n_successful": 1}
        });
        fs::write(
            dir.join("alpha_id.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .expect("write duplicate fixture");

        let run = audit_run("duplicate", &dir, &["alpha".to_string()], &one_mode(), 2).unwrap();
        assert!(!run.status.episode_ids_present);
        assert!(!run.status.seeds_present);
        assert!(
            run.status
                .errors
                .iter()
                .any(|error| error.contains("duplicate episode_id"))
        );
        assert!(
            run.status
                .errors
                .iter()
                .any(|error| error.contains("duplicate seed"))
        );

        // Avoid retaining an unnecessary mutable binding in this fixture:
        result["summary"]["n_successful"] = Value::from(1);
        fs::remove_dir_all(dir).unwrap();
    }
}
