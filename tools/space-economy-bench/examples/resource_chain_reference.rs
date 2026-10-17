use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::process::ExitCode;

const SITE_ENERGY_BUDGET_KWH: u64 = 5_000;
const REGOLITH_BATCH_KG: u64 = 1_000;
const REGOLITH_OXYGEN_INVENTORY_KG: u64 = 400;
const REGOLITH_OXYGEN_RECOVERY_BPS: u64 = 5_000;
const WATER_ICE_BATCH_KG: u64 = 100;
const WATER_FRACTION_BPS: u64 = 8_000;
const WATER_EXTRACTION_RECOVERY_BPS: u64 = 7_500;
const WATER_PURIFICATION_RECOVERY_BPS: u64 = 9_000;

#[derive(Clone, Debug)]
struct Stream {
    stream_id: &'static str,
    mass_kg: u64,
}

#[derive(Clone, Debug)]
struct Pathway {
    pathway_id: &'static str,
    allocation_group: &'static str,
    feedstock_id: &'static str,
    feed_mass_kg: u64,
    evidence_class: &'static str,
    feed_batch_verified_for_scenario: bool,
    process_qualified_for_declared_product_in_fixture: bool,
    product_quality_verified_for_declared_use_in_fixture: bool,
    energy_demand_kwh: u64,
    energy_budget_kwh: u64,
    products: Vec<Stream>,
    residuals: Vec<Stream>,
    intermediates_consumed_kg: Vec<Stream>,
    uncredited_coproducts: &'static [&'static str],
    next_evidence_needed: &'static [&'static str],
}

impl Pathway {
    fn product_mass_kg(&self) -> Result<u64, String> {
        self.products.iter().try_fold(0_u64, |total, stream| {
            total.checked_add(stream.mass_kg)
                .ok_or_else(|| format!("{}: product mass overflow", self.pathway_id))
        })
    }

    fn residual_mass_kg(&self) -> Result<u64, String> {
        self.residuals.iter().try_fold(0_u64, |total, stream| {
            total.checked_add(stream.mass_kg)
                .ok_or_else(|| format!("{}: residual mass overflow", self.pathway_id))
        })
    }

    fn mass_balance_delta_kg(&self) -> Result<i64, String> {
        let accounted = self.product_mass_kg()?.checked_add(self.residual_mass_kg()?)
            .ok_or_else(|| format!("{}: total output mass overflow", self.pathway_id))?;
        let delta = i128::from(self.feed_mass_kg) - i128::from(accounted);
        i64::try_from(delta).map_err(|_| format!("{}: mass delta out of range", self.pathway_id))
    }

    fn blockers(&self) -> Result<Vec<&'static str>, String> {
        let mut blockers = Vec::new();
        if !self.feed_batch_verified_for_scenario {
            blockers.push("RESOURCE_FEED_NOT_SITE_VERIFIED");
        }
        if !self.process_qualified_for_declared_product_in_fixture {
            blockers.push("PROCESS_NOT_QUALIFIED");
        }
        if !self.product_quality_verified_for_declared_use_in_fixture {
            blockers.push("PRODUCT_SPEC_NOT_QUALIFIED");
        }
        if self.energy_demand_kwh > self.energy_budget_kwh {
            blockers.push("ENERGY_BUDGET_EXCEEDED");
        }
        if self.mass_balance_delta_kg()? != 0 {
            blockers.push("MASS_BALANCE_FAILED");
        }
        Ok(blockers)
    }

    fn firm_product_mass_in_fixture_kg(&self) -> Result<u64, String> {
        if self.blockers()?.is_empty() {
            self.product_mass_kg()
        } else {
            Ok(0)
        }
    }
}

fn stream(stream_id: &'static str, mass_kg: u64) -> Stream {
    Stream { stream_id, mass_kg }
}

