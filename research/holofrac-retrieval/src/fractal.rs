//! Geometric scale analysis for HoloFrac.
//!
//! A recursive construction is not automatically a fractal. This module
//! measures a declared box-counting curve against known-dimension and
//! non-fractal controls, reporting scale-window and grid-translation
//! sensitivity rather than returning an unsupported binary "fractal" label.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryFamily {
    SierpinskiTriangle,
    RegularGrid,
    UniformRandom,
    LineSegment,
}

impl GeometryFamily {
    pub const ALL: [Self; 4] = [
        Self::SierpinskiTriangle,
        Self::RegularGrid,
        Self::UniformRandom,
        Self::LineSegment,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SierpinskiTriangle => "sierpinski_triangle",
            Self::RegularGrid => "regular_grid",
            Self::UniformRandom => "uniform_random",
            Self::LineSegment => "line_segment",
        }
    }

    pub const fn expected_dimension(self) -> f64 {
        match self {
            Self::SierpinskiTriangle => 3.0_f64.log2(),
            Self::RegularGrid | Self::UniformRandom => 2.0,
            Self::LineSegment => 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FractalError {
    EmptyPointSet,
    InvalidPoint,
    InvalidResolutionSchedule,
    InvalidFitWindow,
    TooFewFitScales,
}

impl std::fmt::Display for FractalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPointSet => f.write_str("fractal analysis requires at least one point"),
            Self::InvalidPoint => f.write_str("all points must be finite and lie inside the unit square"),
            Self::InvalidResolutionSchedule => {
                f.write_str("box-count resolutions must be strictly increasing and at least two")
            }
            Self::InvalidFitWindow => f.write_str("box-count fit window must be within the scale schedule"),
            Self::TooFewFitScales => f.write_str("box-count regression requires at least three scales"),
        }
    }
}

impl std::error::Error for FractalError {}

#[derive(Clone, Copy, Debug)]
pub struct ScaleCount {
    pub resolution: usize,
    pub mean_occupied_boxes: f64,
    pub min_occupied_boxes: usize,
    pub max_occupied_boxes: usize,
}

#[derive(Clone, Debug)]
pub struct BoxCountingReport {
    pub point_count: usize,
    pub dimension_estimate: f64,
    pub fit_r_squared: f64,
    pub log_count_rmse: f64,
    pub fit_start_resolution: usize,
    pub fit_end_resolution: usize,
    pub fit_scale_count: usize,
    pub min_window_dimension: f64,
    pub max_window_dimension: f64,
    pub window_dimension_spread: f64,
    pub mean_translation_count_spread: f64,
    pub scale_counts: Vec<ScaleCount>,
}

const GRID_OFFSETS: [f64; 2] = [0.0, 0.5];
const WINDOW_LENGTH: usize = 4;

/// Generate deterministic point sets with analytic or known reference
/// dimensions. These fixtures are method controls, not architectural claims.
pub fn generate_geometry_fixture(
    family: GeometryFamily,
    point_count: usize,
    seed: u64,
) -> Result<Vec<Point2>, FractalError> {
    if point_count < 2 {
        return Err(FractalError::EmptyPointSet);
    }

    match family {
        GeometryFamily::SierpinskiTriangle => {
            let mut state = seed.max(1);
            let mut point = Point2 { x: 0.17, y: 0.23 };
            let vertices = [
                Point2 { x: 0.0, y: 0.0 },
                Point2 { x: 1.0, y: 0.0 },
                Point2 {
                    x: 0.5,
                    y: 3.0_f64.sqrt() / 2.0,
                },
            ];
            let mut points = Vec::with_capacity(point_count);
            for index in 0..(point_count + 512) {
                let choice = (next_random(&mut state) % 3) as usize;
                let vertex = vertices[choice];
                point.x = (point.x + vertex.x) * 0.5;
                point.y = (point.y + vertex.y) * 0.5;
                if index >= 512 {
                    points.push(Point2 {
                        x: point.x,
                        y: point.y / (3.0_f64.sqrt() / 2.0),
                    });
                }
            }
            Ok(points)
        }
        GeometryFamily::RegularGrid => {
            let side = integer_ceil_sqrt(point_count);
            let denominator = (side - 1).max(1) as f64;
            Ok((0..point_count)
                .map(|index| Point2 {
                    x: (index % side) as f64 / denominator,
                    y: (index / side) as f64 / denominator,
                })
                .collect())
        }
        GeometryFamily::UniformRandom => {
            let mut state = seed.max(1);
            Ok((0..point_count)
                .map(|_| Point2 {
                    x: unit_random(&mut state),
                    y: unit_random(&mut state),
                })
                .collect())
        }
        GeometryFamily::LineSegment => {
            let denominator = (point_count - 1) as f64;
            Ok((0..point_count)
                .map(|index| Point2 {
                    x: index as f64 / denominator,
                    y: 0.5,
                })
                .collect())
        }
    }
}

