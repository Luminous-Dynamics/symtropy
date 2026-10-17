use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;

const CLAIM_ID: &str = "net_additionality_action_profile";
const BASIS_POINTS: u64 = 10_000;
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub claim_ceiling: String,
    pub scope_id: String,
    pub functional_unit: String,
    pub demand_units: u64,
    pub virgin_material_g_per_unit: u64,
    pub embodied_ghg_gco2e_per_g: u64,
    pub evidence: Vec<Evidence>,
    pub options: Vec<Candidate>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub id: String,
    pub payload: String,
    pub sha256: String,
    pub status: EvidenceStatus,
    pub claim_id: String,
    pub scope_id: String,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Valid,
    Unknown,
    Revoked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Complete,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub display_name: String,
    pub evidence_id: String,
    pub gross_recovered_g_per_unit: u64,
    pub recovery_yield_bp: u32,
    pub claimed_substitution_bp: u32,
    pub rebound_sensitivity_bp: u32,
    pub process_ghg_gco2e_per_unit: u64,
    pub cost_cents_per_unit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub probability_bp: u32,
    pub substitution_cap_bp: u32,
    pub rebound_multiplier_bp: u32,
    pub virgin_ghg_multiplier_bp: u32,
    pub process_ghg_multiplier_bp: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioOutcome {
    pub scenario_id: String,
    pub probability_bp: u32,
    pub service_units_after_rebound: u64,
    pub yield_adjusted_recovered_g_per_unit: u64,
    pub effective_substitution_bp: u32,
    pub displaced_virgin_g_per_unit: u64,
    pub total_virgin_ghg_gco2e: u64,
    pub total_process_ghg_gco2e: u64,
    pub total_ghg_gco2e: u64,
    pub total_cost_cents: u64,
    pub scenario_oracle_option_id: Option<String>,
    pub regret_gco2e: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionResult {
    pub id: String,
    pub display_name: String,
    pub eligible: bool,
    pub eligibility_reason: Option<String>,
    pub gross_circularity_score_g_per_unit: Option<u64>,
    pub expected_ghg_gco2e: Option<u64>,
    pub expected_cost_cents: Option<u64>,
    pub max_regret_gco2e: Option<u64>,
    pub in_expected_ghg_cost_frontier: bool,
    pub scenarios: Vec<ScenarioOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioOracle {
    pub scenario_id: String,
    pub option_id: String,
    pub total_ghg_gco2e: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub schema_version: String,
    pub tool_version: String,
    pub claim_ceiling: String,
    pub overall_status: String,
    pub source_revision: String,
    pub manifest_sha256: String,
    pub functional_unit: String,
    pub gross_circularity_policy_id: String,
    pub expected_impact_policy_id: String,
    pub minimax_regret_policy_id: String,
    pub expected_ghg_cost_frontier_ids: Vec<String>,
    pub scenario_oracles: Vec<ScenarioOracle>,
    pub options: Vec<OptionResult>,
    pub caveats: Vec<String>,
}

pub fn evaluate(input: &[u8], source_revision: &str) -> Result<Report, String> {
    let manifest: Manifest =
        serde_json::from_slice(input).map_err(|error| format!("invalid manifest JSON: {error}"))?;
    validate_manifest(&manifest)?;

    let mut candidates = manifest.options.clone();
    candidates.sort_by(|a, b| a.id.cmp(&b.id));
    let mut scenarios = manifest.scenarios.clone();
    scenarios.sort_by(|a, b| a.id.cmp(&b.id));

    let mut results = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let reason = eligibility_reason(candidate, &manifest);
        let eligible = reason.is_none();
        let score = if eligible {
            Some(
                floor_mul_div(
                    candidate.gross_recovered_g_per_unit,
                    u64::from(candidate.recovery_yield_bp),
                    BASIS_POINTS,
                )?
                .min(manifest.virgin_material_g_per_unit),
            )
        } else {
            None
        };
        let mut outcomes = Vec::new();
        if eligible {
            for scenario in &scenarios {
                outcomes.push(compute_outcome(candidate, scenario, &manifest)?);
            }
        }
        results.push(OptionResult {
            id: candidate.id.clone(),
            display_name: candidate.display_name.clone(),
            eligible,
            eligibility_reason: reason,
            gross_circularity_score_g_per_unit: score,
            expected_ghg_gco2e: None,
            expected_cost_cents: None,
            max_regret_gco2e: None,
            in_expected_ghg_cost_frontier: false,
            scenarios: outcomes,
        });
    }

    if !results.iter().any(|result| result.eligible) {
        return Err("manifest has no evidence-eligible candidate".to_owned());
    }

    let mut scenario_oracles = Vec::with_capacity(scenarios.len());
    for scenario in &scenarios {
        let (oracle_id, oracle_ghg) = results
            .iter()
            .filter(|result| result.eligible)
            .filter_map(|result| {
                result
                    .scenarios
                    .iter()
                    .find(|outcome| outcome.scenario_id == scenario.id)
                    .map(|outcome| (result.id.as_str(), outcome.total_ghg_gco2e))
            })
            .min_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(right.0)))
            .map(|(id, ghg)| (id.to_owned(), ghg))
            .ok_or_else(|| format!("no eligible outcome for scenario {}", scenario.id))?;

        scenario_oracles.push(ScenarioOracle {
            scenario_id: scenario.id.clone(),
            option_id: oracle_id.clone(),
            total_ghg_gco2e: oracle_ghg,
        });

        for result in results.iter_mut().filter(|result| result.eligible) {
            let outcome = result
                .scenarios
                .iter_mut()
                .find(|outcome| outcome.scenario_id == scenario.id)
                .ok_or_else(|| format!("missing outcome for {} / {}", result.id, scenario.id))?;
            outcome.scenario_oracle_option_id = Some(oracle_id.clone());
            outcome.regret_gco2e = Some(
                outcome
                    .total_ghg_gco2e
                    .checked_sub(oracle_ghg)
                    .ok_or_else(|| "oracle regret underflow".to_owned())?,
            );
        }
    }

    for result in results.iter_mut().filter(|result| result.eligible) {
        result.expected_ghg_gco2e = Some(weighted_average(
            &result.scenarios,
            |outcome| outcome.total_ghg_gco2e,
        )?);
        result.expected_cost_cents = Some(weighted_average(
            &result.scenarios,
            |outcome| outcome.total_cost_cents,
        )?);
        result.max_regret_gco2e = result
            .scenarios
            .iter()
            .map(|outcome| outcome.regret_gco2e.unwrap_or(u64::MAX))
            .max();
    }

    let frontier_data: Vec<(String, u64, u64)> = results
        .iter()
        .filter(|result| result.eligible)
        .map(|result| {
            Ok((
                result.id.clone(),
                result
                    .expected_ghg_gco2e
                    .ok_or_else(|| "eligible result missing expected GHG".to_owned())?,
                result
                    .expected_cost_cents
                    .ok_or_else(|| "eligible result missing expected cost".to_owned())?,
            ))
        })
        .collect::<Result<_, String>>()?;

    let mut frontier_ids = Vec::new();
    for result in results.iter_mut().filter(|result| result.eligible) {
        let ghg = result.expected_ghg_gco2e.unwrap_or(u64::MAX);
        let cost = result.expected_cost_cents.unwrap_or(u64::MAX);
        let dominated = frontier_data.iter().any(|(other_id, other_ghg, other_cost)| {
            other_id != &result.id
                && *other_ghg <= ghg
                && *other_cost <= cost
                && (*other_ghg < ghg || *other_cost < cost)
        });
        result.in_expected_ghg_cost_frontier = !dominated;
        if !dominated {
            frontier_ids.push(result.id.clone());
        }
    }
    frontier_ids.sort();

    let gross_circularity_policy_id = results
        .iter()
        .filter(|result| result.eligible)
        .max_by(|left, right| {
            left.gross_circularity_score_g_per_unit
                .cmp(&right.gross_circularity_score_g_per_unit)
                .then_with(|| right.id.cmp(&left.id))
        })
        .map(|result| result.id.clone())
        .ok_or_else(|| "no eligible gross-circularity candidate".to_owned())?;

    let expected_impact_policy_id = results
        .iter()
        .filter(|result| result.eligible)
        .min_by(|left, right| {
            left.expected_ghg_gco2e
                .cmp(&right.expected_ghg_gco2e)
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|result| result.id.clone())
        .ok_or_else(|| "no eligible expected-impact candidate".to_owned())?;

    let minimax_regret_policy_id = results
        .iter()
        .filter(|result| result.eligible)
        .min_by(|left, right| {
            left.max_regret_gco2e
                .cmp(&right.max_regret_gco2e)
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|result| result.id.clone())
        .ok_or_else(|| "no eligible minimax-regret candidate".to_owned())?;

    scenario_oracles.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

    Ok(Report {
        schema_version: "0.1".to_owned(),
        tool_version: TOOL_VERSION.to_owned(),
        claim_ceiling: "synthetic_only".to_owned(),
        overall_status: "PASS_SYNTHETIC_CONTRACT".to_owned(),
        source_revision: if source_revision.trim().is_empty() {
            "unprovided".to_owned()
        } else {
            source_revision.to_owned()
        },
        manifest_sha256: sha256_hex(input),
        functional_unit: manifest.functional_unit,
        gross_circularity_policy_id,
        expected_impact_policy_id,
        minimax_regret_policy_id,
        expected_ghg_cost_frontier_ids: frontier_ids,
        scenario_oracles,
        options: results,
        caveats: vec![
            "All quantities are synthetic and are not measured environmental impacts.".to_owned(),
            "Substitution caps and rebound multipliers are declared scenarios, not inferred probabilities.".to_owned(),
            "The hindsight scenario oracle is a lower-bound comparator, not an implementable policy.".to_owned(),
            "Expected and minimax-regret rankings depend on the declared scenario set.".to_owned(),
        ],
    })
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema_version != "0.1" || manifest.claim_ceiling != "synthetic_only" {
        return Err("unsupported schema version or claim ceiling".to_owned());
    }
    if manifest.scope_id.trim().is_empty() || manifest.functional_unit.trim().is_empty() {
        return Err("scope and functional unit must be non-empty".to_owned());
    }
    if manifest.demand_units == 0
        || manifest.virgin_material_g_per_unit == 0
        || manifest.embodied_ghg_gco2e_per_g == 0
    {
        return Err("demand, virgin material, and embodied GHG factor must be positive".to_owned());
    }
    if manifest.options.is_empty()
        || manifest.options.len() > 128
        || manifest.scenarios.is_empty()
        || manifest.scenarios.len() > 64
        || manifest.options.len().saturating_mul(manifest.scenarios.len()) > 4096
    {
        return Err("option/scenario space is empty or exceeds the bounded contract".to_owned());
    }
    if manifest.evidence.is_empty() || manifest.evidence.len() > 512 {
        return Err("evidence list is empty or exceeds the bounded contract".to_owned());
    }

    check_unique_ids(manifest.evidence.iter().map(|item| item.id.as_str()), "evidence")?;
    check_unique_ids(manifest.options.iter().map(|item| item.id.as_str()), "option")?;
    check_unique_ids(manifest.scenarios.iter().map(|item| item.id.as_str()), "scenario")?;

    for evidence in &manifest.evidence {
        if evidence.payload.is_empty() || evidence.claim_id.trim().is_empty() || evidence.scope_id.trim().is_empty() {
            return Err(format!("evidence {} has an empty required field", evidence.id));
        }
        if sha256_hex(evidence.payload.as_bytes()) != evidence.sha256 {
            return Err(format!("evidence payload digest mismatch: {}", evidence.id));
        }
    }

    let mut probability_sum = 0_u64;
    for scenario in &manifest.scenarios {
        if scenario.substitution_cap_bp > 10_000
            || scenario.rebound_multiplier_bp > 10_000
            || scenario.virgin_ghg_multiplier_bp > 20_000
            || scenario.process_ghg_multiplier_bp > 20_000
        {
            return Err(format!("scenario {} has a rate outside its declared bound", scenario.id));
        }
        probability_sum = probability_sum
            .checked_add(u64::from(scenario.probability_bp))
            .ok_or_else(|| "scenario probability sum overflow".to_owned())?;
    }
    if probability_sum != BASIS_POINTS {
        return Err(format!("scenario probabilities must sum to 10000 bp, got {probability_sum}"));
    }

    let evidence_ids: BTreeSet<&str> = manifest.evidence.iter().map(|item| item.id.as_str()).collect();
    for candidate in &manifest.options {
        if candidate.display_name.trim().is_empty() {
            return Err(format!("candidate {} has an empty display name", candidate.id));
        }
        if candidate.recovery_yield_bp > 10_000
            || candidate.claimed_substitution_bp > 10_000
            || candidate.rebound_sensitivity_bp > 10_000
        {
            return Err(format!("candidate {} has a rate outside [0, 10000] bp", candidate.id));
        }
        if !evidence_ids.contains(candidate.evidence_id.as_str()) {
            return Err(format!("candidate {} references missing evidence", candidate.id));
        }
    }
    Ok(())
}

fn check_unique_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(format!("{label} ID must be non-empty"));
        }
        if !seen.insert(id) {
            return Err(format!("duplicate {label} ID: {id}"));
        }
    }
    Ok(())
}

