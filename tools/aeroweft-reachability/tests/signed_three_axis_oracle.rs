use aeroweft_reachability_core::{
    propagate_bounded_velocity, AxisVelocityEnvelope, InitialPosition, Interval, ProfileIdentity,
    ReachabilityRequest, VelocityEnvelope, VelocityStep,
};

fn interval(lower: f64, upper: f64) -> Interval {
    Interval::new(lower, upper).expect("fixture intervals are finite and ordered")
}

fn axis(
    nominal: Interval,
    state: Interval,
    disturbance: Interval,
) -> AxisVelocityEnvelope {
    AxisVelocityEnvelope {
        nominal_mps: nominal,
        state_uncertainty_mps: state,
        model_uncertainty_mps: interval(0.0, 0.0),
        control_mps: interval(0.0, 0.0),
        disturbance_mps: disturbance,
    }
}

#[test]
fn independent_integer_oracle_checks_signed_motion_on_all_axes() {
    let request = ReachabilityRequest {
        profile: ProfileIdentity {
            profile_id: "synthetic-signed-three-axis-oracle".into(),
            revision: "0.1".into(),
            horizontal_frame_id: "LOCAL-ENU-SYNTHETIC".into(),
            vertical_datum_id: "LOCAL-UP-SYNTHETIC".into(),
            profile_sha256: "c".repeat(64),
        },
        initial_position: InitialPosition {
            east_m: interval(2.0, 3.0),
            north_m: interval(-2.0, -1.0),
            up_m: interval(10.0, 11.0),
        },
        steps: vec![VelocityStep {
            start_utc_ms: 0,
            end_utc_ms: 2_000,
            velocity: VelocityEnvelope {
                east: axis(interval(-10.0, -10.0), interval(-2.0, 2.0), interval(-1.0, 1.0)),
                north: axis(interval(-3.0, -1.0), interval(0.0, 0.0), interval(0.0, 0.0)),
                up: axis(interval(0.5, 1.5), interval(0.0, 0.0), interval(0.0, 0.0)),
            },
        }],
    };

    let tube = propagate_bounded_velocity(&request).expect("synthetic bounds propagate");
    let occupancy = tube.slices[0];

    // Independent reference arithmetic uses integer millimetres and
    // milliseconds. It does not call the production interval operators.
    for initial_mm in [2_000_i128, 3_000_i128] {
        for velocity_mmps in (-13_000_i128..=-7_000_i128).step_by(1_000) {
            for elapsed_ms in (0_i128..=2_000_i128).step_by(100) {
                let position_mm = initial_mm + (velocity_mmps * elapsed_ms) / 1_000;
                let position_m = position_mm as f64 / 1_000.0;
                assert!(occupancy.east_m.lower() <= position_m);
                assert!(occupancy.east_m.upper() >= position_m);
            }
        }
    }

    for initial_mm in [-2_000_i128, -1_000_i128] {
        for velocity_mmps in (-3_000_i128..=-1_000_i128).step_by(1_000) {
            for elapsed_ms in (0_i128..=2_000_i128).step_by(100) {
                let position_mm = initial_mm + (velocity_mmps * elapsed_ms) / 1_000;
                let position_m = position_mm as f64 / 1_000.0;
                assert!(occupancy.north_m.lower() <= position_m);
                assert!(occupancy.north_m.upper() >= position_m);
            }
        }
    }

    for initial_mm in [10_000_i128, 11_000_i128] {
        for velocity_mmps in (500_i128..=1_500_i128).step_by(250) {
            for elapsed_ms in (0_i128..=2_000_i128).step_by(100) {
                let position_mm = initial_mm + (velocity_mmps * elapsed_ms) / 1_000;
                let position_m = position_mm as f64 / 1_000.0;
                assert!(occupancy.up_m.lower() <= position_m);
                assert!(occupancy.up_m.upper() >= position_m);
            }
        }
    }

    assert!(tube.final_position.east_m.lower() <= -24.0);
    assert!(tube.final_position.east_m.upper() >= -11.0);
    assert!(tube.final_position.north_m.lower() <= -8.0);
    assert!(tube.final_position.north_m.upper() >= -3.0);
    assert!(tube.final_position.up_m.lower() <= 11.0);
    assert!(tube.final_position.up_m.upper() >= 14.0);
}
