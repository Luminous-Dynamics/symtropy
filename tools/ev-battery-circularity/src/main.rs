use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const MASS_TOLERANCE_KG: f64 = 1.0e-6;
const CLAIM_CEILING: &str = "Declared-input lifecycle arithmetic and material-flow screen only. Inputs must retain evidence provenance and boundary assumptions. Outputs are not a full LCA, battery aging model, safety qualification, certification, or manufacturing recommendation.";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: String,
    suite_id: String,
    functional_unit: String,
    life_cycle_boundary: String,
    claim_ceiling: String,
    designs: Vec<Design>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Design {
    design_id: String,
    label: String,
    architecture: String,
    configuration_id: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    nominal_capacity_kwh: f64,
    usable_capacity_fraction: f64,
    pack_mass_kg: f64,
    initial_pack_cost_usd: f64,
    initial_pack_embodied_kgco2e: f64,
    lifetime_delivered_energy_kwh: f64,
    use_phase_grid_intensity_kgco2e_per_kwh: f64,
    electricity_price_usd_per_kwh: f64,
    labor_rate_usd_per_hour: f64,
    downtime_cost_usd_per_hour: f64,
    serviceable_unit_count: usize,
    smallest_replaceable_unit_kwh: f64,
    maintenance: Maintenance,
    operations: OperationProfile,
    materials: Vec<MaterialFlow>,
    qualification_gates: Vec<QualificationGate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Maintenance {
    repair_events_over_life: u64,
    whole_pack_replacements_over_life: u64,
    repair_unit_cost_usd: f64,
    repair_unit_embodied_kgco2e: f64,
    replacement_pack_cost_usd: f64,
    replacement_pack_embodied_kgco2e: f64,
    parts_wait_hours_per_repair: f64,
    parts_wait_hours_per_pack_replacement: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationProfile {
    isolation_minutes: f64,
    diagnosis_minutes: f64,
    enclosure_access_minutes: f64,
    replace_unit_minutes: f64,
    reassembly_minutes: f64,
    post_service_validation_minutes: f64,
    pack_removal_minutes: f64,
    pack_installation_minutes: f64,
    post_pack_validation_minutes: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterialFlow {
    material_id: String,
    material_family: String,
    mass_kg_per_pack: f64,
    collection_fraction: f64,
    material_recovery_fraction: f64,
    end_of_life_processing_kgco2e_per_kg: f64,
    recovery_route: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct QualificationGate {
    gate_id: String,
    status: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
}

#[derive(Debug, Serialize, PartialEq)]
struct MaterialResult {
    material_id: String,
    material_family: String,
    mass_kg_per_pack: f64,
    total_mass_through_end_of_life_kg: f64,
    collectable_mass_kg: f64,
    recovered_mass_kg: f64,
    unrecovered_mass_kg: f64,
    end_of_life_processing_impact_kgco2e: f64,
    recovery_route: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
}

#[derive(Debug, Serialize, PartialEq)]
struct DesignResult {
    design_id: String,
    label: String,
    architecture: String,
    configuration_id: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    functional_unit: String,
    life_cycle_boundary: String,
    nominal_capacity_kwh: f64,
    usable_capacity_kwh: f64,
    pack_mass_kg: f64,
    usable_pack_specific_energy_wh_per_kg: f64,
    serviceable_unit_count: usize,
    smallest_replaceable_unit_kwh: f64,
    lifetime_delivered_energy_kwh: f64,
    repair_minutes_per_event: f64,
    pack_replacement_minutes_per_event: f64,
    repair_labor_hours_over_life: f64,
    pack_replacement_labor_hours_over_life: f64,
    total_service_labor_hours_over_life: f64,
    planned_parts_wait_hours_over_life: f64,
    declared_service_downtime_hours_over_life: f64,
    repair_parts_cost_over_life_usd: f64,
    replacement_pack_cost_over_life_usd: f64,
    service_labor_cost_over_life_usd: f64,
    downtime_cost_over_life_usd: f64,
    lifetime_electricity_cost_usd: f64,
    declared_lifecycle_cost_usd: f64,
    initial_pack_embodied_kgco2e: f64,
    replacement_pack_embodied_kgco2e_over_life: f64,
    repair_parts_embodied_kgco2e_over_life: f64,
    use_phase_embodied_kgco2e: f64,
    end_of_life_processing_kgco2e: f64,
    declared_lifecycle_kgco2e: f64,
    lifecycle_kgco2e_per_delivered_kwh: f64,
    total_material_mass_kg_per_pack: f64,
    total_recovered_material_kg_over_life: f64,
    total_unrecovered_material_kg_over_life: f64,
    material_mass_balance_residual_kg_per_pack: f64,
    all_material_flows_conserve: bool,
    qualification_gate_count: usize,
    qualification_gates_passed: usize,
    release_eligible: bool,
    pareto_eligible: bool,
    ranking_status: String,
    materials: Vec<MaterialResult>,
    qualification_gates: Vec<QualificationGate>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    suite_id: String,
    method: &'static str,
    functional_unit: String,
    life_cycle_boundary: String,
    results: Vec<DesignResult>,
    claim_ceiling: &'static str,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/ev-battery-circularity/fixtures/ev_battery_circularity_v0_1.json".to_string()
    });
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let corpus: Corpus = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid battery-circularity JSON: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_corpus(&corpus) {
        eprintln!("invalid battery-circularity corpus: {error}");
        process::exit(2);
    }
    let mut results: Vec<DesignResult> = corpus
        .designs
        .iter()
        .map(|design| evaluate_design(design, &corpus.functional_unit, &corpus.life_cycle_boundary))
        .collect();
    results.sort_by(|a, b| a.design_id.cmp(&b.design_id));
    let report = Report {
        schema_version: corpus.schema_version,
        suite_id: corpus.suite_id,
        method: "operation-level maintenance accounting, declared-factor lifecycle impact totals, and material-flow conservation; no compensatory aggregate score",
        functional_unit: corpus.functional_unit,
        life_cycle_boundary: corpus.life_cycle_boundary,
        results,
        claim_ceiling: CLAIM_CEILING,
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize battery-circularity report: {error}");
            process::exit(2);
        }
    }
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_version != "0.1.0" || corpus.suite_id != "EV-BATTERY-CIRCULARITY-001" {
        return Err("unsupported_schema_or_suite_id".to_string());
    }
    if corpus.functional_unit.trim().is_empty()
        || corpus.life_cycle_boundary.trim().is_empty()
        || corpus.claim_ceiling.trim().is_empty()
        || corpus.designs.is_empty()
    {
        return Err("functional_unit_boundary_claim_ceiling_and_designs_are_required".to_string());
    }
    let mut ids = BTreeSet::new();
    for design in &corpus.designs {
        if !ids.insert(design.design_id.clone()) {
            return Err(format!("duplicate_design_id: {}", design.design_id));
        }
        validate_design(design).map_err(|error| format!("{}: {error}", design.design_id))?;
    }
    Ok(())
}

fn validate_evidence(class: &str, reference: &str, note: &str) -> Result<(), String> {
    if ![
        "synthetic",
        "measured",
        "manufacturer",
        "literature",
        "estimate",
        "unknown",
    ]
    .contains(&class)
    {
        return Err("unsupported_evidence_class".to_string());
    }
    if note.trim().is_empty() {
        return Err("evidence_note_must_be_nonempty".to_string());
    }
    if class != "synthetic" && class != "unknown" && reference.trim().is_empty() {
        return Err("non_synthetic_evidence_requires_reference".to_string());
    }
    Ok(())
}

fn finite_nonnegative(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{name}_must_be_finite_and_nonnegative"));
    }
    Ok(())
}

