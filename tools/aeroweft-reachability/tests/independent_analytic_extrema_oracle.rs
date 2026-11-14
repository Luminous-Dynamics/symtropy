use aeroweft_reachability_core::{
    propagate_bounded_velocity, AxisVelocityEnvelope, InitialPosition, Interval, ProfileIdentity,
    ReachabilityRequest, VelocityEnvelope, VelocityStep,
};

fn interval(lower: f64, upper: f64) -> Interval {
    Interval::new(lower, upper).expect("fixture interval is finite and ordered")
}

fn axis(velocity_mps: Interval) -> AxisVelocityEnvelope {
    AxisVelocityEnvelope {
        nominal_velocity_mps: velocity_mps,
        velocity_estimation_uncertainty_mps: interval(0.0, 0.0),
        model_parameter_uncertainty_mps: interval(0.0, 0.0),
        control_velocity_mps: interval(0.0, 0.0),
        environmental_disturbance_mps: interval(0.0, 0.0),
    }
}

fn profile() -> ProfileIdentity {
    ProfileIdentity {
        profile_id: "synthetic-analytic-extrema-oracle-v0".into(),
        revision: "0.1".into(),
        horizontal_frame_id: "LOCAL-ENU-SYNTHETIC".into(),
        vertical_datum_id: "LOCAL-UP-SYNTHETIC".into(),
        profile_sha256: "d".repeat(64),
    }
}

fn assert_contains(interval: Interval, exact_lower: f64, exact_upper: f64) {
    assert!(
        interval.lower() <= exact_lower,
        "lower bound {} excludes analytic minimum {}",
        interval.lower(),
        exact_lower
    );
    assert!(
        interval.upper() >= exact_upper,
        "upper bound {} excludes analytic maximum {}",
        interval.upper(),
        exact_upper
    );
}

#[test]
fn analytic_extrema_oracle_covers_continuous_slices_and_composed_endpoints() {
    // This oracle is analytic rather than sampled. For a constant velocity
    // chosen from [v_min, v_max] over t in [0, dt], the exact axis-aligned
    // occupancy hull is:
    //
    //   [x_min + min(0, v_min * dt), x_max + max(0, v_max * dt)].
    //
    // The extrema occur at t=0 or t=dt. All fixture values below are exactly
    // representable binary fractions, so the known-answer boundaries do not
    // depend on reusing the production interval operators or a sample grid.
    let request = ReachabilityRequest {
        profile: profile(),
        initial_position: InitialPosition {
            east_m: interval(0.0, 1.0),
            north_m: interval(-2.0, -1.0),
            up_m: interval(10.0, 11.0),
        },
        steps: vec![
            VelocityStep {
                start_utc_ms: 0,
                end_utc_ms: 2_000,
                velocity: VelocityEnvelope {
                    east: axis(interval(2.0, 4.0)),
                    north: axis(interval(-3.0, -1.0)),
                    up: axis(interval(0.5, 1.5)),
                },
            },
            VelocityStep {
                start_utc_ms: 2_000,
                end_utc_ms: 3_000,
                velocity: VelocityEnvelope {
                    east: axis(interval(-5.0, -1.0)),
                    north: axis(interval(0.0, 2.0)),
                    up: axis(interval(-2.0, -0.5)),
                },
            },
        ],
    };

    let tube = propagate_bounded_velocity(&request)
        .expect("the bounded analytic fixture must propagate");

    // First complete continuous slice, including t=0 and its endpoint limit.
    assert_contains(tube.slices[0].east_m, 0.0, 9.0);
    assert_contains(tube.slices[0].north_m, -8.0, -1.0);
    assert_contains(tube.slices[0].up_m, 10.0, 14.0);

    // Second slice starts from the first slice's endpoint enclosure. Its
    // occupancy includes every intermediate time, not only its final state.
    assert_contains(tube.slices[1].east_m, -1.0, 9.0);
    assert_contains(tube.slices[1].north_m, -8.0, -1.0);
    assert_contains(tube.slices[1].up_m, 9.0, 14.0);

    // Exact endpoint hulls after both contiguous slices.
    assert_contains(tube.final_position.east_m, -1.0, 8.0);
    assert_contains(tube.final_position.north_m, -8.0, -1.0);
    assert_contains(tube.final_position.up_m, 9.0, 13.5);
}
