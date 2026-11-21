//! Read-only validator for the observed GAUGE rigid-body pose JSON profile.
//!
//! The parser intentionally separates structural facts from physical semantics.
//! It validates array shape and declared metadata, but does not infer handedness,
//! quaternion algebra, transform direction, Euler units, absolute timestamps,
//! calibration, uncertainty, or physical truth.

use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

const RESERVED_ROOT_KEYS: [&str; 3] = ["FPS", "translation unit", "euler order"];
const REQUIRED_CHANNELS: [&str; 10] = ["x", "y", "z", "qx", "qy", "qz", "qw", "ex", "ey", "ez"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrialError {
    InvalidJson,
    RootNotObject,
    MissingField(&'static str),
    InvalidFps,
    UnsupportedTranslationUnit,
    UnsupportedEulerOrder,
    NoBodyStreams,
    BodyStreamNotObject(String),
    MissingChannel {
        body: String,
        channel: &'static str,
    },
    ChannelNotArray {
        body: String,
        channel: &'static str,
    },
    EmptyChannel {
        body: String,
        channel: &'static str,
    },
    ChannelSampleCountMismatch {
        body: String,
        channel: &'static str,
        expected: usize,
        actual: usize,
    },
    InvalidSample {
        body: String,
        channel: &'static str,
        index: usize,
    },
    DegenerateQuaternion {
        body: String,
        index: usize,
    },
    InvalidDerivedValue,
}

impl Display for TrialError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson => write!(formatter, "trial payload is not valid JSON"),
            Self::RootNotObject => write!(formatter, "trial root must be a JSON object"),
            Self::MissingField(field) => write!(formatter, "missing required root field {field}"),
            Self::InvalidFps => write!(formatter, "FPS must be a finite positive number"),
            Self::UnsupportedTranslationUnit => {
                write!(
                    formatter,
                    "translation unit is outside the observed v0.1 profile"
                )
            }
            Self::UnsupportedEulerOrder => {
                write!(
                    formatter,
                    "Euler order is outside the observed v0.1 profile"
                )
            }
            Self::NoBodyStreams => write!(formatter, "trial has no body trajectory streams"),
            Self::BodyStreamNotObject(body) => {
                write!(formatter, "body stream {body} must be a JSON object")
            }
            Self::MissingChannel { body, channel } => {
                write!(formatter, "body stream {body} is missing channel {channel}")
            }
            Self::ChannelNotArray { body, channel } => {
                write!(
                    formatter,
                    "body stream {body} channel {channel} must be an array"
                )
            }
            Self::EmptyChannel { body, channel } => {
                write!(formatter, "body stream {body} channel {channel} is empty")
            }
            Self::ChannelSampleCountMismatch {
                body,
                channel,
                expected,
                actual,
            } => write!(
                formatter,
                "body stream {body} channel {channel} has {actual} samples; expected {expected}"
            ),
            Self::InvalidSample {
                body,
                channel,
                index,
            } => write!(
                formatter,
                "body stream {body} channel {channel} sample {index} must be finite numeric data"
            ),
            Self::DegenerateQuaternion { body, index } => write!(
                formatter,
                "body stream {body} quaternion sample {index} has zero or invalid norm"
            ),
            Self::InvalidDerivedValue => write!(formatter, "requested derived value is invalid"),
        }
    }
}

