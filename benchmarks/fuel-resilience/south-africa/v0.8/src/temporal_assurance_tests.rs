use super::*;

const BASE: u64 = 1_800_000_000;
const HOUR: u64 = 3_600;

fn service_window(
    index: usize,
    start: u64,
    duration: u64,
    delivered_lower_wh: u128,
    delivered_upper_wh: u128,
    critical_floor_wh: u128,
    target_wh: u128,
) -> ServiceWindow {
    let end = start.checked_add(duration).expect("fixture timestamp");
    ServiceWindow {
        window_id: format!("temporal-window-{index}"),
        period_id: format!("temporal-period-{index}"),
        service_id: "temporal-assurance-fixture".into(),
        boundary_id: "service-boundary-a".into(),
        start_unix_s: start,
        duration_seconds: duration,
        critical_floor_wh,
        target_wh,
        delivered_lower_wh,
        delivered_upper_wh,
        evidence: Evidence {
            source_id: "fixture:temporal-assurance".into(),
            source_uri: "urn:safrb:fixture:temporal-assurance-v0.1".into(),
            evidence_class: EvidenceClass::Scenario,
            raw_sha256: None,
            upstream_artifact_id: None,
            measurement_start_unix_s: start,
            measurement_end_unix_s: end,
            recorded_at_unix_s: end,
        },
    }
}

// 0 = confirmed below floor, 1 = uncertain against floor, 2 = guaranteed.
fn windows_from_states(states: &[u8]) -> Vec<ServiceWindow> {
    states
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let (lower, upper) = match state {
                0 => (0, 5),
                1 => (45, 55),
                2 => (60, 65),
                _ => panic!("unknown oracle state"),
            };
            service_window(
                index,
                BASE + index as u64 * HOUR,
                HOUR,
                lower,
                upper,
                50,
                80,
            )
        })
        .collect()
}

#[derive(Clone, Debug)]
struct OracleEpisode {
    start_index: usize,
    recovery_candidate_index: Option<usize>,
    recovery_streak_seconds: u64,
    confirmed_shortfall_windows: u32,
    uncertain_windows: u32,
    critical_shortfall_lower_wh: u128,
    critical_shortfall_upper_wh: u128,
}

#[derive(Clone, Debug)]
struct ExpectedEpisode {
    start_index: usize,
    recovery_candidate_index: Option<usize>,
    recovery_confirmed_end_index: Option<usize>,
    restoration_latency_seconds: Option<u64>,
    confirmation_delay_seconds: Option<u64>,
    observed_duration_seconds: u64,
    confirmed_shortfall_windows: u32,
    uncertain_windows: u32,
    critical_shortfall_lower_wh: u128,
    critical_shortfall_upper_wh: u128,
    right_censored: bool,
}