/// Estimate box-counting dimension by regression of log(N(boxes)) on
/// log(resolution), where resolution is the inverse box width.
///
/// The fit window is declared by indices into the supplied schedule. The
/// finest and coarsest scales should not be used blindly: finite sample counts
/// bias large boxes, while digital resolution and point discreteness bias
/// very small boxes. Four grid translations are reported at every scale.
pub fn estimate_box_counting_dimension(
    points: &[Point2],
    resolutions: &[usize],
    fit_start_index: usize,
    fit_end_index_inclusive: usize,
) -> Result<BoxCountingReport, FractalError> {
    validate_points(points)?;
    if resolutions.len() < 3
        || resolutions.iter().any(|resolution| *resolution < 2)
        || resolutions.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(FractalError::InvalidResolutionSchedule);
    }
    if fit_start_index > fit_end_index_inclusive
        || fit_end_index_inclusive >= resolutions.len()
    {
        return Err(FractalError::InvalidFitWindow);
    }
    let fit_count = fit_end_index_inclusive - fit_start_index + 1;
    if fit_count < 3 {
        return Err(FractalError::TooFewFitScales);
    }

    let mut scale_counts = Vec::with_capacity(resolutions.len());
    let mut translation_spreads = Vec::with_capacity(resolutions.len());
    for &resolution in resolutions {
        let counts = [
            count_occupied_boxes(points, resolution, 0.0, 0.0),
            count_occupied_boxes(points, resolution, 0.0, 0.5),
            count_occupied_boxes(points, resolution, 0.5, 0.0),
            count_occupied_boxes(points, resolution, 0.5, 0.5),
        ];
        let min_count = *counts.iter().min().unwrap_or(&0);
        let max_count = *counts.iter().max().unwrap_or(&0);
        let mean_count = counts.iter().sum::<usize>() as f64 / counts.len() as f64;
        scale_counts.push(ScaleCount {
            resolution,
            mean_occupied_boxes: mean_count,
            min_occupied_boxes: min_count,
            max_occupied_boxes: max_count,
        });
        translation_spreads.push(if mean_count > 0.0 {
            (max_count - min_count) as f64 / mean_count
        } else {
            0.0
        });
    }

    let fit = fit_log_counts(
        &scale_counts[fit_start_index..=fit_end_index_inclusive],
    );
    let mut window_dimensions = Vec::new();
    if resolutions.len() >= WINDOW_LENGTH {
        for start in 0..=(resolutions.len() - WINDOW_LENGTH) {
            let end = start + WINDOW_LENGTH - 1;
            if start <= fit_end_index_inclusive && end >= fit_start_index {
                let window = fit_log_counts(&scale_counts[start..=end]);
                window_dimensions.push(window.slope);
            }
        }
    }
    let min_window_dimension = window_dimensions
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let max_window_dimension = window_dimensions
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let window_dimension_spread = if window_dimensions.is_empty() {
        0.0
    } else {
        max_window_dimension - min_window_dimension
    };

    Ok(BoxCountingReport {
        point_count: points.len(),
        dimension_estimate: fit.slope,
        fit_r_squared: fit.r_squared,
        log_count_rmse: fit.rmse,
        fit_start_resolution: resolutions[fit_start_index],
        fit_end_resolution: resolutions[fit_end_index_inclusive],
        fit_scale_count: fit_count,
        min_window_dimension,
        max_window_dimension,
        window_dimension_spread,
        mean_translation_count_spread: mean(&translation_spreads),
        scale_counts,
    })
}

#[derive(Clone, Copy)]
struct Regression {
    slope: f64,
    r_squared: f64,
    rmse: f64,
}

fn fit_log_counts(counts: &[ScaleCount]) -> Regression {
    let xs: Vec<f64> = counts
        .iter()
        .map(|count| (count.resolution as f64).ln())
        .collect();
    let ys: Vec<f64> = counts
        .iter()
        .map(|count| count.mean_occupied_boxes.ln())
        .collect();
    let x_mean = mean(&xs);
    let y_mean = mean(&ys);
    let ss_xx = xs.iter().map(|x| (x - x_mean).powi(2)).sum::<f64>();
    let ss_xy = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| (x - x_mean) * (y - y_mean))
        .sum::<f64>();
    let slope = if ss_xx > f64::EPSILON {
        ss_xy / ss_xx
    } else {
        0.0
    };
    let intercept = y_mean - slope * x_mean;
    let residuals: Vec<f64> = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| y - (intercept + slope * x))
        .collect();
    let ss_res = residuals.iter().map(|value| value * value).sum::<f64>();
    let ss_tot = ys.iter().map(|value| (value - y_mean).powi(2)).sum::<f64>();
    let r_squared = if ss_tot <= f64::EPSILON {
        if ss_res <= f64::EPSILON { 1.0 } else { 0.0 }
    } else {
        1.0 - ss_res / ss_tot
    };
    Regression {
        slope,
        r_squared,
        rmse: (ss_res / residuals.len() as f64).sqrt(),
    }
}