fn build_pathways() -> Result<Vec<Pathway>, String> {
    // The fractions and batch sizes below are deliberately synthetic scenario
    // inputs; they are not asserted lunar grade estimates or resource reserves.
    let regolith_oxygen_kg =
        REGOLITH_OXYGEN_INVENTORY_KG * REGOLITH_OXYGEN_RECOVERY_BPS / 10_000;
    let regolith_oxygen_residue_kg = REGOLITH_BATCH_KG - regolith_oxygen_kg;

    let usable_aggregate_kg = REGOLITH_BATCH_KG * 9_000 / 10_000;
    let aggregate_rejects_kg = REGOLITH_BATCH_KG - usable_aggregate_kg;

    let water_in_feed_kg = WATER_ICE_BATCH_KG * WATER_FRACTION_BPS / 10_000;
    let other_regolith_kg = WATER_ICE_BATCH_KG - water_in_feed_kg;
    let extracted_crude_water_kg =
        water_in_feed_kg * WATER_EXTRACTION_RECOVERY_BPS / 10_000;
    let unrecovered_water_kg = water_in_feed_kg - extracted_crude_water_kg;
    let purified_water_kg =
        extracted_crude_water_kg * WATER_PURIFICATION_RECOVERY_BPS / 10_000;
    let purification_reject_water_kg = extracted_crude_water_kg - purified_water_kg;

    // 2 H2O -> 2 H2 + O2: on a mass basis, 54 kg H2O yields 48 kg O2
    // and 6 kg H2. The purified water is an intermediate in this branch, not
    // an additional saleable output.
    if purified_water_kg % 9 != 0 {
        return Err("purified water must divide exactly into 9 mass parts in this fixture".to_owned());
    }
    let oxygen_from_water_kg = purified_water_kg * 8 / 9;
    let hydrogen_from_water_kg = purified_water_kg / 9;
    let paths = vec![
        Pathway {
            pathway_id: "regolith_to_bulk_shielding_aggregate",
            allocation_group: "regolith_batch_001",
            feedstock_id: "lunar_regolith",
            feed_mass_kg: REGOLITH_BATCH_KG,
            evidence_class: "synthetic_fixture_site_batch",
            feed_batch_verified_for_scenario: true,
            process_qualified_for_declared_product_in_fixture: true,
            product_quality_verified_for_declared_use_in_fixture: true,
            energy_demand_kwh: 1_000,
            energy_budget_kwh: SITE_ENERGY_BUDGET_KWH,
            products: vec![stream("bulk_shielding_aggregate", usable_aggregate_kg)],
            residuals: vec![stream("screening_rejects", aggregate_rejects_kg)],
            intermediates_consumed_kg: vec![],
            uncredited_coproducts: &["none declared"],
            next_evidence_needed: &["real site-specific geotechnical characterization", "dust-wear lifetime and equipment throughput under lunar conditions"],
        },
        Pathway {
            pathway_id: "regolith_to_oxygen",
            allocation_group: "regolith_batch_002",
            feedstock_id: "lunar_regolith",
            feed_mass_kg: REGOLITH_BATCH_KG,
            evidence_class: "synthetic_composition_and_yield_assumption",
            feed_batch_verified_for_scenario: true,
            process_qualified_for_declared_product_in_fixture: false,
            product_quality_verified_for_declared_use_in_fixture: false,
            energy_demand_kwh: 6_000,
            energy_budget_kwh: SITE_ENERGY_BUDGET_KWH,
            products: vec![stream("oxygen", regolith_oxygen_kg)],
            residuals: vec![stream("oxide_rich_residue_and_unrecovered_oxygen", regolith_oxygen_residue_kg)],
            intermediates_consumed_kg: vec![],
            uncredited_coproducts: &["iron-bearing stream", "aluminium-bearing stream", "titanium-bearing stream", "silicon-bearing stream"],
            next_evidence_needed: &["measured site mineralogy and oxygen-bearing phase distribution", "qualified long-duration extraction yield", "measured energy per kilogram of qualified oxygen", "downstream metal separation and product-purity evidence"],
        },
        Pathway {
            pathway_id: "polar_volatiles_to_purified_water",
            allocation_group: "polar_volatile_batch_001",
            feedstock_id: "polar_volatile_bearing_material",
            feed_mass_kg: WATER_ICE_BATCH_KG,
            evidence_class: "inferred_occurrence_not_site_verified_reserve",
            feed_batch_verified_for_scenario: false,
            process_qualified_for_declared_product_in_fixture: false,
            product_quality_verified_for_declared_use_in_fixture: false,
            energy_demand_kwh: 800,
            energy_budget_kwh: SITE_ENERGY_BUDGET_KWH,
            products: vec![stream("purified_water", purified_water_kg)],
            residuals: vec![
                stream("non_water_regolith_contaminants", other_regolith_kg),
                stream("unrecovered_water", unrecovered_water_kg),
                stream("purification_reject_water", purification_reject_water_kg),
            ],
            intermediates_consumed_kg: vec![],
            uncredited_coproducts: &["no metal or volatile coproduct credited"],
            next_evidence_needed: &["site-specific concentration and depth measurements", "excavation and thermal-extraction tests on site-representative material", "purity and recovery measurement", "long-duration energy and wear-part data"],
        },
        Pathway {
            pathway_id: "polar_volatiles_to_electrolysis_products",
            allocation_group: "polar_volatile_batch_001",
            feedstock_id: "polar_volatile_bearing_material",
            feed_mass_kg: WATER_ICE_BATCH_KG,
            evidence_class: "inferred_occurrence_not_site_verified_reserve",
            feed_batch_verified_for_scenario: false,
            process_qualified_for_declared_product_in_fixture: false,
            product_quality_verified_for_declared_use_in_fixture: false,
            energy_demand_kwh: 1_300,
            energy_budget_kwh: SITE_ENERGY_BUDGET_KWH,
            products: vec![
                stream("oxygen_from_water_electrolysis", oxygen_from_water_kg),
                stream("hydrogen_from_water_electrolysis", hydrogen_from_water_kg),
            ],
            residuals: vec![
                stream("non_water_regolith_contaminants", other_regolith_kg),
                stream("unrecovered_water", unrecovered_water_kg),
                stream("purification_reject_water", purification_reject_water_kg),
            ],
            intermediates_consumed_kg: vec![stream("purified_water_consumed_by_electrolysis", purified_water_kg)],
            uncredited_coproducts: &["no cryogenic propellant inventory credited without storage, conditioning, and delivery qualification"],
            next_evidence_needed: &["all water-path prospecting and process gates", "electrolyzer durability and purity data", "hydrogen/oxygen storage and boil-off performance", "end-to-end delivery/use demonstration"],
        },
    ];
    Ok(paths)
}