// Independent reference trace over discrete states. This intentionally
// consumes the state alphabet, not ServiceWindow assessments or production
// EpisodeBuilder internals.
fn oracle_episodes(states: &[u8], confirmation_seconds: u64) -> Vec<ExpectedEpisode> {
    let mut expected = Vec::new();
    let mut active: Option<OracleEpisode> = None;

    for (index, state) in states.iter().copied().enumerate() {
        match state {
            2 => {
                if let Some(mut episode) = active.take() {
                    episode.recovery_candidate_index.get_or_insert(index);
                    episode.recovery_streak_seconds += HOUR;
                    if episode.recovery_streak_seconds >= confirmation_seconds {
                        let candidate = episode
                            .recovery_candidate_index
                            .expect("guaranteed state creates candidate");
                        let confirmed_end = index + 1;
                        expected.push(ExpectedEpisode {
                            start_index: episode.start_index,
                            recovery_candidate_index: Some(candidate),
                            restoration_latency_seconds: Some(
                                (candidate - episode.start_index) as u64 * HOUR,
                            ),
                            confirmation_delay_seconds: Some(
                                (confirmed_end - candidate) as u64 * HOUR,
                            ),
                            observed_duration_seconds: (confirmed_end - episode.start_index) as u64
                                * HOUR,
                            confirmed_shortfall_windows: episode.confirmed_shortfall_windows,
                            uncertain_windows: episode.uncertain_windows,
                            critical_shortfall_lower_wh: episode.critical_shortfall_lower_wh,
                            critical_shortfall_upper_wh: episode.critical_shortfall_upper_wh,
                            right_censored: false,
                        });
                    } else {
                        active = Some(episode);
                    }
                }
            }
            0 | 1 => {
                let episode = active.get_or_insert_with(|| OracleEpisode {
                    start_index: index,
                    recovery_candidate_index: None,
                    recovery_streak_seconds: 0,
                    confirmed_shortfall_windows: 0,
                    uncertain_windows: 0,
                    critical_shortfall_lower_wh: 0,
                    critical_shortfall_upper_wh: 0,
                });
                // Any non-guaranteed window breaks a recovery streak.
                episode.recovery_candidate_index = None;
                episode.recovery_streak_seconds = 0;
                if state == 0 {
                    episode.confirmed_shortfall_windows += 1;
                    episode.critical_shortfall_lower_wh += 45;
                    episode.critical_shortfall_upper_wh += 50;
                } else {
                    episode.uncertain_windows += 1;
                    episode.critical_shortfall_upper_wh += 5;
                }
            }
            _ => unreachable!("state alphabet is bounded"),
        }
    }

    if let Some(episode) = active {
        let horizon_end_index = states.len();
        expected.push(ExpectedEpisode {
            start_index: episode.start_index,
            recovery_candidate_index: episode.recovery_candidate_index,
            recovery_confirmed_end_index: None,
            restoration_latency_seconds: None,
            confirmation_delay_seconds: None,
            observed_duration_seconds: (horizon_end_index - episode.start_index) as u64 * HOUR,
            confirmed_shortfall_windows: episode.confirmed_shortfall_windows,
            uncertain_windows: episode.uncertain_windows,
            critical_shortfall_lower_wh: episode.critical_shortfall_lower_wh,
            critical_shortfall_upper_wh: episode.critical_shortfall_upper_wh,
            right_censored: true,
        });
    }

    expected
}

fn encode_states(mut encoded: usize, length: usize) -> Vec<u8> {
    let mut states = Vec::with_capacity(length);
    for _ in 0..length {
        states.push((encoded % 3) as u8);
        encoded /= 3;
    }
    states
}

fn disposition_rank(disposition: WindowDisposition) -> u8 {
    match disposition {
        WindowDisposition::ConfirmedBelowCriticalFloor => 0,
        WindowDisposition::UncertainAgainstCriticalFloor => 1,
        WindowDisposition::GuaranteedAtOrAboveCriticalFloor => 2,
    }
}