fn validate_points(points: &[Point2]) -> Result<(), FractalError> {
    if points.is_empty() {
        return Err(FractalError::EmptyPointSet);
    }
    if points.iter().any(|point| {
        !point.x.is_finite()
            || !point.y.is_finite()
            || !(0.0..=1.0).contains(&point.x)
            || !(0.0..=1.0).contains(&point.y)
    }) {
        return Err(FractalError::InvalidPoint);
    }
    Ok(())
}

fn count_occupied_boxes(
    points: &[Point2],
    resolution: usize,
    offset_x: f64,
    offset_y: f64,
) -> usize {
    let mut boxes = HashSet::with_capacity(points.len());
    for point in points {
        let x = (point.x * resolution as f64 + offset_x).floor() as i64;
        let y = (point.y * resolution as f64 + offset_y).floor() as i64;
        boxes.insert((x, y));
    }
    boxes.len()
}

fn integer_ceil_sqrt(value: usize) -> usize {
    let root = (value as f64).sqrt() as usize;
    if root.saturating_mul(root) < value {
        root + 1
    } else {
        root
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn unit_random(state: &mut u64) -> f64 {
    (next_random(state) as f64) / (u64::MAX as f64)
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

pub fn standard_resolutions() -> [usize; 7] {
    [4, 8, 16, 32, 64, 128, 256]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dimension_controls_are_recovered_with_predeclared_window() {
        let resolutions = standard_resolutions();
        let families = [
            (GeometryFamily::LineSegment, 1.0, 0.08),
            (GeometryFamily::SierpinskiTriangle, 3.0_f64.log2(), 0.12),
            (GeometryFamily::UniformGrid, 2.0, 0.08),
            (GeometryFamily::UniformRandom, 2.0, 0.12),
        ];
        for (family, expected, tolerance) in families {
            let points = generate_geometry_fixture(family, 65_536, 0x484f_4c4f_4652_4143).unwrap();
            let report =
                estimate_box_counting_dimension(&points, &resolutions, 0, 5).unwrap();
            assert!(
                (report.dimension_estimate - expected).abs() < tolerance,
                "{family:?}: estimated {}, expected {expected}",
                report.dimension_estimate
            );
            assert!(report.fit_r_squared > 0.98, "{family:?}: R2 {}", report.fit_r_squared);
            assert_eq!(report.fit_scale_count, 6);
            assert!(report.mean_translation_count_spread.is_finite());
            assert!(report.window_dimension_spread.is_finite());
        }
    }

    #[test]
    fn scale_window_and_translation_diagnostics_are_reported() {
        let points =
            generate_geometry_fixture(GeometryFamily::SierpinskiTriangle, 16_384, 17).unwrap();
        let report = estimate_box_counting_dimension(
            &points,
            &standard_resolutions(),
            0,
            5,
        )
        .unwrap();
        assert_eq!(report.fit_start_resolution, 4);
        assert_eq!(report.fit_end_resolution, 128);
        assert!(report.min_window_dimension <= report.max_window_dimension);
        assert!(report.log_count_rmse.is_finite());
    }

    #[test]
    fn invalid_coordinates_and_scale_schedules_fail_closed() {
        assert_eq!(
            estimate_box_counting_dimension(&[], &[4, 8, 16], 0, 2).unwrap_err(),
            FractalError::EmptyPointSet
        );
        assert_eq!(
            estimate_box_counting_dimension(
                &[Point2 { x: 1.2, y: 0.5 }],
                &[4, 8, 16],
                0,
                2,
            )
            .unwrap_err(),
            FractalError::InvalidPoint
        );
        let point = [Point2 { x: 0.5, y: 0.5 }];
        assert_eq!(
            estimate_box_counting_dimension(&point, &[4, 4, 16], 0, 2).unwrap_err(),
            FractalError::InvalidResolutionSchedule
        );
        assert_eq!(
            estimate_box_counting_dimension(&point, &[4, 8, 16], 0, 1).unwrap_err(),
            FractalError::TooFewFitScales
        );
    }
}
