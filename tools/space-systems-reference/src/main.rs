use space_systems_reference::{
    evaluate_qdrive, evaluate_sunshade, illustrative_qdrive_case, illustrative_sunshade_case,
};

fn main() {
    let shade = evaluate_sunshade(illustrative_sunshade_case())
        .expect("frozen shade arithmetic fixture must be valid");
    let qdrive = evaluate_qdrive(illustrative_qdrive_case())
        .expect("frozen q-drive arithmetic fixture must be valid");

    println!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1.0\",\n",
            "  \"evidence_class\": \"synthetic_analytical_screening\",\n",
            "  \"sunshade\": {{\n",
            "    \"target_flux_reduction_fraction\": 0.01,\n",
            "    \"equivalent_aperture_area_km2\": {:.12},\n",
            "    \"installed_mass_kg\": {:.12e},\n",
            "    \"locally_sourced_mass_kg\": {:.12e},\n",
            "    \"earth_supplied_mass_kg\": {:.12e},\n",
            "    \"intercepted_power_reduction_w\": {:.12e}\n",
            "  }},\n",
            "  \"qdrive\": {{\n",
            "    \"beta\": {:.12},\n",
            "    \"dynamic_pressure_pa\": {:.12e},\n",
            "    \"harvesting_drag_n\": {:.12e},\n",
            "    \"harvested_mechanical_power_w\": {:.12e},\n",
            "    \"bus_power_w\": {:.12e},\n",
            "    \"power_conversion_loss_w\": {:.12e},\n",
            "    \"jet_power_w\": {:.12e},\n",
            "    \"thruster_conversion_loss_w\": {:.12e},\n",
            "    \"energy_chain_residual_w\": {:.12e},\n",
            "    \"ideal_thrust_n\": {:.12e},\n",
            "    \"propellant_mass_flow_kg_s\": {:.12e},\n",
            "    \"net_acceleration_m_s2\": {:.12e},\n",
            "    \"jet_power_identity_residual_w\": {:.12e}\n",
            "  }},\n",
            "  \"claim_ceiling\": \"arithmetic_screening_only_not_orbit_optics_climate_plasma_or_hardware_validation\"\n",
            "}}"
        ),
        shade.equivalent_aperture_area_km2,
        shade.installed_mass_kg,
        shade.locally_sourced_mass_kg,
        shade.earth_supplied_mass_kg,
        shade.intercepted_power_reduction_w,
        qdrive.beta,
        qdrive.dynamic_pressure_pa,
        qdrive.harvesting_drag_n,
        qdrive.harvested_mechanical_power_w,
        qdrive.bus_power_w,
        qdrive.power_conversion_loss_w,
        qdrive.jet_power_w,
        qdrive.thruster_conversion_loss_w,
        qdrive.energy_chain_residual_w,
        qdrive.ideal_thrust_n,
        qdrive.propellant_mass_flow_kg_s,
        qdrive.net_acceleration_m_s2,
        qdrive.jet_power_identity_residual_w,
    );
}