#[test]
fn exhaustive_temporal_sequences_match_independent_oracle() {
    let mut sequences_checked = 0_usize;

    // 3^1 + ... + 3^6 = 1,092 sequences; three recovery horizons
    // produce 3,276 deterministic evaluator/oracle comparisons.
    for length in 1..=6 {
        let sequence_count = 3_usize.pow(length as u32);
        for encoded in 0..sequence_count {
            let states = encode_states(encoded, length);
            let windows = windows_from_states(&states);
            sequences_checked += 1;

            for confirmation_hours in 1..=3 {
                let confirmation_seconds = u64::from(confirmation_hours) * HOUR;
                let actual = evaluate(&windows, confirmation_seconds)
                    .expect("all oracle-generated windows are valid");
                let expected_episodes = oracle_episodes(&states, confirmation_seconds);

                let expected_confirmed = states.iter().filter(|state| **state == 0).count();
                let expected_uncertain = states.iter().filter(|state| **state == 1).count();
                let expected_guaranteed = states.iter().filter(|state| **state == 2).count();

                let expected_disposition = if expected_confirmed > 0 {
                    CertificateDisposition::ConfirmedCriticalServiceShortfall
                } else if expected_uncertain > 0 {
                    CertificateDisposition::ContinuityNotProven
                } else {
                    CertificateDisposition::ContinuitySupportedWithinHorizon
                };

                assert_eq!(
                    actual.disposition, expected_disposition,
                    "states={states:?}"
                );
                assert_eq!(actual.windows_total as usize, length, "states={states:?}");
                assert_eq!(
                    actual.confirmed_shortfall_windows as usize, expected_confirmed,
                    "states={states:?}"
                );
                assert_eq!(
                    actual.uncertain_windows as usize, expected_uncertain,
                    "states={states:?}"
                );
                assert_eq!(
                    actual.guaranteed_windows as usize, expected_guaranteed,
                    "states={states:?}"
                );
                assert_eq!(
                    actual.guaranteed_service_seconds,
                    expected_guaranteed as u64 * HOUR,
                    "states={states:?}"
                );
                assert_eq!(
                    actual.guaranteed_time_ppm,
                    ((expected_guaranteed as u128 * 1_000_000) / length as u128) as u32,
                    "states={states:?}"
                );
                assert_eq!(
                    actual.critical_shortfall_lower_wh,
                    expected_confirmed as u128 * 45,
                    "states={states:?}"
                );
                assert_eq!(
                    actual.critical_shortfall_upper_wh,
                    expected_confirmed as u128 * 50 + expected_uncertain as u128 * 5,
                    "states={states:?}"
                );
                assert_eq!(
                    actual.restoration_episodes.len(),
                    expected_episodes.len(),
                    "states={states:?}, confirmation_hours={confirmation_hours}"
                );

                for (actual_episode, expected_episode) in
                    actual.restoration_episodes.iter().zip(&expected_episodes)
                {
                    assert_eq!(
                        actual_episode.episode_start_unix_s,
                        BASE + expected_episode.start_index as u64 * HOUR,
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.recovery_candidate_start_unix_s,
                        expected_episode
                            .recovery_candidate_index
                            .map(|index| BASE + index as u64 * HOUR),
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.recovery_confirmed_at_unix_s,
                        expected_episode
                            .recovery_confirmed_end_index
                            .map(|index| BASE + index as u64 * HOUR),
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.restoration_latency_seconds,
                        expected_episode.restoration_latency_seconds,
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.confirmation_delay_seconds,
                        expected_episode.confirmation_delay_seconds,
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.observed_duration_seconds,
                        expected_episode.observed_duration_seconds,
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.confirmed_shortfall_windows,
                        expected_episode.confirmed_shortfall_windows,
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.uncertain_windows, expected_episode.uncertain_windows,
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.critical_shortfall_lower_wh,
                        expected_episode.critical_shortfall_lower_wh,
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.critical_shortfall_upper_wh,
                        expected_episode.critical_shortfall_upper_wh,
                        "states={states:?}"
                    );
                    assert_eq!(
                        actual_episode.right_censored, expected_episode.right_censored,
                        "states={states:?}"
                    );
                }
            }
        }
    }

    assert_eq!(sequences_checked, 1_092);
}

#[test]
fn widening_delivery_intervals_never_improves_continuity_certainty() {
    for lower in 0_u128..=100 {
        for upper in lower..=100 {
            let original_window = service_window(0, BASE, HOUR, lower, upper, 50, 80);
            let original = evaluate(&[original_window], HOUR).expect("original interval");
            let widened_lower = lower.saturating_sub(5);
            let widened_upper = (upper + 5).min(100);
            let widened_window =
                service_window(0, BASE, HOUR, widened_lower, widened_upper, 50, 80);
            let widened = evaluate(&[widened_window], HOUR).expect("widened interval");

            match original.window_assessments[0].disposition {
                WindowDisposition::GuaranteedAtOrAboveCriticalFloor => assert_ne!(
                    widened.window_assessments[0].disposition,
                    WindowDisposition::ConfirmedBelowCriticalFloor
                ),
                WindowDisposition::ConfirmedBelowCriticalFloor => assert_ne!(
                    widened.window_assessments[0].disposition,
                    WindowDisposition::GuaranteedAtOrAboveCriticalFloor
                ),
                WindowDisposition::UncertainAgainstCriticalFloor => assert_eq!(
                    widened.window_assessments[0].disposition,
                    WindowDisposition::UncertainAgainstCriticalFloor
                ),
            }

            assert!(widened.critical_shortfall_lower_wh <= original.critical_shortfall_lower_wh);
            assert!(widened.critical_shortfall_upper_wh >= original.critical_shortfall_upper_wh);
        }
    }
}

