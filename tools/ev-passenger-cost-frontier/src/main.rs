use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const CLAIM_CEILING: &str = "Parametric cost-envelope arithmetic only. Synthetic fixture results are not supplier quotes, achievable retail pricing, a manufacturing plan, a safety finding, homologation, or proof of superiority.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    benchmark_id: String,
    currency: String,
    market: Market,
    target: VehicleTarget,
    candidates: Vec<Candidate>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Market {
    country: String,
    baseline_vehicle: String,
    baseline_variant: String,
    observed_retail_price: f64,
    observed_on: String,
    price_source_url: String,
    undercut_fraction: f64,
    retail_price_includes_vat: bool,
    vat_fraction: f64,
    distributor_margin_fraction: f64,
    target_oem_gross_margin_fraction: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VehicleTarget {
    minimum_seats: u32,
    minimum_payload_kg: f64,
    minimum_declared_range_km: f64,
    maximum_declared_consumption_kwh_per_100km: f64,
    maximum_curb_mass_kg: f64,
    comparison_note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    candidate_id: String,
    label: String,
    architecture: String,
    envelope: VehicleEnvelope,
    manufacturing_yield_fraction: f64,
    amortization_units: u64,
    envelope_evidence_class: EvidenceClass,
    envelope_source_reference: String,
    envelope_as_of: String,
    technical_gate: Gate,
    safety_gate: Gate,
    components: Vec<Component>,
    factory_conversion_cost: CostRange,
    warranty_service_reserve: CostRange,
    fixed_program_investment: CostRange,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VehicleEnvelope {
    seats: u32,
    nominal_pack_kwh: f64,
    usable_pack_fraction: f64,
    payload_capacity_kg: f64,
    declared_range_km: f64,
    declared_consumption_kwh_per_100km: f64,
    curb_mass_kg: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gate {
    status: GateStatus,
    evidence_class: EvidenceClass,
    source_reference: String,
    as_of: String,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GateStatus {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Component {
    component_id: String,
    quantity_per_vehicle: f64,
    quantity_unit: String,
    unit_cost_basis: String,
    unit_cost: CostRange,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CostRange {
    low: f64,
    high: f64,
    evidence_class: EvidenceClass,
    source_reference: String,
    as_of: String,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    Synthetic,
    Estimate,
    Literature,
    ManufacturerQuote,
    ManufacturerSpec,
    Measured,
    IndependentTest,
    TypeApproval,
    AuditedCost,
    Unknown,
}

impl EvidenceClass {
    fn is_cost_release_grade(self) -> bool {
        matches!(
            self,
            Self::ManufacturerQuote | Self::Measured | Self::AuditedCost
        )
    }

    fn is_metrics_release_grade(self) -> bool {
        matches!(
            self,
            Self::ManufacturerSpec | Self::Measured | Self::IndependentTest | Self::TypeApproval
        )
    }

    fn is_safety_release_grade(self) -> bool {
        matches!(self, Self::IndependentTest | Self::TypeApproval)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::Estimate => "estimate",
            Self::Literature => "literature",
            Self::ManufacturerQuote => "manufacturer_quote",
            Self::ManufacturerSpec => "manufacturer_spec",
            Self::Measured => "measured",
            Self::IndependentTest => "independent_test",
            Self::TypeApproval => "type_approval",
            Self::AuditedCost => "audited_cost",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EconomicsResult {
    AboveTargetEvenAtLowCost,
    RangeCrossesTargetCeiling,
    WithinTargetEvenAtHighCost,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReleaseDisposition {
    BlockedByDeclaredGate,
    BlockedByEvidenceBackedCostEnvelope,
    NeedsEvidence,
    EvidenceBackedCandidate,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ComponentCostResult {
    component_id: String,
    quantity_per_vehicle: f64,
    quantity_unit: String,
    unit_cost_basis: String,
    unit_cost_low: f64,
    unit_cost_high: f64,
    bom_cost_low: f64,
    bom_cost_high: f64,
    evidence_class: EvidenceClass,
    source_reference: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct CandidateResult {
    candidate_id: String,
    label: String,
    architecture: String,
    declared_mission_fit: bool,
    mission_findings: Vec<String>,
    engineering_gate_status: GateStatus,
    safety_gate_status: GateStatus,
    cost_inputs_release_grade: bool,
    envelope_evidence_release_grade: bool,
    technical_gate_evidence_qualified: bool,
    safety_gate_evidence_qualified: bool,
    economics_result: EconomicsResult,
    release_disposition: ReleaseDisposition,
    bill_of_materials_low: f64,
    bill_of_materials_high: f64,
    bom_per_good_vehicle_low: f64,
    bom_per_good_vehicle_high: f64,
    factory_conversion_cost_low: f64,
    factory_conversion_cost_high: f64,
    warranty_service_reserve_low: f64,
    warranty_service_reserve_high: f64,
    fixed_program_cost_per_vehicle_low: f64,
    fixed_program_cost_per_vehicle_high: f64,
    total_cost_of_goods_low: f64,
    total_cost_of_goods_high: f64,
    maximum_affordable_cost_of_goods_at_target: f64,
    oem_net_revenue_at_target: f64,
    implied_retail_price_low: f64,
    implied_retail_price_high: f64,
    retail_price_gap_low: f64,
    retail_price_gap_high: f64,
    cost_headroom_low: f64,
    cost_headroom_high: f64,
    component_costs: Vec<ComponentCostResult>,
    evidence_gaps: Vec<String>,
    notes: Vec<String>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    currency: String,
    country: String,
    baseline_vehicle: String,
    baseline_variant: String,
    baseline_observed_retail_price: f64,
    baseline_observed_on: String,
    baseline_price_source_url: String,
    target_undercut_fraction: f64,
    retail_price_ceiling: f64,
    retail_price_includes_vat: bool,
    vat_fraction: f64,
    distributor_margin_fraction: f64,
    target_oem_gross_margin_fraction: f64,
    target_oem_net_revenue: f64,
    maximum_affordable_cost_of_goods: f64,
    vehicle_target: VehicleTargetReport,
    results: Vec<CandidateResult>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct VehicleTargetReport {
    minimum_seats: u32,
    minimum_payload_kg: f64,
    minimum_declared_range_km: f64,
    maximum_declared_consumption_kwh_per_100km: f64,
    maximum_curb_mass_kg: f64,
    comparison_note: String,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/ev-passenger-cost-frontier/fixtures/ev_passenger_cost_v0_1.json".to_string()
    });
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };

    let catalog: Catalog = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid EV passenger cost catalog: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_catalog(&catalog) {
        eprintln!("invalid EV passenger cost catalog: {error}");
        process::exit(2);
    }

    let report = evaluate_catalog(&catalog);
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize cost frontier report: {error}");
            process::exit(2);
        }
    }
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

fn fraction(name: &str, value: f64, allow_one: bool) -> Result<(), String> {
    let valid_upper = if allow_one { value <= 1.0 } else { value < 1.0 };
    if !value.is_finite() || value < 0.0 || !valid_upper {
        return Err(format!("{name}_outside_allowed_fraction_range"));
    }
    Ok(())
}

fn nonblank(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name}_must_not_be_blank"));
    }
    Ok(())
}

fn validate_cost_range(name: &str, range: &CostRange) -> Result<(), String> {
    finite_nonnegative(&format!("{name}_low"), range.low)?;
    finite_nonnegative(&format!("{name}_high"), range.high)?;
    if range.low > range.high {
        return Err(format!("{name}_low_exceeds_high"));
    }
    nonblank(&format!("{name}_source_reference"), &range.source_reference)?;
    nonblank(&format!("{name}_as_of"), &range.as_of)?;
    nonblank(&format!("{name}_note"), &range.note)?;
    Ok(())
}

fn validate_gate(name: &str, gate: &Gate) -> Result<(), String> {
    nonblank(&format!("{name}_source_reference"), &gate.source_reference)?;
    nonblank(&format!("{name}_as_of"), &gate.as_of)?;
    nonblank(&format!("{name}_note"), &gate.note)?;
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != "0.1.0" || catalog.benchmark_id != "EV-PASSENGER-COST-001" {
        return Err("unsupported_schema_or_benchmark_id".to_string());
    }
    nonblank("currency", &catalog.currency)?;
    nonblank("claim_ceiling", &catalog.claim_ceiling)?;
    nonblank("market_country", &catalog.market.country)?;
    nonblank("baseline_vehicle", &catalog.market.baseline_vehicle)?;
    nonblank("baseline_variant", &catalog.market.baseline_variant)?;
    nonblank("market_observed_on", &catalog.market.observed_on)?;
    if !catalog.market.price_source_url.starts_with("https://") {
        return Err("market_price_source_must_be_https".to_string());
    }
    finite_positive(
        "observed_retail_price",
        catalog.market.observed_retail_price,
    )?;
    fraction("undercut_fraction", catalog.market.undercut_fraction, false)?;
    fraction("vat_fraction", catalog.market.vat_fraction, true)?;
    fraction(
        "distributor_margin_fraction",
        catalog.market.distributor_margin_fraction,
        false,
    )?;
    fraction(
        "target_oem_gross_margin_fraction",
        catalog.market.target_oem_gross_margin_fraction,
        false,
    )?;

    if catalog.target.minimum_seats == 0 {
        return Err("minimum_seats_must_be_positive".to_string());
    }
    finite_nonnegative("minimum_payload_kg", catalog.target.minimum_payload_kg)?;
    finite_positive(
        "minimum_declared_range_km",
        catalog.target.minimum_declared_range_km,
    )?;
    finite_positive(
        "maximum_declared_consumption_kwh_per_100km",
        catalog.target.maximum_declared_consumption_kwh_per_100km,
    )?;
    finite_positive("maximum_curb_mass_kg", catalog.target.maximum_curb_mass_kg)?;
    nonblank("comparison_note", &catalog.target.comparison_note)?;

    if catalog.candidates.is_empty() {
        return Err("candidate_list_must_not_be_empty".to_string());
    }
    let mut candidate_ids = BTreeSet::new();
    for candidate in &catalog.candidates {
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(format!(
                "duplicate_candidate_id: {}",
                candidate.candidate_id
            ));
        }
        validate_candidate(candidate)
            .map_err(|error| format!("{}: {error}", candidate.candidate_id))?;
    }
    Ok(())
}

fn validate_candidate(candidate: &Candidate) -> Result<(), String> {
    for (name, value) in [
        ("candidate_id", candidate.candidate_id.as_str()),
        ("label", candidate.label.as_str()),
        ("architecture", candidate.architecture.as_str()),
        (
            "envelope_source_reference",
            candidate.envelope_source_reference.as_str(),
        ),
        ("envelope_as_of", candidate.envelope_as_of.as_str()),
    ] {
        nonblank(name, value)?;
    }
    finite_positive("curb_mass_kg", candidate.envelope.curb_mass_kg)?;
    finite_positive("nominal_pack_kwh", candidate.envelope.nominal_pack_kwh)?;
    fraction(
        "usable_pack_fraction",
        candidate.envelope.usable_pack_fraction,
        true,
    )?;
    if candidate.envelope.usable_pack_fraction == 0.0 {
        return Err("usable_pack_fraction_must_be_positive".to_string());
    }
    finite_nonnegative(
        "payload_capacity_kg",
        candidate.envelope.payload_capacity_kg,
    )?;
    finite_nonnegative("declared_range_km", candidate.envelope.declared_range_km)?;
    finite_nonnegative(
        "declared_consumption_kwh_per_100km",
        candidate.envelope.declared_consumption_kwh_per_100km,
    )?;
    if candidate.envelope.seats == 0 {
        return Err("seats_must_be_positive".to_string());
    }
    fraction(
        "manufacturing_yield_fraction",
        candidate.manufacturing_yield_fraction,
        true,
    )?;
    if candidate.manufacturing_yield_fraction == 0.0 {
        return Err("manufacturing_yield_fraction_must_be_positive".to_string());
    }
    if candidate.amortization_units == 0 {
        return Err("amortization_units_must_be_positive".to_string());
    }
    validate_gate("technical_gate", &candidate.technical_gate)?;
    validate_gate("safety_gate", &candidate.safety_gate)?;
    validate_cost_range(
        "factory_conversion_cost",
        &candidate.factory_conversion_cost,
    )?;
    validate_cost_range(
        "warranty_service_reserve",
        &candidate.warranty_service_reserve,
    )?;
    validate_cost_range(
        "fixed_program_investment",
        &candidate.fixed_program_investment,
    )?;
    if candidate.components.is_empty() {
        return Err("component_list_must_not_be_empty".to_string());
    }
    let mut component_ids = BTreeSet::new();
    for component in &candidate.components {
        nonblank("component_id", &component.component_id)?;
        nonblank("quantity_unit", &component.quantity_unit)?;
        nonblank("unit_cost_basis", &component.unit_cost_basis)?;
        if !component_ids.insert(component.component_id.clone()) {
            return Err(format!(
                "duplicate_component_id: {}",
                component.component_id
            ));
        }
        finite_positive("quantity_per_vehicle", component.quantity_per_vehicle)?;
        validate_cost_range(
            &format!("component_{}_unit_cost", component.component_id),
            &component.unit_cost,
        )?;
    }
    if candidate.notes.is_empty() || candidate.notes.iter().any(|note| note.trim().is_empty()) {
        return Err("candidate_notes_must_be_nonempty".to_string());
    }
    Ok(())
}

fn ex_vat_retail(market: &Market, retail_price: f64) -> f64 {
    if market.retail_price_includes_vat {
        retail_price / (1.0 + market.vat_fraction)
    } else {
        retail_price
    }
}

fn target_waterfall(catalog: &Catalog) -> (f64, f64, f64) {
    let retail_ceiling =
        catalog.market.observed_retail_price * (1.0 - catalog.market.undercut_fraction);
    let oem_net_revenue = ex_vat_retail(&catalog.market, retail_ceiling)
        * (1.0 - catalog.market.distributor_margin_fraction);
    let maximum_cogs = oem_net_revenue * (1.0 - catalog.market.target_oem_gross_margin_fraction);
    (retail_ceiling, oem_net_revenue, maximum_cogs)
}

fn gate_is_qualified(gate: &Gate, is_safety: bool) -> bool {
    if gate.status != GateStatus::Passed || gate.source_reference.trim().is_empty() {
        return false;
    }
    if is_safety {
        gate.evidence_class.is_safety_release_grade()
    } else {
        gate.evidence_class.is_metrics_release_grade()
    }
}

fn declared_mission_findings(candidate: &Candidate, target: &VehicleTarget) -> Vec<String> {
    let mut findings = Vec::new();
    let required_battery_energy_kwh = candidate.envelope.declared_range_km
        * candidate.envelope.declared_consumption_kwh_per_100km
        / 100.0;
    let declared_usable_pack_kwh =
        candidate.envelope.nominal_pack_kwh * candidate.envelope.usable_pack_fraction;
    if required_battery_energy_kwh > declared_usable_pack_kwh + 1e-9 {
        findings.push(format!(
            "range_energy_exceeds_declared_usable_pack_kwh: {:.3} > {:.3}",
            required_battery_energy_kwh, declared_usable_pack_kwh
        ));
    }
    if candidate.envelope.seats < target.minimum_seats {
        findings.push(format!(
            "seat_count_below_target: {} < {}",
            candidate.envelope.seats, target.minimum_seats
        ));
    }
    if candidate.envelope.payload_capacity_kg < target.minimum_payload_kg {
        findings.push(format!(
            "payload_below_target_kg: {:.3} < {:.3}",
            candidate.envelope.payload_capacity_kg, target.minimum_payload_kg
        ));
    }
    if candidate.envelope.declared_range_km < target.minimum_declared_range_km {
        findings.push(format!(
            "declared_range_below_target_km: {:.3} < {:.3}",
            candidate.envelope.declared_range_km, target.minimum_declared_range_km
        ));
    }
    if candidate.envelope.declared_consumption_kwh_per_100km
        > target.maximum_declared_consumption_kwh_per_100km
    {
        findings.push(format!(
            "declared_consumption_above_target_kwh_per_100km: {:.3} > {:.3}",
            candidate.envelope.declared_consumption_kwh_per_100km,
            target.maximum_declared_consumption_kwh_per_100km
        ));
    }
    if candidate.envelope.curb_mass_kg > target.maximum_curb_mass_kg {
        findings.push(format!(
            "declared_curb_mass_above_target_kg: {:.3} > {:.3}",
            candidate.envelope.curb_mass_kg, target.maximum_curb_mass_kg
        ));
    }
    findings
}

fn evaluate_candidate(
    candidate: &Candidate,
    catalog: &Catalog,
    oem_net_revenue: f64,
    maximum_cogs: f64,
    retail_price_ceiling: f64,
) -> CandidateResult {
    let mut components = candidate.components.clone();
    components.sort_by(|a, b| a.component_id.cmp(&b.component_id));

    let bom_low = components
        .iter()
        .map(|component| component.quantity_per_vehicle * component.unit_cost.low)
        .sum::<f64>();
    let bom_high = components
        .iter()
        .map(|component| component.quantity_per_vehicle * component.unit_cost.high)
        .sum::<f64>();
    let bom_good_low = bom_low / candidate.manufacturing_yield_fraction;
    let bom_good_high = bom_high / candidate.manufacturing_yield_fraction;
    let fixed_low = candidate.fixed_program_investment.low / candidate.amortization_units as f64;
    let fixed_high = candidate.fixed_program_investment.high / candidate.amortization_units as f64;
    let cogs_low = bom_good_low
        + candidate.factory_conversion_cost.low
        + candidate.warranty_service_reserve.low
        + fixed_low;
    let cogs_high = bom_good_high
        + candidate.factory_conversion_cost.high
        + candidate.warranty_service_reserve.high
        + fixed_high;

    let economics_result = if cogs_low > maximum_cogs {
        EconomicsResult::AboveTargetEvenAtLowCost
    } else if cogs_high <= maximum_cogs {
        EconomicsResult::WithinTargetEvenAtHighCost
    } else {
        EconomicsResult::RangeCrossesTargetCeiling
    };
    let required_revenue_low = cogs_low / (1.0 - catalog.market.target_oem_gross_margin_fraction);
    let required_revenue_high = cogs_high / (1.0 - catalog.market.target_oem_gross_margin_fraction);
    let price_factor = if catalog.market.retail_price_includes_vat {
        1.0 + catalog.market.vat_fraction
    } else {
        1.0
    };
    let implied_retail_low =
        required_revenue_low / (1.0 - catalog.market.distributor_margin_fraction) * price_factor;
    let implied_retail_high =
        required_revenue_high / (1.0 - catalog.market.distributor_margin_fraction) * price_factor;

    let mission_findings = declared_mission_findings(candidate, &catalog.target);
    let mission_fit = mission_findings.is_empty();
    let technical_gate_qualified = gate_is_qualified(&candidate.technical_gate, false);
    let safety_gate_qualified = gate_is_qualified(&candidate.safety_gate, true);
    let cost_inputs_release_grade = candidate
        .components
        .iter()
        .all(|component| component.unit_cost.evidence_class.is_cost_release_grade())
        && candidate
            .factory_conversion_cost
            .evidence_class
            .is_cost_release_grade()
        && candidate
            .warranty_service_reserve
            .evidence_class
            .is_cost_release_grade()
        && candidate
            .fixed_program_investment
            .evidence_class
            .is_cost_release_grade();
    let envelope_evidence_release_grade =
        candidate.envelope_evidence_class.is_metrics_release_grade()
            && !candidate.envelope_source_reference.trim().is_empty();

    let mut evidence_gaps = Vec::new();
    if !cost_inputs_release_grade {
        evidence_gaps.push(
            "one_or_more_cost_ranges_are_synthetic_estimated_literature_or_unknown".to_string(),
        );
    }
    if !envelope_evidence_release_grade {
        evidence_gaps.push("vehicle_envelope_lacks_release_grade_evidence".to_string());
    }
    if !technical_gate_qualified {
        evidence_gaps.push("engineering_gate_not_independently_qualified".to_string());
    }
    if !safety_gate_qualified {
        evidence_gaps.push("safety_gate_not_independently_qualified".to_string());
    }
    if !mission_fit {
        evidence_gaps.push("declared_vehicle_envelope_misses_one_or_more_targets".to_string());
    }

    let failed_gate = candidate.technical_gate.status == GateStatus::Failed
        || candidate.safety_gate.status == GateStatus::Failed;
    let evidence_backed_all = cost_inputs_release_grade
        && envelope_evidence_release_grade
        && technical_gate_qualified
        && safety_gate_qualified
        && mission_fit;

    let release_disposition = if failed_gate || !mission_fit {
        ReleaseDisposition::BlockedByDeclaredGate
    } else if evidence_backed_all && cogs_low > maximum_cogs {
        ReleaseDisposition::BlockedByEvidenceBackedCostEnvelope
    } else if evidence_backed_all && cogs_high <= maximum_cogs {
        ReleaseDisposition::EvidenceBackedCandidate
    } else {
        ReleaseDisposition::NeedsEvidence
    };

    let mut component_costs = components
        .iter()
        .map(|component| ComponentCostResult {
            component_id: component.component_id.clone(),
            quantity_per_vehicle: component.quantity_per_vehicle,
            quantity_unit: component.quantity_unit.clone(),
            unit_cost_basis: component.unit_cost_basis.clone(),
            unit_cost_low: component.unit_cost.low,
            unit_cost_high: component.unit_cost.high,
            bom_cost_low: component.quantity_per_vehicle * component.unit_cost.low,
            bom_cost_high: component.quantity_per_vehicle * component.unit_cost.high,
            evidence_class: component.unit_cost.evidence_class,
            source_reference: component.unit_cost.source_reference.clone(),
        })
        .collect::<Vec<_>>();
    component_costs.sort_by(|a, b| {
        b.bom_cost_high
            .total_cmp(&a.bom_cost_high)
            .then_with(|| a.component_id.cmp(&b.component_id))
    });

    CandidateResult {
        candidate_id: candidate.candidate_id.clone(),
        label: candidate.label.clone(),
        architecture: candidate.architecture.clone(),
        declared_mission_fit: mission_fit,
        mission_findings,
        engineering_gate_status: candidate.technical_gate.status,
        safety_gate_status: candidate.safety_gate.status,
        cost_inputs_release_grade,
        envelope_evidence_release_grade,
        technical_gate_evidence_qualified: technical_gate_qualified,
        safety_gate_evidence_qualified: safety_gate_qualified,
        economics_result,
        release_disposition,
        bill_of_materials_low: bom_low,
        bill_of_materials_high: bom_high,
        bom_per_good_vehicle_low: bom_good_low,
        bom_per_good_vehicle_high: bom_good_high,
        factory_conversion_cost_low: candidate.factory_conversion_cost.low,
        factory_conversion_cost_high: candidate.factory_conversion_cost.high,
        warranty_service_reserve_low: candidate.warranty_service_reserve.low,
        warranty_service_reserve_high: candidate.warranty_service_reserve.high,
        fixed_program_cost_per_vehicle_low: fixed_low,
        fixed_program_cost_per_vehicle_high: fixed_high,
        total_cost_of_goods_low: cogs_low,
        total_cost_of_goods_high: cogs_high,
        maximum_affordable_cost_of_goods_at_target: maximum_cogs,
        oem_net_revenue_at_target: oem_net_revenue,
        implied_retail_price_low: implied_retail_low,
        implied_retail_price_high: implied_retail_high,
        retail_price_gap_low: implied_retail_low - retail_price_ceiling,
        retail_price_gap_high: implied_retail_high - retail_price_ceiling,
        cost_headroom_low: maximum_cogs - cogs_low,
        cost_headroom_high: maximum_cogs - cogs_high,
        component_costs,
        evidence_gaps,
        notes: candidate.notes.clone(),
        claim_ceiling: CLAIM_CEILING,
    }
}

fn evaluate_catalog(catalog: &Catalog) -> Report {
    let (retail_price_ceiling, oem_net_revenue, maximum_cogs) = target_waterfall(catalog);
    let mut candidates = catalog.candidates.iter().collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    let results = candidates
        .into_iter()
        .map(|candidate| {
            evaluate_candidate(
                candidate,
                catalog,
                oem_net_revenue,
                maximum_cogs,
                retail_price_ceiling,
            )
        })
        .collect();

    Report {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        currency: catalog.currency.clone(),
        country: catalog.market.country.clone(),
        baseline_vehicle: catalog.market.baseline_vehicle.clone(),
        baseline_variant: catalog.market.baseline_variant.clone(),
        baseline_observed_retail_price: catalog.market.observed_retail_price,
        baseline_observed_on: catalog.market.observed_on.clone(),
        baseline_price_source_url: catalog.market.price_source_url.clone(),
        target_undercut_fraction: catalog.market.undercut_fraction,
        retail_price_ceiling,
        retail_price_includes_vat: catalog.market.retail_price_includes_vat,
        vat_fraction: catalog.market.vat_fraction,
        distributor_margin_fraction: catalog.market.distributor_margin_fraction,
        target_oem_gross_margin_fraction: catalog.market.target_oem_gross_margin_fraction,
        target_oem_net_revenue: oem_net_revenue,
        maximum_affordable_cost_of_goods: maximum_cogs,
        vehicle_target: VehicleTargetReport {
            minimum_seats: catalog.target.minimum_seats,
            minimum_payload_kg: catalog.target.minimum_payload_kg,
            minimum_declared_range_km: catalog.target.minimum_declared_range_km,
            maximum_declared_consumption_kwh_per_100km: catalog
                .target
                .maximum_declared_consumption_kwh_per_100km,
            maximum_curb_mass_kg: catalog.target.maximum_curb_mass_kg,
            comparison_note: catalog.target.comparison_note.clone(),
        },
        results,
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band(low: f64, high: f64, evidence_class: EvidenceClass) -> CostRange {
        CostRange {
            low,
            high,
            evidence_class,
            source_reference: "test://cost-source".to_string(),
            as_of: "2026-10-20".to_string(),
            note: "Test-only value".to_string(),
        }
    }

    fn gate(status: GateStatus, evidence_class: EvidenceClass) -> Gate {
        Gate {
            status,
            evidence_class,
            source_reference: "test://gate-evidence".to_string(),
            as_of: "2026-10-20".to_string(),
            note: "Test-only gate".to_string(),
        }
    }

    fn minimal_catalog() -> Catalog {
        Catalog {
            schema_version: "0.1.0".to_string(),
            benchmark_id: "EV-PASSENGER-COST-001".to_string(),
            currency: "ZAR".to_string(),
            market: Market {
                country: "South Africa".to_string(),
                baseline_vehicle: "Reference vehicle".to_string(),
                baseline_variant: "Reference trim".to_string(),
                observed_retail_price: 115.0,
                observed_on: "2026-10-20".to_string(),
                price_source_url: "https://example.invalid/price".to_string(),
                undercut_fraction: 0.0,
                retail_price_includes_vat: true,
                vat_fraction: 0.15,
                distributor_margin_fraction: 0.10,
                target_oem_gross_margin_fraction: 0.20,
            },
            target: VehicleTarget {
                minimum_seats: 5,
                minimum_payload_kg: 300.0,
                minimum_declared_range_km: 300.0,
                maximum_declared_consumption_kwh_per_100km: 12.0,
                maximum_curb_mass_kg: 1400.0,
                comparison_note: "Test target".to_string(),
            },
            candidates: vec![Candidate {
                candidate_id: "candidate-a".to_string(),
                label: "Test car".to_string(),
                architecture: "single motor".to_string(),
                envelope: VehicleEnvelope {
                    seats: 5,
                    nominal_pack_kwh: 42.0,
                    usable_pack_fraction: 0.90,
                    payload_capacity_kg: 350.0,
                    declared_range_km: 300.0,
                    declared_consumption_kwh_per_100km: 11.9,
                    curb_mass_kg: 1350.0,
                },
                manufacturing_yield_fraction: 1.0,
                amortization_units: 1,
                envelope_evidence_class: EvidenceClass::Measured,
                envelope_source_reference: "test://envelope".to_string(),
                envelope_as_of: "2026-10-20".to_string(),
                technical_gate: gate(GateStatus::Passed, EvidenceClass::Measured),
                safety_gate: gate(GateStatus::Passed, EvidenceClass::IndependentTest),
                components: vec![Component {
                    component_id: "core".to_string(),
                    quantity_per_vehicle: 1.0,
                    quantity_unit: "unit".to_string(),
                    unit_cost_basis: "ZAR_per_unit".to_string(),
                    unit_cost: band(50.0, 50.0, EvidenceClass::Measured),
                }],
                factory_conversion_cost: band(10.0, 10.0, EvidenceClass::Measured),
                warranty_service_reserve: band(8.0, 8.0, EvidenceClass::AuditedCost),
                fixed_program_investment: band(4.0, 4.0, EvidenceClass::AuditedCost),
                notes: vec!["Test note".to_string()],
            }],
            claim_ceiling: "Test only".to_string(),
        }
    }

    #[test]
    fn retail_waterfall_matches_hand_computed_oracle() {
        let catalog = minimal_catalog();
        let (ceiling, revenue, max_cogs) = target_waterfall(&catalog);
        assert!((ceiling - 115.0).abs() < 1e-9);
        assert!((revenue - 90.0).abs() < 1e-9);
        assert!((max_cogs - 72.0).abs() < 1e-9);
        let report = evaluate_catalog(&catalog);
        let candidate = &report.results[0];
        assert!((candidate.total_cost_of_goods_low - 72.0).abs() < 1e-9);
        assert!((candidate.implied_retail_price_low - 115.0).abs() < 1e-9);
        assert_eq!(
            candidate.release_disposition,
            ReleaseDisposition::EvidenceBackedCandidate
        );
    }

    #[test]
    fn lower_manufacturing_yield_cannot_lower_cogs() {
        let mut catalog = minimal_catalog();
        let baseline = evaluate_catalog(&catalog).results[0].total_cost_of_goods_high;
        catalog.candidates[0].manufacturing_yield_fraction = 0.8;
        let degraded = evaluate_catalog(&catalog).results[0].total_cost_of_goods_high;
        assert!(degraded > baseline);
    }

    #[test]
    fn more_fixed_program_investment_cannot_improve_headroom() {
        let mut catalog = minimal_catalog();
        let baseline = evaluate_catalog(&catalog).results[0].cost_headroom_high;
        catalog.candidates[0].fixed_program_investment.high = 40.0;
        let changed = evaluate_catalog(&catalog).results[0].cost_headroom_high;
        assert!(changed < baseline);
    }

    #[test]
    fn synthetic_costs_never_qualify_as_production_feasible() {
        let raw = include_str!("../fixtures/ev_passenger_cost_v0_1.json");
        let catalog: Catalog = serde_json::from_str(raw).expect("fixture parses");
        validate_catalog(&catalog).expect("fixture validates");
        let report = evaluate_catalog(&catalog);
        assert!(report.results.iter().all(|item| {
            !item.cost_inputs_release_grade
                && item.release_disposition == ReleaseDisposition::NeedsEvidence
        }));
    }

    #[test]
    fn unknown_safety_gate_never_qualifies() {
        let mut catalog = minimal_catalog();
        catalog.candidates[0].safety_gate = gate(GateStatus::Unknown, EvidenceClass::Unknown);
        let result = &evaluate_catalog(&catalog).results[0];
        assert!(!result.safety_gate_evidence_qualified);
        assert_eq!(
            result.release_disposition,
            ReleaseDisposition::NeedsEvidence
        );
    }

    #[test]
    fn candidate_and_component_permutations_keep_report_deterministic() {
        let raw = include_str!("../fixtures/ev_passenger_cost_v0_1.json");
        let original: Catalog = serde_json::from_str(raw).expect("fixture parses");
        let first = serde_json::to_string(&evaluate_catalog(&original)).expect("serialize");
        let mut permuted = original.clone();
        permuted.candidates.reverse();
        for candidate in &mut permuted.candidates {
            candidate.components.reverse();
        }
        let second = serde_json::to_string(&evaluate_catalog(&permuted)).expect("serialize");
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_component_ids_are_rejected() {
        let mut catalog = minimal_catalog();
        let first_component = catalog.candidates[0].components[0].clone();
        catalog.candidates[0].components.push(first_component);
        assert!(
            validate_catalog(&catalog)
                .unwrap_err()
                .contains("duplicate_component_id")
        );
    }

    #[test]
    fn invalid_yield_is_rejected() {
        let mut catalog = minimal_catalog();
        catalog.candidates[0].manufacturing_yield_fraction = 0.0;
        assert!(
            validate_catalog(&catalog)
                .unwrap_err()
                .contains("manufacturing_yield_fraction_must_be_positive")
        );
    }

    #[test]
    fn higher_cost_range_cannot_improve_affordability() {
        let mut catalog = minimal_catalog();
        let original = evaluate_catalog(&catalog).results[0].cost_headroom_high;
        catalog.candidates[0].components[0].unit_cost.high += 15.0;
        let changed = evaluate_catalog(&catalog).results[0].cost_headroom_high;
        assert!(changed < original);
    }

    #[test]
    fn synthetic_frontier_exposes_high_cost_shortfall_without_claiming_feasibility() {
        let raw = include_str!("../fixtures/ev_passenger_cost_v0_1.json");
        let catalog: Catalog = serde_json::from_str(raw).expect("fixture parses");
        let report = evaluate_catalog(&catalog);
        assert!((report.retail_price_ceiling - 307_710.0).abs() < 1e-9);
        assert!((report.maximum_affordable_cost_of_goods - 216_627.84).abs() < 1e-6);
        assert!(report.results.iter().all(|candidate| {
            candidate.economics_result == EconomicsResult::RangeCrossesTargetCeiling
                && candidate.release_disposition == ReleaseDisposition::NeedsEvidence
        }));
    }

    #[test]
    fn inadequate_usable_pack_energy_blocks_range_target() {
        let mut catalog = minimal_catalog();
        catalog.candidates[0].envelope.nominal_pack_kwh = 30.0;
        let findings = declared_mission_findings(&catalog.candidates[0], &catalog.target);
        assert!(findings.iter().any(|finding| {
            finding.starts_with("range_energy_exceeds_declared_usable_pack_kwh")
        }));
    }

    #[test]
    fn higher_inclusive_vat_cannot_increase_allowable_cogs() {
        let mut catalog = minimal_catalog();
        let baseline = target_waterfall(&catalog).2;
        catalog.market.vat_fraction = 0.20;
        let changed = target_waterfall(&catalog).2;
        assert!(changed < baseline);
    }

    #[test]
    fn higher_distributor_share_cannot_increase_allowable_cogs() {
        let mut catalog = minimal_catalog();
        let baseline = target_waterfall(&catalog).2;
        catalog.market.distributor_margin_fraction = 0.20;
        let changed = target_waterfall(&catalog).2;
        assert!(changed < baseline);
    }
}
