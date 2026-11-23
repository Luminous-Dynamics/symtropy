use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

const UPSTREAM_COMMIT: &str = "b3db3fb98ab5215fd8612a5ea3638702a1d39728";
const CLAIM_CEILING: &str = "Pinned upstream benchmark metadata and README reference tables are checked for integrity and completeness only. No DRSS code is executed here, no paper result is reproduced, and no Symtropy dispatch or factory-performance claim is established.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ContractManifest {
    schema_version: String,
    benchmark_id: String,
    source: SourcePin,
    simulator_protocol: SimulatorProtocol,
    decision_interface: DecisionInterface,
    score_contract: ScoreContract,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct SourcePin {
    repository_url: String,
    revision: String,
    readme_path: String,
    readme_git_blob_sha1: String,
    readme_bytes: u64,
    periodic_source_archive: ArchivePin,
    real_time_source_archive: ArchivePin,
    license_status: String,
    reuse_policy: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ArchivePin {
    path: String,
    git_blob_sha1: String,
    bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct SimulatorProtocol {
    machines: usize,
    jobs_per_episode: usize,
    warmup_jobs_excluded_from_tardiness: usize,
    replications_per_scenario: usize,
    operations_per_job_min: usize,
    operations_per_job_max: usize,
    processing_time_min: u64,
    validation: ScenarioGrid,
    test: ScenarioGrid,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ScenarioGrid {
    utilisation_levels: Vec<f64>,
    allowance_factors: Vec<f64>,
    processing_time_max_values: Vec<u64>,
    scenario_count: usize,
    reference_rows: Vec<ScenarioReference>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ScenarioReference {
    utilisation: f64,
    allowance_factor: f64,
    processing_time_max: u64,
    reference_mean_tardiness: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct DecisionInterface {
    selection_modes: Vec<String>,
    periodic_window_intervals_jobs: Vec<u64>,
    natural_log_transform_applied_to_all_state_variables: bool,
    state_sets: Vec<StateSet>,
    actions: Vec<DispatchAction>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct StateSet {
    id: String,
    features: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct DispatchAction {
    name: String,
    code: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ScoreContract {
    formula_id: String,
    scenario_score: String,
    aggregation: String,
    lower_is_better: bool,
    published_algorithm_scores: Vec<PublishedScore>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct PublishedScore {
    algorithm: String,
    validation_score: f64,
    test_score: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct VerifiedAsset {
    path: String,
    git_blob_sha1: String,
    bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct BestScore {
    algorithm: String,
    score: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ContractReport {
    schema_version: String,
    status: String,
    benchmark_id: String,
    source_repository_url: String,
    upstream_revision: String,
    source_revision_and_git_blob_pins_verified: bool,
    license_status: String,
    no_upstream_code_archives_redistributed: bool,
    machine_count: usize,
    jobs_per_episode: usize,
    warmup_jobs_excluded_from_tardiness: usize,
    scored_jobs_per_episode: usize,
    validation_scenario_count: usize,
    test_scenario_count: usize,
    total_scenario_count: usize,
    replications_per_scenario: usize,
    full_grid_episode_count: usize,
    full_grid_generated_job_count: usize,
    full_grid_scored_job_count: usize,
    validation_reference_rows_verified: usize,
    test_reference_rows_verified: usize,
    normalized_score_formula: String,
    published_algorithm_scores: Vec<PublishedScore>,
    best_published_validation_score: BestScore,
    best_published_test_score: BestScore,
    verified_assets: Vec<VerifiedAsset>,
    claim_ceiling: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("dynamic_drss_contract_error: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: ev-dynamic-drss-contract <contract.json> <pinned-upstream-checkout>".into());
    }
    let manifest_text = fs::read_to_string(&args[1])
        .map_err(|error| format!("cannot_read_contract: {error}"))?;
    let manifest: ContractManifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("invalid_contract_json: {error}"))?;
    validate_manifest(&manifest)?;

    let checkout = PathBuf::from(&args[2]);
    let verified_assets = verify_source_checkout(&manifest.source, &checkout)?;
    let readme_path = checkout.join(&manifest.source.readme_path);
    let readme = fs::read_to_string(&readme_path)
        .map_err(|error| format!("cannot_read_pinned_upstream_readme: {error}"))?;
    verify_published_tables(&manifest, &readme)?;

    let protocol = &manifest.simulator_protocol;
    let validation_count = manifest.simulator_protocol.validation.scenario_count;
    let test_count = manifest.simulator_protocol.test.scenario_count;
    let total_scenario_count = validation_count
        .checked_add(test_count)
        .ok_or_else(|| "scenario_count_overflow".to_string())?;
    let full_grid_episode_count = total_scenario_count
        .checked_mul(protocol.replications_per_scenario)
        .ok_or_else(|| "episode_count_overflow".to_string())?;
    let full_grid_generated_job_count = full_grid_episode_count
        .checked_mul(protocol.jobs_per_episode)
        .ok_or_else(|| "generated_job_count_overflow".to_string())?;
    let scored_jobs_per_episode = protocol
        .jobs_per_episode
        .checked_sub(protocol.warmup_jobs_excluded_from_tardiness)
        .ok_or_else(|| "warmup_exceeds_episode_length".to_string())?;
    let full_grid_scored_job_count = full_grid_episode_count
        .checked_mul(scored_jobs_per_episode)
        .ok_or_else(|| "scored_job_count_overflow".to_string())?;
    let best_validation = best_score(&manifest.score_contract.published_algorithm_scores, true)?;
    let best_test = best_score(&manifest.score_contract.published_algorithm_scores, false)?;

    let report = ContractReport {
        schema_version: "dynamic-drss-contract-report/0.1".into(),
        status: "source_and_score_contract_validated_not_benchmark_reproduced".into(),
        benchmark_id: manifest.benchmark_id,
        source_repository_url: manifest.source.repository_url.clone(),
        upstream_revision: manifest.source.revision.clone(),
        source_revision_and_git_blob_pins_verified: true,
        license_status: manifest.source.license_status.clone(),
        no_upstream_code_archives_redistributed: true,
        machine_count: protocol.machines,
        jobs_per_episode: protocol.jobs_per_episode,
        warmup_jobs_excluded_from_tardiness: protocol.warmup_jobs_excluded_from_tardiness,
        scored_jobs_per_episode,
        validation_scenario_count: validation_count,
        test_scenario_count: test_count,
        total_scenario_count,
        replications_per_scenario: protocol.replications_per_scenario,
        full_grid_episode_count,
        full_grid_generated_job_count,
        full_grid_scored_job_count,
        validation_reference_rows_verified: protocol.validation.reference_rows.len(),
        test_reference_rows_verified: protocol.test.reference_rows.len(),
        normalized_score_formula: manifest.score_contract.formula_id.clone(),
        published_algorithm_scores: manifest.score_contract.published_algorithm_scores.clone(),
        best_published_validation_score: best_validation,
        best_published_test_score: best_test,
        verified_assets,
        claim_ceiling: manifest.claim_ceiling,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot_serialize_report: {error}"))?
    );
    Ok(())
}

fn validate_manifest(manifest: &ContractManifest) -> Result<(), String> {
    if manifest.schema_version != "dynamic-drss-benchmark-contract/0.1"
        || manifest.benchmark_id != "DRSS-DYNAMIC-JSP-2025"
        || manifest.claim_ceiling != CLAIM_CEILING
    {
        return Err("unsupported_contract_version_benchmark_or_claim_ceiling".into());
    }
    let source = &manifest.source;
    if source.repository_url != "https://github.com/nuno-a-marques/drss"
        || source.revision != UPSTREAM_COMMIT
        || source.readme_path != "README.md"
        || source.readme_git_blob_sha1 != "c3bbb6d68b864a511cc9bce03d26d795a9b5a8ee"
        || source.readme_bytes != 16_903
        || source.periodic_source_archive.path != "drss-prs.zip"
        || source.periodic_source_archive.git_blob_sha1 != "a64ea1ae19054d7da894320c7c7b0b9c8dda18e2"
        || source.periodic_source_archive.bytes != 24_023_748
        || source.real_time_source_archive.path != "drss-rtrs.zip"
        || source.real_time_source_archive.git_blob_sha1 != "713c46453481e5afb0e15c58bfa94d9db6e1a435"
        || source.real_time_source_archive.bytes != 19_575_462
        || source.license_status
            != "unspecified_at_pinned_revision_no_root_LICENSE_or_COPYING_and_no_GitHub_SPDX_metadata"
        || source.reuse_policy
            != "integrity_check_only_do_not_commit_or_upload_upstream_source_archives_or_models"
    {
        return Err("upstream_source_or_reuse_contract_mismatch".into());
    }

    let protocol = &manifest.simulator_protocol;
    if protocol.machines != 20
        || protocol.jobs_per_episode != 2_500
        || protocol.warmup_jobs_excluded_from_tardiness != 500
        || protocol.replications_per_scenario != 100
        || protocol.operations_per_job_min != 2
        || protocol.operations_per_job_max != 14
        || protocol.processing_time_min != 1
    {
        return Err("published_simulator_protocol_mismatch".into());
    }
    validate_grid(
        "validation",
        &protocol.validation,
        &[0.83, 0.87, 0.93, 0.96],
        &[2.5, 3.5, 5.0, 7.0],
        &[50, 100],
        32,
    )?;
    validate_grid(
        "test",
        &protocol.test,
        &[0.80, 0.85, 0.90, 0.95, 0.97],
        &[2.0, 3.0, 4.0, 6.0, 8.0],
        &[50, 100, 200],
        75,
    )?;

    let decision = &manifest.decision_interface;
    if decision.selection_modes
        != vec!["periodic_rule_selection".to_string(), "real_time_rule_selection".to_string()]
        || decision.periodic_window_intervals_jobs != vec![50, 100, 200, 400, 800]
        || !decision.natural_log_transform_applied_to_all_state_variables
        || decision.state_sets
            != vec![
                StateSet {
                    id: "SS_1".into(),
                    features: vec![
                        "processing_times_of_pending_operations".into(),
                        "tardiness_of_pending_jobs".into(),
                    ],
                },
                StateSet {
                    id: "SS_2".into(),
                    features: vec![
                        "processing_times_of_pending_operations".into(),
                        "earliness_of_pending_jobs".into(),
                    ],
                },
                StateSet {
                    id: "SS_3".into(),
                    features: vec![
                        "processing_times_of_pending_operations".into(),
                        "tardiness_of_pending_jobs".into(),
                        "earliness_of_pending_jobs".into(),
                    ],
                },
            ]
        || decision.actions
            != vec![
                DispatchAction { name: "RR".into(), code: 18 },
                DispatchAction { name: "H2".into(), code: 214 },
                DispatchAction { name: "S9".into(), code: 508 },
            ]
    {
        return Err("published_decision_interface_mismatch".into());
    }

    let score = &manifest.score_contract;
    if score.formula_id
        != "geometric_mean_over_scenarios_of_mean_tardiness_divided_by_scenario_reference_mean_tardiness"
        || score.scenario_score
            != "mean_tardiness_on_scenario / published_reference_mean_tardiness_for_that_scenario"
        || score.aggregation != "geometric_mean_of_scenario_scores"
        || !score.lower_is_better
    {
        return Err("published_score_formula_mismatch".into());
    }
    let expected_scores = [
        ("RR", 1.146, 1.219),
        ("H2", 1.624, 1.509),
        ("S9", 1.102, 1.213),
        ("RF-Class.", 0.995, 1.060),
        ("RF-Reg.", 0.998, 1.000),
        ("NN-Class.", 0.989, 0.997),
        ("NN-Reg.", 1.023, 1.039),
        ("SVM-Class.", 0.994, 0.997),
        ("SVM-Reg.", 0.992, 0.995),
        ("QL", 1.007, 1.060),
        ("DQL", 0.981, 1.034),
    ];
    if score.published_algorithm_scores.len() != expected_scores.len()
        || score.published_algorithm_scores.iter().zip(expected_scores).any(|(actual, expected)| {
            actual.algorithm != expected.0
                || actual.validation_score != expected.1
                || actual.test_score != expected.2
        })
    {
        return Err("published_algorithm_score_table_mismatch".into());
    }
    Ok(())
}

fn validate_grid(
    name: &str,
    grid: &ScenarioGrid,
    expected_utilisation: &[f64],
    expected_allowance: &[f64],
    expected_processing_max: &[u64],
    expected_count: usize,
) -> Result<(), String> {
    if grid.utilisation_levels.as_slice() != expected_utilisation
        || grid.allowance_factors.as_slice() != expected_allowance
        || grid.processing_time_max_values.as_slice() != expected_processing_max
        || grid.scenario_count != expected_count
        || grid.reference_rows.len() != expected_count
    {
        return Err(format!("{name}_grid_dimensions_or_levels_mismatch"));
    }
    let mut expected = BTreeSet::new();
    for utilisation in expected_utilisation {
        for allowance in expected_allowance {
            for processing_max in expected_processing_max {
                expected.insert(format!("{utilisation:.2}|{allowance:.1}|{processing_max}"));
            }
        }
    }
    let mut actual = BTreeSet::new();
    for row in &grid.reference_rows {
        if !row.utilisation.is_finite()
            || !row.allowance_factor.is_finite()
            || row.reference_mean_tardiness == 0
            || !expected.contains(&reference_key(row))
            || !actual.insert(reference_key(row))
        {
            return Err(format!("{name}_reference_rows_have_invalid_or_duplicate_scenario"));
        }
    }
    if actual != expected {
        return Err(format!("{name}_reference_rows_do_not_cover_full_grid"));
    }
    Ok(())
}

fn reference_key(row: &ScenarioReference) -> String {
    format!(
        "{:.2}|{:.1}|{}",
        row.utilisation, row.allowance_factor, row.processing_time_max
    )
}

fn verify_source_checkout(
    source: &SourcePin,
    checkout: &Path,
) -> Result<Vec<VerifiedAsset>, String> {
    let revision = run_git(checkout, &["rev-parse", "HEAD"])?;
    if revision != source.revision {
        return Err("upstream_checkout_revision_mismatch".into());
    }
    let roots = run_git(checkout, &["ls-tree", "--name-only", "HEAD"])?;
    for forbidden in ["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING", "COPYING.txt"] {
        if roots.lines().any(|line| line == forbidden) {
            return Err("upstream_license_surface_changed_recheck_reuse_policy".into());
        }
    }

    let assets = [
        (
            source.readme_path.as_str(),
            source.readme_git_blob_sha1.as_str(),
            source.readme_bytes,
        ),
        (
            source.periodic_source_archive.path.as_str(),
            source.periodic_source_archive.git_blob_sha1.as_str(),
            source.periodic_source_archive.bytes,
        ),
        (
            source.real_time_source_archive.path.as_str(),
            source.real_time_source_archive.git_blob_sha1.as_str(),
            source.real_time_source_archive.bytes,
        ),
    ];
    let mut verified = Vec::with_capacity(assets.len());
    for (path, expected_sha, expected_bytes) in assets {
        let size = fs::metadata(checkout.join(path))
            .map_err(|error| format!("upstream_asset_missing_or_unreadable_{path}: {error}"))?
            .len();
        if size != expected_bytes {
            return Err(format!("upstream_asset_size_mismatch_{path}"));
        }
        let actual_sha = run_git(checkout, &["hash-object", path])?;
        if actual_sha != expected_sha {
            return Err(format!("upstream_asset_git_blob_mismatch_{path}"));
        }
        verified.push(VerifiedAsset {
            path: path.into(),
            git_blob_sha1: actual_sha,
            bytes: size,
        });
    }
    Ok(verified)
}

fn run_git(checkout: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(args)
        .output()
        .map_err(|error| format!("cannot_run_git: {error}"))?;
    if !output.status.success() {
        return Err(format!("git_command_failed_{}: {}", args.join("_"), String::from_utf8_lossy(&output.stderr).trim()));
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim().to_string())
        .map_err(|error| format!("git_output_not_utf8: {error}"))
}

fn verify_published_tables(manifest: &ContractManifest, readme: &str) -> Result<(), String> {
    let actual_validation = parse_reference_table(
        readme,
        "#### Reference values - validation",
        "#### Reference values - test",
    )?;
    let actual_test =
        parse_reference_table(readme, "#### Reference values - test", "### Benchmark")?;
    let actual_scores = parse_published_scores(readme)?;
    if sorted_reference_rows(actual_validation) != sorted_reference_rows(manifest.simulator_protocol.validation.reference_rows.clone()) {
        return Err("validation_reference_rows_do_not_match_pinned_readme".into());
    }
    if sorted_reference_rows(actual_test) != sorted_reference_rows(manifest.simulator_protocol.test.reference_rows.clone()) {
        return Err("test_reference_rows_do_not_match_pinned_readme".into());
    }
    if actual_scores != manifest.score_contract.published_algorithm_scores {
        return Err("published_algorithm_scores_do_not_match_pinned_readme".into());
    }
    Ok(())
}

fn sorted_reference_rows(mut rows: Vec<ScenarioReference>) -> Vec<ScenarioReference> {
    rows.sort_by(|left, right| {
        left.utilisation
            .total_cmp(&right.utilisation)
            .then_with(|| left.allowance_factor.total_cmp(&right.allowance_factor))
            .then_with(|| left.processing_time_max.cmp(&right.processing_time_max))
    });
    rows
}

fn parse_reference_table(
    readme: &str,
    start_heading: &str,
    end_heading: &str,
) -> Result<Vec<ScenarioReference>, String> {
    let lines: Vec<&str> = readme.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim() == start_heading)
        .ok_or_else(|| format!("missing_upstream_heading_{start_heading}"))?;
    let end = lines
        .iter()
        .enumerate()
        .find(|(index, line)| *index > start && line.trim() == end_heading)
        .map(|(index, _)| index)
        .ok_or_else(|| format!("missing_upstream_heading_{end_heading}"))?;
    let mut rows = Vec::new();
    for line in &lines[start + 1..end] {
        if !line.contains('|') {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).filter(|cell| !cell.is_empty()).collect();
        if cells.is_empty() || cells.len() % 4 != 0 {
            continue;
        }
        for group in cells.chunks(4) {
            let parsed = (
                group[0].parse::<f64>(),
                group[1].parse::<f64>(),
                group[2].parse::<u64>(),
                group[3].parse::<u64>(),
            );
            if let (Ok(utilisation), Ok(allowance_factor), Ok(processing_time_max), Ok(reference_mean_tardiness)) = parsed {
                if utilisation > 0.0 && allowance_factor > 0.0 && reference_mean_tardiness > 0 {
                    rows.push(ScenarioReference { utilisation, allowance_factor, processing_time_max, reference_mean_tardiness });
                }
            }
        }
    }
    if rows.is_empty() {
        return Err(format!("no_reference_rows_parsed_from_{start_heading}"));
    }
    Ok(rows)
}

fn parse_published_scores(readme: &str) -> Result<Vec<PublishedScore>, String> {
    let lines: Vec<&str> = readme.lines().collect();
    let heading = lines
        .iter()
        .position(|line| line.trim() == "| Algorithm | Validation Score | Test Score |")
        .ok_or_else(|| "missing_published_algorithm_score_table".to_string())?;
    let mut scores = Vec::new();
    for line in &lines[heading + 1..] {
        if !line.trim().starts_with('|') {
            break;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).filter(|cell| !cell.is_empty()).collect();
        if cells.len() != 3 {
            continue;
        }
        if let (Ok(validation_score), Ok(test_score)) = (cells[1].parse::<f64>(), cells[2].parse::<f64>()) {
            scores.push(PublishedScore { algorithm: cells[0].into(), validation_score, test_score });
        }
    }
    if scores.is_empty() {
        return Err("no_published_algorithm_scores_parsed".into());
    }
    Ok(scores)
}

fn best_score(scores: &[PublishedScore], validation: bool) -> Result<BestScore, String> {
    scores
        .iter()
        .map(|entry| BestScore {
            algorithm: entry.algorithm.clone(),
            score: if validation { entry.validation_score } else { entry.test_score },
        })
        .min_by(|left, right| left.score.total_cmp(&right.score).then_with(|| left.algorithm.cmp(&right.algorithm)))
        .ok_or_else(|| "published_score_table_is_empty".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ContractManifest {
        serde_json::from_str(include_str!("../fixtures/drss_benchmark_contract_v0_1.json"))
            .expect("dynamic DRSS contract fixture parses")
    }

    #[test]
    fn pinned_contract_dimensions_and_known_results_are_stable() {
        let contract = fixture();
        validate_manifest(&contract).expect("pinned protocol contract validates");
        assert_eq!(contract.simulator_protocol.validation.reference_rows.len(), 32);
        assert_eq!(contract.simulator_protocol.test.reference_rows.len(), 75);
        assert_eq!(contract.score_contract.published_algorithm_scores.len(), 11);
        assert_eq!(contract.score_contract.published_algorithm_scores[10].algorithm, "DQL");
        assert_eq!(contract.score_contract.published_algorithm_scores[10].test_score, 1.034);
        let episodes = (32 + 75) * contract.simulator_protocol.replications_per_scenario;
        assert_eq!(episodes, 10_700);
        assert_eq!(episodes * contract.simulator_protocol.jobs_per_episode, 26_750_000);
    }

    #[test]
    fn pinned_source_commit_mismatch_fails_closed() {
        let mut contract = fixture();
        contract.source.revision = "0000000000000000000000000000000000000000".into();
        assert_eq!(validate_manifest(&contract).unwrap_err(), "upstream_source_or_reuse_contract_mismatch");
    }

    #[test]
    fn duplicate_scenarios_fail_closed() {
        let mut contract = fixture();
        let first = contract.simulator_protocol.validation.reference_rows[0].clone();
        contract.simulator_protocol.validation.reference_rows[1] = first;
        assert_eq!(validate_manifest(&contract).unwrap_err(), "validation_reference_rows_have_invalid_or_duplicate_scenario");
    }

    #[test]
    fn published_scores_are_not_treated_as_candidate_results() {
        let contract = fixture();
        let best_validation = best_score(&contract.score_contract.published_algorithm_scores, true)
            .expect("validation score baseline");
        let best_test = best_score(&contract.score_contract.published_algorithm_scores, false)
            .expect("test score baseline");
        assert_eq!(best_validation.algorithm, "DQL");
        assert_eq!(best_validation.score, 0.981);
        assert_eq!(best_test.algorithm, "SVM-Reg.");
        assert_eq!(best_test.score, 0.995);
    }

    #[test]
    fn parser_reads_multiple_reference_rows_per_markdown_row() {
        let markdown = "#### Reference values - validation\n| u | a | ptMax | Reference | | u | a | ptMax | Reference |\n| :--: | :--: | :--: | --: | | :--: | :--: | :--: | --: |\n| 0.83 | 2.5 | 50 | 331823 | | 0.83 | 2.5 | 100 | 590031 |\n#### Reference values - test\n| u | a | ptMax | Reference |\n| 0.80 | 2 | 50 | 369241 |\n### Benchmark\n";
        let rows = parse_reference_table(markdown, "#### Reference values - validation", "#### Reference values - test")
            .expect("table parser reads validation block");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].reference_mean_tardiness, 331_823);
        assert_eq!(rows[1].processing_time_max, 100);
    }
}