#[test]
fn improving_both_delivery_bounds_never_worsens_continuity() {
    for lower in 0_u128..=100 {
        for upper in lower..=100 {
            let original = evaluate(&[service_window(0, BASE, HOUR, lower, upper, 50, 80)], HOUR)
                .expect("original interval");
            for delta in [1_u128, 5, 25, 100] {
                let improved = evaluate(
                    &[service_window(
                        0,
                        BASE,
                        HOUR,
                        lower + delta,
                        upper + delta,
                        50,
                        80,
                    )],
                    HOUR,
                )
                .expect("improved interval");

                assert!(
                    disposition_rank(improved.window_assessments[0].disposition)
                        >= disposition_rank(original.window_assessments[0].disposition)
                );
                assert!(
                    improved.critical_shortfall_lower_wh <= original.critical_shortfall_lower_wh
                );
                assert!(
                    improved.critical_shortfall_upper_wh <= original.critical_shortfall_upper_wh
                );
            }
        }
    }
}

#[test]
fn equivalent_time_partition_and_clock_translation_preserve_service_outcomes() {
    let original = vec![
        service_window(0, BASE, HOUR, 60, 65, 50, 80),
        service_window(1, BASE + HOUR, HOUR, 10, 15, 50, 80),
        service_window(2, BASE + 2 * HOUR, 2 * HOUR, 120, 130, 100, 160),
    ];
    let split = vec![
        service_window(0, BASE, HOUR, 60, 65, 50, 80),
        service_window(1, BASE + HOUR, HOUR, 10, 15, 50, 80),
        service_window(2, BASE + 2 * HOUR, HOUR, 60, 65, 50, 80),
        service_window(3, BASE + 3 * HOUR, HOUR, 60, 65, 50, 80),
    ];

    let original_result = evaluate(&original, 2 * HOUR).expect("original trajectory");
    let split_result = evaluate(&split, 2 * HOUR).expect("partitioned trajectory");

    assert_eq!(original_result.disposition, split_result.disposition);
    assert_eq!(
        original_result.horizon_seconds,
        split_result.horizon_seconds
    );
    assert_eq!(
        original_result.guaranteed_service_seconds,
        split_result.guaranteed_service_seconds
    );
    assert_eq!(
        original_result.guaranteed_time_ppm,
        split_result.guaranteed_time_ppm
    );
    assert_eq!(
        original_result.critical_shortfall_lower_wh,
        split_result.critical_shortfall_lower_wh
    );
    assert_eq!(
        original_result.critical_shortfall_upper_wh,
        split_result.critical_shortfall_upper_wh
    );
    assert_eq!(
        original_result.target_shortfall_lower_wh,
        split_result.target_shortfall_lower_wh
    );
    assert_eq!(
        original_result.target_shortfall_upper_wh,
        split_result.target_shortfall_upper_wh
    );
    assert_eq!(
        original_result.restoration_episodes,
        split_result.restoration_episodes
    );

    let offset = 900_000_000_u64;
    let translated: Vec<ServiceWindow> = original
        .iter()
        .cloned()
        .map(|mut window| {
            window.start_unix_s += offset;
            window.evidence.measurement_start_unix_s += offset;
            window.evidence.measurement_end_unix_s += offset;
            window.evidence.recorded_at_unix_s += offset;
            window
        })
        .collect();
    let translated_result = evaluate(&translated, 2 * HOUR).expect("translated trajectory");

    assert_eq!(original_result.disposition, translated_result.disposition);
    assert_eq!(
        original_result.horizon_seconds,
        translated_result.horizon_seconds
    );
    assert_eq!(
        original_result.guaranteed_service_seconds,
        translated_result.guaranteed_service_seconds
    );
    assert_eq!(
        original_result.critical_shortfall_lower_wh,
        translated_result.critical_shortfall_lower_wh
    );
    assert_eq!(
        original_result.critical_shortfall_upper_wh,
        translated_result.critical_shortfall_upper_wh
    );
    for (left, right) in original_result
        .restoration_episodes
        .iter()
        .zip(&translated_result.restoration_episodes)
    {
        assert_eq!(
            left.restoration_latency_seconds,
            right.restoration_latency_seconds
        );
        assert_eq!(
            left.confirmation_delay_seconds,
            right.confirmation_delay_seconds
        );
        assert_eq!(
            left.observed_duration_seconds,
            right.observed_duration_seconds
        );
        assert_eq!(
            left.confirmed_shortfall_windows,
            right.confirmed_shortfall_windows
        );
        assert_eq!(left.uncertain_windows, right.uncertain_windows);
        assert_eq!(
            left.critical_shortfall_lower_wh,
            right.critical_shortfall_lower_wh
        );
        assert_eq!(
            left.critical_shortfall_upper_wh,
            right.critical_shortfall_upper_wh
        );
        assert_eq!(left.right_censored, right.right_censored);
    }
}

