use symtropy_artificial_gravity::{
    constant_axis_spin_state, rotating_frame_apparent_acceleration, vector_magnitude,
};

fn main() -> Result<(), String> {
    let scenarios = [
        ("spin_up", [50.0_f64, 3.0, 2.0], 0.1_f64, 0.002_f64),
        ("steady", [12.0_f64, -4.0, 5.0], 0.2_f64, 0.0_f64),
        ("spin_down", [20.0_f64, 0.0, 1.0], 0.3_f64, -0.01_f64),
    ];
    let sample_times_s = [0.0_f64, 2.0, 5.0, 10.0, 20.0, 30.0];

    println!(
        "scenario,time_s,theta_rad,omega_rad_s,x_m,y_m,z_m,vx_m_s,vy_m_s,vz_m_s,ax_m_s2,ay_m_s2,az_m_s2,rotating_frame_residual_m_s2"
    );

    for (name, initial_position, initial_omega, alpha) in scenarios {
        for time_s in sample_times_s {
            let state = constant_axis_spin_state(
                initial_position,
                initial_omega,
                alpha,
                time_s,
            )
            .map_err(|error| format!("exact spin state failed: {error}"))?;
            let apparent = rotating_frame_apparent_acceleration(
                [0.0, 0.0, state.angular_velocity_rad_s],
                [0.0, 0.0, alpha],
                state.position_m,
                [0.0, 0.0, 0.0],
            )
            .map_err(|error| format!("rotating-frame cross-check failed: {error}"))?;
            let residual = [
                state.acceleration_m_s2[0] + apparent.apparent_total_m_s2[0],
                state.acceleration_m_s2[1] + apparent.apparent_total_m_s2[1],
                state.acceleration_m_s2[2] + apparent.apparent_total_m_s2[2],
            ];
            let residual_magnitude = vector_magnitude(residual);

            println!(
                "{name},{time_s:.3},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{residual_magnitude:.12}",
                state.angle_rad,
                state.angular_velocity_rad_s,
                state.position_m[0],
                state.position_m[1],
                state.position_m[2],
                state.velocity_m_s[0],
                state.velocity_m_s[1],
                state.velocity_m_s[2],
                state.acceleration_m_s2[0],
                state.acceleration_m_s2[1],
                state.acceleration_m_s2[2]
            );
        }
    }

    Ok(())
}
