use aeroweift_service_quality_oracle::{
    fixed_points, iterate, mean_wait, next_demand, Cohort, FeedbackConfig,
    FeedbackOutcome, WaitTotal,
};

const GOLDEN_FIXTURE: &str =
    include_str!("../../../fixtures/aeroweift-service-quality/tiny-demand-queue-v0.json");

const COHORTS: [Cohort; 3] = [
    Cohort {
        id: "cohort-a-time-sensitive",
        maximum_wait_seconds: 420,
    },
    Cohort {
        id: "cohort-b-moderately-time-sensitive",
        maximum_wait_seconds: 240,
    },
    Cohort {
        id: "cohort-c-low-wait-tolerance",
        maximum_wait_seconds: 120,
    },
];

const ONE_SERVER: FeedbackConfig = FeedbackConfig {
    servers: 1,
    service_seconds: 300,
    cohorts: &COHORTS,
};

const TWO_SERVERS: FeedbackConfig = FeedbackConfig {
    servers: 2,
    service_seconds: 300,
    cohorts: &COHORTS,
};

#[test]
fn golden_json_manifest_matches_the_frozen_fixture_identity() {
    assert!(GOLDEN_FIXTURE.contains("AW-SQ-TINY-001"));
    assert!(GOLDEN_FIXTURE.contains("fixed_points"));
    assert!(GOLDEN_FIXTURE.contains("cycles"));
    assert!(GOLDEN_FIXTURE.contains("5/3"));
}

#[test]
fn one_server_has_exact_queue_waits_and_choice_map() {
    assert_eq!(
        mean_wait(&ONE_SERVER, 0),
        Some(WaitTotal {
            total_wait_seconds: 0,
            customers: 0
        })
    );
    assert_eq!(
        mean_wait(&ONE_SERVER, 1),
        Some(WaitTotal {
            total_wait_seconds: 0,
            customers: 1
        })
    );
    assert_eq!(
        mean_wait(&ONE_SERVER, 2),
        Some(WaitTotal {
            total_wait_seconds: 300,
            customers: 2
        })
    );
    assert_eq!(
        mean_wait(&ONE_SERVER, 3),
        Some(WaitTotal {
            total_wait_seconds: 900,
            customers: 3
        })
    );

    assert_eq!(next_demand(&ONE_SERVER, 0), Some(3));
    assert_eq!(next_demand(&ONE_SERVER, 1), Some(3));
    assert_eq!(next_demand(&ONE_SERVER, 2), Some(2));
    assert_eq!(next_demand(&ONE_SERVER, 3), Some(1));
}

#[test]
fn one_server_has_one_fixed_point_and_a_two_state_cycle() {
    assert_eq!(
        fixed_points(&ONE_SERVER),
        Some(vec![FeedbackOutcome::FixedPoint {
            demand: 2,
            wait: WaitTotal {
                total_wait_seconds: 300,
                customers: 2
            }
        }])
    );

    assert_eq!(
        iterate(&ONE_SERVER, 2),
        Some(FeedbackOutcome::FixedPoint {
            demand: 2,
            wait: WaitTotal {
                total_wait_seconds: 300,
                customers: 2
            }
        })
    );

    assert_eq!(
        iterate(&ONE_SERVER, 1),
        Some(FeedbackOutcome::Cycle {
            demand_states: vec![1, 3]
        })
    );
    assert_eq!(
        iterate(&ONE_SERVER, 3),
        Some(FeedbackOutcome::Cycle {
            demand_states: vec![1, 3]
        })
    );
}

#[test]
fn two_servers_have_zero_wait_for_two_and_five_thirds_minutes_for_three() {
    assert_eq!(
        mean_wait(&TWO_SERVERS, 2),
        Some(WaitTotal {
            total_wait_seconds: 0,
            customers: 2
        })
    );
    assert_eq!(
        mean_wait(&TWO_SERVERS, 3),
        Some(WaitTotal {
            total_wait_seconds: 300,
            customers: 3
        })
    );

    // Exact mean is 300 / 3 = 100 seconds, or 5/3 minutes.
    assert_eq!(
        fixed_points(&TWO_SERVERS),
        Some(vec![FeedbackOutcome::FixedPoint {
            demand: 3,
            wait: WaitTotal {
                total_wait_seconds: 300,
                customers: 3
            }
        }])
    );
    assert_eq!(
        iterate(&TWO_SERVERS, 0),
        Some(FeedbackOutcome::FixedPoint {
            demand: 3,
            wait: WaitTotal {
                total_wait_seconds: 300,
                customers: 3
            }
        })
    );
}

#[test]
fn adding_a_server_does_not_increase_wait_for_the_same_demand_trace() {
    for demand in 0..=COHORTS.len() {
        let one = mean_wait(&ONE_SERVER, demand).expect("valid one-server trace");
        let two = mean_wait(&TWO_SERVERS, demand).expect("valid two-server trace");

        if one.customers == 0 {
            assert_eq!(one.total_wait_seconds, two.total_wait_seconds);
        } else {
            // Compare rational means without floating point.
            assert!(
                two.total_wait_seconds * one.customers
                    <= one.total_wait_seconds * two.customers
            );
        }
    }
}

#[test]
fn invalid_server_service_or_demand_states_are_rejected() {
    let no_servers = FeedbackConfig {
        servers: 0,
        ..ONE_SERVER
    };
    assert_eq!(mean_wait(&no_servers, 1), None);
    assert_eq!(iterate(&no_servers, 0), None);

    let zero_service = FeedbackConfig {
        service_seconds: 0,
        ..ONE_SERVER
    };
    assert_eq!(mean_wait(&zero_service, 1), None);

    assert_eq!(mean_wait(&ONE_SERVER, 4), None);
    assert_eq!(iterate(&ONE_SERVER, 4), None);
}