#[test]
fn sustained_recovery_boundary_is_exact_and_conservative_at_coarse_resolution() {
    let exact = vec![
        service_window(0, BASE, HOUR, 0, 5, 50, 80),
        service_window(1, BASE + HOUR, HOUR, 60, 65, 50, 80),
    ];
    let exact_result = evaluate(&exact, HOUR).expect("exact threshold");
    let exact_episode = &exact_result.restoration_episodes[0];
    assert!(!exact_episode.right_censored);
    assert_eq!(exact_episode.restoration_latency_seconds, Some(HOUR));
    assert_eq!(exact_episode.confirmation_delay_seconds, Some(HOUR));
    assert_eq!(exact_episode.observed_duration_seconds, 2 * HOUR);

    let one_second_short = evaluate(&exact, HOUR + 1).expect("threshold exceeds observed streak");
    let censored = &one_second_short.restoration_episodes[0];
    assert!(censored.right_censored);
    assert_eq!(censored.recovery_candidate_start_unix_s, Some(BASE + HOUR));
    assert_eq!(censored.recovery_confirmed_at_unix_s, None);

    // A single three-hour sample cannot claim that the threshold was reached
    // at minute 60. Confirmation is conservatively timestamped at sample end.
    let coarse = vec![
        service_window(0, BASE, HOUR, 0, 5, 50, 80),
        service_window(1, BASE + HOUR, 3 * HOUR, 180, 195, 150, 240),
    ];
    let coarse_result = evaluate(&coarse, HOUR).expect("coarse sample");
    let coarse_episode = &coarse_result.restoration_episodes[0];
    assert!(!coarse_episode.right_censored);
    assert_eq!(coarse_episode.restoration_latency_seconds, Some(HOUR));
    assert_eq!(coarse_episode.confirmation_delay_seconds, Some(3 * HOUR));
    assert_eq!(coarse_episode.observed_duration_seconds, 4 * HOUR);
    assert_eq!(
        coarse_episode.recovery_confirmed_at_unix_s,
        Some(BASE + 4 * HOUR)
    );
}

#[test]
fn a_later_independent_outage_opens_a_second_restoration_episode() {
    let states = [0, 2, 2, 0, 2];
    let windows = windows_from_states(&states);
    let result = evaluate(&windows, 2 * HOUR).expect("two incident trajectory");

    assert_eq!(result.restoration_episodes.len(), 2);
    let first = &result.restoration_episodes[0];
    assert!(!first.right_censored);
    assert_eq!(first.episode_start_unix_s, BASE);
    assert_eq!(first.restoration_latency_seconds, Some(HOUR));
    assert_eq!(first.confirmation_delay_seconds, Some(2 * HOUR));
    assert_eq!(first.observed_duration_seconds, 3 * HOUR);

    let second = &result.restoration_episodes[1];
    assert!(second.right_censored);
    assert_eq!(second.episode_start_unix_s, BASE + 3 * HOUR);
    assert_eq!(
        second.recovery_candidate_start_unix_s,
        Some(BASE + 4 * HOUR)
    );
    assert_eq!(second.recovery_confirmed_at_unix_s, None);
    assert_eq!(second.observed_duration_seconds, 2 * HOUR);
}
