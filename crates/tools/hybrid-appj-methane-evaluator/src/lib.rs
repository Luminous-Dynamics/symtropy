use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const SCHEMA_VERSION: &str = "luminous.hybrid-appj-methane-process.v0";
const BENCHMARK_ID: &str = "CH4-PLASMA-THERMAL-APPJ-TWIN-001";
const REL_TOLERANCE: f64 = 1.0e-9;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    schema_version: String,
    benchmark_id: String,
    evidence_class: String,
    luminous_reproduction: bool,
    source: Source,
    source_configurations: Vec<String>,
    factorial_design: FactorialDesign,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    doi: String,
    publication_date: String,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactorialDesign {
    design_class: String,
    factors: Vec<String>,
    cells: Vec<FactorialCell>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactorialCell {
    plasma_enabled: bool,
    thermal_zone_enabled: bool,
    measurement_status: String,
    observed_absolute_conversion_fraction: Option<f64>,
    observed_accepted_h2_mass_g: Option<f64>,
    raw_artifact_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    id: String,
    kind: String,
    source_claim: Option<SourceClaim>,
    product_evidence: Option<ProductEvidence>,
    comparison: Option<Comparison>,
    factorial_design: Option<FactorialDesign>,
    expected_disposition: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceClaim {
    relative_conversion_gain_lower_bound_fold: f64,
    absolute_conversion_fraction: Option<f64>,
    carbon_deposition_fraction_approx: f64,
    carbon_deposition_denominator_status: String,
    captured_carbon_mass_g: Option<f64>,
    feed_carbon_recovery_fraction: Option<f64>,
    carbon_product_grade_status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductEvidence {
    species: String,
    accepted_mass_g: Option<f64>,
    rate_g_per_hour: Option<f64>,
    purity_fraction: Option<f64>,
    pressure_kpa: Option<f64>,
    calibration_ref: Option<String>,
    quantitative_measurement_ref: Option<String>,
    raw_artifact_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Comparison {
    candidate_product_contract_id: String,
    reference_product_contract_id: String,
    candidate_energy_boundary_id: String,
    reference_energy_boundary_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResult {
    pub id: String,
    pub disposition: String,
    pub expected_disposition: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    pub benchmark_id: String,
    pub evidence_class: String,
    pub luminous_reproduction: bool,
    pub scenario_count: usize,
    pub matched_expectations: usize,
    pub mismatched_expectations: usize,
    pub factorial_design_disposition: String,
    pub results: Vec<ScenarioResult>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn close(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= REL_TOLERANCE * a.abs().max(b.abs()).max(1.0)
}

fn validate_factorial_design(design: &FactorialDesign) -> Result<(), String> {
    if design.design_class != "PROPOSED_DESIGN_NOT_SOURCE_EXPERIMENT" {
        return Err(
            "factorial design must be explicitly marked proposed, not source experiment".to_owned(),
        );
    }
    if design.factors != ["plasma_enabled", "thermal_zone_enabled"] {
        return Err("factorial design factors must be the declared plasma/thermal 2x2".to_owned());
    }
    let mut cells = BTreeSet::new();
    for cell in &design.cells {
        if !cells.insert((cell.plasma_enabled, cell.thermal_zone_enabled)) {
            return Err("factorial design contains a duplicate treatment cell".to_owned());
        }
        if cell.measurement_status != "NOT_MEASURED" {
            return Err(
                "synthetic design fixture cannot claim measured factorial results".to_owned(),
            );
        }
        if cell.observed_absolute_conversion_fraction.is_some()
            || cell.observed_accepted_h2_mass_g.is_some()
            || cell.raw_artifact_ref.is_some()
        {
            return Err(
                "proposed factorial design must not contain invented physical observations"
                    .to_owned(),
            );
        }
    }
    if cells.len() != 4
        || !cells.contains(&(false, false))
        || !cells.contains(&(true, false))
        || !cells.contains(&(false, true))
        || !cells.contains(&(true, true))
    {
        return Err(
            "factorial design must contain all four plasma/thermal treatment cells".to_owned(),
        );
    }
    Ok(())
}

fn evaluate_scenario(scenario: &Scenario) -> ScenarioResult {
    let mut rejected = Vec::new();
    let mut blocked = Vec::new();
    let mut not_comparable = Vec::new();
    let mut design_only = Vec::new();

    if !nonempty(&scenario.id) {
        rejected.push("scenario identity is empty".to_owned());
    }

    match scenario.kind.as_str() {
        "SOURCE_CLAIM" => {
            let Some(claim) = &scenario.source_claim else {
                blocked.push("source claim fields are absent".to_owned());
                return result(scenario, rejected, blocked, not_comparable, design_only);
            };
            if !close(claim.relative_conversion_gain_lower_bound_fold, 5.0) {
                rejected.push(
                    "source-relative conversion claim must remain a >5-fold lower bound".to_owned(),
                );
            }
            if claim.absolute_conversion_fraction.is_some() {
                rejected.push("relative conversion gain cannot be promoted to an absolute conversion fraction".to_owned());
            }
            if !close(claim.carbon_deposition_fraction_approx, 0.36) {
                rejected.push(
                    "source-reported carbon deposition must remain approximately 36%".to_owned(),
                );
            }
            if claim.carbon_deposition_denominator_status != "RECONSTRUCT_FROM_SOURCE_DATA" {
                blocked.push(
                    "carbon-deposition denominator remains unresolved until source reconstruction"
                        .to_owned(),
                );
            }
            if claim.captured_carbon_mass_g.is_some()
                || claim.feed_carbon_recovery_fraction.is_some()
            {
                rejected.push(
                    "carbon deposition cannot be promoted to captured/recovered feed carbon"
                        .to_owned(),
                );
            }
            if claim.carbon_product_grade_status != "NOT_VALIDATED" {
                rejected.push(
                    "turbostratic carbon characterization is not product-grade qualification"
                        .to_owned(),
                );
            }
        }
        "PRODUCT_CLAIM" => {
            let Some(product) = &scenario.product_evidence else {
                blocked.push("product evidence is absent".to_owned());
                return result(scenario, rejected, blocked, not_comparable, design_only);
            };
            if !matches!(product.species.as_str(), "H2" | "C2-C3" | "SOLID_CARBON") {
                rejected
                    .push("product species is outside the declared multi-product slate".to_owned());
            }
            let any_quantitative = product.accepted_mass_g.is_some()
                || product.rate_g_per_hour.is_some()
                || product.purity_fraction.is_some()
                || product.pressure_kpa.is_some();
            if any_quantitative {
                let has_refs = product.calibration_ref.as_deref().is_some_and(nonempty)
                    && product
                        .quantitative_measurement_ref
                        .as_deref()
                        .is_some_and(nonempty)
                    && product.raw_artifact_ref.as_deref().is_some_and(nonempty);
                if !has_refs {
                    blocked.push("quantitative product claim lacks calibration, measurement and raw-artifact lineage".to_owned());
                }
            } else if product.calibration_ref.is_some()
                || product.quantitative_measurement_ref.is_some()
                || product.raw_artifact_ref.is_some()
            {
                blocked.push("artifact identifiers without a quantitative product measurand are not a product measurement".to_owned());
            } else {
                design_only.push(
                    "product remains source-described but not quantitatively accepted".to_owned(),
                );
            }
            if product.species == "SOLID_CARBON"
                && (product.accepted_mass_g.is_some() || product.purity_fraction.is_some())
                && product
                    .calibration_ref
                    .as_deref()
                    .is_none_or(|v| !nonempty(v))
            {
                blocked
                    .push("solid carbon requires a qualified mass/grade evidence chain".to_owned());
            }
        }
        "COMPARISON" => {
            let Some(comparison) = &scenario.comparison else {
                blocked.push("comparison contract is absent".to_owned());
                return result(scenario, rejected, blocked, not_comparable, design_only);
            };
            if [
                &comparison.candidate_product_contract_id,
                &comparison.reference_product_contract_id,
                &comparison.candidate_energy_boundary_id,
                &comparison.reference_energy_boundary_id,
            ]
            .iter()
            .any(|v| !nonempty(v))
            {
                blocked.push(
                    "comparison product or energy boundary identity is incomplete".to_owned(),
                );
            } else if comparison.candidate_product_contract_id
                != comparison.reference_product_contract_id
                || comparison.candidate_energy_boundary_id
                    != comparison.reference_energy_boundary_id
            {
                not_comparable
                    .push("candidate and reference product/boundary contracts differ".to_owned());
            } else {
                design_only.push(
                    "matched identifiers alone do not prove comparable measured performance"
                        .to_owned(),
                );
            }
        }
        "FACTORIAL_DESIGN" => {
            let Some(design) = &scenario.factorial_design else {
                blocked.push("factorial design is absent".to_owned());
                return result(scenario, rejected, blocked, not_comparable, design_only);
            };
            if let Err(reason) = validate_factorial_design(design) {
                rejected.push(reason);
            } else {
                design_only.push("2x2 plasma/thermal controls are a proposed design, not source-reported measurements".to_owned());
            }
        }
        _ => rejected.push("unsupported scenario kind".to_owned()),
    }

    result(scenario, rejected, blocked, not_comparable, design_only)
}

fn result(
    scenario: &Scenario,
    rejected: Vec<String>,
    blocked: Vec<String>,
    not_comparable: Vec<String>,
    design_only: Vec<String>,
) -> ScenarioResult {
    let (disposition, reasons) = if !rejected.is_empty() {
        ("REJECTED".to_owned(), rejected)
    } else if !blocked.is_empty() {
        ("BLOCKED".to_owned(), blocked)
    } else if !not_comparable.is_empty() {
        ("NOT_COMPARABLE".to_owned(), not_comparable)
    } else if !design_only.is_empty() {
        ("DESIGN_ONLY".to_owned(), design_only)
    } else {
        ("SOURCE_CLAIM_ONLY".to_owned(), Vec::new())
    };
    ScenarioResult {
        id: scenario.id.clone(),
        disposition,
        expected_disposition: scenario.expected_disposition.clone(),
        reasons,
    }
}

pub fn audit_manifest_json(source: &str) -> Result<AuditReport, String> {
    let manifest: Manifest =
        serde_json::from_str(source).map_err(|error| format!("invalid manifest: {error}"))?;

    if manifest.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version: {}",
            manifest.schema_version
        ));
    }
    if manifest.benchmark_id != BENCHMARK_ID {
        return Err(format!(
            "unexpected benchmark ID: {}",
            manifest.benchmark_id
        ));
    }
    if manifest.evidence_class != "published_source_register_plus_synthetic_design_controls" {
        return Err("unexpected top-level evidence class".to_owned());
    }
    if manifest.luminous_reproduction {
        return Err("source/synthetic fixture cannot claim Luminous reproduction".to_owned());
    }
    if manifest.source.doi != "10.1016/j.ijhydene.2026.158179"
        || manifest.source.publication_date != "2026-11-16"
        || manifest.source.claim_ceiling != "SOURCE_ONLY_RELATIVE_CLAIMS_NO_LUMINOUS_REPRODUCTION"
    {
        return Err("source identity or claim ceiling mismatch".to_owned());
    }
    let expected_configs = [
        "single-electrode APPJ with plasma outside the heated zone",
        "single-electrode APPJ with plasma directly inside the heated zone",
        "double-electrode APPJ configuration",
    ];
    if manifest.source_configurations.len() != expected_configs.len()
        || expected_configs
            .iter()
            .any(|expected| !manifest.source_configurations.iter().any(|v| v == expected))
    {
        return Err("source configuration identities are incomplete or altered".to_owned());
    }
    validate_factorial_design(&manifest.factorial_design)?;

    if manifest.scenarios.is_empty() {
        return Err("manifest contains no scenarios".to_owned());
    }
    let mut ids = BTreeSet::new();
    for scenario in &manifest.scenarios {
        if !ids.insert(scenario.id.as_str()) {
            return Err("scenario IDs must be unique".to_owned());
        }
        if !matches!(
            scenario.expected_disposition.as_str(),
            "SOURCE_CLAIM_ONLY" | "DESIGN_ONLY" | "BLOCKED" | "REJECTED" | "NOT_COMPARABLE"
        ) {
            return Err(format!(
                "scenario {} has unsupported expected disposition",
                scenario.id
            ));
        }
    }

    let results: Vec<_> = manifest.scenarios.iter().map(evaluate_scenario).collect();
    let matched = results
        .iter()
        .filter(|r| r.disposition == r.expected_disposition)
        .count();
    let total = results.len();
    Ok(AuditReport {
        benchmark_id: manifest.benchmark_id,
        evidence_class: manifest.evidence_class,
        luminous_reproduction: manifest.luminous_reproduction,
        scenario_count: total,
        matched_expectations: matched,
        mismatched_expectations: total - matched,
        factorial_design_disposition: "DESIGN_ONLY".to_owned(),
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../../benchmarks/plasma/appj-methane-process-v0.json");

    fn audit() -> AuditReport {
        audit_manifest_json(FIXTURE).expect("APPJ fixture is syntactically and structurally valid")
    }

    fn scenario<'a>(report: &'a AuditReport, id: &str) -> &'a ScenarioResult {
        report
            .results
            .iter()
            .find(|r| r.id == id)
            .expect("scenario exists")
    }

    #[test]
    fn frozen_fixture_matches_expected_dispositions() {
        let report = audit();
        assert_eq!(report.scenario_count, 8);
        assert_eq!(report.matched_expectations, 8);
        assert_eq!(report.mismatched_expectations, 0);
    }

    #[test]
    fn source_relative_gain_remains_relative_and_carbon_deposition_is_not_recovery() {
        assert_eq!(
            scenario(&audit(), "source-claim-positive").disposition,
            "SOURCE_CLAIM_ONLY"
        );
    }

    #[test]
    fn absolute_conversion_promotion_is_rejected() {
        assert_eq!(
            scenario(&audit(), "reject-absolute-conversion-from-relative-claim").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn carbon_deposition_promotion_to_recovery_is_rejected() {
        assert_eq!(
            scenario(&audit(), "reject-deposition-as-carbon-recovery").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn quantitative_h2_without_calibration_and_measurement_is_blocked() {
        assert_eq!(
            scenario(&audit(), "block-quantitative-h2-without-metrology").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn proposed_factorial_is_design_only_not_published_result() {
        assert_eq!(
            scenario(&audit(), "factorial-controls-design-only").disposition,
            "DESIGN_ONLY"
        );
    }

    #[test]
    fn incomplete_factorial_controls_are_rejected() {
        assert_eq!(
            scenario(&audit(), "reject-incomplete-factorial").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn candidate_and_reference_with_different_boundary_are_not_comparable() {
        assert_eq!(
            scenario(&audit(), "not-comparable-mismatched-system-boundary").disposition,
            "NOT_COMPARABLE"
        );
    }
}
