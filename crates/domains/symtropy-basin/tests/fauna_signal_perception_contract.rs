// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable oracle for non-omniscient fauna signal perception.
//!
//! This freezes a deliberately narrow causal boundary:
//!
//! physical/ecological emission -> propagation sample -> receptor -> percept
//!
//! It does not model final animal behavior, belief, animation, or rendering.
//! The detector has no API for exact hidden world state; without an emitted
//! signal compatible with the receptor, no percept can be produced.

use symtropy_basin::SignalKind;

const Q: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SignalEmissionId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SignalEmission {
    id: SignalEmissionId,
    kind: SignalKind,
    emitted_tick: u64,
    intensity_q: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PropagationSample {
    /// Fraction of source signal remaining at the receptor after distance,
    /// occlusion, medium, wind/substrate, etc. 0..=Q.
    transmission_q: u64,
    /// Canonical competing/noise floor at the receptor in the same score scale.
    ambient_noise_q: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Receptor {
    accepted_kind: SignalKind,
    sensitivity_q: u64,
    orientation_gain_q: u64,
    physiological_capacity_q: u64,
    detection_threshold_q: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Percept {
    source_emission: SignalEmissionId,
    source_tick: u64,
    kind: SignalKind,
    strength_q: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectionOutcome {
    NoEmission,
    UnsupportedSignal,
    BelowThreshold { strength_q: u64 },
    Detected(Percept),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectionError {
    OutOfRange,
}

fn mul_q(lhs: u64, rhs: u64) -> u64 {
    ((lhs as u128 * rhs as u128) / Q as u128) as u64
}

fn validate_q(value: u64) -> Result<(), DetectionError> {
    if value <= Q {
        Ok(())
    } else {
        Err(DetectionError::OutOfRange)
    }
}

fn detect(
    emission: Option<SignalEmission>,
    propagation: PropagationSample,
    receptor: Receptor,
) -> Result<DetectionOutcome, DetectionError> {
    validate_q(propagation.transmission_q)?;
    validate_q(propagation.ambient_noise_q)?;
    validate_q(receptor.sensitivity_q)?;
    validate_q(receptor.orientation_gain_q)?;
    validate_q(receptor.physiological_capacity_q)?;
    validate_q(receptor.detection_threshold_q)?;

    let Some(emission) = emission else {
        return Ok(DetectionOutcome::NoEmission);
    };
    validate_q(emission.intensity_q)?;

    if emission.kind != receptor.accepted_kind {
        return Ok(DetectionOutcome::UnsupportedSignal);
    }

    let propagated = mul_q(emission.intensity_q, propagation.transmission_q);
    let sensed = mul_q(propagated, receptor.sensitivity_q);
    let oriented = mul_q(sensed, receptor.orientation_gain_q);
    let physiological = mul_q(oriented, receptor.physiological_capacity_q);
    let strength_q = physiological.saturating_sub(propagation.ambient_noise_q);

    if strength_q < receptor.detection_threshold_q {
        return Ok(DetectionOutcome::BelowThreshold { strength_q });
    }

    Ok(DetectionOutcome::Detected(Percept {
        source_emission: emission.id,
        source_tick: emission.emitted_tick,
        kind: emission.kind,
        strength_q,
    }))
}

fn bird_receptor() -> Receptor {
    Receptor {
        accepted_kind: SignalKind::BirdAlarm,
        sensitivity_q: 800_000,
        orientation_gain_q: 900_000,
        physiological_capacity_q: 1_000_000,
        detection_threshold_q: 100_000,
    }
}

fn bird_alarm() -> SignalEmission {
    SignalEmission {
        id: SignalEmissionId(41),
        kind: SignalKind::BirdAlarm,
        emitted_tick: 900,
        intensity_q: 800_000,
    }
}

#[test]
fn hidden_world_fact_without_emission_cannot_create_a_percept() {
    let result = detect(
        None,
        PropagationSample {
            transmission_q: Q,
            ambient_noise_q: 0,
        },
        bird_receptor(),
    )
    .unwrap();

    assert_eq!(result, DetectionOutcome::NoEmission);
}

#[test]
fn incompatible_signal_kind_cannot_be_sensed_by_the_wrong_receptor() {
    let mut emission = bird_alarm();
    emission.kind = SignalKind::ChemicalGradient;

    let result = detect(
        Some(emission),
        PropagationSample {
            transmission_q: Q,
            ambient_noise_q: 0,
        },
        bird_receptor(),
    )
    .unwrap();

    assert_eq!(result, DetectionOutcome::UnsupportedSignal);
}

#[test]
fn propagation_can_make_the_same_emission_detectable_or_undetectable() {
    let clear = detect(
        Some(bird_alarm()),
        PropagationSample {
            transmission_q: 900_000,
            ambient_noise_q: 10_000,
        },
        bird_receptor(),
    )
    .unwrap();

    let occluded = detect(
        Some(bird_alarm()),
        PropagationSample {
            transmission_q: 100_000,
            ambient_noise_q: 10_000,
        },
        bird_receptor(),
    )
    .unwrap();

    assert!(matches!(clear, DetectionOutcome::Detected(_)));
    assert!(matches!(
        occluded,
        DetectionOutcome::BelowThreshold { .. }
    ));
}

#[test]
fn receptor_orientation_and_physiology_are_part_of_detection_causality() {
    let baseline = detect(
        Some(bird_alarm()),
        PropagationSample {
            transmission_q: 700_000,
            ambient_noise_q: 0,
        },
        bird_receptor(),
    )
    .unwrap();

    let mut compromised = bird_receptor();
    compromised.orientation_gain_q = 250_000;
    compromised.physiological_capacity_q = 300_000;

    let impaired = detect(
        Some(bird_alarm()),
        PropagationSample {
            transmission_q: 700_000,
            ambient_noise_q: 0,
        },
        compromised,
    )
    .unwrap();

    assert!(matches!(baseline, DetectionOutcome::Detected(_)));
    assert!(matches!(
        impaired,
        DetectionOutcome::BelowThreshold { .. }
    ));
}

#[test]
fn canonical_noise_can_mask_an_otherwise_detectable_signal() {
    let quiet = detect(
        Some(bird_alarm()),
        PropagationSample {
            transmission_q: 800_000,
            ambient_noise_q: 10_000,
        },
        bird_receptor(),
    )
    .unwrap();

    let noisy = detect(
        Some(bird_alarm()),
        PropagationSample {
            transmission_q: 800_000,
            ambient_noise_q: 500_000,
        },
        bird_receptor(),
    )
    .unwrap();

    assert!(matches!(quiet, DetectionOutcome::Detected(_)));
    assert!(matches!(
        noisy,
        DetectionOutcome::BelowThreshold { .. }
    ));
}

#[test]
fn detected_percept_preserves_source_provenance() {
    let emission = bird_alarm();
    let outcome = detect(
        Some(emission),
        PropagationSample {
            transmission_q: Q,
            ambient_noise_q: 0,
        },
        bird_receptor(),
    )
    .unwrap();

    let DetectionOutcome::Detected(percept) = outcome else {
        panic!("expected detection");
    };

    assert_eq!(percept.source_emission, emission.id);
    assert_eq!(percept.source_tick, emission.emitted_tick);
    assert_eq!(percept.kind, SignalKind::BirdAlarm);
}

#[test]
fn detection_is_exactly_replay_deterministic() {
    let input = (
        Some(bird_alarm()),
        PropagationSample {
            transmission_q: 733_333,
            ambient_noise_q: 17_777,
        },
        bird_receptor(),
    );

    let first = detect(input.0, input.1, input.2).unwrap();
    for _ in 0..100 {
        assert_eq!(detect(input.0, input.1, input.2).unwrap(), first);
    }
}

#[test]
fn stronger_source_or_path_never_reduces_pre_threshold_strength() {
    let receptor = Receptor {
        detection_threshold_q: Q,
        ..bird_receptor()
    };

    let levels = [0, 100_000, 250_000, 500_000, 750_000, Q];
    let mut last_source_strength = 0;
    for intensity_q in levels {
        let mut emission = bird_alarm();
        emission.intensity_q = intensity_q;
        let result = detect(
            Some(emission),
            PropagationSample {
                transmission_q: 600_000,
                ambient_noise_q: 0,
            },
            receptor,
        )
        .unwrap();
        let strength = match result {
            DetectionOutcome::BelowThreshold { strength_q } => strength_q,
            DetectionOutcome::Detected(percept) => percept.strength_q,
            other => panic!("unexpected outcome: {other:?}"),
        };
        assert!(strength >= last_source_strength);
        last_source_strength = strength;
    }

    let mut last_path_strength = 0;
    for transmission_q in levels {
        let result = detect(
            Some(bird_alarm()),
            PropagationSample {
                transmission_q,
                ambient_noise_q: 0,
            },
            receptor,
        )
        .unwrap();
        let strength = match result {
            DetectionOutcome::BelowThreshold { strength_q } => strength_q,
            DetectionOutcome::Detected(percept) => percept.strength_q,
            other => panic!("unexpected outcome: {other:?}"),
        };
        assert!(strength >= last_path_strength);
        last_path_strength = strength;
    }
}

#[test]
fn out_of_range_fixed_point_inputs_fail_closed() {
    let result = detect(
        Some(bird_alarm()),
        PropagationSample {
            transmission_q: Q + 1,
            ambient_noise_q: 0,
        },
        bird_receptor(),
    );

    assert_eq!(result, Err(DetectionError::OutOfRange));
}