fn finite_positive(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name}_must_be_finite_and_positive"));
    }
    Ok(())
}

fn fraction(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name}_must_be_in_[0,1]"));
    }
    Ok(())
}

fn validate_design(design: &Design) -> Result<(), String> {
    for (name, value) in [
        ("design_id", design.design_id.as_str()),
        ("label", design.label.as_str()),
        ("architecture", design.architecture.as_str()),
        ("configuration_id", design.configuration_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name}_must_be_nonempty"));
        }
    }
    validate_evidence(
        &design.evidence_class,
        &design.evidence_reference,
        &design.evidence_note,
    )?;
    for (name, value) in [
        ("nominal_capacity_kwh", design.nominal_capacity_kwh),
        ("pack_mass_kg", design.pack_mass_kg),
        (
            "lifetime_delivered_energy_kwh",
            design.lifetime_delivered_energy_kwh,
        ),
        (
            "smallest_replaceable_unit_kwh",
            design.smallest_replaceable_unit_kwh,
        ),
    ] {
        finite_positive(name, value)?;
    }
    for (name, value) in [
        ("usable_capacity_fraction", design.usable_capacity_fraction),
        (
            "use_phase_grid_intensity_kgco2e_per_kwh",
            design.use_phase_grid_intensity_kgco2e_per_kwh,
        ),
        (
            "electricity_price_usd_per_kwh",
            design.electricity_price_usd_per_kwh,
        ),
        ("labor_rate_usd_per_hour", design.labor_rate_usd_per_hour),
        (
            "downtime_cost_usd_per_hour",
            design.downtime_cost_usd_per_hour,
        ),
        ("initial_pack_cost_usd", design.initial_pack_cost_usd),
        (
            "initial_pack_embodied_kgco2e",
            design.initial_pack_embodied_kgco2e,
        ),
    ] {
        finite_nonnegative(name, value)?;
    }
    if design.usable_capacity_fraction <= 0.0 || design.usable_capacity_fraction > 1.0 {
        return Err("usable_capacity_fraction_must_be_in_(0,1]".to_string());
    }
    if design.serviceable_unit_count == 0 {
        return Err("serviceable_unit_count_must_be_positive".to_string());
    }
    if design.smallest_replaceable_unit_kwh
        > design.nominal_capacity_kwh * design.usable_capacity_fraction + MASS_TOLERANCE_KG
    {
        return Err("smallest_replaceable_unit_exceeds_usable_pack_capacity".to_string());
    }
    if design.materials.is_empty() {
        return Err("material_inventory_must_not_be_empty".to_string());
    }
    if design.qualification_gates.is_empty() {
        return Err("qualification_gates_must_not_be_empty".to_string());
    }

    for (name, value) in [
        (
            "repair_unit_cost_usd",
            design.maintenance.repair_unit_cost_usd,
        ),
        (
            "repair_unit_embodied_kgco2e",
            design.maintenance.repair_unit_embodied_kgco2e,
        ),
        (
            "replacement_pack_cost_usd",
            design.maintenance.replacement_pack_cost_usd,
        ),
        (
            "replacement_pack_embodied_kgco2e",
            design.maintenance.replacement_pack_embodied_kgco2e,
        ),
        (
            "parts_wait_hours_per_repair",
            design.maintenance.parts_wait_hours_per_repair,
        ),
        (
            "parts_wait_hours_per_pack_replacement",
            design.maintenance.parts_wait_hours_per_pack_replacement,
        ),
    ] {
        finite_nonnegative(name, value)?;
    }
    for (name, value) in [
        ("isolation_minutes", design.operations.isolation_minutes),
        ("diagnosis_minutes", design.operations.diagnosis_minutes),
        (
            "enclosure_access_minutes",
            design.operations.enclosure_access_minutes,
        ),
        (
            "replace_unit_minutes",
            design.operations.replace_unit_minutes,
        ),
        ("reassembly_minutes", design.operations.reassembly_minutes),
        (
            "post_service_validation_minutes",
            design.operations.post_service_validation_minutes,
        ),
        (
            "pack_removal_minutes",
            design.operations.pack_removal_minutes,
        ),
        (
            "pack_installation_minutes",
            design.operations.pack_installation_minutes,
        ),
        (
            "post_pack_validation_minutes",
            design.operations.post_pack_validation_minutes,
        ),
    ] {
        finite_nonnegative(name, value)?;
    }

    let mut material_ids = BTreeSet::new();
    let mut material_mass = 0.0;
    let mut material_flows: Vec<&MaterialFlow> = design.materials.iter().collect();
    material_flows.sort_by(|a, b| a.material_id.cmp(&b.material_id));
    for material in material_flows {
        if material.material_id.trim().is_empty()
            || material.material_family.trim().is_empty()
            || material.recovery_route.trim().is_empty()
            || !material_ids.insert(material.material_id.clone())
        {
            return Err(format!(
                "empty_or_duplicate_material_identity: {}",
                material.material_id
            ));
        }
        finite_positive("material_mass_kg_per_pack", material.mass_kg_per_pack)?;
        fraction("collection_fraction", material.collection_fraction)?;
        fraction(
            "material_recovery_fraction",
            material.material_recovery_fraction,
        )?;
        finite_nonnegative(
            "end_of_life_processing_kgco2e_per_kg",
            material.end_of_life_processing_kgco2e_per_kg,
        )?;
        validate_evidence(
            &material.evidence_class,
            &material.evidence_reference,
            &material.evidence_note,
        )?;
        material_mass += material.mass_kg_per_pack;
    }
    if !material_mass.is_finite() || (material_mass - design.pack_mass_kg).abs() > MASS_TOLERANCE_KG
    {
        return Err(format!(
            "material_mass_does_not_conserve_pack_mass: inventory={material_mass}, declared_pack_mass={}",
            design.pack_mass_kg
        ));
    }

    let expected_gates: BTreeSet<&str> = [
        "structural_integrity",
        "thermal_containment",
        "electrical_isolation",
        "diagnostic_access",
        "repair_validation",
        "end_of_life_handling",
    ]
    .into_iter()
    .collect();
    let mut gate_ids = BTreeSet::new();
    for gate in &design.qualification_gates {
        if gate.gate_id.trim().is_empty() || !gate_ids.insert(gate.gate_id.as_str()) {
            return Err(format!("empty_or_duplicate_gate_id: {}", gate.gate_id));
        }
        if !expected_gates.contains(gate.gate_id.as_str()) {
            return Err(format!("unsupported_gate_id: {}", gate.gate_id));
        }
        if !["verified_pass", "failed", "unknown", "not_assessed"].contains(&gate.status.as_str()) {
            return Err(format!("unsupported_gate_status: {}", gate.status));
        }
        validate_evidence(
            &gate.evidence_class,
            &gate.evidence_reference,
            &gate.evidence_note,
        )?;
        if gate.status == "verified_pass"
            && (gate.evidence_class == "synthetic"
                || gate.evidence_class == "unknown"
                || gate.evidence_reference.trim().is_empty())
        {
            return Err(format!(
                "gate_{}_cannot_pass_without_non_synthetic_evidence",
                gate.gate_id
            ));
        }
    }
    if gate_ids.len() != expected_gates.len() {
        return Err(
            "all_six_required_qualification_gates_must_be_present_exactly_once".to_string(),
        );
    }
    Ok(())
}

