use symtropy_artificial_gravity::{
    angular_velocity_for_zero_net_momentum, rotor_momentum_balance,
};

fn main() -> Result<(), String> {
    let primary_inertia_kg_m2 = 1_000_000.0_f64;
    let primary_omega_rad_s = 0.16755160819145562_f64; // 1.6 RPM
    let inertia_ratios = [0.25_f64, 0.5, 1.0, 2.0, 4.0];
    let speed_multipliers = [0.9_f64, 1.0, 1.1];

    println!(
        "counter_inertia_ratio,counter_omega_rad_s,speed_multiplier,net_angular_momentum_kg_m2_s,absolute_module_momentum_sum_kg_m2_s,net_momentum_fraction,stored_rotational_energy_j"
    );

    // Single-rotor reference: no counter-rotating module.
    let single = rotor_momentum_balance(&[(primary_inertia_kg_m2, primary_omega_rad_s)])
        .map_err(|error| format!("single-rotor reference failed: {error}"))?;
    println!(
        "0.000000,0.000000,0.000000,{:.9},{:.9},{:.12},{:.9}",
        single.net_angular_momentum_kg_m2_s,
        single.sum_absolute_module_momenta_kg_m2_s,
        single.net_momentum_fraction,
        single.stored_rotational_energy_j
    );

    for ratio in inertia_ratios {
        let counter_inertia = primary_inertia_kg_m2 * ratio;
        let cancel_omega = angular_velocity_for_zero_net_momentum(
            primary_inertia_kg_m2,
            primary_omega_rad_s,
            counter_inertia,
        )
        .map_err(|error| format!("cancellation speed failed: {error}"))?;

        for multiplier in speed_multipliers {
            let counter_omega = cancel_omega * multiplier;
            let balance = rotor_momentum_balance(&[
                (primary_inertia_kg_m2, primary_omega_rad_s),
                (counter_inertia, counter_omega),
            ])
            .map_err(|error| format!("rotor balance failed: {error}"))?;

            println!(
                "{ratio:.6},{counter_omega:.9},{multiplier:.3},{:.9},{:.9},{:.12},{:.9}",
                balance.net_angular_momentum_kg_m2_s,
                balance.sum_absolute_module_momenta_kg_m2_s,
                balance.net_momentum_fraction,
                balance.stored_rotational_energy_j
            );
        }
    }

    Ok(())
}
