#![forbid(unsafe_code)]

//! Deterministic fixed-size tactile-field measurement reference.
//!
//! V0 accepts one flat, calibrated 32x32 tile reporting normal pressure.
//! It computes normal force, thresholded contact area, and the 3D center of
//! pressure in the declared surface frame.
//!
//! It does not infer intent, consent, shear, slip, tissue safety, or actions.
//! It does not interpolate missing taxels, de-saturate readings, synchronize
//! clocks, or authorize motion.

use core::fmt;

pub const TACTILE_WIDTH: usize = 32;
pub const TACTILE_HEIGHT: usize = 32;
pub const TACTILE_TAXEL_COUNT: usize = TACTILE_WIDTH * TACTILE_HEIGHT;
pub const TACTILE_MASK_WORDS: usize = TACTILE_TAXEL_COUNT / 64;
const UNIT_VECTOR_TOLERANCE: f64 = 1.0e-6;

/// Identifies one monotonic clock instance and boot epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClockDomain {
    pub clock_id: u64,
    pub boot_epoch: u64,
}

/// Immutable calibration and flat-surface registration for one tactile tile.
///
/// Axis vectors must be orthonormal unit vectors in the declared spatial
/// frame. The measured normal is their right-handed cross product.
/// Calibration and geometry revisions are separately identified and both
/// are bound by the profile digest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TactileTileCalibration {
    pub tile_id: u64,
    pub surface_frame_id: u64,
    pub calibration_digest: [u8; 32],
    pub geometry_revision_digest: [u8; 32],
    pub measurement_profile_digest: [u8; 32],
    pub origin_m: [f64; 3],
    pub axis_u_unit: [f64; 3],
    pub axis_v_unit: [f64; 3],
    pub pitch_u_m: f64,
    pub pitch_v_m: f64,
    pub taxel_area_m2: f64,
    /// Calibrated normal pressure must be strictly greater than this threshold
    /// to count as a loaded taxel. This is not a tissue-safety threshold.
    pub contact_threshold_pa: f64,
}

/// One complete 32x32 sample from a single tactile tile.
///
/// pressure_pa is calibrated normal pressure in pascals. Every taxel must be
/// marked valid and no taxel may be marked saturated for V0 features to be
/// emitted. This intentionally rejects partial fields instead of inventing
/// a contact force or center-of-pressure estimate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TactileFieldFrame {
    pub tile_id: u64,
    pub sample_sequence: u64,
    pub timestamp_ns: u64,
    pub clock: ClockDomain,
    pub surface_frame_id: u64,
    pub measurement_profile_digest: [u8; 32],
    pub valid_taxels: [u64; TACTILE_MASK_WORDS],
    pub saturated_taxels: [u64; TACTILE_MASK_WORDS],
    pub pressure_pa: [f64; TACTILE_TAXEL_COUNT],
}

/// Derived contact features for one exact tactile frame.
///
/// Normal force is the integral of calibrated normal pressure over active
/// taxel area. Center of pressure is expressed in the declared surface frame.
/// A no-contact frame has zero force/area and no center of pressure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TactileFieldFeatures {
    pub tile_id: u64,
    pub sample_sequence: u64,
    pub timestamp_ns: u64,
    pub clock: ClockDomain,
    pub surface_frame_id: u64,
    pub measurement_profile_digest: [u8; 32],
    pub normal_direction_in_frame: [f64; 3],
    pub total_normal_force_n: f64,
    pub contact_area_m2: f64,
    pub center_of_pressure_m: Option<[f64; 3]>,
    pub active_taxel_count: u16,
    pub maximum_pressure_pa: f64,
}

/// Fail-closed errors. Missing or invalid data never becomes zero pressure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TactileFieldError {
    ZeroTileIdentity,
    ZeroSurfaceFrameIdentity,
    ZeroClockIdentity,
    MissingCalibrationDigest,
    MissingGeometryRevisionDigest,
    MissingMeasurementProfileDigest,
    InvalidCalibrationGeometry,
    InvalidCalibrationDimensions,
    TileIdentityMismatch,
    SurfaceFrameMismatch,
    MeasurementProfileMismatch,
    IncompleteField,
    SaturatedField,
    InvalidPressureValue,
    ArithmeticOverflow,
}

impl fmt::Display for TactileFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TactileFieldError {}

