// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reference producer-input envelope checks for SYM-EVAL-001E Q2.
//!
//! This freezes canonical bytes for one producer-visible synthetic sensor frame.
//! It does not repair the frozen scaffold's full-future capability or semantic
//! observation-id/order shortcuts; those remain explicit A3/A4 blockers.

use std::fmt::Write as _;

use symtropy_vision_eval::{ObjectPermanencePair, SensorBlob, SensorFrame, SensorObservationId};

const DOMAIN: &[u8] = b"sym-eval.producer-input-envelope.v1\0";
const LAST_IDENTICAL_PREFIX_FRAME: usize = 8;
const FIRST_LEGAL_DIVERGENCE_FRAME: usize = 9;
const PROFILE: &str =
    include_str!("../../../../docs/research/SYM_EVAL_001E_Q2_ENVELOPE_PROFILE_V1.json");

fn push_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn encode_frame(frame: &SensorFrame) -> Vec<u8> {
    let observation_count =
        u32::try_from(frame.observations.len()).expect("reference frame count must fit u32");
    let mut out = Vec::with_capacity(DOMAIN.len() + 12 + frame.observations.len() * 36);
    out.extend_from_slice(DOMAIN);
    out.extend_from_slice(&frame.frame_index.to_be_bytes());
    out.extend_from_slice(&observation_count.to_be_bytes());

    for observation in &frame.observations {
        out.extend_from_slice(&observation.observation_id.0.to_be_bytes());
        push_f32(&mut out, observation.x_norm);
        push_f32(&mut out, observation.y_norm);
        push_f32(&mut out, observation.visible_fraction);
        for appearance in observation.appearance {
            push_f32(&mut out, appearance);
        }
    }

    out
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut out, "{byte:02x}").expect("writing into String cannot fail");
    }
    out
}

#[test]
fn q2_canonical_envelope_has_frozen_language_neutral_vector() {
    let frame = SensorFrame {
        frame_index: 3,
        observations: vec![SensorBlob {
            observation_id: SensorObservationId(0x0102_0304_0506_0708),
            x_norm: 0.25,
            y_norm: 0.5,
            visible_fraction: 1.0,
            appearance: [0.1, 0.2, 0.3, 0.4],
        }],
    };
    let encoded = encode_frame(&frame);

    assert_eq!(encoded.len(), 84);
    assert_eq!(
        hex(&encoded),
        concat!(
            "73796d2d6576616c2e70726f64756365722d696e7075742d656e76656c6f70652e763100",
            "0000000000000003",
            "00000001",
            "0102030405060708",
            "3e800000",
            "3f000000",
            "3f800000",
            "3dcccccd",
            "3e4ccccd",
            "3e99999a",
            "3ecccccd"
        )
    );
}

#[test]
fn q2_blind_prefix_envelopes_are_byte_identical_until_legal_divergence() {
    let pair = ObjectPermanencePair::blinded(7, 7_001);
    let trial_a = pair.trial_a().sensor_frames();
    let trial_b = pair.trial_b().sensor_frames();

    for frame_index in 0..=LAST_IDENTICAL_PREFIX_FRAME {
        assert_eq!(
            encode_frame(&trial_a[frame_index]),
            encode_frame(&trial_b[frame_index]),
            "producer-visible envelope diverged early at frame {frame_index}"
        );
    }

    assert_ne!(
        encode_frame(&trial_a[FIRST_LEGAL_DIVERGENCE_FRAME]),
        encode_frame(&trial_b[FIRST_LEGAL_DIVERGENCE_FRAME])
    );
}

#[test]
fn q2_blinding_nonce_does_not_change_shared_prefix_envelopes() {
    let first = ObjectPermanencePair::blinded(41, 1);
    let second = ObjectPermanencePair::blinded(41, 9_999);

    for frame_index in 0..=LAST_IDENTICAL_PREFIX_FRAME {
        assert_eq!(
            encode_frame(&first.trial_a().sensor_frames()[frame_index]),
            encode_frame(&second.trial_a().sensor_frames()[frame_index])
        );
        assert_eq!(
            encode_frame(&first.trial_b().sensor_frames()[frame_index]),
            encode_frame(&second.trial_b().sensor_frames()[frame_index])
        );
    }
}

#[test]
fn q2_every_declared_producer_visible_field_changes_envelope_bytes() {
    let pair = ObjectPermanencePair::blinded(53, 53_001);
    let original = pair.trial_a().sensor_frames()[0].clone();
    let baseline = encode_frame(&original);

    let mut changed = original.clone();
    changed.frame_index += 1;
    assert_ne!(baseline, encode_frame(&changed));

    let mut changed = original.clone();
    changed.observations[0].observation_id.0 ^= 1;
    assert_ne!(baseline, encode_frame(&changed));

    let mut changed = original.clone();
    changed.observations[0].x_norm += 0.001;
    assert_ne!(baseline, encode_frame(&changed));

    let mut changed = original.clone();
    changed.observations[0].y_norm += 0.001;
    assert_ne!(baseline, encode_frame(&changed));

    let mut changed = original.clone();
    changed.observations[0].visible_fraction -= 0.001;
    assert_ne!(baseline, encode_frame(&changed));

    let mut changed = original.clone();
    changed.observations[0].appearance[0] += 0.001;
    assert_ne!(baseline, encode_frame(&changed));

    let mut changed = original.clone();
    changed.observations.reverse();
    assert_ne!(baseline, encode_frame(&changed));
}

#[test]
fn q2_envelope_commits_only_one_frame_not_future_horizon() {
    let pair = ObjectPermanencePair::blinded(61, 61_001);
    let frame = pair.trial_a().sensor_frames()[0].clone();
    let short_sequence = [frame.clone()];
    let long_sequence = [frame.clone(), frame];

    assert_eq!(encode_frame(&short_sequence[0]), encode_frame(&long_sequence[0]));
}

#[test]
fn q2_profile_keeps_known_blockers_and_nonclaims_explicit() {
    assert!(PROFILE.contains(r#""schema_id": "sym-eval.producer-input-envelope.v1""#));
    assert!(PROFILE.contains(r#""implementation_parent_sha": "c582a0762e8e23eb16f7cffd674c7149e8ee6378""#));
    assert!(PROFILE.contains(r#""full_future_sequence": "LeakDetected""#));
    assert!(PROFILE.contains(r#""semantic_observation_identity": "Unresolved""#));
    assert!(PROFILE.contains(r#""online_benchmark_validity": "NotEstablished""#));
    assert!(PROFILE.contains(r#""model_performance": "NotEvaluated""#));
}