fn validate_pathway(path: &Pathway) -> Result<(), String> {
    if path.pathway_id.is_empty()
        || path.feedstock_id.is_empty()
        || path.feed_mass_kg == 0
        || path.energy_budget_kwh == 0
    {
        return Err("pathway ID, feedstock, positive feed mass and positive energy budget are required".to_owned());
    }
    let mut stream_ids = BTreeSet::new();
    for item in path.products.iter().chain(path.residuals.iter()) {
        if item.stream_id.is_empty() || !stream_ids.insert(item.stream_id) {
            return Err(format!("{}: empty or duplicate final stream ID", path.pathway_id));
        }
    }
    if path.mass_balance_delta_kg()? != 0 {
        return Err(format!("{}: feed mass does not equal final products plus residuals", path.pathway_id));
    }
    Ok(())
}

fn pathway_json(path: &Pathway) -> Result<Value, String> {
    validate_pathway(path)?;
    let blockers = path.blockers()?;
    let product_mass = path.product_mass_kg()?;
    let residual_mass = path.residual_mass_kg()?;
    Ok(json!({
        "pathway_id": path.pathway_id,
        "allocation_group": path.allocation_group,
        "feedstock_id": path.feedstock_id,
        "feed_mass_kg": path.feed_mass_kg,
        "resource_evidence_class": path.evidence_class,
        "feed_batch_verified_for_scenario": path.feed_batch_verified_for_scenario,
        "process_qualified_for_declared_product_in_fixture": path.process_qualified_for_declared_product_in_fixture,
        "product_quality_verified_for_declared_use_in_fixture": path.product_quality_verified_for_declared_use_in_fixture,
        "energy_demand_kwh_synthetic": path.energy_demand_kwh,
        "energy_budget_kwh_synthetic": path.energy_budget_kwh,
        "energy_gate_pass": path.energy_demand_kwh <= path.energy_budget_kwh,
        "product_streams": path.products.iter().map(|s| json!({"stream_id":s.stream_id,"mass_kg":s.mass_kg})).collect::<Vec<_>>(),
        "residual_streams": path.residuals.iter().map(|s| json!({"stream_id":s.stream_id,"mass_kg":s.mass_kg})).collect::<Vec<_>>(),
        "intermediates_consumed_not_final_products": path.intermediates_consumed_kg.iter().map(|s| json!({"stream_id":s.stream_id,"mass_kg":s.mass_kg})).collect::<Vec<_>>(),
        "conditional_product_mass_kg": product_mass,
        "residual_mass_kg": residual_mass,
        "mass_balance_delta_kg": path.mass_balance_delta_kg()?,
        "mass_balance_pass": path.mass_balance_delta_kg()? == 0,
        "firm_product_mass_in_synthetic_fixture_kg": path.firm_product_mass_in_fixture_kg()?,
        "blockers": blockers,
        "uncredited_coproducts": path.uncredited_coproducts,
        "next_evidence_needed": path.next_evidence_needed,
    }))
}

