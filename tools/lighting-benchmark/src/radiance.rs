//! Strict parser and deterministic input builder for the legacy-RGB Radiance `rtrace` workplane profile.
//!
//! This module does not launch Radiance, build an octree, validate a source model,
//! or certify photometric accuracy. It only accepts the explicit six-number
//! `rtrace -h- -I+ -ov -cs 3 -x 1` output profile:
//! echoed input origin (x, y, z) followed by RGB irradiance (R, G, B).
//! RGB-to-lux conversion is versioned because spectral output requires a distinct
//! conversion path and calibration evidence.

use std::fmt::Write as FmtWrite;

/// Versioned approximate photometric conversion for Radiance's legacy RGB mode.
/// Keep this separate from spectral-mode conversions and measured calibration.
pub const LEGACY_RGB_LUX_CONVERSION_ID: &str = "radiance_rgb_legacy_47_4_120_11_6_v1";
pub const LEGACY_RGB_RED_TO_LUX: f64 = 47.4;
pub const LEGACY_RGB_GREEN_TO_LUX: f64 = 120.0;
pub const LEGACY_RGB_BLUE_TO_LUX: f64 = 11.6;

/// A point in the benchmark's frozen task-plane grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasurementPoint {
    pub grid_index: usize,
    pub x_m: f64,
    pub y_m: f64,
    pub z_m: f64,
}

/// One complete decoded line from the explicit legacy-RGB `rtrace` output profile.
/// Both expected and echoed input-origin coordinates are retained; they are never
/// silently replaced by one another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadianceGridSample {
    pub grid_index: usize,
    pub expected_position_m: [f64; 3],
    pub echoed_origin_m: [f64; 3],
    pub irradiance_rgb: [f64; 3],
    pub illuminance_lx: f64,
    pub conversion_id: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RtraceParseError {
    EmptyExpectedGrid,
    InvalidCoordinateTolerance,
    NonFiniteExpectedCoordinate,
    NonSequentialGridIndex,
    OutputRecordCountMismatch,
    EmptyOutputRecord,
    FieldCountMismatch,
    InvalidNumber,
    NonFiniteOutputValue,
    NegativeIrradianceChannel,
    CoordinateMismatch,
    NonFiniteIlluminance,
}

impl RtraceParseError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyExpectedGrid => "rtrace_expected_grid_empty",
            Self::InvalidCoordinateTolerance => "rtrace_coordinate_tolerance_invalid",
            Self::NonFiniteExpectedCoordinate => "rtrace_expected_coordinate_non_finite",
            Self::NonSequentialGridIndex => "rtrace_grid_indices_not_sequential",
            Self::OutputRecordCountMismatch => "rtrace_output_record_count_mismatch",
            Self::EmptyOutputRecord => "rtrace_output_record_empty",
            Self::FieldCountMismatch => "rtrace_output_field_count_mismatch",
            Self::InvalidNumber => "rtrace_output_invalid_number",
            Self::NonFiniteOutputValue => "rtrace_output_non_finite_value",
            Self::NegativeIrradianceChannel => "rtrace_negative_irradiance_channel",
            Self::CoordinateMismatch => "rtrace_reported_coordinate_mismatch",
            Self::NonFiniteIlluminance => "rtrace_derived_illuminance_non_finite",
        }
    }
}

fn validate_expected_grid(
    expected: &[MeasurementPoint],
    coordinate_tolerance_m: f64,
) -> Result<(), RtraceParseError> {
    if expected.is_empty() {
        return Err(RtraceParseError::EmptyExpectedGrid);
    }
    if !coordinate_tolerance_m.is_finite() || coordinate_tolerance_m <= 0.0 {
        return Err(RtraceParseError::InvalidCoordinateTolerance);
    }
    for (index, point) in expected.iter().enumerate() {
        if point.grid_index != index {
            return Err(RtraceParseError::NonSequentialGridIndex);
        }
        if !point.x_m.is_finite() || !point.y_m.is_finite() || !point.z_m.is_finite() {
            return Err(RtraceParseError::NonFiniteExpectedCoordinate);
        }
    }
    Ok(())
}

/// Render deterministic ray input for `rtrace -I`.
///
/// `-I` interprets each six-number input row as a measurement point followed by
/// an orientation vector. This benchmark profile measures the upward-facing
/// horizontal plane, so the normal is exactly (0, 0, 1). Coordinates are written
/// with 17 digits after the decimal in scientific notation to round-trip f64
/// values and to make input bytes stable across repeated invocations.
pub fn render_rtrace_irradiance_input(
    expected: &[MeasurementPoint],
) -> Result<String, RtraceParseError> {
    validate_expected_grid(expected, 1.0e-6)?;
    let mut input = String::with_capacity(expected.len() * 100);
    for point in expected {
        writeln!(
            &mut input,
            "{:.17e} {:.17e} {:.17e} 0 0 1",
            point.x_m, point.y_m, point.z_m
        )
        .expect("writing to a String cannot fail");
    }
    Ok(input)
}

