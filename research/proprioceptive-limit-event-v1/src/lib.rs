// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fixed-point joint-limit event detection for embodied sensorimotor research.
//!
//! This crate detects a threshold crossing from a sequence of timestamped
//! joint-angle observations. It emits a bounded, source-addressable event only;
//! it does not generate muscle activations, control physics, or claim to
//! reproduce a biological receptor such as Drosophila CxHP8.
//!
//! Angles use signed microradians (1 radian = 1_000_000 units). Thresholds
//! must be supplied by a separately qualified calibration/model authority.

#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitSide {
    Lower,
    Upper,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitDetectorConfig {
    pub joint_id: String,
    pub side: LimitSide,
    pub minimum_angle_urad: i64,
    pub maximum_angle_urad: i64,
    pub trigger_angle_urad: i64,
    pub release_hysteresis_urad: u64,
    pub maximum_sample_gap_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointAngleSample {
    pub joint_id: String,
    pub tick: u64,
    pub angle_urad: i64,
    /// Digest reference for the authoritative body snapshot used to derive the sample.
    pub body_state_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointLimitEvent {
    pub joint_id: String,
    pub side: LimitSide,
    pub trigger_angle_urad: i64,
    /// Crossing is only localized to this closed sample interval.
    pub bracket_start_tick: u64,
    pub bracket_end_tick: u64,
    pub previous_body_state_sha256: String,
    pub current_body_state_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorError {
    InvalidJointId,
    InvalidConfiguration,
    JointMismatch,
    InvalidBodyStateDigest,
    ZeroSampleTick,
    AngleOutOfRange,
    NonMonotonicTick,
    SampleGapExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointLimitDetector {
    config: LimitDetectorConfig,
    previous: Option<JointAngleSample>,
    armed: bool,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl LimitDetectorConfig {
    pub fn validate(&self) -> Result<(), DetectorError> {
        if !valid_identifier(&self.joint_id) {
            return Err(DetectorError::InvalidJointId);
        }
        if self.minimum_angle_urad >= self.maximum_angle_urad
            || self.maximum_sample_gap_ticks == 0
            || self.release_hysteresis_urad == 0
            || self.trigger_angle_urad < self.minimum_angle_urad
            || self.trigger_angle_urad > self.maximum_angle_urad
        {
            return Err(DetectorError::InvalidConfiguration);
        }

        let available_hysteresis = match self.side {
            LimitSide::Lower => self
                .maximum_angle_urad
                .checked_sub(self.trigger_angle_urad)
                .and_then(|value| u64::try_from(value).ok()),
            LimitSide::Upper => self
                .trigger_angle_urad
                .checked_sub(self.minimum_angle_urad)
                .and_then(|value| u64::try_from(value).ok()),
        }
        .ok_or(DetectorError::InvalidConfiguration)?;

        if self.release_hysteresis_urad >= available_hysteresis {
            return Err(DetectorError::InvalidConfiguration);
        }

        Ok(())
    }

    fn release_angle(&self) -> Result<i64, DetectorError> {
        let hysteresis = i64::try_from(self.release_hysteresis_urad)
            .map_err(|_| DetectorError::InvalidConfiguration)?;
        match self.side {
            LimitSide::Lower => self
                .trigger_angle_urad
                .checked_add(hysteresis)
                .ok_or(DetectorError::InvalidConfiguration),
            LimitSide::Upper => self
                .trigger_angle_urad
                .checked_sub(hysteresis)
                .ok_or(DetectorError::InvalidConfiguration),
        }
    }

    fn beyond_trigger(&self, angle_urad: i64) -> bool {
        match self.side {
            LimitSide::Lower => angle_urad <= self.trigger_angle_urad,
            LimitSide::Upper => angle_urad >= self.trigger_angle_urad,
        }
    }

    fn beyond_release(&self, angle_urad: i64) -> Result<bool, DetectorError> {
        let release_angle = self.release_angle()?;
        Ok(match self.side {
            LimitSide::Lower => angle_urad >= release_angle,
            LimitSide::Upper => angle_urad <= release_angle,
        })
    }
}

impl JointLimitDetector {
    pub fn new(config: LimitDetectorConfig) -> Result<Self, DetectorError> {
        config.validate()?;
        Ok(Self {
            config,
            previous: None,
            armed: true,
        })
    }

    /// Observe one sample. A detected crossing returns an event whose time is
    /// bounded by the previous/current sample ticks. A sample gap beyond the
    /// configured bound re-baselines the detector and returns an error: no
    /// crossing is inferred across an unobserved interval.
    pub fn observe(
        &mut self,
        sample: JointAngleSample,
    ) -> Result<Option<JointLimitEvent>, DetectorError> {
        if sample.joint_id != self.config.joint_id {
            return Err(DetectorError::JointMismatch);
        }
        if sample.tick == 0 {
            return Err(DetectorError::ZeroSampleTick);
        }
        if !valid_sha256(&sample.body_state_sha256) {
            return Err(DetectorError::InvalidBodyStateDigest);
        }
        if sample.angle_urad < self.config.minimum_angle_urad
            || sample.angle_urad > self.config.maximum_angle_urad
        {
            return Err(DetectorError::AngleOutOfRange);
        }

        let Some(previous) = self.previous.as_ref() else {
            self.armed = !self.config.beyond_trigger(sample.angle_urad);
            self.previous = Some(sample);
            return Ok(None);
        };

        if sample.tick <= previous.tick {
            return Err(DetectorError::NonMonotonicTick);
        }

        let gap = sample.tick - previous.tick;
        if gap > self.config.maximum_sample_gap_ticks {
            // A missing interval must not bypass the release hysteresis of an
            // already-fired detector. Preserve its latch until the actual
            // release boundary is observed.
            self.armed = if self.armed {
                !self.config.beyond_trigger(sample.angle_urad)
            } else {
                self.config.beyond_release(sample.angle_urad)?
            };
            self.previous = Some(sample);
            return Err(DetectorError::SampleGapExceeded);
        }

        let crossed = self.armed
            && !self.config.beyond_trigger(previous.angle_urad)
            && self.config.beyond_trigger(sample.angle_urad);

        let event = if crossed {
            self.armed = false;
            Some(JointLimitEvent {
                joint_id: self.config.joint_id.clone(),
                side: self.config.side,
                trigger_angle_urad: self.config.trigger_angle_urad,
                bracket_start_tick: previous.tick,
                bracket_end_tick: sample.tick,
                previous_body_state_sha256: previous.body_state_sha256.clone(),
                current_body_state_sha256: sample.body_state_sha256.clone(),
            })
        } else {
            if !self.armed && self.config.beyond_release(sample.angle_urad)? {
                self.armed = true;
            }
            None
        };

        self.previous = Some(sample);
        Ok(event)
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    pub fn last_sample_tick(&self) -> Option<u64> {
        self.previous.as_ref().map(|sample| sample.tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn config(side: LimitSide) -> LimitDetectorConfig {
        LimitDetectorConfig {
            joint_id: "thorax-coxa.front-left".to_string(),
            side,
            minimum_angle_urad: -2_000_000,
            maximum_angle_urad: 2_000_000,
            trigger_angle_urad: match side {
                LimitSide::Lower => -1_000_000,
                LimitSide::Upper => 1_000_000,
            },
            release_hysteresis_urad: 50_000,
            maximum_sample_gap_ticks: 10,
        }
    }

    fn sample(tick: u64, angle_urad: i64) -> JointAngleSample {
        JointAngleSample {
            joint_id: "thorax-coxa.front-left".to_string(),
            tick,
            angle_urad,
            body_state_sha256: digest('a'),
        }
    }

    #[test]
    fn upper_limit_emits_one_event_on_safe_to_limit_crossing() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        assert_eq!(detector.observe(sample(1, 900_000)), Ok(None));
        let event = detector.observe(sample(2, 1_000_000)).expect("observation");
        let event = event.expect("crossing should emit event");
        assert_eq!(event.side, LimitSide::Upper);
        assert_eq!(event.bracket_start_tick, 1);
        assert_eq!(event.bracket_end_tick, 2);
        assert!(!detector.is_armed());
    }

    #[test]
    fn upper_limit_does_not_repeat_until_hysteresis_release() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        detector.observe(sample(1, 900_000)).expect("sample");
        assert!(
            detector
                .observe(sample(2, 1_000_000))
                .expect("sample")
                .is_some()
        );
        assert_eq!(detector.observe(sample(3, 1_100_000)), Ok(None));
        assert_eq!(detector.observe(sample(4, 950_000)), Ok(None));
        assert!(detector.is_armed());
        assert!(
            detector
                .observe(sample(5, 1_000_000))
                .expect("sample")
                .is_some()
        );
    }

    #[test]
    fn lower_limit_emits_on_downward_crossing() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Lower)).expect("valid config");
        assert_eq!(detector.observe(sample(1, -900_000)), Ok(None));
        let event = detector.observe(sample(2, -1_000_000)).expect("sample");
        assert_eq!(event.expect("crossing").side, LimitSide::Lower);
    }

    #[test]
    fn initial_state_beyond_limit_does_not_invent_crossing() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        assert_eq!(detector.observe(sample(1, 1_100_000)), Ok(None));
        assert!(!detector.is_armed());
    }

    #[test]
    fn sample_gap_rebaselines_without_interpolating_a_crossing() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        detector.observe(sample(1, 900_000)).expect("sample");
        assert_eq!(
            detector.observe(sample(12, 1_100_000)),
            Err(DetectorError::SampleGapExceeded)
        );
        assert!(!detector.is_armed());
        assert_eq!(detector.last_sample_tick(), Some(12));
    }

    #[test]
    fn sampling_gap_does_not_bypass_latched_hysteresis() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        detector.observe(sample(1, 900_000)).expect("initial");
        assert!(
            detector
                .observe(sample(2, 1_000_000))
                .expect("crossing")
                .is_some()
        );
        assert_eq!(
            detector.observe(sample(20, 980_000)),
            Err(DetectorError::SampleGapExceeded)
        );
        assert!(!detector.is_armed());
        assert_eq!(detector.observe(sample(21, 960_000)), Ok(None));
        assert!(!detector.is_armed());
        assert_eq!(detector.observe(sample(22, 950_000)), Ok(None));
        assert!(detector.is_armed());
    }

    #[test]
    fn out_of_order_sample_is_rejected_without_replacing_state() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        detector.observe(sample(4, 900_000)).expect("sample");
        assert_eq!(
            detector.observe(sample(3, 1_100_000)),
            Err(DetectorError::NonMonotonicTick)
        );
        assert_eq!(detector.last_sample_tick(), Some(4));
    }

    #[test]
    fn wrong_joint_is_rejected() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        let mut input = sample(1, 900_000);
        input.joint_id = "knee.front-left".to_string();
        assert_eq!(detector.observe(input), Err(DetectorError::JointMismatch));
    }

    #[test]
    fn malformed_body_snapshot_digest_is_rejected() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        let mut input = sample(1, 900_000);
        input.body_state_sha256 = "unbound".to_string();
        assert_eq!(
            detector.observe(input),
            Err(DetectorError::InvalidBodyStateDigest)
        );
    }

    #[test]
    fn angle_outside_declared_joint_range_is_rejected() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        assert_eq!(
            detector.observe(sample(1, 2_000_001)),
            Err(DetectorError::AngleOutOfRange)
        );
    }

    #[test]
    fn zero_tick_is_rejected() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        assert_eq!(
            detector.observe(sample(0, 900_000)),
            Err(DetectorError::ZeroSampleTick)
        );
    }

    #[test]
    fn zero_hysteresis_is_rejected() {
        let mut invalid = config(LimitSide::Upper);
        invalid.release_hysteresis_urad = 0;
        assert_eq!(
            JointLimitDetector::new(invalid),
            Err(DetectorError::InvalidConfiguration)
        );
    }

    #[test]
    fn hysteresis_reaching_opposite_boundary_is_rejected() {
        let mut invalid = config(LimitSide::Upper);
        invalid.release_hysteresis_urad = 3_000_000;
        assert_eq!(
            JointLimitDetector::new(invalid),
            Err(DetectorError::InvalidConfiguration)
        );
    }

    #[test]
    fn trigger_equal_to_declared_boundary_is_valid() {
        let mut boundary = config(LimitSide::Upper);
        boundary.trigger_angle_urad = boundary.maximum_angle_urad;
        boundary.release_hysteresis_urad = 1;
        assert!(JointLimitDetector::new(boundary).is_ok());
    }

    #[test]
    fn event_binds_both_samples_and_their_body_states() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        let first = JointAngleSample {
            body_state_sha256: digest('b'),
            ..sample(1, 900_000)
        };
        let second = JointAngleSample {
            body_state_sha256: digest('c'),
            ..sample(2, 1_000_000)
        };
        detector.observe(first).expect("first");
        let event = detector.observe(second).expect("second").expect("event");
        assert_eq!(event.previous_body_state_sha256, digest('b'));
        assert_eq!(event.current_body_state_sha256, digest('c'));
    }

    #[test]
    fn release_requires_reaching_the_hysteresis_boundary() {
        let mut detector = JointLimitDetector::new(config(LimitSide::Upper)).expect("valid config");
        detector.observe(sample(1, 900_000)).expect("sample");
        detector.observe(sample(2, 1_000_000)).expect("sample");
        assert_eq!(detector.observe(sample(3, 960_000)), Ok(None));
        assert!(!detector.is_armed());
        assert_eq!(detector.observe(sample(4, 950_000)), Ok(None));
        assert!(detector.is_armed());
    }
}