fn find_pathway<'a>(paths: &'a [Pathway], pathway_id: &str) -> Result<&'a Pathway, String> {
    paths.iter()
        .find(|path| path.pathway_id == pathway_id)
        .ok_or_else(|| format!("missing pathway {pathway_id}"))
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if source_commit.len() != 40
        || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }
    let paths = build_pathways()?;
    let mut ids = BTreeSet::new();
    for path in &paths {
        if !ids.insert(path.pathway_id) {
            return Err(format!("duplicate pathway ID {}", path.pathway_id));
        }
        validate_pathway(path)?;
    }

    let shielding = find_pathway(&paths, "regolith_to_bulk_shielding_aggregate")?;
    let oxygen = find_pathway(&paths, "regolith_to_oxygen")?;
    let water = find_pathway(&paths, "polar_volatiles_to_purified_water")?;
    let electrolysis = find_pathway(&paths, "polar_volatiles_to_electrolysis_products")?;

    if shielding.product_mass_kg()? != 900
        || shielding.firm_product_mass_in_fixture_kg()? != 900
        || shielding.energy_demand_kwh > SITE_ENERGY_BUDGET_KWH
    {
        return Err("bulk shielding aggregate oracle mismatch".to_owned());
    }
    if oxygen.product_mass_kg()? != 200
        || oxygen.firm_product_mass_in_fixture_kg()? != 0
        || oxygen.energy_demand_kwh <= SITE_ENERGY_BUDGET_KWH
    {
        return Err("regolith oxygen gate oracle mismatch".to_owned());
    }
    if water.product_mass_kg()? != 54
        || water.firm_product_mass_in_fixture_kg()? != 0
        || electrolysis.product_mass_kg()? != 54
        || electrolysis.firm_product_mass_in_fixture_kg()? != 0
    {
        return Err("water-chain or alternative-allocation oracle mismatch".to_owned());
    }
    if water.allocation_group != electrolysis.allocation_group
        || water.feed_mass_kg != electrolysis.feed_mass_kg
    {
        return Err("water allocation alternatives must refer to the same feed batch".to_owned());
    }
    if electrolysis.products[0].mass_kg != 48 || electrolysis.products[1].mass_kg != 6
        || electrolysis.residual_mass_kg()? != 46
    {
        return Err("water electrolysis stoichiometric oracle mismatch".to_owned());
    }
    if !oxygen.uncredited_coproducts.iter().any(|x| x.contains("iron"))
        || !oxygen.uncredited_coproducts.iter().any(|x| x.contains("silicon"))
    {
        return Err("potential metal streams must remain explicitly uncredited".to_owned());
    }

    let source_digest =
        format!("{:x}", Sha256::digest(include_bytes!("resource_chain_reference.rs")));
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "resource-to-production-chain-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_mass_balance_and_evidence_gate_contract",
        "site_energy_budget_kwh_synthetic": SITE_ENERGY_BUDGET_KWH,
        "mass_balance_contract": "For each mutually exclusive route, feed mass equals final product streams plus residual streams; consumed intermediates are reported separately and are not double counted.",
        "firm_supply_contract": "Only fixture pathways with scenario-verified feed batch, process qualification, declared-use product quality, in-budget energy and exact mass balance contribute firm product mass. These flags are synthetic test inputs, not real lunar qualification.",
        "shared_feed_allocation_rule": "Pathways with the same allocation_group are alternatives over the same batch. Their production quantities must not be summed.",
        "pathways": paths.iter().map(pathway_json).collect::<Result<Vec<_>, _>>()?,
        "claim_ceiling": "Synthetic batch arithmetic only. These outputs are not lunar resource reserves, measured grades, real energy intensities, economically recoverable deposits, qualified propellants or mission-ready production systems."
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example resource_chain_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    println!("{}", serde_json::to_string_pretty(&execute(&args[2])?).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("resource_chain_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_conserves_feed_mass_exactly() {
        for path in build_pathways().unwrap() {
            assert_eq!(path.mass_balance_delta_kg().unwrap(), 0, "{}", path.pathway_id);
        }
    }

    #[test]
    fn bulk_shielding_is_the_fixture_positive_control() {
        let paths = build_pathways().unwrap();
        let path = paths.iter().find(|p| p.pathway_id == "regolith_to_bulk_shielding_aggregate").unwrap();
        assert_eq!(path.product_mass_kg().unwrap(), 900);
        assert_eq!(path.residual_mass_kg().unwrap(), 100);
        assert_eq!(path.firm_product_mass_in_fixture_kg().unwrap(), 900);
    }

    #[test]
    fn regolith_oxygen_is_conditional_when_energy_and_process_gates_fail() {
        let paths = build_pathways().unwrap();
        let path = paths.iter().find(|p| p.pathway_id == "regolith_to_oxygen").unwrap();
        assert_eq!(path.product_mass_kg().unwrap(), 200);
        assert_eq!(path.firm_product_mass_in_fixture_kg().unwrap(), 0);
        assert!(path.blockers().unwrap().contains(&"ENERGY_BUDGET_EXCEEDED"));
        assert!(path.blockers().unwrap().contains(&"PROCESS_NOT_QUALIFIED"));
    }

    #[test]
    fn inferred_polar_volatiles_do_not_count_as_firm_feedstock() {
        let paths = build_pathways().unwrap();
        for path in paths.iter().filter(|p| p.allocation_group == "polar_volatile_batch_001") {
            assert_eq!(path.firm_product_mass_in_fixture_kg().unwrap(), 0);
            assert!(path.blockers().unwrap().contains(&"RESOURCE_FEED_NOT_SITE_VERIFIED"));
        }
    }

    #[test]
    fn water_to_water_and_water_to_electrolysis_are_alternatives_not_additive() {
        let paths = build_pathways().unwrap();
        let water = paths.iter().find(|p| p.pathway_id == "polar_volatiles_to_purified_water").unwrap();
        let electrolysis = paths.iter().find(|p| p.pathway_id == "polar_volatiles_to_electrolysis_products").unwrap();
        assert_eq!(water.allocation_group, electrolysis.allocation_group);
        assert_eq!(water.feed_mass_kg, electrolysis.feed_mass_kg);
        assert_eq!(water.product_mass_kg().unwrap(), 54);
        assert_eq!(electrolysis.product_mass_kg().unwrap(), 54);
        assert_eq!(electrolysis.products[0].mass_kg, 48);
        assert_eq!(electrolysis.products[1].mass_kg, 6);
        assert_eq!(electrolysis.intermediates_consumed_kg[0].mass_kg, 54);
    }

    #[test]
    fn metal_bearing_residues_are_not_counted_as_refined_metal_products() {
        let paths = build_pathways().unwrap();
        let oxygen = paths.iter().find(|p| p.pathway_id == "regolith_to_oxygen").unwrap();
        assert!(oxygen.products.iter().all(|p| !p.stream_id.contains("iron") && !p.stream_id.contains("aluminium") && !p.stream_id.contains("titanium") && !p.stream_id.contains("silicon")));
        assert_eq!(oxygen.uncredited_coproducts.len(), 4);
    }

    #[test]
    fn duplicate_final_stream_ids_are_rejected() {
        let mut path = build_pathways().unwrap().remove(0);
        path.products.push(stream("bulk_shielding_aggregate", 1));
        assert!(validate_pathway(&path).is_err());
    }

    #[test]
    fn receipt_binds_source_and_contains_four_routes() {
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["pathways"].as_array().unwrap().len(), 4);
        assert_eq!(receipt["reference_source_sha256"].as_str().unwrap().len(), 64);
    }
}