fn eligibility_reason(candidate: &Candidate, manifest: &Manifest) -> Option<String> {
    let evidence = manifest
        .evidence
        .iter()
        .find(|evidence| evidence.id == candidate.evidence_id)?;
    if evidence.status != EvidenceStatus::Valid {
        return Some("evidence_status_not_valid".to_owned());
    }
    if evidence.coverage != Coverage::Complete {
        return Some("evidence_coverage_not_complete".to_owned());
    }
    if evidence.claim_id != CLAIM_ID {
        return Some("evidence_claim_mismatch".to_owned());
    }
    if evidence.scope_id != manifest.scope_id {
        return Some("evidence_scope_mismatch".to_owned());
    }
    None
}

fn compute_outcome(
    candidate: &Candidate,
    scenario: &Scenario,
    manifest: &Manifest,
) -> Result<ScenarioOutcome, String> {
    let yielded = floor_mul_div(
        candidate.gross_recovered_g_per_unit,
        u64::from(candidate.recovery_yield_bp),
        BASIS_POINTS,
    )?;
    let effective_substitution_bp = candidate
        .claimed_substitution_bp
        .min(scenario.substitution_cap_bp);
    let displaced = floor_mul_div(
        yielded,
        u64::from(effective_substitution_bp),
        BASIS_POINTS,
    )?
    .min(manifest.virgin_material_g_per_unit);
    let remaining_virgin_g = manifest
        .virgin_material_g_per_unit
        .checked_sub(displaced)
        .ok_or_else(|| "displacement exceeded virgin demand".to_owned())?;
    let virgin_embodied_ghg = remaining_virgin_g
        .checked_mul(manifest.embodied_ghg_gco2e_per_g)
        .ok_or_else(|| "virgin GHG multiplication overflow".to_owned())?;
    let virgin_ghg_per_unit = ceil_mul_div(
        virgin_embodied_ghg,
        u64::from(scenario.virgin_ghg_multiplier_bp),
        BASIS_POINTS,
    )?;
    let process_ghg_per_unit = ceil_mul_div(
        candidate.process_ghg_gco2e_per_unit,
        u64::from(scenario.process_ghg_multiplier_bp),
        BASIS_POINTS,
    )?;

    let effective_rebound_bp = ceil_mul_div(
        u64::from(candidate.rebound_sensitivity_bp),
        u64::from(scenario.rebound_multiplier_bp),
        BASIS_POINTS,
    )?;
    let additional_units = ceil_mul_div(
        manifest.demand_units,
        effective_rebound_bp,
        BASIS_POINTS,
    )?;
    let service_units_after_rebound = manifest
        .demand_units
        .checked_add(additional_units)
        .ok_or_else(|| "rebound-adjusted service units overflow".to_owned())?;

    let total_virgin_ghg_gco2e = virgin_ghg_per_unit
        .checked_mul(service_units_after_rebound)
        .ok_or_else(|| "total virgin GHG overflow".to_owned())?;
    let total_process_ghg_gco2e = process_ghg_per_unit
        .checked_mul(service_units_after_rebound)
        .ok_or_else(|| "total process GHG overflow".to_owned())?;
    let total_ghg_gco2e = total_virgin_ghg_gco2e
        .checked_add(total_process_ghg_gco2e)
        .ok_or_else(|| "total GHG overflow".to_owned())?;
    let total_cost_cents = candidate
        .cost_cents_per_unit
        .checked_mul(service_units_after_rebound)
        .ok_or_else(|| "total cost overflow".to_owned())?;

    Ok(ScenarioOutcome {
        scenario_id: scenario.id.clone(),
        probability_bp: scenario.probability_bp,
        service_units_after_rebound,
        yield_adjusted_recovered_g_per_unit: yielded,
        effective_substitution_bp,
        displaced_virgin_g_per_unit: displaced,
        total_virgin_ghg_gco2e,
        total_process_ghg_gco2e,
        total_ghg_gco2e,
        total_cost_cents,
        scenario_oracle_option_id: None,
        regret_gco2e: None,
    })
}

