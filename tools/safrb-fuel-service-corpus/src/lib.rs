#![forbid(unsafe_code)]

//! Deterministic, synthetic-only fuel-to-service corpus generator.
//! No operational data, dispatch authority, physical solver or field claim.

use std::collections::BTreeSet;

pub const SCHEMA_VERSION: &str = "0.1.0";
pub const HORIZON_HOURS: u32 = 72;
pub const SEEDS_PER_ARCHETYPE: u64 = 10;
pub const SYNTHETIC_LHV_MILLI_WH_PER_GRAM: u64 = 11_667;
pub const SYNTHETIC_EFFICIENCY_MIN_BPS: u64 = 3_200;
pub const SYNTHETIC_EFFICIENCY_SPAN_BPS: u64 = 800;
pub const SYNTHETIC_POWER_CAPACITY_KW: u64 = 100;
pub const SERVICE_DEMAND_WH: u64 = 4_000_000;
pub const SERVICE_CRITICAL_FLOOR_BPS: u64 = 8_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Archetype {
    BaselineAllPaths,
    CrudeCargoDelay,
    RefinedDieselCargoDelay,
    TankQualityQuarantine,
    TankGaugeUncertainty,
    PumpPipelineFailure,
    BerthDischargeRestriction,
    RefineryProcessTrip,
    GeneratorDerating,
    GeneratorFuelGradeMismatch,
    GridIslandedBackupConstraint,
    CommonCauseSupplierPortGridComms,
}