impl Error for TrialError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KnownSemantics {
    Declared,
    Derived,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticDisposition {
    pub fps: KnownSemantics,
    pub translation_unit: KnownSemantics,
    pub euler_order: KnownSemantics,
    pub quaternion_component_storage_order: KnownSemantics,
    pub quaternion_algebra: KnownSemantics,
    pub transform_direction: KnownSemantics,
    pub coordinate_handedness: KnownSemantics,
    pub frame_identifiers: KnownSemantics,
    pub euler_angle_unit: KnownSemantics,
    pub absolute_clock_origin: KnownSemantics,
    pub per_sample_measurement_uncertainty: KnownSemantics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BodyStreamReport {
    pub body_id: String,
    pub sample_count: usize,
    /// Maximum |norm(q) - 1| observed across samples. Values are not normalized.
    pub maximum_quaternion_norm_deviation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrialReport {
    pub fps_hz: f64,
    pub translation_unit: &'static str,
    pub euler_order: &'static str,
    pub quaternion_component_order: &'static str,
    pub position_to_metres_scale: f64,
    pub bodies: Vec<BodyStreamReport>,
    pub semantics: SemanticDisposition,
}

impl TrialReport {
    /// Convert a raw position component into metres using the declared mm unit.
    /// This does not mutate or replace the original raw record.
    pub fn position_component_metres(&self, raw_value: f64) -> Result<f64, TrialError> {
        if !raw_value.is_finite() {
            return Err(TrialError::InvalidDerivedValue);
        }
        let value = raw_value * self.position_to_metres_scale;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(TrialError::InvalidDerivedValue)
        }
    }

    /// Relative frame-index time only: frame 0 is assigned t=0 and each
    /// subsequent frame advances by 1/FPS. This is not an absolute sensor clock.
    pub fn relative_frame_time_seconds(
        &self,
        zero_based_frame_index: usize,
    ) -> Result<f64, TrialError> {
        let time = zero_based_frame_index as f64 / self.fps_hz;
        if time.is_finite() {
            Ok(time)
        } else {
            Err(TrialError::InvalidDerivedValue)
        }
    }
}

/// Validate the observed GAUGE rigid-body trial JSON profile.
///
/// v0.1 intentionally supports only the observed declarations FPS>0,
/// translation unit "mm", and Euler order "ZYX". It requires 3 position,
/// 4 quaternion-component and 3 Euler-component arrays per body, all finite
/// and sample-aligned. Other quaternion/frame/Euler semantics remain unresolved.
pub fn validate_trial_json(input: &str) -> Result<TrialReport, TrialError> {
    let root: Value = serde_json::from_str(input).map_err(|_| TrialError::InvalidJson)?;
    let root_object = root.as_object().ok_or(TrialError::RootNotObject)?;

    let fps = root_object
        .get("FPS")
        .ok_or(TrialError::MissingField("FPS"))?
        .as_f64()
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or(TrialError::InvalidFps)?;

    let translation_unit = root_object
        .get("translation unit")
        .ok_or(TrialError::MissingField("translation unit"))?
        .as_str()
        .ok_or(TrialError::UnsupportedTranslationUnit)?;
    if translation_unit != "mm" {
        return Err(TrialError::UnsupportedTranslationUnit);
    }

    let euler_order = root_object
        .get("euler order")
        .ok_or(TrialError::MissingField("euler order"))?
        .as_str()
        .ok_or(TrialError::UnsupportedEulerOrder)?;
    if euler_order != "ZYX" {
        return Err(TrialError::UnsupportedEulerOrder);
    }

    let mut bodies = Vec::new();
    for (body_id, body_value) in root_object {
        if RESERVED_ROOT_KEYS.contains(&body_id.as_str()) {
            continue;
        }
        let body = body_value
            .as_object()
            .ok_or_else(|| TrialError::BodyStreamNotObject(body_id.clone()))?;

        let mut lengths = BTreeMap::new();
        let mut channel_values: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
        let mut expected_count = None;

        for channel in REQUIRED_CHANNELS {
            let raw_values = body
                .get(channel)
                .ok_or_else(|| TrialError::MissingChannel {
                    body: body_id.clone(),
                    channel,
                })?
                .as_array()
                .ok_or_else(|| TrialError::ChannelNotArray {
                    body: body_id.clone(),
                    channel,
                })?;

            if raw_values.is_empty() {
                return Err(TrialError::EmptyChannel {
                    body: body_id.clone(),
                    channel,
                });
            }
            if let Some(expected) = expected_count {
                if raw_values.len() != expected {
                    return Err(TrialError::ChannelSampleCountMismatch {
                        body: body_id.clone(),
                        channel,
                        expected,
                        actual: raw_values.len(),
                    });
                }
            } else {
                expected_count = Some(raw_values.len());
            }

            let mut values = Vec::with_capacity(raw_values.len());
            for (index, raw) in raw_values.iter().enumerate() {
                let value = raw
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| TrialError::InvalidSample {
                        body: body_id.clone(),
                        channel,
                        index,
                    })?;
                values.push(value);
            }
            lengths.insert(channel, values.len());
            channel_values.insert(channel, values);
        }

        let sample_count = expected_count.ok_or(TrialError::NoBodyStreams)?;
        let qx = &channel_values["qx"];
        let qy = &channel_values["qy"];
        let qz = &channel_values["qz"];
        let qw = &channel_values["qw"];
        let mut maximum_quaternion_norm_deviation: f64 = 0.0;

        for index in 0..sample_count {
            let norm = (qx[index] * qx[index]
                + qy[index] * qy[index]
                + qz[index] * qz[index]
                + qw[index] * qw[index])
                .sqrt();
            if !norm.is_finite() || norm <= f64::EPSILON {
                return Err(TrialError::DegenerateQuaternion {
                    body: body_id.clone(),
                    index,
                });
            }
            maximum_quaternion_norm_deviation =
                maximum_quaternion_norm_deviation.max((norm - 1.0).abs());
        }

        // Keep a consistency assertion close to the parser so new channels
        // cannot accidentally be included without an aligned-length check.
        if lengths.values().any(|length| *length != sample_count) {
            return Err(TrialError::NoBodyStreams);
        }

        bodies.push(BodyStreamReport {
            body_id: body_id.clone(),
            sample_count,
            maximum_quaternion_norm_deviation,
        });
    }

    if bodies.is_empty() {
        return Err(TrialError::NoBodyStreams);
    }
    bodies.sort_by(|left, right| left.body_id.cmp(&right.body_id));

    Ok(TrialReport {
        fps_hz: fps,
        translation_unit: "mm",
        euler_order: "ZYX",
        quaternion_component_order: "XYZW",
        position_to_metres_scale: 0.001,
        bodies,
        semantics: SemanticDisposition {
            fps: KnownSemantics::Declared,
            translation_unit: KnownSemantics::Declared,
            euler_order: KnownSemantics::Declared,
            quaternion_component_storage_order: KnownSemantics::Declared,
            quaternion_algebra: KnownSemantics::Unresolved,
            transform_direction: KnownSemantics::Unresolved,
            coordinate_handedness: KnownSemantics::Unresolved,
            frame_identifiers: KnownSemantics::Unresolved,
            euler_angle_unit: KnownSemantics::Unresolved,
            absolute_clock_origin: KnownSemantics::Unresolved,
            per_sample_measurement_uncertainty: KnownSemantics::Unresolved,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE_BODY: &str = r#"{
        "FPS": 30,
        "translation unit": "mm",
        "euler order": "ZYX",
        "ball": {
            "x": [1000.0, 500.0],
            "y": [0.0, 10.0],
            "z": [450.0, 440.0],
            "qx": [0.0, 0.0],
            "qy": [0.0, 0.0],
            "qz": [0.0, 0.0],
            "qw": [1.0, 1.0],
            "ex": [0.0, 1.0],
            "ey": [0.0, 1.0],
            "ez": [0.0, 1.0]
        }
    }"#;

    const MULTI_BODY: &str = r#"{
        "FPS": 30,
        "translation unit": "mm",
        "euler order": "ZYX",
        "ball_passive": {
            "x": [1.0, 2.0], "y": [1.0, 2.0], "z": [1.0, 2.0],
            "qx": [0.0, 0.0], "qy": [0.0, 0.0], "qz": [0.0, 0.0], "qw": [1.0, 1.0],
            "ex": [0.0, 0.0], "ey": [0.0, 0.0], "ez": [0.0, 0.0]
        },
        "ball_active": {
            "x": [3.0, 4.0], "y": [3.0, 4.0], "z": [3.0, 4.0],
            "qx": [0.0, 0.0], "qy": [0.0, 0.0], "qz": [0.0, 0.0], "qw": [1.0, 1.0],
            "ex": [0.0, 0.0], "ey": [0.0, 0.0], "ez": [0.0, 0.0]
        }
    }"#;

    fn replace_once(input: &str, from: &str, to: &str) -> String {
        input.replacen(from, to, 1)
    }

    #[test]
    fn accepts_single_body_record_with_observed_profile() {
        let report = validate_trial_json(SINGLE_BODY).expect("valid single-body fixture");
        assert_eq!(report.fps_hz, 30.0);
        assert_eq!(report.translation_unit, "mm");
        assert_eq!(report.euler_order, "ZYX");
        assert_eq!(report.quaternion_component_order, "XYZW");
        assert_eq!(report.bodies.len(), 1);
        assert_eq!(report.bodies[0].body_id, "ball");
        assert_eq!(report.bodies[0].sample_count, 2);
        assert_eq!(report.bodies[0].maximum_quaternion_norm_deviation, 0.0);
    }

    #[test]
    fn accepts_multiple_named_body_streams() {
        let report = validate_trial_json(MULTI_BODY).expect("valid multi-body fixture");
        assert_eq!(report.bodies.len(), 2);
        assert_eq!(report.bodies[0].body_id, "ball_active");
        assert_eq!(report.bodies[1].body_id, "ball_passive");
        assert!(report.bodies.iter().all(|body| body.sample_count == 2));
    }

    #[test]
    fn converts_declared_millimetres_to_metres_without_mutating_raw_values() {
        let report = validate_trial_json(SINGLE_BODY).expect("valid fixture");
        assert_eq!(report.position_component_metres(1000.0), Ok(1.0));
        assert_eq!(report.position_component_metres(450.0), Ok(0.45));
        assert_eq!(
            report.position_component_metres(f64::INFINITY),
            Err(TrialError::InvalidDerivedValue)
        );
    }

    #[test]
    fn derives_relative_frame_time_but_does_not_claim_absolute_clock() {
        let report = validate_trial_json(SINGLE_BODY).expect("valid fixture");
        assert_eq!(report.relative_frame_time_seconds(0), Ok(0.0));
        assert_eq!(report.relative_frame_time_seconds(30), Ok(1.0));
        assert_eq!(
            report.semantics.absolute_clock_origin,
            KnownSemantics::Unresolved
        );
    }

    #[test]
    fn reports_unresolved_rotation_and_uncertainty_semantics() {
        let report = validate_trial_json(SINGLE_BODY).expect("valid fixture");
        assert_eq!(
            report.semantics.quaternion_component_storage_order,
            KnownSemantics::Declared
        );
        assert_eq!(
            report.semantics.quaternion_algebra,
            KnownSemantics::Unresolved
        );
        assert_eq!(
            report.semantics.transform_direction,
            KnownSemantics::Unresolved
        );
        assert_eq!(
            report.semantics.coordinate_handedness,
            KnownSemantics::Unresolved
        );
        assert_eq!(
            report.semantics.frame_identifiers,
            KnownSemantics::Unresolved
        );
        assert_eq!(
            report.semantics.euler_angle_unit,
            KnownSemantics::Unresolved
        );
        assert_eq!(
            report.semantics.per_sample_measurement_uncertainty,
            KnownSemantics::Unresolved
        );
    }

    #[test]
    fn reports_quaternion_norm_deviation_without_normalizing() {
        let input = replace_once(SINGLE_BODY, "\"qw\": [1.0, 1.0]", "\"qw\": [2.0, 1.0]");
        let report = validate_trial_json(&input).expect("finite nonzero quaternion components");
        assert_eq!(report.bodies[0].maximum_quaternion_norm_deviation, 1.0);
    }

    #[test]
    fn rejects_zero_norm_quaternion() {
        let input = replace_once(SINGLE_BODY, "\"qw\": [1.0, 1.0]", "\"qw\": [0.0, 1.0]");
        assert_eq!(
            validate_trial_json(&input),
            Err(TrialError::DegenerateQuaternion {
                body: "ball".to_owned(),
                index: 0,
            })
        );
    }

    #[test]
    fn rejects_unequal_position_and_orientation_channel_lengths() {
        let input = replace_once(SINGLE_BODY, "\"z\": [450.0, 440.0]", "\"z\": [450.0]");
        assert!(matches!(
            validate_trial_json(&input),
            Err(TrialError::ChannelSampleCountMismatch {
                body,
                channel: "z",
                expected: 2,
                actual: 1,
            }) if body == "ball"
        ));
    }

    #[test]
    fn rejects_missing_channel() {
        let input = replace_once(SINGLE_BODY, "\"qy\": [0.0, 0.0],", "");
        assert!(matches!(
            validate_trial_json(&input),
            Err(TrialError::MissingChannel {
                body,
                channel: "qy",
            }) if body == "ball"
        ));
    }

    #[test]
    fn rejects_non_numeric_channel_sample() {
        let input = replace_once(
            SINGLE_BODY,
            "\"x\": [1000.0, 500.0]",
            "\"x\": [1000.0, null]",
        );
        assert!(matches!(
            validate_trial_json(&input),
            Err(TrialError::InvalidSample {
                body,
                channel: "x",
                index: 1,
            }) if body == "ball"
        ));
    }

    #[test]
    fn rejects_non_positive_or_non_finite_fps() {
        let zero = replace_once(SINGLE_BODY, "\"FPS\": 30", "\"FPS\": 0");
        assert_eq!(validate_trial_json(&zero), Err(TrialError::InvalidFps));

        let non_numeric = replace_once(SINGLE_BODY, "\"FPS\": 30", "\"FPS\": \"thirty\"");
        assert_eq!(
            validate_trial_json(&non_numeric),
            Err(TrialError::InvalidFps)
        );
    }

    #[test]
    fn rejects_unsupported_units_and_euler_order() {
        let wrong_unit = replace_once(
            SINGLE_BODY,
            "\"translation unit\": \"mm\"",
            "\"translation unit\": \"cm\"",
        );
        assert_eq!(
            validate_trial_json(&wrong_unit),
            Err(TrialError::UnsupportedTranslationUnit)
        );

        let wrong_order = replace_once(
            SINGLE_BODY,
            "\"euler order\": \"ZYX\"",
            "\"euler order\": \"XYZ\"",
        );
        assert_eq!(
            validate_trial_json(&wrong_order),
            Err(TrialError::UnsupportedEulerOrder)
        );
    }

    #[test]
    fn rejects_missing_body_streams_and_malformed_json() {
        let no_body = r#"{"FPS":30,"translation unit":"mm","euler order":"ZYX"}"#;
        assert_eq!(validate_trial_json(no_body), Err(TrialError::NoBodyStreams));
        assert_eq!(validate_trial_json("{"), Err(TrialError::InvalidJson));
    }
}