fn weighted_average<F>(outcomes: &[ScenarioOutcome], value: F) -> Result<u64, String>
where
    F: Fn(&ScenarioOutcome) -> u64,
{
    let numerator = outcomes.iter().try_fold(0_u128, |sum, outcome| {
        let term = u128::from(value(outcome)) * u128::from(outcome.probability_bp);
        sum.checked_add(term)
            .ok_or_else(|| "weighted average numerator overflow".to_owned())
    })?;
    let rounded = numerator
        .checked_add(u128::from(BASIS_POINTS / 2))
        .ok_or_else(|| "weighted average rounding overflow".to_owned())?
        / u128::from(BASIS_POINTS);
    u64::try_from(rounded).map_err(|_| "weighted average exceeds u64".to_owned())
}

fn floor_mul_div(left: u64, right: u64, denominator: u64) -> Result<u64, String> {
    let product = left
        .checked_mul(right)
        .ok_or_else(|| "fixed-point multiplication overflow".to_owned())?;
    Ok(product / denominator)
}

fn ceil_mul_div(left: u64, right: u64, denominator: u64) -> Result<u64, String> {
    let product = left
        .checked_mul(right)
        .ok_or_else(|| "fixed-point multiplication overflow".to_owned())?;
    let quotient = product / denominator;
    if product % denominator == 0 {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or_else(|| "fixed-point ceiling overflow".to_owned())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_manifest() -> Manifest {
        serde_json::from_slice(include_bytes!("../fixtures/net-additionality-v0-1.json"))
            .expect("frozen test fixture parses")
    }

    fn fixture_report() -> Report {
        evaluate(
            include_bytes!("../fixtures/net-additionality-v0-1.json"),
            "test-source-revision",
        )
        .expect("frozen fixture satisfies the synthetic contract")
    }

    fn result<'a>(report: &'a Report, id: &str) -> &'a OptionResult {
        report.options.iter().find(|item| item.id == id).expect("result exists")
    }

    #[test]
    fn fixture_separates_gross_expected_and_robust_rankings() {
        let report = fixture_report();
        assert_eq!(report.overall_status, "PASS_SYNTHETIC_CONTRACT");
        assert_eq!(report.claim_ceiling, "synthetic_only");
        assert_eq!(report.source_revision, "test-source-revision");
        assert_eq!(report.gross_circularity_policy_id, "high-loop");
        assert_eq!(report.expected_impact_policy_id, "high-loop");
        assert_eq!(report.minimax_regret_policy_id, "verified-reuse");
        assert_eq!(result(&report, "high-loop").expected_ghg_gco2e, Some(40_776));
        assert_eq!(result(&report, "verified-reuse").expected_ghg_gco2e, Some(41_361));
        assert_eq!(result(&report, "verified-reuse").max_regret_gco2e, Some(8_490));
        assert_eq!(result(&report, "high-loop").max_regret_gco2e, Some(9_800));
        assert_eq!(report.expected_ghg_cost_frontier_ids, vec!["high-loop", "verified-reuse"]);
    }

    #[test]
    fn unknown_evidence_cannot_enter_the_policy_set_or_claim_a_score() {
        let report = fixture_report();
        let item = result(&report, "unverified-loop");
        assert!(!item.eligible);
        assert_eq!(item.eligibility_reason.as_deref(), Some("evidence_status_not_valid"));
        assert_eq!(item.gross_circularity_score_g_per_unit, None);
        assert_eq!(item.expected_ghg_gco2e, None);
        assert!(item.scenarios.is_empty());
    }

    #[test]
    fn gross_circularity_score_cannot_exceed_virgin_material_demand() {
        let mut manifest = fixture_manifest();
        let high_loop = manifest.options.iter_mut()
            .find(|candidate| candidate.id == "high-loop")
            .expect("high-loop candidate exists");
        high_loop.gross_recovered_g_per_unit = 500;
        let bytes = serde_json::to_vec(&manifest).expect("mutated fixture serializes");
        let report = evaluate(&bytes, "mutated-source").expect("bounded fixture evaluates");
        assert_eq!(result(&report, "high-loop").gross_circularity_score_g_per_unit, Some(100));
        let stable = result(&report, "high-loop").scenarios.iter()
            .find(|outcome| outcome.scenario_id == "stable-supply")
            .expect("stable outcome exists");
        assert_eq!(stable.displaced_virgin_g_per_unit, 100);
    }

    #[test]
    fn displacement_is_capped_by_both_yield_and_scenario_substitution() {
        let report = fixture_report();
        let high_loop = result(&report, "high-loop");
        let stable = high_loop.scenarios.iter().find(|item| item.scenario_id == "stable-supply").unwrap();
        let weak = high_loop.scenarios.iter().find(|item| item.scenario_id == "weak-displacement").unwrap();
        assert_eq!(stable.yield_adjusted_recovered_g_per_unit, 85);
        assert_eq!(stable.displaced_virgin_g_per_unit, 76);
        assert_eq!(weak.displaced_virgin_g_per_unit, 21);
        assert!(stable.displaced_virgin_g_per_unit <= 100);
        assert!(weak.displaced_virgin_g_per_unit <= weak.yield_adjusted_recovered_g_per_unit);
    }

    #[test]
    fn zero_substitution_credit_can_reverse_the_expected_impact_choice() {
        let mut manifest = fixture_manifest();
        let weak = manifest.scenarios.iter_mut()
            .find(|scenario| scenario.id == "weak-displacement")
            .expect("weak scenario exists");
        weak.substitution_cap_bp = 0;
        let bytes = serde_json::to_vec(&manifest).expect("mutated fixture serializes");
        let report = evaluate(&bytes, "mutated-source").expect("mutated scenario remains valid");
        assert_eq!(report.expected_impact_policy_id, "verified-reuse");
        assert_eq!(report.minimax_regret_policy_id, "verified-reuse");
        assert_eq!(result(&report, "high-loop").expected_ghg_gco2e, Some(46_152));
        assert_eq!(result(&report, "verified-reuse").expected_ghg_gco2e, Some(44_541));
    }

    #[test]
    fn partial_or_wrong_scope_evidence_fails_closed() {
        let mut manifest = fixture_manifest();
        let evidence = manifest.evidence.iter_mut()
            .find(|evidence| evidence.id == "ev-baseline")
            .expect("baseline evidence exists");
        evidence.coverage = Coverage::Partial;
        let bytes = serde_json::to_vec(&manifest).expect("mutated fixture serializes");
        let report = evaluate(&bytes, "mutated-source").expect("other candidates remain eligible");
        assert!(!result(&report, "baseline-virgin").eligible);
        assert_eq!(
            result(&report, "baseline-virgin").eligibility_reason.as_deref(),
            Some("evidence_coverage_not_complete")
        );

        let mut scope_manifest = fixture_manifest();
        scope_manifest.evidence.iter_mut()
            .find(|evidence| evidence.id == "ev-baseline")
            .expect("baseline evidence exists")
            .scope_id = "factory:other-line".to_owned();
        let scope_bytes = serde_json::to_vec(&scope_manifest).expect("scope mutation serializes");
        let scope_report = evaluate(&scope_bytes, "scope-mutated").expect("other candidates remain eligible");
        assert_eq!(
            result(&scope_report, "baseline-virgin").eligibility_reason.as_deref(),
            Some("evidence_scope_mismatch")
        );
    }

    #[test]
    fn scenario_probabilities_must_reconcile_exactly() {
        let mut manifest = fixture_manifest();
        manifest.scenarios[0].probability_bp -= 1;
        let bytes = serde_json::to_vec(&manifest).expect("mutated fixture serializes");
        assert!(evaluate(&bytes, "test").unwrap_err().contains("sum to 10000 bp"));
    }

    #[test]
    fn evidence_payload_digest_tampering_rejects_the_manifest() {
        let mut manifest = fixture_manifest();
        manifest.evidence[0].payload.push_str(" tampered");
        let bytes = serde_json::to_vec(&manifest).expect("mutated fixture serializes");
        assert!(evaluate(&bytes, "test").unwrap_err().contains("digest mismatch"));
    }

    #[test]
    fn candidate_and_scenario_input_order_does_not_change_decisions() {
        let mut manifest = fixture_manifest();
        let original = fixture_report();
        manifest.options.reverse();
        manifest.scenarios.reverse();
        let bytes = serde_json::to_vec(&manifest).expect("reordered fixture serializes");
        let reordered = evaluate(&bytes, "different-manifest-order").expect("reordered fixture evaluates");
        assert_eq!(original.gross_circularity_policy_id, reordered.gross_circularity_policy_id);
        assert_eq!(original.expected_impact_policy_id, reordered.expected_impact_policy_id);
        assert_eq!(original.minimax_regret_policy_id, reordered.minimax_regret_policy_id);
        assert_eq!(original.expected_ghg_cost_frontier_ids, reordered.expected_ghg_cost_frontier_ids);
        assert_eq!(
            original.options.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
            reordered.options.iter().map(|item| item.id.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn dominated_candidate_is_not_on_the_expected_ghg_cost_frontier() {
        let report = fixture_report();
        assert!(!result(&report, "remanufacture").in_expected_ghg_cost_frontier);
        assert!(!result(&report, "baseline-virgin").in_expected_ghg_cost_frontier);
        assert!(result(&report, "high-loop").in_expected_ghg_cost_frontier);
        assert!(result(&report, "verified-reuse").in_expected_ghg_cost_frontier);
    }

    #[test]
    fn every_eligible_candidate_scenario_pair_matches_a_separate_reference_formula() {
        let manifest = fixture_manifest();
        let report = fixture_report();
        for candidate in manifest.options.iter().filter(|item| eligibility_reason(item, &manifest).is_none()) {
            for scenario in &manifest.scenarios {
                let actual = report.options.iter()
                    .find(|item| item.id == candidate.id)
                    .unwrap()
                    .scenarios.iter()
                    .find(|item| item.scenario_id == scenario.id)
                    .unwrap();
                let expected = reference_totals(candidate, scenario, &manifest);
                assert_eq!(actual.service_units_after_rebound, expected.0, "{} / {}", candidate.id, scenario.id);
                assert_eq!(actual.yield_adjusted_recovered_g_per_unit, expected.1);
                assert_eq!(actual.displaced_virgin_g_per_unit, expected.2);
                assert_eq!(actual.total_ghg_gco2e, expected.3);
                assert_eq!(actual.total_cost_cents, expected.4);
            }
        }
    }

    #[test]
    fn selected_policies_satisfy_their_declared_objectives() {
        let report = fixture_report();
        let expected = result(&report, &report.expected_impact_policy_id);
        let robust = result(&report, &report.minimax_regret_policy_id);
        let gross = result(&report, &report.gross_circularity_policy_id);
        for item in report.options.iter().filter(|item| item.eligible) {
            assert!(
                expected.expected_ghg_gco2e <= item.expected_ghg_gco2e,
                "expected-impact policy must minimize expected GHG"
            );
            assert!(
                robust.max_regret_gco2e <= item.max_regret_gco2e,
                "minimax policy must minimize maximum regret"
            );
            for outcome in &item.scenarios {
                assert!(outcome.regret_gco2e.is_some_and(|regret| regret <= outcome.total_ghg_gco2e));
                assert!(outcome.total_ghg_gco2e >= report.scenario_oracles
                    .iter()
                    .find(|oracle| oracle.scenario_id == outcome.scenario_id)
                    .expect("scenario oracle exists")
                    .total_ghg_gco2e);
            }
        }
        assert!(robust.max_regret_gco2e <= gross.max_regret_gco2e);
    }

    #[test]
    fn identical_manifest_and_source_revision_emit_identical_receipts() {
        let input = include_bytes!("../fixtures/net-additionality-v0-1.json");
        let first = evaluate(input, "frozen-revision").expect("first evaluation succeeds");
        let second = evaluate(input, "frozen-revision").expect("second evaluation succeeds");
        let first_bytes = serde_json::to_vec(&first).expect("first receipt serializes");
        let second_bytes = serde_json::to_vec(&second).expect("second receipt serializes");
        assert_eq!(first_bytes, second_bytes);
    }

    #[test]
    fn arithmetic_overflow_is_rejected_without_wrapping() {
        let mut manifest = fixture_manifest();
        manifest.demand_units = u64::MAX;
        let bytes = serde_json::to_vec(&manifest).expect("overflow fixture serializes");
        assert!(evaluate(&bytes, "overflow-test").is_err());
    }

    fn reference_totals(candidate: &Candidate, scenario: &Scenario, manifest: &Manifest) -> (u64, u64, u64, u64, u64) {
        // Deliberately expressed as direct integer equations rather than calling production helpers.
        let bp = 10_000_u64;
        let recovered = candidate.gross_recovered_g_per_unit * u64::from(candidate.recovery_yield_bp) / bp;
        let substitution = candidate.claimed_substitution_bp.min(scenario.substitution_cap_bp);
        let displaced = (recovered * u64::from(substitution) / bp).min(manifest.virgin_material_g_per_unit);
        let rebound_bp = (u64::from(candidate.rebound_sensitivity_bp)
            * u64::from(scenario.rebound_multiplier_bp) + bp - 1) / bp;
        let extra_units = (manifest.demand_units * rebound_bp + bp - 1) / bp;
        let units = manifest.demand_units + extra_units;
        let virgin_base = (manifest.virgin_material_g_per_unit - displaced)
            * manifest.embodied_ghg_gco2e_per_g;
        let virgin_per_unit = (virgin_base * u64::from(scenario.virgin_ghg_multiplier_bp) + bp - 1) / bp;
        let process_per_unit = (candidate.process_ghg_gco2e_per_unit
            * u64::from(scenario.process_ghg_multiplier_bp) + bp - 1) / bp;
        (units, recovered, displaced, units * (virgin_per_unit + process_per_unit),
            units * candidate.cost_cents_per_unit)
    }
}
