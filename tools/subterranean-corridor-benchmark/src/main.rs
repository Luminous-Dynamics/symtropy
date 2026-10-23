use subterranean_corridor_benchmark::{
    Candidate, CandidateKind, EvidenceClass, GateState, HardGates, ServiceBasis,
    ThermalOpportunity, evaluate, pareto_frontier,
};

fn base_candidate(
    id: &str,
    kind: CandidateKind,
    basis: &str,
    service: ServiceBasis,
    distance_km: f64,
    hours: f64,
) -> Candidate {
    Candidate {
        candidate_id: id.to_owned(),
        comparison_basis_id: basis.to_owned(),
        cost_profile_id: "SYNTHETIC-USD-2026-COST-BOUNDARY-V1".to_owned(),
        externality_profile_id: "SYNTHETIC-ANNUAL-EXTERNALITY-V1".to_owned(),
        kind,
        service_basis: service,
        evidence_class: EvidenceClass::Synthetic,
        source_refs: Vec::new(),
        price_year: 2026,
        horizon_years: 30,
        discount_rate: 0.05,
        annual_demand_units: 100_000.0,
        annual_capacity_units: 150_000.0,
        service_availability_fraction: 0.92,
        route_distance_km: distance_km,
        door_to_door_hours: hours,
        civil_capex_usd: 0.0,
        terminal_capex_usd: 0.0,
        fleet_capex_usd: 0.0,
        utility_capex_usd: 0.0,
        annual_fixed_opex_usd: 1_500_000.0,
        annual_maintenance_usd: 500_000.0,
        variable_cost_usd_per_unit_km: 0.05,
        access_transfer_cost_usd_per_unit: 20.0,
        annual_external_cost_usd: 300_000.0,
        energy_kwh_per_unit_km: 0.05,
        emissions_kg_co2e_per_unit_km: 0.008,
        capital_embodied_energy_kwh: 0.0,
        capital_embodied_emissions_kg_co2e: 0.0,
        decommissioning_cost_usd: 500_000.0,
        residual_value_usd: 0.0,
        gates: HardGates::all_satisfied(),
        thermal_opportunity: None,
    }
}

