use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const ELIGIBILITY_CLAIM: &str = "lifecycle_intervention_eligibility";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub claim_ceiling: String,
    pub functional_unit: String,
    pub scope_id: String,
    pub evidence: Vec<EvidenceRecord>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub id: String,
    pub payload: String,
    pub sha256: String,
    pub status: EvidenceStatus,
    pub claim_id: String,
    pub scope_id: String,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub options: Vec<InterventionOption>,
    pub expected_dispositions: Vec<ExpectedDisposition>,
    pub expected_frontier: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InterventionOption {
    pub id: String,
    pub intervention: Intervention,
    pub functional_unit: String,
    pub evidence_id: String,
    pub safety: Gate,
    pub function: Gate,
    pub inspection: Gate,
    #[serde(default)]
    pub metrics: Metrics,
    pub flows: MaterialFlows,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    pub cost_cents: Option<u64>,
    pub virgin_material_g: Option<u64>,
    pub energy_wh: Option<u64>,
    pub ghg_co2e_g: Option<u64>,
    pub water_ml: Option<u64>,
    pub residual_waste_g: Option<u64>,
    pub original_asset_service_hours_retained: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MaterialFlows {
    pub input_g: u64,
    pub kept_in_service_g: u64,
    pub reused_component_g: u64,
    pub recycled_material_g: u64,
    pub energy_recovery_g: u64,
    pub residual_g: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedDisposition {
    pub option_id: String,
    pub disposition: Disposition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Valid,
    Stale,
    Revoked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Complete,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Intervention {
    Repair,
    Reuse,
    Remanufacture,
    ReplaceWithSelectiveDisassembly,
    ReplaceWithRecycling,
    ReplaceWithEnergyRecovery,
    ReplaceWithMixedDisposal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Eligible,
    RejectedHardGate,
    RejectedEvidence,
    InvalidDigest,
    OutOfScope,
    IncompatibleFunctionalUnit,
    RejectedMassBalance,
    BlockedIncompleteCoverage,
    BlockedUnknown,
    InvalidMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionResult {
    pub option_id: String,
    pub intervention: Intervention,
    pub disposition: Disposition,
    pub reasons: Vec<String>,
    pub digest_valid: bool,
    pub flow_balance_valid: bool,
    pub metrics_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub option_results: Vec<OptionResult>,
    pub eligible_option_ids: Vec<String>,
    pub pareto_frontier: Vec<String>,
    pub oracle_frontier: Vec<String>,
    pub oracle_agreement: bool,
    pub fixture_expectations_match: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub tool_version: String,
    pub schema_version: String,
    pub source_revision: String,
    pub manifest_sha256: String,
    pub claim_ceiling: String,
    pub functional_unit: String,
    pub evidence_count: usize,
    pub scenario_count: usize,
    pub option_count: usize,
    pub all_evidence_digests_valid: bool,
    pub scenarios: Vec<ScenarioResult>,
    pub overall_status: String,
}

pub fn evaluate(input: &[u8], source_revision: &str) -> Result<Report, String> {
    let manifest: Manifest =
        serde_json::from_slice(input).map_err(|error| format!("manifest rejected: {error}"))?;
    validate_manifest(&manifest)?;

    let evidence_by_id: BTreeMap<&str, &EvidenceRecord> = manifest
        .evidence
        .iter()
        .map(|record| (record.id.as_str(), record))
        .collect();
    let digests: BTreeMap<String, bool> = manifest
        .evidence
        .iter()
        .map(|record| (record.id.clone(), sha256_hex(record.payload.as_bytes()) == record.sha256))
        .collect();
    let all_evidence_digests_valid = digests.values().all(|valid| *valid);
    let mut scenarios: Vec<&Scenario> = manifest.scenarios.iter().collect();
    scenarios.sort_by(|a, b| a.id.cmp(&b.id));
    let mut results = Vec::with_capacity(scenarios.len());
    let mut option_count = 0usize;

    for scenario in scenarios {
        let mut evaluated = Vec::with_capacity(scenario.options.len());
        for option in &scenario.options {
            let evidence = evidence_by_id[option.evidence_id.as_str()];
            evaluated.push(evaluate_option(option, evidence, &manifest, digests[&evidence.id]));
        }
        evaluated.sort_by(|a, b| a.option_id.cmp(&b.option_id));
        option_count += evaluated.len();

        let eligible: Vec<&InterventionOption> = scenario
            .options
            .iter()
            .filter(|option| {
                evaluated.iter().any(|result| {
                    result.option_id == option.id && result.disposition == Disposition::Eligible
                })
            })
            .collect();
        let frontier = production_frontier(&eligible);
        let oracle = oracle_frontier(&eligible);
        let oracle_agreement = frontier == oracle;
        let fixture_expectations_match = matches_expectations(scenario, &evaluated, &frontier);
        let status = if oracle_agreement && fixture_expectations_match {
            "PASS"
        } else {
            "FAIL"
        };
        results.push(ScenarioResult {
            scenario_id: scenario.id.clone(),
            option_results: evaluated,
            eligible_option_ids: eligible.iter().map(|option| option.id.clone()).collect::<BTreeSet<_>>().into_iter().collect(),
            pareto_frontier: frontier.into_iter().collect(),
            oracle_frontier: oracle.into_iter().collect(),
            oracle_agreement,
            fixture_expectations_match,
            status: status.to_owned(),
        });
    }

    let all_scenarios_pass = results.iter().all(|result| result.status == "PASS");
    let overall_status = if all_evidence_digests_valid && all_scenarios_pass {
        "PASS_SYNTHETIC_CONTRACT"
    } else {
        "FAIL"
    };
    Ok(Report {
        tool_version: TOOL_VERSION.to_owned(),
        schema_version: manifest.schema_version,
        source_revision: source_revision.to_owned(),
        manifest_sha256: sha256_hex(input),
        claim_ceiling: manifest.claim_ceiling,
        functional_unit: manifest.functional_unit,
        evidence_count: manifest.evidence.len(),
        scenario_count: results.len(),
        option_count,
        all_evidence_digests_valid,
        scenarios: results,
        overall_status: overall_status.to_owned(),
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing into String cannot fail");
    }
    output
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema_version != "0.1" || manifest.claim_ceiling != "synthetic_only" {
        return Err("schema_version must be 0.1 and claim_ceiling must be synthetic_only".to_owned());
    }
    if manifest.functional_unit.trim().is_empty() || manifest.scope_id.trim().is_empty() {
        return Err("functional_unit and scope_id must be non-empty".to_owned());
    }
    if manifest.evidence.is_empty() || manifest.scenarios.is_empty() {
        return Err("manifest requires evidence and scenarios".to_owned());
    }
    let mut evidence_ids = BTreeSet::new();
    for evidence in &manifest.evidence {
        if evidence.id.trim().is_empty() || evidence.payload.is_empty() || evidence.scope_id.trim().is_empty() {
            return Err("evidence IDs, payloads, and scopes must be non-empty".to_owned());
        }
        if !is_sha256_hex(&evidence.sha256) {
            return Err(format!("evidence {} has invalid SHA-256 encoding", evidence.id));
        }
        if !evidence_ids.insert(evidence.id.as_str()) {
            return Err(format!("duplicate evidence ID {}", evidence.id));
        }
    }
    let evidence_map: BTreeMap<&str, &EvidenceRecord> = manifest.evidence.iter()
        .map(|record| (record.id.as_str(), record)).collect();
    let mut scenario_ids = BTreeSet::new();
    for scenario in &manifest.scenarios {
        if scenario.id.trim().is_empty() || !scenario_ids.insert(scenario.id.as_str()) {
            return Err(format!("empty or duplicate scenario ID {}", scenario.id));
        }
        if scenario.options.is_empty() {
            return Err(format!("scenario {} has no options", scenario.id));
        }
        let mut option_ids = BTreeSet::new();
        for option in &scenario.options {
            if option.id.trim().is_empty() || !option_ids.insert(option.id.as_str()) {
                return Err(format!("scenario {} has empty or duplicate option ID", scenario.id));
            }
            if !evidence_map.contains_key(option.evidence_id.as_str()) {
                return Err(format!("option {} references missing evidence {}", option.id, option.evidence_id));
            }
            if option.flows.input_g == 0 {
                return Err(format!("option {} has zero flow input", option.id));
            }
        }
        let expected_ids: BTreeSet<&str> = scenario.expected_dispositions.iter().map(|item| item.option_id.as_str()).collect();
        if expected_ids.len() != scenario.expected_dispositions.len()
            || expected_ids != option_ids.iter().copied().collect::<BTreeSet<_>>()
        {
            return Err(format!("scenario {} expected dispositions must name each option exactly once", scenario.id));
        }
        let expected_frontier: BTreeSet<&str> = scenario.expected_frontier.iter().map(String::as_str).collect();
        let known_option_ids: BTreeSet<&str> = option_ids.iter().copied().collect();
        if expected_frontier.len() != scenario.expected_frontier.len()
            || !expected_frontier.is_subset(&known_option_ids)
        {
            return Err(format!("scenario {} expected frontier contains duplicate or unknown option IDs", scenario.id));
        }
    }
    Ok(())
}

fn evaluate_option(
    option: &InterventionOption,
    evidence: &EvidenceRecord,
    manifest: &Manifest,
    digest_valid: bool,
) -> OptionResult {
    let mut reasons = Vec::new();
    let balance = flow_balance_valid(&option.flows)
        && option.metrics.residual_waste_g.map(|value| value == option.flows.residual_g).unwrap_or(true);
    let complete_metrics = metrics_complete(&option.metrics);
    if !digest_valid {
        reasons.push("evidence digest does not match exact payload bytes".to_owned());
    }
    match &evidence.status {
        EvidenceStatus::Valid => {}
        EvidenceStatus::Stale => reasons.push("evidence is stale".to_owned()),
        EvidenceStatus::Revoked => reasons.push("evidence is revoked".to_owned()),
        EvidenceStatus::Unknown => reasons.push("evidence validity is unknown".to_owned()),
    }
    if evidence.claim_id != ELIGIBILITY_CLAIM || evidence.scope_id != manifest.scope_id {
        reasons.push("evidence claim or scope does not match this lifecycle decision".to_owned());
    }
    match &evidence.coverage {
        Coverage::Complete => {}
        Coverage::Partial => reasons.push("evidence coverage is partial".to_owned()),
        Coverage::Unknown => reasons.push("evidence coverage is unknown".to_owned()),
    }
    if option.functional_unit != manifest.functional_unit {
        reasons.push("option does not deliver the manifest's required functional unit".to_owned());
    }
    for (name, gate) in [("safety", &option.safety), ("function", &option.function), ("inspection", &option.inspection)] {
        match gate {
            Gate::Pass => {}
            Gate::Fail => reasons.push(format!("{name} gate failed")),
            Gate::Unknown => reasons.push(format!("{name} gate is unknown")),
        }
    }
    if !balance {
        reasons.push("material fates do not conserve the declared input mass or residual metric".to_owned());
    }
    if !complete_metrics {
        reasons.push("one or more required lifecycle metrics are missing".to_owned());
    }
    if option.metrics.original_asset_service_hours_retained.is_some_and(|value| value > 10_000) {
        reasons.push("original_asset_service_hours_retained must be between 0 and 10000".to_owned());
    }

    let disposition = if !digest_valid {
        Disposition::InvalidDigest
    } else if evidence.status == EvidenceStatus::Stale || evidence.status == EvidenceStatus::Revoked {
        Disposition::RejectedEvidence
    } else if evidence.claim_id != ELIGIBILITY_CLAIM || evidence.scope_id != manifest.scope_id {
        Disposition::OutOfScope
    } else if option.functional_unit != manifest.functional_unit {
        Disposition::IncompatibleFunctionalUnit
    } else if !balance {
        Disposition::RejectedMassBalance
    } else if option.metrics.original_asset_service_hours_retained.is_some_and(|value| value > 10_000) {
        Disposition::InvalidMetrics
    } else if [&option.safety, &option.function, &option.inspection].iter().any(|gate| **gate == Gate::Fail) {
        Disposition::RejectedHardGate
    } else if evidence.coverage == Coverage::Partial {
        Disposition::BlockedIncompleteCoverage
    } else if evidence.status == EvidenceStatus::Unknown
        || evidence.coverage == Coverage::Unknown
        || [&option.safety, &option.function, &option.inspection].iter().any(|gate| **gate == Gate::Unknown)
        || !complete_metrics
    {
        Disposition::BlockedUnknown
    } else if !reasons.is_empty() {
        Disposition::BlockedUnknown
    } else {
        Disposition::Eligible
    };

    OptionResult {
        option_id: option.id.clone(),
        intervention: option.intervention.clone(),
        disposition,
        reasons,
        digest_valid,
        flow_balance_valid: balance,
        metrics_complete: complete_metrics,
    }
}

fn flow_balance_valid(flows: &MaterialFlows) -> bool {
    [
        flows.kept_in_service_g,
        flows.reused_component_g,
        flows.recycled_material_g,
        flows.energy_recovery_g,
        flows.residual_g,
    ]
    .into_iter()
    .try_fold(0u64, |total, quantity| total.checked_add(quantity))
    == Some(flows.input_g)
}

fn metrics_complete(metrics: &Metrics) -> bool {
    metrics.cost_cents.is_some()
        && metrics.virgin_material_g.is_some()
        && metrics.energy_wh.is_some()
        && metrics.ghg_co2e_g.is_some()
        && metrics.water_ml.is_some()
        && metrics.residual_waste_g.is_some()
        && metrics.original_asset_service_hours_retained.is_some()
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Production implementation: explicit objective-wise dominance comparison.
fn production_dominates(a: &InterventionOption, b: &InterventionOption) -> bool {
    let am = &a.metrics;
    let bm = &b.metrics;
    let minimizing_no_worse =
        am.cost_cents.unwrap() <= bm.cost_cents.unwrap()
        && am.virgin_material_g.unwrap() <= bm.virgin_material_g.unwrap()
        && am.energy_wh.unwrap() <= bm.energy_wh.unwrap()
        && am.ghg_co2e_g.unwrap() <= bm.ghg_co2e_g.unwrap()
        && am.water_ml.unwrap() <= bm.water_ml.unwrap()
        && am.residual_waste_g.unwrap() <= bm.residual_waste_g.unwrap();
    let retention_no_worse = am.original_asset_service_hours_retained.unwrap() >= bm.original_asset_service_hours_retained.unwrap();
    let strict =
        am.cost_cents.unwrap() < bm.cost_cents.unwrap()
        || am.virgin_material_g.unwrap() < bm.virgin_material_g.unwrap()
        || am.energy_wh.unwrap() < bm.energy_wh.unwrap()
        || am.ghg_co2e_g.unwrap() < bm.ghg_co2e_g.unwrap()
        || am.water_ml.unwrap() < bm.water_ml.unwrap()
        || am.residual_waste_g.unwrap() < bm.residual_waste_g.unwrap()
        || am.original_asset_service_hours_retained.unwrap() > bm.original_asset_service_hours_retained.unwrap();
    minimizing_no_worse && retention_no_worse && strict
}

fn production_frontier(options: &[&InterventionOption]) -> BTreeSet<String> {
    options.iter()
        .filter(|candidate| !options.iter().any(|other| other.id != candidate.id && production_dominates(other, candidate)))
        .map(|option| option.id.clone())
        .collect()
}

/// Independent exhaustive reference: enumerate every ordered eligible pair,
/// compare normalized vectors (all objectives are minimization; retained value
/// is sign-reversed), and eliminate any candidate with a strict dominator.
fn oracle_frontier(options: &[&InterventionOption]) -> BTreeSet<String> {
    let normalized = |o: &InterventionOption| -> [i128; 7] {
        [
            o.metrics.cost_cents.unwrap() as i128,
            o.metrics.virgin_material_g.unwrap() as i128,
            o.metrics.energy_wh.unwrap() as i128,
            o.metrics.ghg_co2e_g.unwrap() as i128,
            o.metrics.water_ml.unwrap() as i128,
            o.metrics.residual_waste_g.unwrap() as i128,
            -(o.metrics.original_asset_service_hours_retained.unwrap() as i128),
        ]
    };
    let mut frontier = BTreeSet::new();
    for candidate in options {
        let target = normalized(candidate);
        let mut dominated = false;
        for other in options {
            if candidate.id == other.id {
                continue;
            }
            let source = normalized(other);
            let every_objective_no_worse = (0..target.len()).all(|index| source[index] <= target[index]);
            let some_objective_better = (0..target.len()).any(|index| source[index] < target[index]);
            if every_objective_no_worse && some_objective_better {
                dominated = true;
                break;
            }
        }
        if !dominated {
            frontier.insert(candidate.id.clone());
        }
    }
    frontier
}

fn matches_expectations(
    scenario: &Scenario,
    evaluated: &[OptionResult],
    frontier: &BTreeSet<String>,
) -> bool {
    let expected_dispositions: BTreeMap<&str, &Disposition> = scenario.expected_dispositions.iter()
        .map(|item| (item.option_id.as_str(), &item.disposition)).collect();
    let actual_dispositions: BTreeMap<&str, &Disposition> = evaluated.iter()
        .map(|item| (item.option_id.as_str(), &item.disposition)).collect();
    let expected_frontier: BTreeSet<String> = scenario.expected_frontier.iter().cloned().collect();
    expected_dispositions == actual_dispositions && *frontier == expected_frontier
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/lifecycle-options-v0-1.json");

    fn manifest() -> Manifest {
        serde_json::from_str(FIXTURE).expect("fixture should parse")
    }

    fn evaluate_fixture() -> Report {
        evaluate(FIXTURE.as_bytes(), "test-revision").expect("fixture should evaluate")
    }

    #[test]
    fn frozen_scenarios_match_oracle_and_expected_frontiers() {
        let report = evaluate_fixture();
        assert_eq!(report.overall_status, "PASS_SYNTHETIC_CONTRACT");
        assert!(report.all_evidence_digests_valid);
        assert_eq!(report.scenario_count, 5);
        for scenario in &report.scenarios {
            assert_eq!(scenario.status, "PASS", "scenario {}", scenario.scenario_id);
            assert!(scenario.oracle_agreement, "scenario {}", scenario.scenario_id);
            assert!(scenario.fixture_expectations_match, "scenario {}", scenario.scenario_id);
        }
    }

    #[test]
    fn cost_and_virgin_material_tradeoff_remains_pareto_non_dominated() {
        let result = evaluate_fixture().scenarios.into_iter()
            .find(|scenario| scenario.scenario_id == "baseline-pareto").unwrap();
        assert_eq!(result.pareto_frontier, vec!["remanufacture-low-virgin", "repair-low-cost"]);
        assert!(!result.pareto_frontier.contains(&"dominated-option".to_owned()));
    }

    #[test]
    fn unknown_or_partial_evidence_never_becomes_eligible() {
        let result = evaluate_fixture().scenarios.into_iter()
            .find(|scenario| scenario.scenario_id == "reuse-evidence-gate").unwrap();
        let reuse = result.option_results.iter().find(|option| option.option_id == "reuse-unknown-inspection").unwrap();
        assert_eq!(reuse.disposition, Disposition::BlockedUnknown);
        assert!(!result.eligible_option_ids.contains(&reuse.option_id));
        let partial = result.option_results.iter()
            .find(|option| option.option_id == "reuse-partial-coverage").unwrap();
        assert_eq!(partial.disposition, Disposition::BlockedIncompleteCoverage);
        assert!(!result.eligible_option_ids.contains(&partial.option_id));
    }

    #[test]
    fn revoked_certificates_and_failed_contamination_inspection_are_rejected() {
        let result = evaluate_fixture().scenarios.into_iter()
            .find(|scenario| scenario.scenario_id == "contamination-control").unwrap();
        let revoked = result.option_results.iter().find(|option| option.option_id == "recycle-revoked-certificate").unwrap();
        let contaminated = result.option_results.iter().find(|option| option.option_id == "recycle-contamination-fail").unwrap();
        assert_eq!(revoked.disposition, Disposition::RejectedEvidence);
        assert_eq!(contaminated.disposition, Disposition::RejectedHardGate);
    }

    #[test]
    fn mass_double_counting_is_rejected_and_energy_recovery_is_not_material_recycling() {
        let result = evaluate_fixture().scenarios.into_iter()
            .find(|scenario| scenario.scenario_id == "mass-balance-and-missing-metrics").unwrap();
        let double_counted = result.option_results.iter().find(|option| option.option_id == "repair-double-counted-mass").unwrap();
        assert_eq!(double_counted.disposition, Disposition::RejectedMassBalance);
        let energy = result.option_results.iter().find(|option| option.option_id == "replacement-energy-recovery").unwrap();
        assert!(energy.flow_balance_valid);
        let energy_flows = &manifest().scenarios.iter()
            .find(|scenario| scenario.id == "contamination-control").unwrap()
            .options.iter().find(|option| option.id == "replacement-energy-recovery").unwrap().flows;
        assert_eq!(energy_flows.energy_recovery_g, 700);
        assert_eq!(energy_flows.recycled_material_g, 0);
    }

    #[test]
    fn missing_metrics_do_not_become_zero() {
        let result = evaluate_fixture().scenarios.into_iter()
            .find(|scenario| scenario.scenario_id == "mass-balance-and-missing-metrics").unwrap();
        let missing = result.option_results.iter().find(|option| option.option_id == "replacement-missing-energy-data").unwrap();
        assert_eq!(missing.disposition, Disposition::BlockedUnknown);
        assert!(!missing.metrics_complete);
    }

    #[test]
    fn incompatible_functional_units_cannot_be_ranked_together() {
        let result = evaluate_fixture().scenarios.into_iter()
            .find(|scenario| scenario.scenario_id == "functional-unit-mismatch").unwrap();
        let mismatched = result.option_results.iter().find(|option| option.option_id == "repair-shorter-service").unwrap();
        assert_eq!(mismatched.disposition, Disposition::IncompatibleFunctionalUnit);
    }

    #[test]
    fn mutated_evidence_digest_blocks_every_dependent_option() {
        let mut manifest = manifest();
        let evidence = manifest.evidence.iter_mut().find(|entry| entry.id == "ev-repair").unwrap();
        evidence.sha256 = "0".repeat(64);
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let report = evaluate(&bytes, "test-revision").unwrap();
        assert_eq!(report.overall_status, "FAIL");
        let repair = report.scenarios.iter().flat_map(|s| &s.option_results)
            .find(|option| option.option_id == "repair-low-cost").unwrap();
        assert_eq!(repair.disposition, Disposition::InvalidDigest);
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let bad = FIXTURE.replace(
            "\"claim_ceiling\": \"synthetic_only\"",
            "\"claim_ceiling\": \"synthetic_only\", \"green_score\": 100",
        );
        assert!(evaluate(bad.as_bytes(), "test-revision").is_err());
    }

    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
