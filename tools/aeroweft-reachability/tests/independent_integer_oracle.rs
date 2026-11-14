use aeroweft_reachability_core::{
    propagate_bounded_velocity, AxisVelocityEnvelope, InitialPosition, Interval, ProfileIdentity,
    ReachabilityRequest, VelocityEnvelope, VelocityStep,
};

fn interval(lo: f64, hi: f64) -> Interval {
    Interval::new(lo, hi).expect("valid fixture interval")
}

fn axis(
    nominal: Interval,
    state: Interval,
    disturbance: Interval,
) -> AxisVelocityEnvelope {
    AxisVelocityEnvelope {
        nominal_velocity_mps: nominal,
        velocity_estimation_uncertainty_mps: state,
        model_parameter_uncertainty_mps: interval(0.0, 0.0),
        control_velocity_mps: interval(0.0, 0.0),
        environmental_disturbance_mps: disturbance,
    }
}

fn profile() -> ProfileIdentity {
    ProfileIdentity {
        profile_id: "synthetic-integer-oracle-v0".into(),
        revision: "0.1".into(),
        horizontal_frame_id: "LOCAL-ENU-SYNTHETIC".into(),
        vertical_datum_id: "LOCAL-UP-SYNTHETIC".into(),
        profile_sha256: "b".repeat(64),
    }
}

#[test]
fn independent_integer_millimetre_oracle_is_inside_the_tube() {
    let request = ReachabilityRequest {
        profile: profile(),
        initial_position: InitialPosition {
            east_m: interval(2.0, 3.0),
            north_m: interval(0.0, 0.0),
            up_m: interval(0.0, 0.0),
        },
        steps: vec![VelocityStep {
            start_utc_ms: 0,
            end_utc_ms: 1_000,
            velocity: VelocityEnvelope {
                east: axis(interval(10.0, 10.0), interval(-2.0, 2.0), interval(-1.0, 1.0)),
                north: axis(interval(0.0, 0.0), interval(0.0, 0.0), interval(0.0, 0.0)),
                up: axis(interval(0.0, 0.0), interval(0.0, 0.0), interval(0.0, 0.0)),
            },
        }],
    };
    let tube = propagate_bounded_velocity(&request).expect("bounded fixture must propagate");
    let occupancy = tube.slices[0].east_m;

    // Independent reference uses integer millimetres and integer milliseconds;
    // it does not call or reuse the production interval operations.
    for initial_mm in [2_000_i128, 3_000_i128] {
        for velocity_mmps in (7_000_i128..=13_000_i128).step_by(1_000) {
            for elapsed_ms in (0_i128..=1_000_i128).step_by(100) {
                let exact_position_mm =
                    initial_mm + (velocity_mmps * elapsed_ms) / 1_000_i128;
                let exact_position_m = exact_position_mm as f64 / 1_000.0;
                assert!(occupancy.lower() <= exact_position_m);
                assert!(occupancy.upper() >= exact_position_m);
            }
        }
    }

    for initial_mm in [2_000_i128, 3_000_i128] {
        for velocity_mmps in (7_000_i128..=13_000_i128).step_by(1_000) {
            let exact_endpoint_mm = initial_mm + velocity_mmps;
            let exact_endpoint_m = exact_endpoint_mm as f64 / 1_000.0;
            assert!(tube.final_position.east_m.lower() <= exact_endpoint_m);
            assert!(tube.final_position.east_m.upper() >= exact_endpoint_m);
        }
    }
}