fn freight_candidates() -> Vec<Candidate> {
    let basis = "SYNTHETIC-FREIGHT-600KM-100K-TONNES-YEAR-V1";
    let mut existing = base_candidate(
        "B0-existing-multimodal-service",
        CandidateKind::ExistingNetwork,
        basis,
        ServiceBasis::FreightTonne,
        600.0,
        22.0,
    );
    existing.terminal_capex_usd = 3_000_000.0;
    existing.annual_fixed_opex_usd = 1_500_000.0;
    existing.annual_maintenance_usd = 250_000.0;
    existing.variable_cost_usd_per_unit_km = 0.055;
    existing.access_transfer_cost_usd_per_unit = 20.0;
    existing.annual_external_cost_usd = 500_000.0;
    existing.energy_kwh_per_unit_km = 0.070;
    existing.emissions_kg_co2e_per_unit_km = 0.010;
    existing.capital_embodied_energy_kwh = 700_000.0;
    existing.capital_embodied_emissions_kg_co2e = 150_000.0;
    existing.annual_capacity_units = 160_000.0;

    let mut rail = base_candidate(
        "B1-surface-rail-upgrade",
        CandidateKind::SurfaceRailUpgrade,
        basis,
        ServiceBasis::FreightTonne,
        600.0,
        17.0,
    );
    rail.civil_capex_usd = 80_000_000.0;
    rail.terminal_capex_usd = 12_000_000.0;
    rail.fleet_capex_usd = 18_000_000.0;
    rail.utility_capex_usd = 10_000_000.0;
    rail.annual_fixed_opex_usd = 2_400_000.0;
    rail.annual_maintenance_usd = 2_100_000.0;
    rail.variable_cost_usd_per_unit_km = 0.026;
    rail.access_transfer_cost_usd_per_unit = 32.0;
    rail.annual_external_cost_usd = 220_000.0;
    rail.energy_kwh_per_unit_km = 0.032;
    rail.emissions_kg_co2e_per_unit_km = 0.0045;
    rail.capital_embodied_energy_kwh = 80_000_000.0;
    rail.capital_embodied_emissions_kg_co2e = 14_000_000.0;
    rail.annual_capacity_units = 180_000.0;
    rail.service_availability_fraction = 0.96;

    let mut reuse = base_candidate(
        "B2-reuse-refurbished-underground",
        CandidateKind::ReuseUnderground,
        basis,
        ServiceBasis::FreightTonne,
        600.0,
        11.0,
    );
    reuse.civil_capex_usd = 30_000_000.0;
    reuse.terminal_capex_usd = 8_000_000.0;
    reuse.fleet_capex_usd = 12_000_000.0;
    reuse.utility_capex_usd = 6_000_000.0;
    reuse.annual_fixed_opex_usd = 2_200_000.0;
    reuse.annual_maintenance_usd = 2_500_000.0;
    reuse.variable_cost_usd_per_unit_km = 0.022;
    reuse.access_transfer_cost_usd_per_unit = 38.0;
    reuse.annual_external_cost_usd = 180_000.0;
    reuse.energy_kwh_per_unit_km = 0.028;
    reuse.emissions_kg_co2e_per_unit_km = 0.0035;
    reuse.capital_embodied_energy_kwh = 30_000_000.0;
    reuse.capital_embodied_emissions_kg_co2e = 7_000_000.0;
    reuse.annual_capacity_units = 130_000.0;
    reuse.thermal_opportunity = Some(ThermalOpportunity {
        recoverable_mwh_per_year: 1_200.0,
        compatible_demand_mwh_per_year: 1_500.0,
        delivery_fraction: 0.80,
        source_sink_gate: GateState::Unknown,
    });

    let mut tunnel = base_candidate(
        "B3-new-purpose-built-freight-tunnel",
        CandidateKind::NewFreightTunnel,
        basis,
        ServiceBasis::FreightTonne,
        600.0,
        8.0,
    );
    tunnel.civil_capex_usd = 300_000_000.0;
    tunnel.terminal_capex_usd = 20_000_000.0;
    tunnel.fleet_capex_usd = 22_000_000.0;
    tunnel.utility_capex_usd = 18_000_000.0;
    tunnel.annual_fixed_opex_usd = 4_000_000.0;
    tunnel.annual_maintenance_usd = 4_000_000.0;
    tunnel.variable_cost_usd_per_unit_km = 0.014;
    tunnel.access_transfer_cost_usd_per_unit = 30.0;
    tunnel.annual_external_cost_usd = 160_000.0;
    tunnel.energy_kwh_per_unit_km = 0.020;
    tunnel.emissions_kg_co2e_per_unit_km = 0.0028;
    tunnel.capital_embodied_energy_kwh = 250_000_000.0;
    tunnel.capital_embodied_emissions_kg_co2e = 50_000_000.0;
    tunnel.annual_capacity_units = 220_000.0;
    tunnel.service_availability_fraction = 0.97;
    tunnel.thermal_opportunity = Some(ThermalOpportunity {
        recoverable_mwh_per_year: 2_500.0,
        compatible_demand_mwh_per_year: 2_000.0,
        delivery_fraction: 0.85,
        source_sink_gate: GateState::Unknown,
    });
    vec![existing, rail, reuse, tunnel]
}

fn passenger_candidates() -> Vec<Candidate> {
    let basis = "SYNTHETIC-PASSENGER-300KM-100K-TRIPS-YEAR-V1";
    let mut rail = base_candidate(
        "P0-high-speed-rail",
        CandidateKind::HighSpeedRail,
        basis,
        ServiceBasis::PassengerTrip,
        300.0,
        3.0,
    );
    rail.annual_demand_units = 100_000.0;
    rail.annual_capacity_units = 160_000.0;
    rail.service_availability_fraction = 0.96;
    rail.civil_capex_usd = 220_000_000.0;
    rail.terminal_capex_usd = 30_000_000.0;
    rail.fleet_capex_usd = 45_000_000.0;
    rail.utility_capex_usd = 25_000_000.0;
    rail.annual_fixed_opex_usd = 3_000_000.0;
    rail.annual_maintenance_usd = 3_500_000.0;
    rail.variable_cost_usd_per_unit_km = 0.010;
    rail.access_transfer_cost_usd_per_unit = 25.0;
    rail.annual_external_cost_usd = 250_000.0;
    rail.energy_kwh_per_unit_km = 0.060;
    rail.emissions_kg_co2e_per_unit_km = 0.006;
    rail.capital_embodied_energy_kwh = 300_000_000.0;
    rail.capital_embodied_emissions_kg_co2e = 60_000_000.0;

    let mut maglev = base_candidate(
        "P1-maglev",
        CandidateKind::Maglev,
        basis,
        ServiceBasis::PassengerTrip,
        300.0,
        2.0,
    );
    maglev.annual_demand_units = 100_000.0;
    maglev.annual_capacity_units = 170_000.0;
    maglev.service_availability_fraction = 0.96;
    maglev.civil_capex_usd = 420_000_000.0;
    maglev.terminal_capex_usd = 38_000_000.0;
    maglev.fleet_capex_usd = 65_000_000.0;
    maglev.utility_capex_usd = 45_000_000.0;
    maglev.annual_fixed_opex_usd = 3_300_000.0;
    maglev.annual_maintenance_usd = 4_200_000.0;
    maglev.variable_cost_usd_per_unit_km = 0.011;
    maglev.access_transfer_cost_usd_per_unit = 25.0;
    maglev.annual_external_cost_usd = 230_000.0;
    maglev.energy_kwh_per_unit_km = 0.070;
    maglev.emissions_kg_co2e_per_unit_km = 0.007;
    maglev.capital_embodied_energy_kwh = 550_000_000.0;
    maglev.capital_embodied_emissions_kg_co2e = 100_000_000.0;
    vec![rail, maglev]
}