impl Archetype {
    pub const ALL: [Self; 12] = [
        Self::BaselineAllPaths,
        Self::CrudeCargoDelay,
        Self::RefinedDieselCargoDelay,
        Self::TankQualityQuarantine,
        Self::TankGaugeUncertainty,
        Self::PumpPipelineFailure,
        Self::BerthDischargeRestriction,
        Self::RefineryProcessTrip,
        Self::GeneratorDerating,
        Self::GeneratorFuelGradeMismatch,
        Self::GridIslandedBackupConstraint,
        Self::CommonCauseSupplierPortGridComms,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::BaselineAllPaths => "baseline-all-paths",
            Self::CrudeCargoDelay => "crude-cargo-delay",
            Self::RefinedDieselCargoDelay => "refined-diesel-cargo-delay",
            Self::TankQualityQuarantine => "tank-quality-quarantine",
            Self::TankGaugeUncertainty => "tank-gauge-uncertainty",
            Self::PumpPipelineFailure => "pump-pipeline-failure",
            Self::BerthDischargeRestriction => "berth-discharge-restriction",
            Self::RefineryProcessTrip => "refinery-process-trip",
            Self::GeneratorDerating => "generator-derating",
            Self::GeneratorFuelGradeMismatch => "generator-fuel-grade-mismatch",
            Self::GridIslandedBackupConstraint => "grid-islanded-backup-constraint",
            Self::CommonCauseSupplierPortGridComms => "common-cause-supplier-port-grid-comms",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Product {
    CrudeOil,
    Diesel,
}

impl Product {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CrudeOil => "crude_oil",
            Self::Diesel => "diesel",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryState {
    FreshSynthetic,
    StaleOrConflicting,
}

impl TelemetryState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FreshSynthetic => "fresh_synthetic",
            Self::StaleOrConflicting => "stale_or_conflicting",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceDisposition {
    Guaranteed,
    ConfirmedShortfall,
    Indeterminate,
}

impl ServiceDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guaranteed => "guaranteed",
            Self::ConfirmedShortfall => "confirmed_shortfall",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventoryLedger {
    pub product: Product,
    pub opening_grams: u64,
    pub accepted_inflow_grams: u64,
    pub withdrawals_grams: u64,
    pub declared_losses_grams: u64,
    pub closing_grams: u64,
    pub measurement_uncertainty_basis_points: u64,
    pub observed_lower_grams: u64,
    pub observed_upper_grams: u64,
    pub quarantined_grams: u64,
    pub operating_heel_grams: u64,
    pub usable_grams: u64,
    pub encumbered_grams: u64,
    pub authority_available: bool,
    pub accessible_grams: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportPath {
    pub cargo_arrival_hour: u32,
    pub transit_delay_hours: u32,
    pub primary_capacity_grams_per_hour: u64,
    pub alternate_capacity_grams_per_hour: u64,
    pub effective_capacity_grams_per_hour: u64,
    pub deliverable_grams: u64,
    pub common_cause_group: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionReceipt {
    pub input_product: Product,
    pub output_product: Product,
    pub input_grams: u64,
    pub output_grams: u64,
    pub declared_process_loss_grams: u64,
    pub process_capacity_grams: u64,
    pub yield_basis_points: u64,
    pub process_model_id: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratorReceipt {
    pub fuel_grade_compatible: bool,
    pub fuel_consumed_grams: u64,
    pub synthetic_lhv_milli_wh_per_gram: u64,
    pub efficiency_basis_points: u64,
    pub rated_power_capacity_kw: u64,
    pub derate_basis_points: u64,
    pub network_delivery_basis_points: u64,
    pub gross_electricity_wh: u64,
    pub network_deliverable_wh: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceOutcome {
    pub demand_wh: u64,
    pub critical_floor_wh: u64,
    pub delivered_lower_wh: u64,
    pub delivered_upper_wh: u64,
    pub uncertainty_basis_points: u64,
    pub telemetry_state: TelemetryState,
    pub disposition: ServiceDisposition,
    pub evidence_maturity: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FuelServiceCase {
    pub schema_version: &'static str,
    pub case_id: String,
    pub archetype: Archetype,
    pub seed: u64,
    pub scenario_start_unix_s: u64,
    pub horizon_hours: u32,
    pub inventory: InventoryLedger,
    pub transport: TransportPath,
    pub conversion: ConversionReceipt,
    pub generator: GeneratorReceipt,
    pub service: ServiceOutcome,
}

fn mix_seed(seed: u64) -> u64 {
    let mut value = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn service_disposition(
    telemetry_state: TelemetryState,
    lower: u64,
    upper: u64,
    floor: u64,
) -> ServiceDisposition {
    if telemetry_state == TelemetryState::StaleOrConflicting {
        ServiceDisposition::Indeterminate
    } else if lower >= floor {
        ServiceDisposition::Guaranteed
    } else if upper < floor {
        ServiceDisposition::ConfirmedShortfall
    } else {
        ServiceDisposition::Indeterminate
    }
}

pub fn generate_case(archetype: Archetype, seed: u64) -> FuelServiceCase {
    let archetype_index = Archetype::ALL.iter().position(|item| *item == archetype).unwrap_or(0) as u64;
    let noise = mix_seed(seed ^ (archetype_index << 32));
    let inventory_product = match archetype {
        Archetype::CrudeCargoDelay | Archetype::RefineryProcessTrip => Product::CrudeOil,
        _ => Product::Diesel,
    };

    let cargo_arrival_hour = match archetype {
        Archetype::BaselineAllPaths => 12,
        Archetype::CrudeCargoDelay => 96,
        Archetype::RefinedDieselCargoDelay => 84,
        Archetype::BerthDischargeRestriction => 96,
        Archetype::CommonCauseSupplierPortGridComms => 120,
        _ => 24,
    };
    let accepted_inflow_grams = if cargo_arrival_hour < HORIZON_HOURS {
        1_500_000 + (noise % 100_000)
    } else {
        0
    };

    let opening_grams = 10_000_000 + (noise % 200_000);
    let withdrawals_grams = 1_000_000 + (noise % 100_000);
    let declared_losses_grams = 10_000 + (noise % 2_000);
    let closing_grams = opening_grams
        .checked_add(accepted_inflow_grams)
        .and_then(|value| value.checked_sub(withdrawals_grams))
        .and_then(|value| value.checked_sub(declared_losses_grams))
        .expect("bounded synthetic inventory arithmetic");

    let measurement_uncertainty_basis_points = if archetype == Archetype::TankGaugeUncertainty {
        2_000
    } else {
        500
    };
    let observed_lower_grams = closing_grams
        .saturating_mul(10_000 - measurement_uncertainty_basis_points)
        / 10_000;
    let observed_upper_grams = closing_grams
        .saturating_mul(10_000 + measurement_uncertainty_basis_points)
        / 10_000;
    let quarantined_grams = if archetype == Archetype::TankQualityQuarantine { 7_000_000 } else { 0 };
    let operating_heel_grams = 100_000;
    // The conservative observed lower quantity, not hidden synthetic truth,
    // controls the amount admitted as usable and accessible.
    let usable_grams = observed_lower_grams
        .checked_sub(quarantined_grams)
        .and_then(|value| value.checked_sub(operating_heel_grams))
        .expect("synthetic usable stock does not underflow");
    let authority_available = seed % SEEDS_PER_ARCHETYPE != 7;
    let encumbered_grams = if seed % 4 == 0 { usable_grams / 10 } else { 0 };
    let accessible_grams = if authority_available { usable_grams - encumbered_grams } else { 0 };

    let (primary_capacity, alternate_capacity, transit_delay, common_cause_group) =
        match archetype {
            Archetype::PumpPipelineFailure => (0, 12_000, 12, "pump-pipeline"),
            Archetype::BerthDischargeRestriction => (10_000, 0, 48, "port-berth"),
            Archetype::CommonCauseSupplierPortGridComms => (5_000, 0, 48, "supplier-port-grid-telecom"),
            _ => (60_000, 0, 4, "none"),
        };
    let effective_capacity = primary_capacity + alternate_capacity;
    let effective_hours = HORIZON_HOURS.saturating_sub(transit_delay);
    let transport_limit = effective_capacity.saturating_mul(u64::from(effective_hours));
    let deliverable_grams = accessible_grams.min(transport_limit);

    let process_capacity_grams = if archetype == Archetype::RefineryProcessTrip { 0 } else { 3_000_000 };
    let conversion_input_grams = deliverable_grams.min(process_capacity_grams);
    let yield_basis_points = match inventory_product {
        Product::CrudeOil => 4_500,
        Product::Diesel => 10_000,
    };
    let conversion_output_grams = conversion_input_grams.saturating_mul(yield_basis_points) / 10_000;
    let declared_process_loss_grams = conversion_input_grams.saturating_sub(conversion_output_grams);

    let fuel_grade_compatible = archetype != Archetype::GeneratorFuelGradeMismatch;
    let planned_fuel_grams = if fuel_grade_compatible { conversion_output_grams.min(900_000) } else { 0 };
    let efficiency_basis_points = SYNTHETIC_EFFICIENCY_MIN_BPS + (noise % SYNTHETIC_EFFICIENCY_SPAN_BPS);
    let fuel_energy_milli_wh = planned_fuel_grams.saturating_mul(SYNTHETIC_LHV_MILLI_WH_PER_GRAM);
    let generated_from_fuel_wh =
        fuel_energy_milli_wh.saturating_mul(efficiency_basis_points) / 10_000 / 1_000;
    let derate_basis_points = if archetype == Archetype::GeneratorDerating {
        4_000
    } else if archetype == Archetype::CommonCauseSupplierPortGridComms {
        6_000
    } else {
        10_000
    };
    let rated_capacity_wh = SYNTHETIC_POWER_CAPACITY_KW
        .saturating_mul(1_000)
        .saturating_mul(u64::from(HORIZON_HOURS))
        .saturating_mul(derate_basis_points) / 10_000;
    let gross_electricity_wh = generated_from_fuel_wh.min(rated_capacity_wh);
    let network_delivery_basis_points = match archetype {
        Archetype::GridIslandedBackupConstraint => 4_500,
        Archetype::CommonCauseSupplierPortGridComms => 3_000,
        _ => 10_000,
    };
    let network_deliverable_wh =
        gross_electricity_wh.saturating_mul(network_delivery_basis_points) / 10_000;

    let demand_wh = SERVICE_DEMAND_WH + (noise % 100_000);
    let critical_floor_wh = demand_wh.saturating_mul(SERVICE_CRITICAL_FLOOR_BPS) / 10_000;
    let uncertainty_basis_points = 500;
    let delivered_lower_wh =
        network_deliverable_wh.saturating_mul(10_000 - uncertainty_basis_points) / 10_000;
    let delivered_upper_wh = demand_wh.min(
        network_deliverable_wh.saturating_mul(10_000 + uncertainty_basis_points) / 10_000,
    );
    let telemetry_state = if seed % SEEDS_PER_ARCHETYPE == 9 {
        TelemetryState::StaleOrConflicting
    } else {
        TelemetryState::FreshSynthetic
    };
    let disposition = service_disposition(
        telemetry_state,
        delivered_lower_wh,
        delivered_upper_wh,
        critical_floor_wh,
    );

    FuelServiceCase {
        schema_version: SCHEMA_VERSION,
        case_id: format!("safrb-fuel-v0.1-{}-{:02}", archetype.slug(), seed % 100),
        archetype,
        seed,
        scenario_start_unix_s: 1_800_000_000,
        horizon_hours: HORIZON_HOURS,
        inventory: InventoryLedger {
            product: inventory_product,
            opening_grams,
            accepted_inflow_grams,
            withdrawals_grams,
            declared_losses_grams,
            closing_grams,
            measurement_uncertainty_basis_points,
            observed_lower_grams,
            observed_upper_grams,
            quarantined_grams,
            operating_heel_grams,
            usable_grams,
            encumbered_grams,
            authority_available,
            accessible_grams,
        },
        transport: TransportPath {
            cargo_arrival_hour,
            transit_delay_hours: transit_delay,
            primary_capacity_grams_per_hour: primary_capacity,
            alternate_capacity_grams_per_hour: alternate_capacity,
            effective_capacity_grams_per_hour: effective_capacity,
            deliverable_grams,
            common_cause_group,
        },
        conversion: ConversionReceipt {
            input_product: inventory_product,
            output_product: Product::Diesel,
            input_grams: conversion_input_grams,
            output_grams: conversion_output_grams,
            declared_process_loss_grams,
            process_capacity_grams,
            yield_basis_points,
            process_model_id: if inventory_product == Product::CrudeOil {
                "synthetic-crude-to-diesel-v1"
            } else {
                "synthetic-diesel-release-v1"
            },
        },
        generator: GeneratorReceipt {
            fuel_grade_compatible,
            fuel_consumed_grams: planned_fuel_grams,
            synthetic_lhv_milli_wh_per_gram: SYNTHETIC_LHV_MILLI_WH_PER_GRAM,
            efficiency_basis_points,
            rated_power_capacity_kw: SYNTHETIC_POWER_CAPACITY_KW,
            derate_basis_points,
            network_delivery_basis_points,
            gross_electricity_wh,
            network_deliverable_wh,
        },
        service: ServiceOutcome {
            demand_wh,
            critical_floor_wh,
            delivered_lower_wh,
            delivered_upper_wh,
            uncertainty_basis_points,
            telemetry_state,
            disposition,
            evidence_maturity: "synthetic_fixture",
        },
    }
}

pub fn validate_case(case: &FuelServiceCase) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if case.schema_version != SCHEMA_VERSION {
        errors.push("unsupported schema version".to_string());
    }
    if case.horizon_hours != HORIZON_HOURS {
        errors.push("horizon must equal the frozen 72-hour corpus horizon".to_string());
    }
    if !case.case_id.starts_with("safrb-fuel-v0.1-") {
        errors.push("case ID is outside the SAFRB synthetic namespace".to_string());
    }
    if case.scenario_start_unix_s.checked_add(u64::from(case.horizon_hours) * 3_600).is_none() {
        errors.push("scenario absolute time horizon overflows".to_string());
    }

    let ledger = &case.inventory;
    let accounted = ledger.opening_grams
        .checked_add(ledger.accepted_inflow_grams)
        .zip(ledger.withdrawals_grams.checked_add(ledger.declared_losses_grams))
        .and_then(|(available, removed)| removed.checked_add(ledger.closing_grams).map(|right| (available, right)));
    if accounted.is_none_or(|(available, accounted_total)| available != accounted_total) {
        errors.push("inventory mass balance does not reconcile exactly".to_string());
    }
    if ledger.measurement_uncertainty_basis_points > 10_000 {
        errors.push("inventory measurement uncertainty exceeds 100 percent".to_string());
    }
    let expected_observed_lower = ledger.closing_grams
        .saturating_mul(10_000_u64.saturating_sub(ledger.measurement_uncertainty_basis_points))
        / 10_000;
    let expected_observed_upper = ledger.closing_grams
        .saturating_mul(10_000_u64.saturating_add(ledger.measurement_uncertainty_basis_points))
        / 10_000;
    if ledger.observed_lower_grams != expected_observed_lower
        || ledger.observed_upper_grams != expected_observed_upper
        || ledger.observed_lower_grams > ledger.observed_upper_grams
    {
        errors.push("inventory observed quantity bounds do not reconcile with declared measurement uncertainty".to_string());
    }
    let usable_expected = ledger.observed_lower_grams
        .checked_sub(ledger.quarantined_grams)
        .and_then(|value| value.checked_sub(ledger.operating_heel_grams));
    if usable_expected != Some(ledger.usable_grams) {
        errors.push("usable inventory does not reconcile with quarantine and operating heel".to_string());
    }
    if ledger.encumbered_grams > ledger.usable_grams {
        errors.push("encumbered inventory exceeds usable inventory".to_string());
    }
    let accessible_expected = if ledger.authority_available {
        ledger.usable_grams.saturating_sub(ledger.encumbered_grams)
    } else {
        0
    };
    if ledger.accessible_grams != accessible_expected {
        errors.push("accessible inventory does not respect authority and encumbrances".to_string());
    }

    let transport = &case.transport;
    if transport.transit_delay_hours > case.horizon_hours {
        errors.push("transport delay exceeds the frozen horizon".to_string());
    }
    let capacity_sum = transport.primary_capacity_grams_per_hour
        .checked_add(transport.alternate_capacity_grams_per_hour);
    if capacity_sum != Some(transport.effective_capacity_grams_per_hour) {
        errors.push("effective transport capacity does not reconcile with its paths".to_string());
    }
    let transport_window = case.horizon_hours.saturating_sub(transport.transit_delay_hours);
    let transport_limit = transport.effective_capacity_grams_per_hour
        .checked_mul(u64::from(transport_window));
    if transport_limit.is_none_or(|limit| transport.deliverable_grams > limit.min(ledger.accessible_grams)) {
        errors.push("delivered inventory exceeds accessible stock or time-bounded transport capacity".to_string());
    }
    let arrival_before_horizon = transport.cargo_arrival_hour < case.horizon_hours;
    if arrival_before_horizon != (ledger.accepted_inflow_grams > 0) {
        errors.push("cargo inflow does not agree with its declared arrival relative to the horizon".to_string());
    }

    let conversion = &case.conversion;
    if conversion.input_product != ledger.product {
        errors.push("conversion input product does not match the inventory product".to_string());
    }
    if conversion.input_grams > transport.deliverable_grams
        || conversion.input_grams > conversion.process_capacity_grams
    {
        errors.push("conversion input exceeds deliverable feedstock or process capacity".to_string());
    }
    if conversion.output_product != Product::Diesel {
        errors.push("this v0.1 fixture profile requires diesel output for the generator".to_string());
    }
    if conversion.output_grams.checked_add(conversion.declared_process_loss_grams) != Some(conversion.input_grams) {
        errors.push("conversion input does not reconcile with output plus declared process loss".to_string());
    }
    if conversion.yield_basis_points > 10_000 {
        errors.push("conversion yield exceeds 100 percent".to_string());
    }
    let expected_output = conversion.input_grams
        .saturating_mul(conversion.yield_basis_points) / 10_000;
    if expected_output != conversion.output_grams {
        errors.push("conversion output does not match its declared integer yield".to_string());
    }

    let generator = &case.generator;
    if generator.fuel_consumed_grams > conversion.output_grams {
        errors.push("generator fuel consumption exceeds compatible product output".to_string());
    }
    if !generator.fuel_grade_compatible && generator.fuel_consumed_grams != 0 {
        errors.push("incompatible fuel grade cannot be consumed by the generator".to_string());
    }
    if generator.efficiency_basis_points > 10_000
        || generator.derate_basis_points > 10_000
        || generator.network_delivery_basis_points > 10_000
    {
        errors.push("generator efficiency, derate and network factor must not exceed 100 percent".to_string());
    }
    let expected_fuel_energy = generator.fuel_consumed_grams
        .saturating_mul(generator.synthetic_lhv_milli_wh_per_gram)
        .saturating_mul(generator.efficiency_basis_points) / 10_000 / 1_000;
    let rated_cap = generator.rated_power_capacity_kw
        .saturating_mul(1_000)
        .saturating_mul(u64::from(case.horizon_hours))
        .saturating_mul(generator.derate_basis_points) / 10_000;
    if generator.gross_electricity_wh != expected_fuel_energy.min(rated_cap) {
        errors.push("generator output does not reconcile with declared synthetic fuel, efficiency and power cap".to_string());
    }
    let expected_network = generator.gross_electricity_wh
        .saturating_mul(generator.network_delivery_basis_points) / 10_000;
    if generator.network_deliverable_wh != expected_network {
        errors.push("network-deliverable energy does not reconcile with its explicit limit".to_string());
    }

    let service = &case.service;
    if service.evidence_maturity != "synthetic_fixture" {
        errors.push("corpus evidence maturity must remain synthetic_fixture".to_string());
    }
    if service.critical_floor_wh == 0 || service.critical_floor_wh > service.demand_wh {
        errors.push("critical service floor must be positive and no greater than demand".to_string());
    }
    if service.delivered_lower_wh > service.delivered_upper_wh
        || service.delivered_upper_wh > service.demand_wh
    {
        errors.push("delivered energy bounds are inconsistent with each other or demand".to_string());
    }
    if service.uncertainty_basis_points > 10_000 {
        errors.push("delivery uncertainty exceeds 100 percent".to_string());
    }
    // Saturating arithmetic keeps adversarial out-of-range inputs fail-closed
    // rather than panicking while the validator is trying to reject them.
    let expected_lower = generator.network_deliverable_wh
        .saturating_mul(10_000_u64.saturating_sub(service.uncertainty_basis_points)) / 10_000;
    let expected_upper = service.demand_wh.min(
        generator.network_deliverable_wh
            .saturating_mul(10_000_u64.saturating_add(service.uncertainty_basis_points)) / 10_000,
    );
    if service.delivered_lower_wh != expected_lower || service.delivered_upper_wh != expected_upper {
        errors.push("service bounds do not reconcile with network-deliverable energy and declared uncertainty".to_string());
    }
    let expected_disposition = service_disposition(
        service.telemetry_state,
        service.delivered_lower_wh,
        service.delivered_upper_wh,
        service.critical_floor_wh,
    );
    if service.disposition != expected_disposition {
        errors.push("service disposition violates the non-compensatory interval/telemetry rule".to_string());
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

pub fn generate_corpus() -> Vec<FuelServiceCase> {
    let mut cases = Vec::with_capacity(Archetype::ALL.len() * SEEDS_PER_ARCHETYPE as usize);
    for archetype in Archetype::ALL {
        for seed in 0..SEEDS_PER_ARCHETYPE {
            cases.push(generate_case(archetype, seed));
        }
    }
    cases
}

impl FuelServiceCase {
    /// Deterministic JSON Lines projection. Strings are controlled enum values
    /// and synthetic identifiers; no source text or user input is interpolated.
    pub fn to_json_line(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"schemaVersion\":\"{}\",",
                "\"caseId\":\"{}\",",
                "\"archetype\":\"{}\",",
                "\"seed\":{},",
                "\"scenarioStartUnixS\":{},",
                "\"horizonHours\":{},",
                "\"evidenceMaturity\":\"synthetic_fixture\",",
                "\"inventory\":{{",
                    "\"product\":\"{}\",",
                    "\"openingGrams\":{},",
                    "\"acceptedInflowGrams\":{},",
                    "\"withdrawalsGrams\":{},",
                    "\"declaredLossesGrams\":{},",
                    "\"closingGrams\":{},",
                    "\"measurementUncertaintyBasisPoints\":{},",
                    "\"observedLowerGrams\":{},",
                    "\"observedUpperGrams\":{},",
                    "\"quarantinedGrams\":{},",
                    "\"operatingHeelGrams\":{},",
                    "\"usableGrams\":{},",
                    "\"encumberedGrams\":{},",
                    "\"authorityAvailable\":{},",
                    "\"accessibleGrams\":{}",
                "}},",
                "\"transport\":{{",
                    "\"cargoArrivalHour\":{},",
                    "\"transitDelayHours\":{},",
                    "\"primaryCapacityGramsPerHour\":{},",
                    "\"alternateCapacityGramsPerHour\":{},",
                    "\"effectiveCapacityGramsPerHour\":{},",
                    "\"deliverableGrams\":{},",
                    "\"commonCauseGroup\":\"{}\"",
                "}},",
                "\"conversion\":{{",
                    "\"inputProduct\":\"{}\",",
                    "\"outputProduct\":\"{}\",",
                    "\"inputGrams\":{},",
                    "\"outputGrams\":{},",
                    "\"declaredProcessLossGrams\":{},",
                    "\"processCapacityGrams\":{},",
                    "\"yieldBasisPoints\":{},",
                    "\"processModelId\":\"{}\"",
                "}},",
                "\"generator\":{{",
                    "\"fuelGradeCompatible\":{},",
                    "\"fuelConsumedGrams\":{},",
                    "\"syntheticLhvMilliWhPerGram\":{},",
                    "\"efficiencyBasisPoints\":{},",
                    "\"ratedPowerCapacityKw\":{},",
                    "\"derateBasisPoints\":{},",
                    "\"networkDeliveryBasisPoints\":{},",
                    "\"grossElectricityWh\":{},",
                    "\"networkDeliverableWh\":{}",
                "}},",
                "\"service\":{{",
                    "\"demandWh\":{},",
                    "\"criticalFloorWh\":{},",
                    "\"deliveredLowerWh\":{},",
                    "\"deliveredUpperWh\":{},",
                    "\"uncertaintyBasisPoints\":{},",
                    "\"telemetryState\":\"{}\",",
                    "\"disposition\":\"{}\"",
                "}}",
                "}}"
            ),
            self.schema_version,
            self.case_id,
            self.archetype.slug(),
            self.seed,
            self.scenario_start_unix_s,
            self.horizon_hours,
            self.inventory.product.as_str(),
            self.inventory.opening_grams,
            self.inventory.accepted_inflow_grams,
            self.inventory.withdrawals_grams,
            self.inventory.declared_losses_grams,
            self.inventory.closing_grams,
            self.inventory.measurement_uncertainty_basis_points,
            self.inventory.observed_lower_grams,
            self.inventory.observed_upper_grams,
            self.inventory.quarantined_grams,
            self.inventory.operating_heel_grams,
            self.inventory.usable_grams,
            self.inventory.encumbered_grams,
            self.inventory.authority_available,
            self.inventory.accessible_grams,
            self.transport.cargo_arrival_hour,
            self.transport.transit_delay_hours,
            self.transport.primary_capacity_grams_per_hour,
            self.transport.alternate_capacity_grams_per_hour,
            self.transport.effective_capacity_grams_per_hour,
            self.transport.deliverable_grams,
            self.transport.common_cause_group,
            self.conversion.input_product.as_str(),
            self.conversion.output_product.as_str(),
            self.conversion.input_grams,
            self.conversion.output_grams,
            self.conversion.declared_process_loss_grams,
            self.conversion.process_capacity_grams,
            self.conversion.yield_basis_points,
            self.conversion.process_model_id,
            self.generator.fuel_grade_compatible,
            self.generator.fuel_consumed_grams,
            self.generator.synthetic_lhv_milli_wh_per_gram,
            self.generator.efficiency_basis_points,
            self.generator.rated_power_capacity_kw,
            self.generator.derate_basis_points,
            self.generator.network_delivery_basis_points,
            self.generator.gross_electricity_wh,
            self.generator.network_deliverable_wh,
            self.service.demand_wh,
            self.service.critical_floor_wh,
            self.service.delivered_lower_wh,
            self.service.delivered_upper_wh,
            self.service.uncertainty_basis_points,
            self.service.telemetry_state.as_str(),
            self.service.disposition.as_str(),
        )
    }
}

pub fn validate_corpus(cases: &[FuelServiceCase]) -> Result<(), Vec<String>> {
    let expected = Archetype::ALL.len() * SEEDS_PER_ARCHETYPE as usize;
    let mut errors = Vec::new();
    if cases.len() != expected {
        errors.push(format!("expected {expected} cases, found {}", cases.len()));
    }
    let mut ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        if !ids.insert(case.case_id.as_str()) {
            errors.push(format!("duplicate case ID {}", case.case_id));
        }
        if let Err(case_errors) = validate_case(case) {
            errors.extend(case_errors.into_iter().map(|error| format!("case[{index}] {}: {error}", case.case_id)));
        }
    }
    let archetypes: BTreeSet<&str> = cases.iter().map(|case| case.archetype.slug()).collect();
    if archetypes.len() != Archetype::ALL.len() {
        errors.push("corpus does not cover all 12 frozen archetypes".to_string());
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_exactly_120_valid_synthetic_cases() {
        let cases = generate_corpus();
        assert_eq!(cases.len(), 120);
        assert!(validate_corpus(&cases).is_ok());
    }

    #[test]
    fn generated_jsonl_is_deterministic_and_has_120_rows() {
        let left = generate_corpus().iter().map(FuelServiceCase::to_json_line).collect::<Vec<_>>();
        let right = generate_corpus().iter().map(FuelServiceCase::to_json_line).collect::<Vec<_>>();
        assert_eq!(left, right);
        assert_eq!(left.len(), 120);
        assert!(left.iter().all(|line| line.starts_with("{\"schemaVersion\":\"0.1.0\"")));
    }

    #[test]
    fn inventory_mass_balance_detects_double_counting() {
        let mut case = generate_case(Archetype::BaselineAllPaths, 0);
        case.inventory.closing_grams += 1;
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn quarantined_stock_is_never_accessible() {
        let case = generate_case(Archetype::TankQualityQuarantine, 1);
        assert!(case.inventory.quarantined_grams > 0);
        assert!(case.inventory.usable_grams < case.inventory.observed_lower_grams);
        assert!(case.inventory.observed_lower_grams <= case.inventory.closing_grams);
        assert!(case.inventory.closing_grams <= case.inventory.observed_upper_grams);
        assert!(case.inventory.accessible_grams <= case.inventory.usable_grams);
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn access_authority_is_independent_of_physical_inventory() {
        let case = generate_case(Archetype::BaselineAllPaths, 7);
        assert!(case.inventory.closing_grams > 0);
        assert!(!case.inventory.authority_available);
        assert_eq!(case.inventory.accessible_grams, 0);
        assert_eq!(case.transport.deliverable_grams, 0);
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn conversion_conserves_mass_and_declares_loss() {
        let case = generate_case(Archetype::CrudeCargoDelay, 0);
        assert_eq!(
            case.conversion.input_grams,
            case.conversion.output_grams + case.conversion.declared_process_loss_grams
        );
        assert_eq!(case.conversion.input_product, Product::CrudeOil);
        assert_eq!(case.conversion.output_product, Product::Diesel);
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn failed_refinery_does_not_produce_converted_fuel() {
        let case = generate_case(Archetype::RefineryProcessTrip, 1);
        assert_eq!(case.conversion.process_capacity_grams, 0);
        assert_eq!(case.conversion.output_grams, 0);
        assert_eq!(case.generator.fuel_consumed_grams, 0);
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn incompatible_generator_fuel_cannot_produce_electricity() {
        let case = generate_case(Archetype::GeneratorFuelGradeMismatch, 1);
        assert!(!case.generator.fuel_grade_compatible);
        assert_eq!(case.generator.fuel_consumed_grams, 0);
        assert_eq!(case.generator.gross_electricity_wh, 0);
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn delayed_cargo_outside_horizon_is_not_counted() {
        let case = generate_case(Archetype::CrudeCargoDelay, 0);
        assert_eq!(case.transport.cargo_arrival_hour, 96);
        assert_eq!(case.inventory.accepted_inflow_grams, 0);
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn stale_or_conflicting_telemetry_cannot_produce_guaranteed_service() {
        let case = generate_case(Archetype::BaselineAllPaths, 9);
        assert_eq!(case.service.telemetry_state, TelemetryState::StaleOrConflicting);
        assert_eq!(case.service.disposition, ServiceDisposition::Indeterminate);
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn aggregate_energy_cannot_override_a_failed_critical_floor() {
        let mut case = generate_case(Archetype::GridIslandedBackupConstraint, 3);
        case.service.disposition = ServiceDisposition::Guaranteed;
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn upper_bound_and_lower_bound_cannot_be_swapped() {
        let mut case = generate_case(Archetype::TankGaugeUncertainty, 0);
        std::mem::swap(
            &mut case.service.delivered_lower_wh,
            &mut case.service.delivered_upper_wh,
        );
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn compound_shock_has_one_explicit_common_cause_group() {
        let case = generate_case(Archetype::CommonCauseSupplierPortGridComms, 0);
        assert_eq!(case.transport.common_cause_group, "supplier-port-grid-telecom");
        assert!(case.transport.transit_delay_hours > 0);
        assert!(case.generator.network_delivery_basis_points < 10_000);
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn widening_uncertainty_never_improves_the_guaranteed_lower_bound() {
        let case = generate_case(Archetype::BaselineAllPaths, 1);
        let widened_lower = case.generator.network_deliverable_wh
            .saturating_mul(10_000 - 2_000) / 10_000;
        assert!(widened_lower <= case.service.delivered_lower_wh);
    }



    #[test]
    fn tank_gauge_uncertainty_is_applied_to_inventory_observation_not_service_meter() {
        let case = generate_case(Archetype::TankGaugeUncertainty, 0);
        assert_eq!(case.inventory.measurement_uncertainty_basis_points, 2_000);
        assert!(case.inventory.observed_lower_grams < case.inventory.closing_grams);
        assert!(case.inventory.observed_upper_grams > case.inventory.closing_grams);
        assert_eq!(case.service.uncertainty_basis_points, 500);
        assert_eq!(
            case.inventory.usable_grams,
            case.inventory.observed_lower_grams
                - case.inventory.quarantined_grams
                - case.inventory.operating_heel_grams
        );
        assert!(validate_case(&case).is_ok());
    }

    #[test]
    fn absolute_time_origin_translation_preserves_all_non_temporal_invariants() {
        let mut case = generate_case(Archetype::BerthDischargeRestriction, 2);
        let before = validate_case(&case);
        case.scenario_start_unix_s += 86_400;
        let after = validate_case(&case);
        assert_eq!(before, after);
    }
}