fn evaluate_design(design: &Design, functional_unit: &str, boundary: &str) -> DesignResult {
    let total_pack_count = 1.0 + design.maintenance.whole_pack_replacements_over_life as f64;
    let usable_capacity_kwh = design.nominal_capacity_kwh * design.usable_capacity_fraction;
    let repair_minutes = design.operations.isolation_minutes
        + design.operations.diagnosis_minutes
        + design.operations.enclosure_access_minutes
        + design.operations.replace_unit_minutes
        + design.operations.reassembly_minutes
        + design.operations.post_service_validation_minutes;
    let pack_replacement_minutes = design.operations.isolation_minutes
        + design.operations.diagnosis_minutes
        + design.operations.pack_removal_minutes
        + design.operations.pack_installation_minutes
        + design.operations.post_pack_validation_minutes;
    let repair_labor_hours =
        repair_minutes / 60.0 * design.maintenance.repair_events_over_life as f64;
    let replacement_labor_hours = pack_replacement_minutes / 60.0
        * design.maintenance.whole_pack_replacements_over_life as f64;
    let total_labor_hours = repair_labor_hours + replacement_labor_hours;
    let parts_wait_hours = design.maintenance.parts_wait_hours_per_repair
        * design.maintenance.repair_events_over_life as f64
        + design.maintenance.parts_wait_hours_per_pack_replacement
            * design.maintenance.whole_pack_replacements_over_life as f64;
    let declared_downtime_hours = total_labor_hours + parts_wait_hours;
    let repair_parts_cost =
        design.maintenance.repair_unit_cost_usd * design.maintenance.repair_events_over_life as f64;
    let replacement_parts_cost = design.maintenance.replacement_pack_cost_usd
        * design.maintenance.whole_pack_replacements_over_life as f64;
    let labor_cost = total_labor_hours * design.labor_rate_usd_per_hour;
    let downtime_cost = declared_downtime_hours * design.downtime_cost_usd_per_hour;
    let electricity_cost =
        design.lifetime_delivered_energy_kwh * design.electricity_price_usd_per_kwh;
    let lifecycle_cost = design.initial_pack_cost_usd
        + repair_parts_cost
        + replacement_parts_cost
        + labor_cost
        + downtime_cost
        + electricity_cost;

    let replacement_embodied = design.maintenance.replacement_pack_embodied_kgco2e
        * design.maintenance.whole_pack_replacements_over_life as f64;
    let repair_embodied = design.maintenance.repair_unit_embodied_kgco2e
        * design.maintenance.repair_events_over_life as f64;
    let use_phase_impact =
        design.lifetime_delivered_energy_kwh * design.use_phase_grid_intensity_kgco2e_per_kwh;
    let mut material_results = Vec::with_capacity(design.materials.len());
    let mut total_recovered_mass = 0.0;
    let mut total_unrecovered_mass = 0.0;
    let mut eol_impact = 0.0;
    let mut total_material_mass = 0.0;
    let mut material_flows: Vec<&MaterialFlow> = design.materials.iter().collect();
    material_flows.sort_by(|a, b| a.material_id.cmp(&b.material_id));
    for flow in material_flows {
        let total_mass = flow.mass_kg_per_pack * total_pack_count;
        let collectable = total_mass * flow.collection_fraction;
        let recovered = collectable * flow.material_recovery_fraction;
        let unrecovered = total_mass - recovered;
        let impact = total_mass * flow.end_of_life_processing_kgco2e_per_kg;
        total_recovered_mass += recovered;
        total_unrecovered_mass += unrecovered;
        eol_impact += impact;
        total_material_mass += flow.mass_kg_per_pack;
        material_results.push(MaterialResult {
            material_id: flow.material_id.clone(),
            material_family: flow.material_family.clone(),
            mass_kg_per_pack: flow.mass_kg_per_pack,
            total_mass_through_end_of_life_kg: total_mass,
            collectable_mass_kg: collectable,
            recovered_mass_kg: recovered,
            unrecovered_mass_kg: unrecovered,
            end_of_life_processing_impact_kgco2e: impact,
            recovery_route: flow.recovery_route.clone(),
            evidence_class: flow.evidence_class.clone(),
            evidence_reference: flow.evidence_reference.clone(),
            evidence_note: flow.evidence_note.clone(),
        });
    }
    let lifecycle_impact = design.initial_pack_embodied_kgco2e
        + replacement_embodied
        + repair_embodied
        + use_phase_impact
        + eol_impact;
    let gates_passed = design
        .qualification_gates
        .iter()
        .filter(|gate| gate.status == "verified_pass")
        .count();
    let design_evidence_is_actionable =
        !["synthetic", "unknown"].contains(&design.evidence_class.as_str());
    let material_evidence_is_actionable = design
        .materials
        .iter()
        .all(|material| !["synthetic", "unknown"].contains(&material.evidence_class.as_str()));
    let release_eligible =
        gates_passed == 6 && design_evidence_is_actionable && material_evidence_is_actionable;
    let mass_residual = total_material_mass - design.pack_mass_kg;
    let flow_conserves = mass_residual.abs() <= MASS_TOLERANCE_KG
        && material_results.iter().all(|material| {
            (material.total_mass_through_end_of_life_kg
                - material.recovered_mass_kg
                - material.unrecovered_mass_kg)
                .abs()
                <= MASS_TOLERANCE_KG
        });
    DesignResult {
        design_id: design.design_id.clone(),
        label: design.label.clone(),
        architecture: design.architecture.clone(),
        configuration_id: design.configuration_id.clone(),
        evidence_class: design.evidence_class.clone(),
        evidence_reference: design.evidence_reference.clone(),
        evidence_note: design.evidence_note.clone(),
        functional_unit: functional_unit.to_string(),
        life_cycle_boundary: boundary.to_string(),
        nominal_capacity_kwh: design.nominal_capacity_kwh,
        usable_capacity_kwh,
        pack_mass_kg: design.pack_mass_kg,
        usable_pack_specific_energy_wh_per_kg: usable_capacity_kwh * 1000.0 / design.pack_mass_kg,
        serviceable_unit_count: design.serviceable_unit_count,
        smallest_replaceable_unit_kwh: design.smallest_replaceable_unit_kwh,
        lifetime_delivered_energy_kwh: design.lifetime_delivered_energy_kwh,
        repair_minutes_per_event: repair_minutes,
        pack_replacement_minutes_per_event: pack_replacement_minutes,
        repair_labor_hours_over_life: repair_labor_hours,
        pack_replacement_labor_hours_over_life: replacement_labor_hours,
        total_service_labor_hours_over_life: total_labor_hours,
        planned_parts_wait_hours_over_life: parts_wait_hours,
        declared_service_downtime_hours_over_life: declared_downtime_hours,
        repair_parts_cost_over_life_usd: repair_parts_cost,
        replacement_pack_cost_over_life_usd: replacement_parts_cost,
        service_labor_cost_over_life_usd: labor_cost,
        downtime_cost_over_life_usd: downtime_cost,
        lifetime_electricity_cost_usd: electricity_cost,
        declared_lifecycle_cost_usd: lifecycle_cost,
        initial_pack_embodied_kgco2e: design.initial_pack_embodied_kgco2e,
        replacement_pack_embodied_kgco2e_over_life: replacement_embodied,
        repair_parts_embodied_kgco2e_over_life: repair_embodied,
        use_phase_embodied_kgco2e: use_phase_impact,
        end_of_life_processing_kgco2e: eol_impact,
        declared_lifecycle_kgco2e: lifecycle_impact,
        lifecycle_kgco2e_per_delivered_kwh: lifecycle_impact / design.lifetime_delivered_energy_kwh,
        total_material_mass_kg_per_pack: total_material_mass,
        total_recovered_material_kg_over_life: total_recovered_mass,
        total_unrecovered_material_kg_over_life: total_unrecovered_mass,
        material_mass_balance_residual_kg_per_pack: mass_residual,
        all_material_flows_conserve: flow_conserves,
        qualification_gate_count: design.qualification_gates.len(),
        qualification_gates_passed: gates_passed,
        release_eligible,
        pareto_eligible: release_eligible && design.evidence_class != "synthetic",
        ranking_status: if release_eligible && design.evidence_class != "synthetic" {
            "eligible_for_separately_qualified_pareto_analysis".to_string()
        } else {
            "not_ranked_qualification_or_evidence_incomplete".to_string()
        },
        materials: material_results,
        qualification_gates: design.qualification_gates.clone(),
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(id: &str, family: &str, mass: f64, collection: f64, recovery: f64) -> MaterialFlow {
        MaterialFlow {
            material_id: id.to_string(),
            material_family: family.to_string(),
            mass_kg_per_pack: mass,
            collection_fraction: collection,
            material_recovery_fraction: recovery,
            end_of_life_processing_kgco2e_per_kg: 0.1,
            recovery_route: "synthetic-route".to_string(),
            evidence_class: "synthetic".to_string(),
            evidence_reference: "synthetic".to_string(),
            evidence_note: "test input".to_string(),
        }
    }

    fn gate(id: &str) -> QualificationGate {
        QualificationGate {
            gate_id: id.to_string(),
            status: "not_assessed".to_string(),
            evidence_class: "unknown".to_string(),
            evidence_reference: "".to_string(),
            evidence_note: "no qualification evidence supplied".to_string(),
        }
    }

    fn design() -> Design {
        Design {
            design_id: "test-pack".to_string(),
            label: "Test pack".to_string(),
            architecture: "serviceable-modular".to_string(),
            configuration_id: "test-config-a".to_string(),
            evidence_class: "synthetic".to_string(),
            evidence_reference: "synthetic".to_string(),
            evidence_note: "test pack".to_string(),
            nominal_capacity_kwh: 100.0,
            usable_capacity_fraction: 0.9,
            pack_mass_kg: 500.0,
            initial_pack_cost_usd: 10000.0,
            initial_pack_embodied_kgco2e: 4000.0,
            lifetime_delivered_energy_kwh: 200000.0,
            use_phase_grid_intensity_kgco2e_per_kwh: 0.4,
            electricity_price_usd_per_kwh: 0.2,
            labor_rate_usd_per_hour: 50.0,
            downtime_cost_usd_per_hour: 80.0,
            serviceable_unit_count: 10,
            smallest_replaceable_unit_kwh: 9.0,
            maintenance: Maintenance {
                repair_events_over_life: 2,
                whole_pack_replacements_over_life: 1,
                repair_unit_cost_usd: 300.0,
                repair_unit_embodied_kgco2e: 75.0,
                replacement_pack_cost_usd: 10000.0,
                replacement_pack_embodied_kgco2e: 4000.0,
                parts_wait_hours_per_repair: 4.0,
                parts_wait_hours_per_pack_replacement: 72.0,
            },
            operations: OperationProfile {
                isolation_minutes: 30.0,
                diagnosis_minutes: 60.0,
                enclosure_access_minutes: 30.0,
                replace_unit_minutes: 45.0,
                reassembly_minutes: 30.0,
                post_service_validation_minutes: 45.0,
                pack_removal_minutes: 60.0,
                pack_installation_minutes: 60.0,
                post_pack_validation_minutes: 60.0,
            },
            materials: vec![
                flow("cells", "battery-cells", 400.0, 0.9, 0.85),
                flow("enclosure", "aluminium-and-steel", 100.0, 0.95, 0.9),
            ],
            qualification_gates: vec![
                gate("structural_integrity"),
                gate("thermal_containment"),
                gate("electrical_isolation"),
                gate("diagnostic_access"),
                gate("repair_validation"),
                gate("end_of_life_handling"),
            ],
        }
    }

    #[test]
    fn declared_simple_cost_and_impact_totals_are_reconciled() {
        let design = design();
        let result = evaluate_design(
            &design,
            "delivered battery kWh",
            "synthetic cradle-to-end-of-life boundary",
        );
        assert!((result.usable_capacity_kwh - 90.0).abs() < 1.0e-8);
        assert!((result.usable_pack_specific_energy_wh_per_kg - 180.0).abs() < 1.0e-8);
        assert!((result.repair_minutes_per_event - 240.0).abs() < 1.0e-8);
        assert!((result.pack_replacement_minutes_per_event - 270.0).abs() < 1.0e-8);
        assert!((result.repair_labor_hours_over_life - 8.0).abs() < 1.0e-8);
        assert!((result.pack_replacement_labor_hours_over_life - 4.5).abs() < 1.0e-8);
        assert!((result.total_service_labor_hours_over_life - 12.5).abs() < 1.0e-8);
        assert!((result.lifetime_electricity_cost_usd - 40000.0).abs() < 1.0e-8);
        assert!((result.use_phase_embodied_kgco2e - 80000.0).abs() < 1.0e-8);
        assert!(
            (result.declared_lifecycle_kgco2e - (4000.0 + 4000.0 + 150.0 + 80000.0 + 100.0)).abs()
                < 1.0e-7
        );
        assert!(result.all_material_flows_conserve);
    }

    #[test]
    fn synthetic_examples_never_become_release_eligible() {
        let result = evaluate_design(&design(), "delivered battery kWh", "test boundary");
        assert!(!result.release_eligible);
        assert!(!result.pareto_eligible);
        assert_eq!(result.qualification_gates_passed, 0);
        assert_eq!(
            result.ranking_status,
            "not_ranked_qualification_or_evidence_incomplete"
        );
    }

    #[test]
    fn mass_inventory_must_equal_declared_pack_mass() {
        let mut value = design();
        value.materials[0].mass_kg_per_pack = 399.0;
        assert!(
            validate_design(&value)
                .unwrap_err()
                .starts_with("material_mass_does_not_conserve_pack_mass")
        );
    }

    #[test]
    fn negative_material_mass_is_rejected() {
        let mut value = design();
        value.materials[0].mass_kg_per_pack = -1.0;
        assert_eq!(
            validate_design(&value).unwrap_err(),
            "material_mass_kg_per_pack_must_be_finite_and_positive"
        );
    }

    #[test]
    fn recovery_fractions_cannot_exceed_one() {
        let mut value = design();
        value.materials[0].material_recovery_fraction = 1.01;
        assert_eq!(
            validate_design(&value).unwrap_err(),
            "material_recovery_fraction_must_be_in_[0,1]"
        );
    }

    #[test]
    fn unknown_material_evidence_is_preserved_not_promoted() {
        let mut value = design();
        value.materials[0].evidence_class = "unknown".to_string();
        value.materials[0].evidence_reference.clear();
        let result = evaluate_design(&value, "delivered battery kWh", "test boundary");
        assert_eq!(result.materials[0].evidence_class, "unknown");
        assert!(!result.pareto_eligible);
    }

    #[test]
    fn synthetic_gate_cannot_claim_verified_pass() {
        let mut value = design();
        value.qualification_gates[0].status = "verified_pass".to_string();
        value.qualification_gates[0].evidence_class = "synthetic".to_string();
        value.qualification_gates[0].evidence_reference = "synthetic".to_string();
        assert!(
            validate_design(&value)
                .unwrap_err()
                .contains("cannot_pass_without_non_synthetic_evidence")
        );
    }

    #[test]
    fn every_qualification_gate_is_required_exactly_once() {
        let mut value = design();
        value.qualification_gates.pop();
        assert_eq!(
            validate_design(&value).unwrap_err(),
            "all_six_required_qualification_gates_must_be_present_exactly_once"
        );
    }

    #[test]
    fn replacement_counts_scale_material_recovery_and_lifecycle_impacts() {
        let mut value = design();
        value.maintenance.whole_pack_replacements_over_life = 2;
        let result = evaluate_design(&value, "delivered battery kWh", "test boundary");
        assert!(
            (result.total_recovered_material_kg_over_life
                - 3.0 * (400.0 * 0.9 * 0.85 + 100.0 * 0.95 * 0.9))
                .abs()
                < 1.0e-7
        );
        assert!((result.replacement_pack_embodied_kgco2e_over_life - 8000.0).abs() < 1.0e-8);
    }

    #[test]
    fn lifecycle_impact_per_delivered_kwh_uses_declared_functional_unit() {
        let mut value = design();
        value.lifetime_delivered_energy_kwh = 100000.0;
        let result = evaluate_design(&value, "delivered battery kWh", "test boundary");
        assert!(
            (result.lifecycle_kgco2e_per_delivered_kwh
                - result.declared_lifecycle_kgco2e / 100000.0)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn input_order_does_not_change_canonical_design_results() {
        let mut a = design();
        a.design_id = "pack-b".to_string();
        let mut b = design();
        b.design_id = "pack-a".to_string();
        let mut first = vec![
            evaluate_design(&a, "delivered battery kWh", "test boundary"),
            evaluate_design(&b, "delivered battery kWh", "test boundary"),
        ];
        let mut second = vec![
            evaluate_design(&b, "delivered battery kWh", "test boundary"),
            evaluate_design(&a, "delivered battery kWh", "test boundary"),
        ];
        first.sort_by(|x, y| x.design_id.cmp(&y.design_id));
        second.sort_by(|x, y| x.design_id.cmp(&y.design_id));
        assert_eq!(first, second);
    }

    #[test]
    fn material_order_permutations_produce_canonical_identical_results() {
        let baseline_design = design();
        let expected = evaluate_design(&baseline_design, "delivered battery kWh", "test boundary");
        for seed in 0_u64..100 {
            let mut shuffled = design();
            let mut state = seed + 7;
            for index in (1..shuffled.materials.len()).rev() {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let swap_index = (state as usize) % (index + 1);
                shuffled.materials.swap(index, swap_index);
            }
            assert_eq!(
                evaluate_design(&shuffled, "delivered battery kWh", "test boundary"),
                expected,
                "material permutation {seed} changed the canonical result"
            );
        }
    }

    #[test]
    fn synthetic_candidate_cannot_be_pareto_eligible_even_if_gates_have_external_pass_records() {
        let mut value = design();
        for gate in &mut value.qualification_gates {
            gate.status = "verified_pass".to_string();
            gate.evidence_class = "measured".to_string();
            gate.evidence_reference = "test-lab-record".to_string();
        }
        let result = evaluate_design(&value, "delivered battery kWh", "test boundary");
        assert!(!result.release_eligible);
        assert!(!result.pareto_eligible);
    }

    #[test]
    fn zero_lifetime_throughput_is_rejected() {
        let mut value = design();
        value.lifetime_delivered_energy_kwh = 0.0;
        assert_eq!(
            validate_design(&value).unwrap_err(),
            "lifetime_delivered_energy_kwh_must_be_finite_and_positive"
        );
    }

    #[test]
    fn mismatched_repairable_unit_size_is_rejected() {
        let mut value = design();
        value.smallest_replaceable_unit_kwh = 100.0;
        assert_eq!(
            validate_design(&value).unwrap_err(),
            "smallest_replaceable_unit_exceeds_usable_pack_capacity"
        );
    }
}