impl TactileTileCalibration {
    /// Validate the immutable flat-tile calibration before using it.
    pub fn validate(&self) -> Result<(), TactileFieldError> {
        if self.tile_id == 0 {
            return Err(TactileFieldError::ZeroTileIdentity);
        }
        if self.surface_frame_id == 0 {
            return Err(TactileFieldError::ZeroSurfaceFrameIdentity);
        }
        if self.calibration_digest == [0; 32] {
            return Err(TactileFieldError::MissingCalibrationDigest);
        }
        if self.geometry_revision_digest == [0; 32] {
            return Err(TactileFieldError::MissingGeometryRevisionDigest);
        }
        if self.measurement_profile_digest == [0; 32] {
            return Err(TactileFieldError::MissingMeasurementProfileDigest);
        }
        if !self.origin_m.iter().all(|value| value.is_finite())
            || !self.axis_u_unit.iter().all(|value| value.is_finite())
            || !self.axis_v_unit.iter().all(|value| value.is_finite())
        {
            return Err(TactileFieldError::InvalidCalibrationGeometry);
        }

        let norm_u = dot3(self.axis_u_unit, self.axis_u_unit);
        let norm_v = dot3(self.axis_v_unit, self.axis_v_unit);
        let dot_uv = dot3(self.axis_u_unit, self.axis_v_unit);
        if (norm_u - 1.0).abs() > UNIT_VECTOR_TOLERANCE
            || (norm_v - 1.0).abs() > UNIT_VECTOR_TOLERANCE
            || dot_uv.abs() > UNIT_VECTOR_TOLERANCE
        {
            return Err(TactileFieldError::InvalidCalibrationGeometry);
        }

        let area_from_pitch = self.pitch_u_m * self.pitch_v_m;
        if !self.pitch_u_m.is_finite()
            || !self.pitch_v_m.is_finite()
            || !self.taxel_area_m2.is_finite()
            || !self.contact_threshold_pa.is_finite()
            || self.pitch_u_m <= 0.0
            || self.pitch_v_m <= 0.0
            || self.taxel_area_m2 <= 0.0
            || self.contact_threshold_pa < 0.0
            || !area_from_pitch.is_finite()
            || self.taxel_area_m2 > area_from_pitch * (1.0 + UNIT_VECTOR_TOLERANCE)
        {
            return Err(TactileFieldError::InvalidCalibrationDimensions);
        }

        let normal = cross3(self.axis_u_unit, self.axis_v_unit);
        let normal_norm = dot3(normal, normal).sqrt();
        if !normal_norm.is_finite() || (normal_norm - 1.0).abs() > UNIT_VECTOR_TOLERANCE {
            return Err(TactileFieldError::InvalidCalibrationGeometry);
        }
        Ok(())
    }
}