/// Parse all records from `rtrace -h- -I+ -ov -cs 3 -x 1` output.
///
/// The `-opv` output profile is six finite numbers per line: reported point xyz
/// then RGB irradiance. Lines are mapped to the frozen grid by their exact order,
/// and coordinates are checked before any sample is returned. Headers, blank
/// records, reordered points, partial output and extra output are errors. The
/// returned vector is all-or-nothing; callers must not score a partial prefix.
///
/// The RGB-to-lux coefficients are the legacy Radiance workplane conversion for
/// this named profile only. They do not establish accuracy for a non-RGB/spectral
/// run, a source with unverified spectral data, or a physical product.
pub fn parse_rtrace_legacy_rgb_irradiance(
    output: &str,
    expected: &[MeasurementPoint],
    coordinate_tolerance_m: f64,
) -> Result<Vec<RadianceGridSample>, RtraceParseError> {
    validate_expected_grid(expected, coordinate_tolerance_m)?;

    let records = output.lines().collect::<Vec<_>>();
    if records.len() != expected.len() {
        return Err(RtraceParseError::OutputRecordCountMismatch);
    }

    let mut samples = Vec::with_capacity(expected.len());
    for (line, point) in records.iter().zip(expected.iter()) {
        if line.trim().is_empty() {
            return Err(RtraceParseError::EmptyOutputRecord);
        }

        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 6 {
            return Err(RtraceParseError::FieldCountMismatch);
        }

        let mut values = [0.0_f64; 6];
        for (index, field) in fields.iter().enumerate() {
            values[index] = field
                .parse::<f64>()
                .map_err(|_| RtraceParseError::InvalidNumber)?;
            if !values[index].is_finite() {
                return Err(RtraceParseError::NonFiniteOutputValue);
            }
        }

        let echoed_origin_m = [values[0], values[1], values[2]];
        let expected_position_m = [point.x_m, point.y_m, point.z_m];
        if echoed_origin_m
            .iter()
            .zip(expected_position_m.iter())
            .any(|(actual, wanted)| (actual - wanted).abs() > coordinate_tolerance_m)
        {
            // The expected point's grid_index is the stable diagnostic identity.
            return Err(RtraceParseError::CoordinateMismatch);
        }

        let irradiance_rgb = [values[3], values[4], values[5]];
        if irradiance_rgb.iter().any(|channel| *channel < 0.0) {
            return Err(RtraceParseError::NegativeIrradianceChannel);
        }

        let illuminance_lx = LEGACY_RGB_RED_TO_LUX * irradiance_rgb[0]
            + LEGACY_RGB_GREEN_TO_LUX * irradiance_rgb[1]
            + LEGACY_RGB_BLUE_TO_LUX * irradiance_rgb[2];
        if !illuminance_lx.is_finite() || illuminance_lx < 0.0 {
            return Err(RtraceParseError::NonFiniteIlluminance);
        }

        samples.push(RadianceGridSample {
            grid_index: point.grid_index,
            expected_position_m,
            echoed_origin_m,
            irradiance_rgb,
            illuminance_lx,
            conversion_id: LEGACY_RGB_LUX_CONVERSION_ID,
        });
    }

    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_points() -> [MeasurementPoint; 2] {
        [
            MeasurementPoint {
                grid_index: 0,
                x_m: 1.9,
                y_m: 1.65,
                z_m: 0.75,
            },
            MeasurementPoint {
                grid_index: 1,
                x_m: 2.0,
                y_m: 1.65,
                z_m: 0.75,
            },
        ]
    }

    #[test]
    fn input_builder_uses_upward_normal_and_stable_round_trip_coordinates() {
        let input = render_rtrace_irradiance_input(&two_points()).unwrap();
        assert_eq!(
            input,
            "1.89999999999999991e0 1.64999999999999991e0 7.50000000000000000e-1 0 0 1\n2.00000000000000000e0 1.64999999999999991e0 7.50000000000000000e-1 0 0 1\n"
        );
    }

    #[test]
    fn known_answer_white_rgb_is_179_lux_under_named_legacy_conversion() {
        let output = "1.9 1.65 0.75 1 1 1\n2.0 1.65 0.75 1 1 1\n";
        let result = parse_rtrace_legacy_rgb_irradiance(output, &two_points(), 1.0e-6).unwrap();
        assert_eq!(result.len(), 2);
        assert!((result[0].illuminance_lx - 179.0).abs() < 1.0e-12);
        assert_eq!(result[0].conversion_id, LEGACY_RGB_LUX_CONVERSION_ID);
        assert_eq!(result[0].grid_index, 0);
        assert_eq!(result[1].grid_index, 1);
    }

    #[test]
    fn known_answer_single_channels_use_explicit_weights() {
        let red = parse_rtrace_legacy_rgb_irradiance(
            "1.9 1.65 0.75 1 0 0\n2.0 1.65 0.75 1 0 0\n",
            &two_points(),
            1.0e-6,
        )
        .unwrap();
        let green = parse_rtrace_legacy_rgb_irradiance(
            "1.9 1.65 0.75 0 1 0\n2.0 1.65 0.75 0 1 0\n",
            &two_points(),
            1.0e-6,
        )
        .unwrap();
        let blue = parse_rtrace_legacy_rgb_irradiance(
            "1.9 1.65 0.75 0 0 1\n2.0 1.65 0.75 0 0 1\n",
            &two_points(),
            1.0e-6,
        )
        .unwrap();
        assert!((red[0].illuminance_lx - 47.4).abs() < 1.0e-12);
        assert!((green[0].illuminance_lx - 120.0).abs() < 1.0e-12);
        assert!((blue[0].illuminance_lx - 11.6).abs() < 1.0e-12);
    }

    #[test]
    fn incomplete_extra_and_blank_output_records_fail_closed() {
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance("1.9 1.65 0.75 1 1 1\n", &two_points(), 1.0e-6),
            Err(RtraceParseError::OutputRecordCountMismatch)
        );
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 1 1 1\n2.0 1.65 0.75 1 1 1\n3.0 1.65 0.75 1 1 1\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::OutputRecordCountMismatch)
        );
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 1 1 1\n\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::EmptyOutputRecord)
        );
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 1 1 1\n  \n2.0 1.65 0.75 1 1 1\n",
                &[two_points()[0], two_points()[0], two_points()[1]],
                1.0e-6
            ),
            Err(RtraceParseError::NonSequentialGridIndex)
        );
    }

    #[test]
    fn malformed_headers_and_wrong_field_count_are_rejected() {
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "Radiance 1.65 0.75 1 1 1\n2.0 1.65 0.75 1 1 1\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::InvalidNumber)
        );
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 1 1\n2.0 1.65 0.75 1 1 1\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::FieldCountMismatch)
        );
    }

    #[test]
    fn non_finite_negative_and_overflowing_values_are_rejected() {
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 NaN 1 1\n2.0 1.65 0.75 1 1 1\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::NonFiniteOutputValue)
        );
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 -0.1 1 1\n2.0 1.65 0.75 1 1 1\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::NegativeIrradianceChannel)
        );
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 1e308 1e308 1e308\n2.0 1.65 0.75 1 1 1\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::NonFiniteIlluminance)
        );
    }

    #[test]
    fn coordinate_drift_and_reordered_lines_are_rejected() {
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 1 1 1\n2.001 1.65 0.75 1 1 1\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::CoordinateMismatch)
        );
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "2.0 1.65 0.75 1 1 1\n1.9 1.65 0.75 1 1 1\n",
                &two_points(),
                1.0e-6
            ),
            Err(RtraceParseError::CoordinateMismatch)
        );
    }

    #[test]
    fn empty_or_malformed_expected_grids_are_rejected() {
        assert_eq!(
            render_rtrace_irradiance_input(&[]),
            Err(RtraceParseError::EmptyExpectedGrid)
        );
        assert_eq!(
            render_rtrace_irradiance_input(&[
                MeasurementPoint {
                    grid_index: 1,
                    x_m: 0.0,
                    y_m: 0.0,
                    z_m: 0.0,
                }
            ]),
            Err(RtraceParseError::NonSequentialGridIndex)
        );
        assert_eq!(
            render_rtrace_irradiance_input(&[
                MeasurementPoint {
                    grid_index: 0,
                    x_m: f64::NAN,
                    y_m: 0.0,
                    z_m: 0.0,
                }
            ]),
            Err(RtraceParseError::NonFiniteExpectedCoordinate)
        );
        assert_eq!(
            parse_rtrace_legacy_rgb_irradiance(
                "1.9 1.65 0.75 1 1 1\n2.0 1.65 0.75 1 1 1\n",
                &two_points(),
                0.0
            ),
            Err(RtraceParseError::InvalidCoordinateTolerance)
        );
    }
}