fn print_group(title: &str, candidates: Vec<Candidate>) {
    println!("\n{title}");
    println!(
        "All numbers below are synthetic fixture values, not route estimates or industry data."
    );
    let evaluations: Vec<_> = candidates.iter().map(evaluate).collect();
    for result in &evaluations {
        println!("\n{}: {}", result.candidate_id, result.disposition.label());
        for reason in &result.reasons {
            println!("  reason={reason}");
        }
        if let Some(m) = &result.metrics {
            println!(
                "  social lifecycle cost/service unit: USD {:.2}",
                m.social_cost_per_service_unit_usd
            );
            println!(
                "  private lifecycle cost/service unit: USD {:.2}",
                m.private_cost_per_service_unit_usd
            );
            println!("  door-to-door time: {:.2} h", m.door_to_door_hours);
            println!(
                "  effective annual capacity: {:.0}",
                m.effective_capacity_units_per_year
            );
            println!(
                "  lifecycle energy/service unit: {:.2} kWh",
                m.energy_kwh_per_service_unit
            );
            println!(
                "  lifecycle emissions/service unit: {:.2} kg CO2e",
                m.emissions_kg_co2e_per_service_unit
            );
            if let Some(heat) = &m.thermal {
                println!(
                    "  potential matched heat (not credited): {:.2} MWh/year",
                    heat.matched_potential_mwh_per_year
                );
                match heat.gate_satisfied_useful_mwh_per_year {
                    Some(value) => println!("  gate-admitted useful heat: {:.2} MWh/year", value),
                    None => println!(
                        "  gate-admitted useful heat: NONE (source/sink evidence unresolved)"
                    ),
                }
            }
        }
    }
    println!("\nPareto frontier (same comparison contract only):");
    for id in pareto_frontier(&evaluations) {
        println!("  {id}");
    }
}

fn print_seismic_negative_controls() {
    let basis = "SYNTHETIC-SEISMIC-NEGATIVE-CONTROLS-V1";
    let mut active_fault = base_candidate(
        "N1-active-fault-displacement-unknown",
        CandidateKind::NewFreightTunnel,
        basis,
        ServiceBasis::FreightTonne,
        600.0,
        8.0,
    );
    active_fault.gates.active_fault_displacement = GateState::Unknown;

    let mut liquefaction = base_candidate(
        "N2-liquefaction-and-lateral-spread-failed",
        CandidateKind::ReuseUnderground,
        basis,
        ServiceBasis::FreightTonne,
        600.0,
        11.0,
    );
    liquefaction.gates.liquefaction_lateral_spread_and_slope = GateState::Failed;

    let mut post_event = base_candidate(
        "N3-post-event-inspection-unknown",
        CandidateKind::ReuseUnderground,
        basis,
        ServiceBasis::FreightTonne,
        600.0,
        11.0,
    );
    post_event.gates.post_event_inspection_and_recovery = GateState::Unknown;

    println!("\nSEISMIC NEGATIVE CONTROLS (synthetic gate semantics only)");
    for candidate in [active_fault, liquefaction, post_event] {
        let result = evaluate(&candidate);
        println!("{}: {}", result.candidate_id, result.disposition.label());
        for reason in &result.reasons {
            println!("  reason={reason}");
        }
        assert!(
            result.metrics.is_none(),
            "unresolved or failed seismic control must not emit comparable cost metrics"
        );
    }
}

fn main() {
    println!(
        "SUBTERRANEAN CORRIDOR BENCHMARK {}",
        subterranean_corridor_benchmark::BENCHMARK_VERSION
    );
    println!(
        "RESEARCH ONLY — synthetic assumptions do not establish cost, safety, feasibility, or SOTA."
    );
    print_group(
        "FREIGHT: illustrative 600 km / 100,000 tonne-year contract",
        freight_candidates(),
    );
    print_group(
        "PASSENGER: illustrative 300 km / 100,000 passenger-trip-year contract",
        passenger_candidates(),
    );
    print_seismic_negative_controls();
    println!(
        "\nClaim ceiling: arithmetic demonstration only. No geography, geology, price, demand, emissions factor, thermal source, permit, or operational claim was independently validated."
    );
}