/// Analyze one complete tactile frame without guessing missing/saturated values.
///
/// This is an offline measurement primitive only. It returns no intent label,
/// controller reference, assistance command, or actuator authorization.
pub fn analyze_tactile_field(
    calibration: &TactileTileCalibration,
    frame: &TactileFieldFrame,
) -> Result<TactileFieldFeatures, TactileFieldError> {
    calibration.validate()?;

    if frame.tile_id != calibration.tile_id {
        return Err(TactileFieldError::TileIdentityMismatch);
    }
    if frame.surface_frame_id != calibration.surface_frame_id {
        return Err(TactileFieldError::SurfaceFrameMismatch);
    }
    if frame.measurement_profile_digest != calibration.measurement_profile_digest {
        return Err(TactileFieldError::MeasurementProfileMismatch);
    }
    if frame.clock.clock_id == 0 {
        return Err(TactileFieldError::ZeroClockIdentity);
    }
    if frame.saturated_taxels.iter().any(|word| *word != 0) {
        return Err(TactileFieldError::SaturatedField);
    }
    if frame.valid_taxels.iter().any(|word| *word != u64::MAX) {
        return Err(TactileFieldError::IncompleteField);
    }

    let normal = cross3(calibration.axis_u_unit, calibration.axis_v_unit);
    let mut total_force_n = 0.0_f64;
    let mut weighted_position = [0.0_f64; 3];
    let mut contact_area_m2 = 0.0_f64;
    let mut active_taxels = 0_u16;
    let mut maximum_pressure_pa = 0.0_f64;

    for (index, pressure_pa) in frame.pressure_pa.iter().copied().enumerate() {
        if !pressure_pa.is_finite() || pressure_pa < 0.0 {
            return Err(TactileFieldError::InvalidPressureValue);
        }
        maximum_pressure_pa = maximum_pressure_pa.max(pressure_pa);
        if pressure_pa <= calibration.contact_threshold_pa {
            continue;
        }

        let force_n = pressure_pa * calibration.taxel_area_m2;
        let row = index / TACTILE_WIDTH;
        let column = index % TACTILE_WIDTH;
        let u_m = (column as f64 + 0.5) * calibration.pitch_u_m;
        let v_m = (row as f64 + 0.5) * calibration.pitch_v_m;
        let position = [
            calibration.origin_m[0]
                + calibration.axis_u_unit[0] * u_m
                + calibration.axis_v_unit[0] * v_m,
            calibration.origin_m[1]
                + calibration.axis_u_unit[1] * u_m
                + calibration.axis_v_unit[1] * v_m,
            calibration.origin_m[2]
                + calibration.axis_u_unit[2] * u_m
                + calibration.axis_v_unit[2] * v_m,
        ];

        total_force_n += force_n;
        for (weighted, coordinate) in weighted_position.iter_mut().zip(position.iter()) {
            *weighted += force_n * coordinate;
        }
        contact_area_m2 += calibration.taxel_area_m2;
        active_taxels = active_taxels
            .checked_add(1)
            .ok_or(TactileFieldError::ArithmeticOverflow)?;

        if !force_n.is_finite()
            || !total_force_n.is_finite()
            || !weighted_position.iter().all(|value| value.is_finite())
            || !contact_area_m2.is_finite()
        {
            return Err(TactileFieldError::ArithmeticOverflow);
        }
    }

    let center_of_pressure_m = if total_force_n > 0.0 {
        Some([
            weighted_position[0] / total_force_n,
            weighted_position[1] / total_force_n,
            weighted_position[2] / total_force_n,
        ])
    } else {
        None
    };

    if center_of_pressure_m
        .map(|position| !position.iter().all(|value| value.is_finite()))
        .unwrap_or(false)
    {
        return Err(TactileFieldError::ArithmeticOverflow);
    }

    Ok(TactileFieldFeatures {
        tile_id: frame.tile_id,
        sample_sequence: frame.sample_sequence,
        timestamp_ns: frame.timestamp_ns,
        clock: frame.clock,
        surface_frame_id: frame.surface_frame_id,
        measurement_profile_digest: frame.measurement_profile_digest,
        normal_direction_in_frame: normal,
        total_normal_force_n: total_force_n,
        contact_area_m2,
        center_of_pressure_m,
        active_taxel_count: active_taxels,
        maximum_pressure_pa,
    })
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: [u8; 32] = [0x42; 32];
    const CALIBRATION: [u8; 32] = [0x13; 32];
    const GEOMETRY: [u8; 32] = [0x24; 32];
    const ALL_VALID: [u64; TACTILE_MASK_WORDS] = [u64::MAX; TACTILE_MASK_WORDS];
    const NONE_SATURATED: [u64; TACTILE_MASK_WORDS] = [0; TACTILE_MASK_WORDS];

    fn calibration() -> TactileTileCalibration {
        TactileTileCalibration {
            tile_id: 7,
            surface_frame_id: 19,
            calibration_digest: CALIBRATION,
            geometry_revision_digest: GEOMETRY,
            measurement_profile_digest: PROFILE,
            origin_m: [1.0, 2.0, 3.0],
            axis_u_unit: [1.0, 0.0, 0.0],
            axis_v_unit: [0.0, 1.0, 0.0],
            pitch_u_m: 0.01,
            pitch_v_m: 0.01,
            taxel_area_m2: 0.0001,
            contact_threshold_pa: 10.0,
        }
    }

    fn frame() -> TactileFieldFrame {
        TactileFieldFrame {
            tile_id: 7,
            sample_sequence: 100,
            timestamp_ns: 1_000_000,
            clock: ClockDomain {
                clock_id: 23,
                boot_epoch: 5,
            },
            surface_frame_id: 19,
            measurement_profile_digest: PROFILE,
            valid_taxels: ALL_VALID,
            saturated_taxels: NONE_SATURATED,
            pressure_pa: [0.0; TACTILE_TAXEL_COUNT],
        }
    }

    #[test]
    fn calibrated_two_taxel_fixture_has_known_force_area_and_center_of_pressure() {
        let mut sample = frame();
        sample.pressure_pa[0] = 1_000.0;
        sample.pressure_pa[1] = 3_000.0;

        let result = analyze_tactile_field(&calibration(), &sample).unwrap();

        assert!((result.total_normal_force_n - 0.4).abs() < 1.0e-12);
        assert!((result.contact_area_m2 - 0.0002).abs() < 1.0e-12);
        assert_eq!(result.active_taxel_count, 2);
        let cop = result.center_of_pressure_m.unwrap();
        assert!((cop[0] - 1.0125).abs() < 1.0e-12);
        assert!((cop[1] - 2.005).abs() < 1.0e-12);
        assert!((cop[2] - 3.0).abs() < 1.0e-12);
        assert_eq!(result.normal_direction_in_frame, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn no_contact_returns_zero_force_and_no_center_of_pressure() {
        let result = analyze_tactile_field(&calibration(), &frame()).unwrap();

        assert_eq!(result.total_normal_force_n, 0.0);
        assert_eq!(result.contact_area_m2, 0.0);
        assert_eq!(result.active_taxel_count, 0);
        assert_eq!(result.center_of_pressure_m, None);
    }

    #[test]
    fn contact_migration_changes_center_of_pressure_without_intent_classification() {
        let mut left = frame();
        left.pressure_pa[0] = 1_000.0;
        let mut right = frame();
        right.pressure_pa[31] = 1_000.0;

        let left_result = analyze_tactile_field(&calibration(), &left).unwrap();
        let right_result = analyze_tactile_field(&calibration(), &right).unwrap();

        let left_cop = left_result.center_of_pressure_m.unwrap();
        let right_cop = right_result.center_of_pressure_m.unwrap();
        assert!(right_cop[0] > left_cop[0]);
        assert_eq!(left_result.total_normal_force_n, right_result.total_normal_force_n);
    }

    #[test]
    fn asymmetric_loads_preserve_distribution_in_force_weighted_center() {
        let mut sample = frame();
        sample.pressure_pa[0] = 2_000.0;
        sample.pressure_pa[32] = 1_000.0;
        sample.pressure_pa[64] = 1_000.0;

        let result = analyze_tactile_field(&calibration(), &sample).unwrap();
        let cop = result.center_of_pressure_m.unwrap();

        assert!((result.total_normal_force_n - 0.4).abs() < 1.0e-12);
        assert!((cop[0] - 1.005).abs() < 1.0e-12);
        assert!((cop[1] - 2.015).abs() < 1.0e-12);
    }

    #[test]
    fn incomplete_taxel_field_returns_unknown_instead_of_interpolating() {
        let mut sample = frame();
        sample.valid_taxels[0] &= !(1_u64 << 12);
        assert_eq!(
            analyze_tactile_field(&calibration(), &sample),
            Err(TactileFieldError::IncompleteField)
        );
    }

    #[test]
    fn saturated_taxel_field_is_rejected_without_rescaling() {
        let mut sample = frame();
        sample.saturated_taxels[0] |= 1_u64 << 12;
        assert_eq!(
            analyze_tactile_field(&calibration(), &sample),
            Err(TactileFieldError::SaturatedField)
        );
    }

    #[test]
    fn changed_tile_frame_or_profile_identity_is_rejected() {
        let mut sample = frame();
        sample.tile_id += 1;
        assert_eq!(
            analyze_tactile_field(&calibration(), &sample),
            Err(TactileFieldError::TileIdentityMismatch)
        );

        sample = frame();
        sample.surface_frame_id += 1;
        assert_eq!(
            analyze_tactile_field(&calibration(), &sample),
            Err(TactileFieldError::SurfaceFrameMismatch)
        );

        sample = frame();
        sample.measurement_profile_digest[0] ^= 0xff;
        assert_eq!(
            analyze_tactile_field(&calibration(), &sample),
            Err(TactileFieldError::MeasurementProfileMismatch)
        );
    }

    #[test]
    fn nonfinite_and_negative_pressures_are_rejected() {
        let mut sample = frame();
        sample.pressure_pa[0] = f64::NAN;
        assert_eq!(
            analyze_tactile_field(&calibration(), &sample),
            Err(TactileFieldError::InvalidPressureValue)
        );

        sample = frame();
        sample.pressure_pa[0] = -1.0;
        assert_eq!(
            analyze_tactile_field(&calibration(), &sample),
            Err(TactileFieldError::InvalidPressureValue)
        );
    }

    #[test]
    fn invalid_surface_geometry_and_taxel_dimensions_are_rejected() {
        let mut invalid = calibration();
        invalid.axis_v_unit = [1.0, 0.0, 0.0];
        assert_eq!(
            invalid.validate(),
            Err(TactileFieldError::InvalidCalibrationGeometry)
        );

        invalid = calibration();
        invalid.taxel_area_m2 = 0.001;
        assert_eq!(
            invalid.validate(),
            Err(TactileFieldError::InvalidCalibrationDimensions)
        );
    }

    #[test]
    fn zero_calibration_or_profile_digest_is_rejected() {
        let mut invalid = calibration();
        invalid.calibration_digest = [0; 32];
        assert_eq!(
            invalid.validate(),
            Err(TactileFieldError::MissingCalibrationDigest)
        );

        invalid = calibration();
        invalid.measurement_profile_digest = [0; 32];
        assert_eq!(
            invalid.validate(),
            Err(TactileFieldError::MissingMeasurementProfileDigest)
        );
    }
}
