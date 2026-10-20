use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const CLAIM_CEILING: &str = "Scenario arithmetic only. This is not a customs ruling, tax opinion, incentive determination, supplier quote, audited BOM, manufacturing plan, homologation, safety finding, or proof of superiority.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    benchmark_id: String,
    currency: String,
    market: Market,
    routes: Vec<Route>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Market {
    country: String,
    baseline_vehicle: String,
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
struct Route {
    route_id: String,
    label: String,
    route_type: RouteType,
    volume_cases: Vec<u64>,
    manufacturing_yield_fraction: f64,
    yield_evidence_class: EvidenceClass,
    yield_source_reference: String,
    yield_as_of: String,
    imports: Vec<ImportLine>,
    local_components_cost: CostRange,
    factory_conversion_cost: CostRange,
    warranty_service_reserve: CostRange,
    fixed_program_investment: CostRange,
    vat_recovery_status: VatRecoveryStatus,
    vat_recovery_evidence: EvidenceClaim,
    local_value_method: String,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RouteType {
    CompleteImport,
    CkdAssembly,
    DeepLocalManufacturing,
    SharedPlatform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportLine {
    line_id: String,
    candidate_tariff_heading: String,
    purchase_value: CostRange,
    freight_insurance: CostRange,
    duty_rate: RateRange,
    classification: EvidenceClaim,
    origin: EvidenceClaim,
    customs_value_basis: String,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceClaim {
    verified: bool,
    evidence_class: EvidenceClass,
    source_reference: String,
    as_of: String,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RateRange {
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
    PublicMarketData,
    OfficialTariff,
    OfficialClassification,
    OfficialOriginDetermination,
    OfficialTaxTreatment,
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

    fn is_process_release_grade(self) -> bool {
        matches!(self, Self::Measured | Self::IndependentTest | Self::AuditedCost)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum VatRecoveryStatus {
    Unknown,
    Recoverable,
    Unrecoverable,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CostFrontier {
    AboveTargetEvenAtLowCost,
    RangeCrossesTargetCeiling,
    WithinTargetEvenAtHighCost,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EconomicsEvidenceStatus {
    NeedsEvidence,
    EvidenceBackedInputs,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct RouteResult {
    route_id: String,
    label: String,
    route_type: RouteType,
    volume_units: u64,
    manufacturing_yield_fraction: f64,
    imported_customs_value_low: f64,
    imported_customs_value_high: f64,
    import_duty_low: f64,
    import_duty_high: f64,
    import_landed_cost_low: f64,
    import_landed_cost_high: f64,
    import_vat_cash_exposure_low: f64,
    import_vat_cash_exposure_high: f64,
    local_components_cost_low: f64,
    local_components_cost_high: f64,
    factory_conversion_cost_low: f64,
    factory_conversion_cost_high: f64,
    local_spend_proxy_per_good_vehicle_low: f64,
    local_spend_proxy_per_good_vehicle_high: f64,
    warranty_service_reserve_low: f64,
    warranty_service_reserve_high: f64,
    fixed_program_cost_per_vehicle_low: f64,
    fixed_program_cost_per_vehicle_high: f64,
    cost_of_goods_if_vat_recovered_low: f64,
    cost_of_goods_if_vat_recovered_high: f64,
    cost_of_goods_if_vat_unrecoverable_low: f64,
    cost_of_goods_if_vat_unrecoverable_high: f64,
    maximum_affordable_cost_of_goods_at_target: f64,
    frontier_if_vat_recovered: CostFrontier,
    frontier_if_vat_unrecoverable: CostFrontier,
    frontier_given_declared_vat_status: Option<CostFrontier>,
    vat_recovery_status: VatRecoveryStatus,
    vat_recovery_evidence_qualified: bool,
    cost_inputs_release_grade: bool,
    tariff_treatments_qualified: bool,
    economics_evidence_status: EconomicsEvidenceStatus,
    evidence_gaps: Vec<String>,
    notes: Vec<String>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    currency: String,
    country: String,
    baseline_vehicle: String,
    baseline_observed_retail_price: f64,
    baseline_observed_on: String,
    baseline_price_source_url: String,
    target_retail_price_ceiling: f64,
    retail_price_includes_vat: bool,
    vat_fraction: f64,
    distributor_margin_fraction: f64,
    target_oem_gross_margin_fraction: f64,
    target_oem_net_revenue: f64,
    maximum_affordable_cost_of_goods: f64,
    result_count: usize,
    results: Vec<RouteResult>,
    claim_ceiling: String,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/ev-passenger-cost-frontier/fixtures/ev_localization_v0_1.json".to_string()
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
            eprintln!("invalid EV localization catalog: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_catalog(&catalog) {
        eprintln!("invalid EV localization catalog: {error}");
        process::exit(2);
    }

    let report = evaluate_catalog(&catalog);
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize localization frontier report: {error}");
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

fn fraction(name: &str, value: f64, allow_one: bool) -> Result<(), String> {
    let upper_bad = if allow_one { value > 1.0 } else { value >= 1.0 };
    if !value.is_finite() || value < 0.0 || upper_bad {
        return Err(format!("{name}_must_be_in_the_allowed_fraction_range"));
    }
    Ok(())
}

fn validate_evidence_claim(name: &str, claim: &EvidenceClaim) -> Result<(), String> {
    if claim.source_reference.trim().is_empty() || claim.as_of.trim().is_empty() {
        return Err(format!("{name}_requires_source_and_date"));
    }
    Ok(())
}

fn validate_range(name: &str, range: &CostRange) -> Result<(), String> {
    finite_nonnegative(&format!("{name}_low"), range.low)?;
    finite_nonnegative(&format!("{name}_high"), range.high)?;
    if range.low > range.high {
        return Err(format!("{name}_low_exceeds_high"));
    }
    if range.source_reference.trim().is_empty()
        || range.as_of.trim().is_empty()
        || range.note.trim().is_empty()
    {
        return Err(format!("{name}_requires_source_date_and_note"));
    }
    Ok(())
}

fn validate_rate(name: &str, rate: &RateRange) -> Result<(), String> {
    fraction(&format!("{name}_low"), rate.low, true)?;
    fraction(&format!("{name}_high"), rate.high, true)?;
    if rate.low > rate.high {
        return Err(format!("{name}_low_exceeds_high"));
    }
    if rate.source_reference.trim().is_empty()
        || rate.as_of.trim().is_empty()
        || rate.note.trim().is_empty()
    {
        return Err(format!("{name}_requires_source_date_and_note"));
    }
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != "0.1.0" {
        return Err("unsupported_schema_version".to_string());
    }
    if catalog.benchmark_id.trim().is_empty() || catalog.currency != "ZAR" {
        return Err("benchmark_id_must_be_nonempty_and_currency_must_be_ZAR".to_string());
    }
    if catalog.claim_ceiling.trim().is_empty() {
        return Err("claim_ceiling_must_be_nonempty".to_string());
    }
    let market = &catalog.market;
    finite_nonnegative("observed_retail_price", market.observed_retail_price)?;
    if market.observed_retail_price <= 0.0 {
        return Err("observed_retail_price_must_be_positive".to_string());
    }
    fraction("undercut_fraction", market.undercut_fraction, false)?;
    fraction("vat_fraction", market.vat_fraction, false)?;
    fraction("distributor_margin_fraction", market.distributor_margin_fraction, false)?;
    fraction("target_oem_gross_margin_fraction", market.target_oem_gross_margin_fraction, false)?;
    if market.distributor_margin_fraction + market.target_oem_gross_margin_fraction >= 1.0 {
        return Err("combined_distributor_and_oem_margins_must_be_below_one".to_string());
    }
    if market.country.trim().is_empty()
        || market.baseline_vehicle.trim().is_empty()
        || market.observed_on.trim().is_empty()
        || market.price_source_url.trim().is_empty()
    {
        return Err("market_baseline_requires_country_vehicle_date_and_source".to_string());
    }
    if catalog.routes.is_empty() {
        return Err("at_least_one_route_is_required".to_string());
    }

    let mut route_ids = BTreeSet::new();
    for route in &catalog.routes {
        if route.route_id.trim().is_empty() || !route_ids.insert(route.route_id.clone()) {
            return Err("route_ids_must_be_nonempty_and_unique".to_string());
        }
        if route.label.trim().is_empty() || route.local_value_method.trim().is_empty() {
            return Err(format!("route_{}_requires_label_and_local_value_method", route.route_id));
        }
        fraction(
            &format!("{}_manufacturing_yield_fraction", route.route_id),
            route.manufacturing_yield_fraction,
            true,
        )?;
        if route.manufacturing_yield_fraction <= 0.0 {
            return Err(format!("route_{}_yield_must_be_positive", route.route_id));
        }
        if route.yield_source_reference.trim().is_empty() || route.yield_as_of.trim().is_empty() {
            return Err(format!("route_{}_yield_requires_source_and_date", route.route_id));
        }
        validate_evidence_claim(
            &format!("route_{}_vat_recovery_evidence", route.route_id),
            &route.vat_recovery_evidence,
        )?;
        if route.volume_cases.is_empty() {
            return Err(format!("route_{}_requires_volume_cases", route.route_id));
        }
        let mut volumes = BTreeSet::new();
        for volume in &route.volume_cases {
            if *volume == 0 || !volumes.insert(*volume) {
                return Err(format!("route_{}_volume_cases_must_be_positive_and_unique", route.route_id));
            }
        }
        if route.imports.is_empty() {
            return Err(format!("route_{}_requires_import_lines", route.route_id));
        }
        let mut line_ids = BTreeSet::new();
        for line in &route.imports {
            if line.line_id.trim().is_empty() || !line_ids.insert(line.line_id.clone()) {
                return Err(format!("route_{}_import_line_ids_must_be_nonempty_and_unique", route.route_id));
            }
            if line.candidate_tariff_heading.trim().is_empty()
                || line.customs_value_basis.trim().is_empty()
            {
                return Err(format!("route_{}_line_{}_requires_tariff_and_customs_basis", route.route_id, line.line_id));
            }
            validate_range(
                &format!("{}_{}_purchase_value", route.route_id, line.line_id),
                &line.purchase_value,
            )?;
            validate_range(
                &format!("{}_{}_freight_insurance", route.route_id, line.line_id),
                &line.freight_insurance,
            )?;
            validate_rate(
                &format!("{}_{}_duty_rate", route.route_id, line.line_id),
                &line.duty_rate,
            )?;
            validate_evidence_claim(
                &format!("{}_{}_classification_evidence", route.route_id, line.line_id),
                &line.classification,
            )?;
            validate_evidence_claim(
                &format!("{}_{}_origin_evidence", route.route_id, line.line_id),
                &line.origin,
            )?;
        }
        validate_range(&format!("{}_local_components", route.route_id), &route.local_components_cost)?;
        validate_range(&format!("{}_factory_conversion", route.route_id), &route.factory_conversion_cost)?;
        validate_range(&format!("{}_warranty_service", route.route_id), &route.warranty_service_reserve)?;
        validate_range(&format!("{}_fixed_program_investment", route.route_id), &route.fixed_program_investment)?;
    }
    Ok(())
}

fn frontier(low: f64, high: f64, ceiling: f64) -> CostFrontier {
    if low > ceiling {
        CostFrontier::AboveTargetEvenAtLowCost
    } else if high <= ceiling {
        CostFrontier::WithinTargetEvenAtHighCost
    } else {
        CostFrontier::RangeCrossesTargetCeiling
    }
}

fn maximum_affordable_cogs(market: &Market) -> (f64, f64, f64) {
    let retail_ceiling = market.observed_retail_price * (1.0 - market.undercut_fraction);
    let ex_vat = if market.retail_price_includes_vat {
        retail_ceiling / (1.0 + market.vat_fraction)
    } else {
        retail_ceiling
    };
    let oem_net = ex_vat * (1.0 - market.distributor_margin_fraction);
    let cogs_ceiling = oem_net * (1.0 - market.target_oem_gross_margin_fraction);
    (retail_ceiling, oem_net, cogs_ceiling)
}

fn evaluate_catalog(catalog: &Catalog) -> Report {
    let (retail_ceiling, oem_net_revenue, cogs_ceiling) = maximum_affordable_cogs(&catalog.market);
    let mut routes: Vec<&Route> = catalog.routes.iter().collect();
    routes.sort_by(|a, b| a.route_id.cmp(&b.route_id));
    let mut results = Vec::new();

    for route in routes {
        let mut imports: Vec<&ImportLine> = route.imports.iter().collect();
        imports.sort_by(|a, b| a.line_id.cmp(&b.line_id));

        let mut customs_low = 0.0;
        let mut customs_high = 0.0;
        let mut duty_low = 0.0;
        let mut duty_high = 0.0;
        let mut landed_import_low = 0.0;
        let mut landed_import_high = 0.0;
        let mut import_vat_low = 0.0;
        let mut import_vat_high = 0.0;
        let mut tariffs_qualified = true;
        let mut cost_inputs_release_grade = true;
        let mut evidence_gaps = Vec::new();

        for line in imports {
            let line_customs_low = line.purchase_value.low + line.freight_insurance.low;
            let line_customs_high = line.purchase_value.high + line.freight_insurance.high;
            let line_duty_low = line_customs_low * line.duty_rate.low;
            let line_duty_high = line_customs_high * line.duty_rate.high;
            let line_landed_low = line_customs_low + line_duty_low;
            let line_landed_high = line_customs_high + line_duty_high;
            customs_low += line_customs_low;
            customs_high += line_customs_high;
            duty_low += line_duty_low;
            duty_high += line_duty_high;
            landed_import_low += line_landed_low;
            landed_import_high += line_landed_high;
            // Cash exposure only: the actual import-VAT assessment base and recovery need tax validation.
            import_vat_low += line_landed_low * catalog.market.vat_fraction;
            import_vat_high += line_landed_high * catalog.market.vat_fraction;

            if !line.purchase_value.evidence_class.is_cost_release_grade() {
                cost_inputs_release_grade = false;
                evidence_gaps.push(format!("{} purchase value is not quote/measured/audited evidence", line.line_id));
            }
            if !line.freight_insurance.evidence_class.is_cost_release_grade() {
                cost_inputs_release_grade = false;
                evidence_gaps.push(format!("{} freight/insurance is not quote/measured/audited evidence", line.line_id));
            }
            let classification_qualified = line.classification.verified
                && line.classification.evidence_class == EvidenceClass::OfficialClassification;
            let origin_qualified = line.origin.verified
                && line.origin.evidence_class == EvidenceClass::OfficialOriginDetermination;
            if !classification_qualified {
                tariffs_qualified = false;
                evidence_gaps.push(format!("{} tariff classification is unverified or lacks an authoritative classification record ({})", line.line_id, line.candidate_tariff_heading));
            }
            if !origin_qualified {
                tariffs_qualified = false;
                evidence_gaps.push(format!("{} country of origin/preference eligibility is unverified or lacks an authoritative origin record", line.line_id));
            }
            if line.duty_rate.evidence_class != EvidenceClass::OfficialTariff
                || !classification_qualified
                || !origin_qualified
            {
                tariffs_qualified = false;
                evidence_gaps.push(format!("{} duty treatment is scenario-only until rate, classification, and origin have qualified evidence", line.line_id));
            }
        }

        for (name, range) in [
            ("local component costs", &route.local_components_cost),
            ("factory conversion costs", &route.factory_conversion_cost),
            ("warranty/service reserve", &route.warranty_service_reserve),
            ("fixed program investment", &route.fixed_program_investment),
        ] {
            if !range.evidence_class.is_cost_release_grade() {
                cost_inputs_release_grade = false;
                evidence_gaps.push(format!("{name} lack quote/measured/audited cost evidence"));
            }
        }
        if !route.yield_evidence_class.is_process_release_grade() {
            evidence_gaps.push("manufacturing yield is not supported by measured process evidence".to_string());
        }
        let vat_recovery_evidence_qualified = route.vat_recovery_status != VatRecoveryStatus::Unknown
            && route.vat_recovery_evidence.verified
            && route.vat_recovery_evidence.evidence_class == EvidenceClass::OfficialTaxTreatment;
        if route.vat_recovery_status == VatRecoveryStatus::Unknown {
            evidence_gaps.push("import VAT recovery status is unresolved; both cost frontiers are shown".to_string());
        } else if !vat_recovery_evidence_qualified {
            evidence_gaps.push("VAT recovery status is declared without a verified authoritative tax-treatment record".to_string());
        }

        let variable_low = landed_import_low + route.local_components_cost.low + route.factory_conversion_cost.low;
        let variable_high = landed_import_high + route.local_components_cost.high + route.factory_conversion_cost.high;
        let fixed_low = route.fixed_program_investment.low;
        let fixed_high = route.fixed_program_investment.high;
        for volume in {
            let mut values = route.volume_cases.clone();
            values.sort_unstable();
            values
        } {
            let program_low = fixed_low / volume as f64;
            let program_high = fixed_high / volume as f64;
            let cogs_recovered_low = variable_low / route.manufacturing_yield_fraction
                + route.warranty_service_reserve.low
                + program_low;
            let cogs_recovered_high = variable_high / route.manufacturing_yield_fraction
                + route.warranty_service_reserve.high
                + program_high;
            let cogs_unrecoverable_low = cogs_recovered_low + import_vat_low / route.manufacturing_yield_fraction;
            let cogs_unrecoverable_high = cogs_recovered_high + import_vat_high / route.manufacturing_yield_fraction;
            let selected_frontier = match (route.vat_recovery_status, vat_recovery_evidence_qualified) {
                (VatRecoveryStatus::Recoverable, true) => Some(frontier(cogs_recovered_low, cogs_recovered_high, cogs_ceiling)),
                (VatRecoveryStatus::Unrecoverable, true) => Some(frontier(cogs_unrecoverable_low, cogs_unrecoverable_high, cogs_ceiling)),
                _ => None,
            };

            if !route.yield_evidence_class.is_process_release_grade() {
                // A synthetic yield may shape a sensitivity, but cannot upgrade the route to evidence-backed.
            }
            let evidence_status = if cost_inputs_release_grade
                && tariffs_qualified
                && route.yield_evidence_class.is_process_release_grade()
                && vat_recovery_evidence_qualified
            {
                EconomicsEvidenceStatus::EvidenceBackedInputs
            } else {
                EconomicsEvidenceStatus::NeedsEvidence
            };

            results.push(RouteResult {
                route_id: route.route_id.clone(),
                label: route.label.clone(),
                route_type: route.route_type,
                volume_units: volume,
                manufacturing_yield_fraction: route.manufacturing_yield_fraction,
                imported_customs_value_low: customs_low,
                imported_customs_value_high: customs_high,
                import_duty_low: duty_low,
                import_duty_high: duty_high,
                import_landed_cost_low: landed_import_low,
                import_landed_cost_high: landed_import_high,
                import_vat_cash_exposure_low: import_vat_low,
                import_vat_cash_exposure_high: import_vat_high,
                local_components_cost_low: route.local_components_cost.low,
                local_components_cost_high: route.local_components_cost.high,
                factory_conversion_cost_low: route.factory_conversion_cost.low,
                factory_conversion_cost_high: route.factory_conversion_cost.high,
                local_spend_proxy_per_good_vehicle_low: (route.local_components_cost.low + route.factory_conversion_cost.low) / route.manufacturing_yield_fraction,
                local_spend_proxy_per_good_vehicle_high: (route.local_components_cost.high + route.factory_conversion_cost.high) / route.manufacturing_yield_fraction,
                warranty_service_reserve_low: route.warranty_service_reserve.low,
                warranty_service_reserve_high: route.warranty_service_reserve.high,
                fixed_program_cost_per_vehicle_low: program_low,
                fixed_program_cost_per_vehicle_high: program_high,
                cost_of_goods_if_vat_recovered_low: cogs_recovered_low,
                cost_of_goods_if_vat_recovered_high: cogs_recovered_high,
                cost_of_goods_if_vat_unrecoverable_low: cogs_unrecoverable_low,
                cost_of_goods_if_vat_unrecoverable_high: cogs_unrecoverable_high,
                maximum_affordable_cost_of_goods_at_target: cogs_ceiling,
                frontier_if_vat_recovered: frontier(cogs_recovered_low, cogs_recovered_high, cogs_ceiling),
                frontier_if_vat_unrecoverable: frontier(cogs_unrecoverable_low, cogs_unrecoverable_high, cogs_ceiling),
                frontier_given_declared_vat_status: selected_frontier,
                vat_recovery_status: route.vat_recovery_status,
                vat_recovery_evidence_qualified,
                cost_inputs_release_grade,
                tariff_treatments_qualified: tariffs_qualified,
                economics_evidence_status: evidence_status,
                evidence_gaps: evidence_gaps.clone(),
                notes: route.notes.clone(),
                claim_ceiling: CLAIM_CEILING.to_string(),
            });
        }
    }

    Report {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        currency: catalog.currency.clone(),
        country: catalog.market.country.clone(),
        baseline_vehicle: catalog.market.baseline_vehicle.clone(),
        baseline_observed_retail_price: catalog.market.observed_retail_price,
        baseline_observed_on: catalog.market.observed_on.clone(),
        baseline_price_source_url: catalog.market.price_source_url.clone(),
        target_retail_price_ceiling: retail_ceiling,
        retail_price_includes_vat: catalog.market.retail_price_includes_vat,
        vat_fraction: catalog.market.vat_fraction,
        distributor_margin_fraction: catalog.market.distributor_margin_fraction,
        target_oem_gross_margin_fraction: catalog.market.target_oem_gross_margin_fraction,
        target_oem_net_revenue: oem_net_revenue,
        maximum_affordable_cost_of_goods: cogs_ceiling,
        result_count: results.len(),
        results,
        claim_ceiling: catalog.claim_ceiling.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cost(low: f64, high: f64, evidence_class: EvidenceClass) -> CostRange {
        CostRange {
            low,
            high,
            evidence_class,
            source_reference: "test://cost".to_string(),
            as_of: "2026-10-20".to_string(),
            note: "test-only input".to_string(),
        }
    }

    fn rate(low: f64, high: f64, evidence_class: EvidenceClass) -> RateRange {
        RateRange {
            low,
            high,
            evidence_class,
            source_reference: "test://rate".to_string(),
            as_of: "2026-10-20".to_string(),
            note: "test-only rate".to_string(),
        }
    }

    fn oracle_catalog() -> Catalog {
        Catalog {
            schema_version: "0.1.0".to_string(),
            benchmark_id: "TEST-LOCALIZATION".to_string(),
            currency: "ZAR".to_string(),
            market: Market {
                country: "South Africa".to_string(),
                baseline_vehicle: "test".to_string(),
                observed_retail_price: 1000.0,
                observed_on: "2026-10-20".to_string(),
                price_source_url: "test://market".to_string(),
                undercut_fraction: 0.1,
                retail_price_includes_vat: true,
                vat_fraction: 0.15,
                distributor_margin_fraction: 0.08,
                target_oem_gross_margin_fraction: 0.12,
            },
            routes: vec![Route {
                route_id: "oracle".to_string(),
                label: "Oracle route".to_string(),
                route_type: RouteType::CompleteImport,
                volume_cases: vec![100],
                manufacturing_yield_fraction: 0.9,
                yield_evidence_class: EvidenceClass::Measured,
                yield_source_reference: "test://yield".to_string(),
                yield_as_of: "2026-10-20".to_string(),
                imports: vec![ImportLine {
                    line_id: "car".to_string(),
                    candidate_tariff_heading: "test-heading".to_string(),
                    purchase_value: cost(100.0, 120.0, EvidenceClass::ManufacturerQuote),
                    freight_insurance: cost(10.0, 20.0, EvidenceClass::ManufacturerQuote),
                    duty_rate: rate(0.25, 0.25, EvidenceClass::OfficialTariff),
                    classification: EvidenceClaim {
                        verified: true,
                        evidence_class: EvidenceClass::OfficialClassification,
                        source_reference: "test://classification".to_string(),
                        as_of: "2026-10-20".to_string(),
                    },
                    origin: EvidenceClaim {
                        verified: true,
                        evidence_class: EvidenceClass::OfficialOriginDetermination,
                        source_reference: "test://origin".to_string(),
                        as_of: "2026-10-20".to_string(),
                    },
                    customs_value_basis: "test purchase plus freight".to_string(),
                    notes: vec![],
                }],
                local_components_cost: cost(20.0, 30.0, EvidenceClass::ManufacturerQuote),
                factory_conversion_cost: cost(10.0, 10.0, EvidenceClass::ManufacturerQuote),
                warranty_service_reserve: cost(5.0, 5.0, EvidenceClass::AuditedCost),
                fixed_program_investment: cost(900.0, 900.0, EvidenceClass::AuditedCost),
                vat_recovery_status: VatRecoveryStatus::Recoverable,
                vat_recovery_evidence: EvidenceClaim {
                    verified: true,
                    evidence_class: EvidenceClass::OfficialTaxTreatment,
                    source_reference: "test://tax-treatment".to_string(),
                    as_of: "2026-10-20".to_string(),
                },
                local_value_method: "internal cost proxy only".to_string(),
                notes: vec![],
            }],
            claim_ceiling: CLAIM_CEILING.to_string(),
        }
    }

    #[test]
    fn retail_to_affordable_cogs_inversion_matches_hand_calculation() {
        let catalog = oracle_catalog();
        let (retail, oem_net, cogs) = maximum_affordable_cogs(&catalog.market);
        assert!((retail - 900.0).abs() < 1e-9);
        assert!((oem_net - 720.0).abs() < 1e-9);
        assert!((cogs - 633.6).abs() < 1e-9);
    }

    #[test]
    fn duty_freight_yield_and_program_volume_are_visible_in_the_equations() {
        let report = evaluate_catalog(&oracle_catalog());
        let result = &report.results[0];
        assert!((result.imported_customs_value_low - 110.0).abs() < 1e-9);
        assert!((result.imported_customs_value_high - 140.0).abs() < 1e-9);
        assert!((result.import_duty_low - 27.5).abs() < 1e-9);
        assert!((result.import_duty_high - 35.0).abs() < 1e-9);
        assert!((result.import_landed_cost_low - 137.5).abs() < 1e-9);
        assert!((result.import_landed_cost_high - 175.0).abs() < 1e-9);
        assert!((result.import_vat_cash_exposure_low - 20.625).abs() < 1e-9);
        assert!((result.import_vat_cash_exposure_high - 26.25).abs() < 1e-9);
        assert!((result.fixed_program_cost_per_vehicle_low - 9.0).abs() < 1e-9);
        assert!((result.cost_of_goods_if_vat_recovered_low - 200.11111111111111).abs() < 1e-6);
        assert!((result.cost_of_goods_if_vat_recovered_high - 252.88888888888889).abs() < 1e-6);
    }

    #[test]
    fn additional_duty_never_improves_cogs() {
        let mut low = oracle_catalog();
        let mut high = oracle_catalog();
        high.routes[0].imports[0].duty_rate = rate(0.3, 0.3, EvidenceClass::OfficialTariff);
        let a = evaluate_catalog(&low).results.remove(0);
        let b = evaluate_catalog(&high).results.remove(0);
        assert!(b.cost_of_goods_if_vat_recovered_low > a.cost_of_goods_if_vat_recovered_low);
        assert!(b.cost_of_goods_if_vat_recovered_high > a.cost_of_goods_if_vat_recovered_high);
        low.routes[0].imports[0].freight_insurance = cost(20.0, 30.0, EvidenceClass::ManufacturerQuote);
        let c = evaluate_catalog(&low).results.remove(0);
        assert!(c.cost_of_goods_if_vat_recovered_low > a.cost_of_goods_if_vat_recovered_low);
    }

    #[test]
    fn fixed_program_cost_per_vehicle_falls_with_declared_volume() {
        let mut catalog = oracle_catalog();
        catalog.routes[0].volume_cases = vec![1000, 100];
        let report = evaluate_catalog(&catalog);
        assert_eq!(report.results.len(), 2);
        assert!(report.results[1].fixed_program_cost_per_vehicle_low < report.results[0].fixed_program_cost_per_vehicle_low);
        assert_eq!(report.results[0].volume_units, 100);
        assert_eq!(report.results[1].volume_units, 1000);
    }

    #[test]
    fn unresolved_vat_yields_two_scenarios_and_no_selected_frontier() {
        let mut catalog = oracle_catalog();
        catalog.routes[0].vat_recovery_status = VatRecoveryStatus::Unknown;
        catalog.routes[0].vat_recovery_evidence.verified = false;
        catalog.routes[0].vat_recovery_evidence.evidence_class = EvidenceClass::Unknown;
        let result = evaluate_catalog(&catalog).results.remove(0);
        assert!(result.cost_of_goods_if_vat_unrecoverable_high > result.cost_of_goods_if_vat_recovered_high);
        assert!(result.frontier_given_declared_vat_status.is_none());
        assert!(result.evidence_gaps.iter().any(|gap| gap.contains("VAT recovery status is unresolved")));
        assert_eq!(result.economics_evidence_status, EconomicsEvidenceStatus::NeedsEvidence);
    }

    #[test]
    fn missing_origin_or_classification_blocks_tariff_qualification() {
        let mut catalog = oracle_catalog();
        catalog.routes[0].imports[0].origin.verified = false;
        catalog.routes[0].imports[0].origin.evidence_class = EvidenceClass::Unknown;
        let result = evaluate_catalog(&catalog).results.remove(0);
        assert!(!result.tariff_treatments_qualified);
        assert_eq!(result.economics_evidence_status, EconomicsEvidenceStatus::NeedsEvidence);
    }

    #[test]
    fn synthetic_costs_never_become_evidence_backed_inputs() {
        let mut catalog = oracle_catalog();
        catalog.routes[0].imports[0].purchase_value.evidence_class = EvidenceClass::Synthetic;
        let result = evaluate_catalog(&catalog).results.remove(0);
        assert!(!result.cost_inputs_release_grade);
        assert_eq!(result.economics_evidence_status, EconomicsEvidenceStatus::NeedsEvidence);
    }

    #[test]
    fn volume_and_import_order_do_not_change_sorted_report() {
        let mut first = oracle_catalog();
        first.routes[0].volume_cases = vec![1000, 100];
        first.routes[0].imports.push(ImportLine {
            line_id: "battery".to_string(),
            candidate_tariff_heading: "test-battery".to_string(),
            purchase_value: cost(50.0, 60.0, EvidenceClass::ManufacturerQuote),
            freight_insurance: cost(2.0, 3.0, EvidenceClass::ManufacturerQuote),
            duty_rate: rate(0.0, 0.0, EvidenceClass::OfficialTariff),
            classification: EvidenceClaim {
                verified: true,
                evidence_class: EvidenceClass::OfficialClassification,
                source_reference: "test://classification-battery".to_string(),
                as_of: "2026-10-20".to_string(),
            },
            origin: EvidenceClaim {
                verified: true,
                evidence_class: EvidenceClass::OfficialOriginDetermination,
                source_reference: "test://origin-battery".to_string(),
                as_of: "2026-10-20".to_string(),
            },
            customs_value_basis: "test purchase plus freight".to_string(),
            notes: vec![],
        });
        let mut second = first.clone();
        second.routes[0].imports.reverse();
        second.routes[0].volume_cases.reverse();
        assert_eq!(evaluate_catalog(&first), evaluate_catalog(&second));
    }

    #[test]
    fn validation_rejects_invalid_rates_yield_and_volume() {
        let mut bad_rate = oracle_catalog();
        bad_rate.routes[0].imports[0].duty_rate.high = 1.2;
        assert!(validate_catalog(&bad_rate).is_err());

        let mut bad_yield = oracle_catalog();
        bad_yield.routes[0].manufacturing_yield_fraction = 0.0;
        assert!(validate_catalog(&bad_yield).is_err());

        let mut bad_volume = oracle_catalog();
        bad_volume.routes[0].volume_cases = vec![100, 100];
        assert!(validate_catalog(&bad_volume).is_err());
    }
}
