//! Continuous spatial-field VSA experiment.
//!
//! This is a research reconstruction path, not an authoritative physics field.
//! The exact sampled field and classical interpolation baselines remain the
//! correctness references. HDC is used only for associative recall.

use std::collections::HashSet;
use std::time::Instant;

use symtropy_hdc_core::{
    BipolarBundle, BipolarHV, EncoderSpec, HdcError, ItemMemory, LevelEncoder,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldFamily {
    Smooth,
    HighFrequency,
    Discontinuous,
    Anisotropic,
    Localized,
}

impl FieldFamily {
    pub const ALL: [Self; 5] = [
        Self::Smooth,
        Self::HighFrequency,
        Self::Discontinuous,
        Self::Anisotropic,
        Self::Localized,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Smooth => "smooth",
            Self::HighFrequency => "high_frequency",
            Self::Discontinuous => "discontinuous",
            Self::Anisotropic => "anisotropic",
            Self::Localized => "localized",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldSample {
    pub x: f64,
    pub y: f64,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldRange {
    pub min: f64,
    pub max: f64,
}

impl FieldRange {
    pub fn validate(self) -> Result<(), FieldError> {
        if !self.min.is_finite() || !self.max.is_finite() || self.max <= self.min {
            return Err(FieldError::InvalidRange);
        }
        Ok(())
    }

    fn contains(self, value: f64) -> bool {
        value.is_finite() && value >= self.min && value <= self.max
    }
}

#[derive(Debug)]
pub enum FieldError {
    EmptySamples,
    InvalidRange,
    SampleOutsideRange,
    DuplicateEncodedPosition,
    InvalidGrid,
    Hdc(HdcError),
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySamples => f.write_str("continuous field requires at least one sample"),
            Self::InvalidRange => {
                f.write_str("field coordinate/value range must be finite with max > min")
            }
            Self::SampleOutsideRange => {
                f.write_str("field sample or query lies outside declared coordinate/value ranges")
            }
            Self::DuplicateEncodedPosition => {
                f.write_str("two field samples quantize to the same encoded spatial position")
            }
            Self::InvalidGrid => {
                f.write_str("regular field grid requires at least two samples per axis")
            }
            Self::Hdc(error) => write!(f, "continuous spatial VSA error: {error}"),
        }
    }
}

impl std::error::Error for FieldError {}

impl From<HdcError> for FieldError {
    fn from(value: HdcError) -> Self {
        Self::Hdc(value)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FieldQueryResult {
    pub value: f64,
    pub codebook_level: usize,
    pub best_similarity: f32,
    pub coordinate_encode_micros: f64,
    pub positional_unbind_micros: f64,
    pub cleanup_micros: f64,
}

#[derive(Clone, Debug)]
pub struct SpatialFieldMemory {
    spec: EncoderSpec,
    x_encoder: LevelEncoder,
    y_encoder: LevelEncoder,
    x_role: BipolarHV,
    y_role: BipolarHV,
    memory: BipolarHV,
    value_codebook: Vec<BipolarHV>,
    x_range: FieldRange,
    y_range: FieldRange,
    value_range: FieldRange,
    sample_count: usize,
    build_micros: f64,
}

impl SpatialFieldMemory {
    pub fn build(
        samples: &[FieldSample],
        spec: EncoderSpec,
        x_range: FieldRange,
        y_range: FieldRange,
        value_range: FieldRange,
    ) -> Result<Self, FieldError> {
        if samples.is_empty() {
            return Err(FieldError::EmptySamples);
        }
        x_range.validate()?;
        y_range.validate()?;
        value_range.validate()?;

        let started = Instant::now();
        let item_memory = ItemMemory::new(spec.clone())?;
        let x_encoder = item_memory.scalar_encoder("continuous-field-x");
        let y_encoder = item_memory.scalar_encoder("continuous-field-y");
        let value_encoder = item_memory.scalar_encoder("continuous-field-value");
        let x_role = item_memory.role("continuous-field-x-role");
        let y_role = item_memory.role("continuous-field-y-role");

        let mut bundle = BipolarBundle::new(spec.dimension);
        let mut encoded_positions = HashSet::with_capacity(samples.len());
        for sample in samples {
            if !x_range.contains(sample.x)
                || !y_range.contains(sample.y)
                || !value_range.contains(sample.value)
            {
                return Err(FieldError::SampleOutsideRange);
            }
            let position_key = (
                x_encoder.quantize(sample.x, x_range.min, x_range.max)?,
                y_encoder.quantize(sample.y, y_range.min, y_range.max)?,
            );
            if !encoded_positions.insert(position_key) {
                return Err(FieldError::DuplicateEncodedPosition);
            }
            let position = encode_position(
                &x_encoder, &y_encoder, &x_role, &y_role, x_range, y_range, sample.x, sample.y,
            )?;
            let value = value_encoder.encode(sample.value, value_range.min, value_range.max)?;
            bundle.add(&position.bind(&value)?)?;
        }

        let memory = bundle.finish(&item_memory.tie_breaker("continuous-field-memory"))?;
        let value_codebook = (0..spec.scalar_levels)
            .map(|level| value_encoder.encode_level(level))
            .collect();
        let build_micros = started.elapsed().as_secs_f64() * 1_000_000.0;

        Ok(Self {
            spec,
            x_encoder,
            y_encoder,
            x_role,
            y_role,
            memory,
            value_codebook,
            x_range,
            y_range,
            value_range,
            sample_count: samples.len(),
            build_micros,
        })
    }

    pub fn dimension(&self) -> usize {
        self.spec.dimension
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    pub fn build_micros(&self) -> f64 {
        self.build_micros
    }

    /// Reconstruct a value at an arbitrary in-range coordinate.
    ///
    /// Bipolar unbinding is involutive under the core's element-wise bind.
    /// Cleanup selects the nearest scalar level by cosine similarity.
    pub fn query(&self, x: f64, y: f64) -> Result<FieldQueryResult, FieldError> {
        if !self.x_range.contains(x) || !self.y_range.contains(y) {
            return Err(FieldError::SampleOutsideRange);
        }

        let encode_started = Instant::now();
        let position = encode_position(
            &self.x_encoder,
            &self.y_encoder,
            &self.x_role,
            &self.y_role,
            self.x_range,
            self.y_range,
            x,
            y,
        )?;
        let coordinate_encode_micros = encode_started.elapsed().as_secs_f64() * 1_000_000.0;

        let unbind_started = Instant::now();
        let noisy_value = self.memory.bind(&position)?;
        let positional_unbind_micros = unbind_started.elapsed().as_secs_f64() * 1_000_000.0;

        let cleanup_started = Instant::now();
        let mut best_level = 0_usize;
        let mut best_similarity = f32::NEG_INFINITY;
        for (level, candidate) in self.value_codebook.iter().enumerate() {
            let similarity = noisy_value.similarity(candidate)?;
            if similarity > best_similarity {
                best_similarity = similarity;
                best_level = level;
            }
        }
        let cleanup_micros = cleanup_started.elapsed().as_secs_f64() * 1_000_000.0;
        let value = self.value_range.min
            + best_level as f64 / (self.spec.scalar_levels - 1) as f64
                * (self.value_range.max - self.value_range.min);

        Ok(FieldQueryResult {
            value,
            codebook_level: best_level,
            best_similarity,
            coordinate_encode_micros,
            positional_unbind_micros,
            cleanup_micros,
        })
    }

    /// Payload estimate for the retained bipolar memory, scalar cleanup
    /// codebook, coordinate encoder base vectors and coordinate role vectors.
    /// Excludes allocator,
    /// flip-order index arrays, graph objects and sample source storage.
    pub fn retained_vector_payload_bytes_estimate(&self) -> usize {
        let dimension_bytes = self.spec.dimension * std::mem::size_of::<i8>();
        dimension_bytes
            + self.value_codebook.len() * dimension_bytes
            + 2 * dimension_bytes
            + 2 * dimension_bytes
    }

    /// Auxiliary scalar-encoder index array bytes, excluding Vec headers.
    pub fn encoder_index_bytes_estimate(&self) -> usize {
        2 * self.spec.dimension * std::mem::size_of::<usize>()
    }
}

fn encode_position(
    x_encoder: &LevelEncoder,
    y_encoder: &LevelEncoder,
    x_role: &BipolarHV,
    y_role: &BipolarHV,
    x_range: FieldRange,
    y_range: FieldRange,
    x: f64,
    y: f64,
) -> Result<BipolarHV, HdcError> {
    let x_value = x_encoder
        .encode(x, x_range.min, x_range.max)?
        .bind(x_role)?;
    let y_value = y_encoder
        .encode(y, y_range.min, y_range.max)?
        .bind(y_role)?;
    Ok(x_value.bind(&y_value)?)
}

#[derive(Clone, Debug)]
pub struct FieldBenchmarkReport {
    pub field_family: FieldFamily,
    pub dimension: usize,
    pub grid_size: usize,
    pub sample_count: usize,
    pub query_count: usize,
    pub hdc_mae: f64,
    pub hdc_rmse: f64,
    pub hdc_r2: f64,
    pub nearest_mae: f64,
    pub nearest_rmse: f64,
    pub nearest_r2: f64,
    pub bilinear_mae: f64,
    pub bilinear_rmse: f64,
    pub bilinear_r2: f64,
    pub mean_coordinate_encode_micros: f64,
    pub mean_positional_unbind_micros: f64,
    pub mean_cleanup_micros: f64,
    pub query_p50_micros: f64,
    pub query_p95_micros: f64,
    pub build_micros: f64,
    pub retained_vector_payload_bytes_estimate: usize,
    pub encoder_index_bytes_estimate: usize,
    pub hdc_payload_plus_encoder_index_bytes_estimate: usize,
    /// Scalar values only for exact regular-grid interpolation.
    pub reference_grid_value_payload_bytes_estimate: usize,
    /// Full (x, y, value) source records for a point-cloud baseline.
    pub reference_sample_record_payload_bytes_estimate: usize,
    /// HDC total payload divided by the scalar-grid value payload.
    pub hdc_to_grid_value_payload_ratio: f64,
}

pub fn run_regular_grid_field_benchmark(
    grid_size: usize,
    dimension: usize,
    seed: u64,
) -> Result<FieldBenchmarkReport, FieldError> {
    run_field_family_benchmark(grid_size, dimension, seed, FieldFamily::Smooth)
}

pub fn run_field_family_benchmark(
    grid_size: usize,
    dimension: usize,
    seed: u64,
    field_family: FieldFamily,
) -> Result<FieldBenchmarkReport, FieldError> {
    if grid_size < 2 {
        return Err(FieldError::InvalidGrid);
    }

    let sample_capacity = grid_size
        .checked_mul(grid_size)
        .ok_or(FieldError::InvalidGrid)?;
    let query_capacity = (grid_size - 1)
        .checked_mul(grid_size - 1)
        .ok_or(FieldError::InvalidGrid)?;
    let mut samples = Vec::with_capacity(sample_capacity);
    for y_index in 0..grid_size {
        for x_index in 0..grid_size {
            let x = x_index as f64 / (grid_size - 1) as f64;
            let y = y_index as f64 / (grid_size - 1) as f64;
            samples.push(FieldSample {
                x,
                y,
                value: field_value(field_family, x, y),
            });
        }
    }

    let memory = SpatialFieldMemory::build(
        &samples,
        EncoderSpec {
            schema_version: 1,
            dimension,
            seed,
            scalar_levels: 257,
        },
        FieldRange { min: 0.0, max: 1.0 },
        FieldRange { min: 0.0, max: 1.0 },
        FieldRange { min: 0.0, max: 1.0 },
    )?;

    let mut truth_values = Vec::with_capacity(query_capacity);
    let mut hdc_errors = Vec::with_capacity(query_capacity);
    let mut nearest_errors = Vec::with_capacity(query_capacity);
    let mut bilinear_errors = Vec::with_capacity(query_capacity);
    let mut encode_times = Vec::with_capacity(query_capacity);
    let mut unbind_times = Vec::with_capacity(query_capacity);
    let mut cleanup_times = Vec::with_capacity(query_capacity);
    let mut total_times = Vec::with_capacity(query_capacity);

    for y_index in 0..(grid_size - 1) {
        for x_index in 0..(grid_size - 1) {
            let x = (x_index as f64 + 0.5) / (grid_size - 1) as f64;
            let y = (y_index as f64 + 0.5) / (grid_size - 1) as f64;
            let truth = field_value(field_family, x, y);
            truth_values.push(truth);
            let started = Instant::now();
            let result = memory.query(x, y)?;
            total_times.push(started.elapsed().as_secs_f64() * 1_000_000.0);
            hdc_errors.push(result.value - truth);
            encode_times.push(result.coordinate_encode_micros);
            unbind_times.push(result.positional_unbind_micros);
            cleanup_times.push(result.cleanup_micros);

            let nearest = nearest_sample_value(&samples, x, y);
            nearest_errors.push(nearest - truth);

            let bilinear = bilinear_grid_value(&samples, grid_size, x, y);
            bilinear_errors.push(bilinear - truth);
        }
    }

    let query_p50_micros = percentile(&mut total_times.clone(), 0.50);
    let query_p95_micros = percentile(&mut total_times, 0.95);
    let retained_vector_payload_bytes_estimate =
        memory.retained_vector_payload_bytes_estimate();
    let encoder_index_bytes_estimate = memory.encoder_index_bytes_estimate();
    let hdc_payload_plus_encoder_index_bytes_estimate =
        retained_vector_payload_bytes_estimate + encoder_index_bytes_estimate;
    let reference_grid_value_payload_bytes_estimate =
        samples.len() * std::mem::size_of::<f64>();
    let reference_sample_record_payload_bytes_estimate =
        samples.len() * std::mem::size_of::<FieldSample>();

    Ok(FieldBenchmarkReport {
        field_family,
        dimension,
        grid_size,
        sample_count: samples.len(),
        query_count: hdc_errors.len(),
        hdc_mae: mean_abs(&hdc_errors),
        hdc_rmse: rmse(&hdc_errors),
        hdc_r2: r2_from_residuals(&hdc_errors, &truth_values),
        nearest_mae: mean_abs(&nearest_errors),
        nearest_rmse: rmse(&nearest_errors),
        nearest_r2: r2_from_residuals(&nearest_errors, &truth_values),
        bilinear_mae: mean_abs(&bilinear_errors),
        bilinear_rmse: rmse(&bilinear_errors),
        bilinear_r2: r2_from_residuals(&bilinear_errors, &truth_values),
        mean_coordinate_encode_micros: mean(&encode_times),
        mean_positional_unbind_micros: mean(&unbind_times),
        mean_cleanup_micros: mean(&cleanup_times),
        query_p50_micros,
        query_p95_micros,
        build_micros: memory.build_micros(),
        retained_vector_payload_bytes_estimate,
        encoder_index_bytes_estimate,
        hdc_payload_plus_encoder_index_bytes_estimate,
        reference_grid_value_payload_bytes_estimate,
        reference_sample_record_payload_bytes_estimate,
        hdc_to_grid_value_payload_ratio: hdc_payload_plus_encoder_index_bytes_estimate as f64
            / reference_grid_value_payload_bytes_estimate as f64,
    })
}

pub fn analytic_field(x: f64, y: f64) -> f64 {
    field_value(FieldFamily::Smooth, x, y)
}

pub fn field_value(family: FieldFamily, x: f64, y: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    match family {
        FieldFamily::Smooth => 0.5 + 0.2 * (tau * x).sin() + 0.2 * (tau * y).cos(),
        FieldFamily::HighFrequency => 0.5 + 0.2 * (8.0 * tau * x).sin() * (6.0 * tau * y).cos(),
        FieldFamily::Discontinuous => {
            if x < 0.5 {
                0.2
            } else {
                0.8
            }
        }
        FieldFamily::Anisotropic => 0.5 + 0.25 * (tau * x).sin() + 0.1 * (12.0 * tau * y).cos(),
        FieldFamily::Localized => {
            let radius_squared = (x - 0.5).powi(2) + (y - 0.5).powi(2);
            0.2 + 0.6 * (-radius_squared / 0.015).exp()
        }
    }
}

fn r2_from_residuals(residuals: &[f64], truth: &[f64]) -> f64 {
    if residuals.is_empty() || residuals.len() != truth.len() {
        return f64::NAN;
    }
    let mean_truth = mean(truth);
    let total_sum_squares = truth
        .iter()
        .map(|value| (value - mean_truth).powi(2))
        .sum::<f64>();
    let residual_sum_squares = residuals.iter().map(|value| value * value).sum::<f64>();
    if total_sum_squares <= f64::EPSILON {
        return if residual_sum_squares <= f64::EPSILON {
            1.0
        } else {
            0.0
        };
    }
    1.0 - residual_sum_squares / total_sum_squares
}

fn nearest_sample_value(samples: &[FieldSample], x: f64, y: f64) -> f64 {
    samples
        .iter()
        .min_by(|left, right| {
            let left_distance = (left.x - x).powi(2) + (left.y - y).powi(2);
            let right_distance = (right.x - x).powi(2) + (right.y - y).powi(2);
            left_distance.total_cmp(&right_distance)
        })
        .map_or(0.0, |sample| sample.value)
}

fn bilinear_grid_value(samples: &[FieldSample], grid_size: usize, x: f64, y: f64) -> f64 {
    let scaled_x = x * (grid_size - 1) as f64;
    let scaled_y = y * (grid_size - 1) as f64;
    let x0 = (scaled_x.floor() as usize).min(grid_size - 2);
    let y0 = (scaled_y.floor() as usize).min(grid_size - 2);
    let tx = scaled_x - x0 as f64;
    let ty = scaled_y - y0 as f64;

    let v00 = samples[y0 * grid_size + x0].value;
    let v10 = samples[y0 * grid_size + x0 + 1].value;
    let v01 = samples[(y0 + 1) * grid_size + x0].value;
    let v11 = samples[(y0 + 1) * grid_size + x0 + 1].value;
    let lower = v00 * (1.0 - tx) + v10 * tx;
    let upper = v01 * (1.0 - tx) + v11 * tx;
    lower * (1.0 - ty) + upper * ty
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn mean_abs(values: &[f64]) -> f64 {
    mean(&values.iter().map(|value| value.abs()).collect::<Vec<_>>())
}

fn rmse(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        (values.iter().map(|value| value * value).sum::<f64>() / values.len() as f64).sqrt()
    }
}

fn percentile(values: &mut [f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let index = ((values.len() - 1) as f64 * quantile).round() as usize;
    values[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(dimension: usize, seed: u64) -> EncoderSpec {
        EncoderSpec {
            schema_version: 1,
            dimension,
            seed,
            scalar_levels: 257,
        }
    }

    #[test]
    fn empty_field_and_out_of_range_values_fail_closed() {
        assert!(matches!(
            SpatialFieldMemory::build(
                &[],
                spec(2_048, 1),
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
            ),
            Err(FieldError::EmptySamples)
        ));
        let sample = [FieldSample {
            x: 1.1,
            y: 0.5,
            value: 0.5,
        }];
        assert!(matches!(
            SpatialFieldMemory::build(
                &sample,
                spec(2_048, 1),
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
            ),
            Err(FieldError::SampleOutsideRange)
        ));
    }

    #[test]
    fn coordinate_quantization_collisions_are_rejected() {
        let samples = [
            FieldSample {
                x: 0.2000,
                y: 0.5,
                value: 0.3,
            },
            FieldSample {
                x: 0.2001,
                y: 0.5,
                value: 0.8,
            },
        ];
        assert!(matches!(
            SpatialFieldMemory::build(
                &samples,
                spec(2_048, 5),
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
            ),
            Err(FieldError::DuplicateEncodedPosition)
        ));
    }

    #[test]
    fn coordinate_and_scalar_encoding_are_deterministic() {
        let samples = [
            FieldSample {
                x: 0.0,
                y: 0.0,
                value: analytic_field(0.0, 0.0),
            },
            FieldSample {
                x: 1.0,
                y: 1.0,
                value: analytic_field(1.0, 1.0),
            },
        ];
        let build = || {
            SpatialFieldMemory::build(
                &samples,
                spec(4_096, 77),
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
            )
            .unwrap()
        };
        let a = build();
        let b = build();
        let qa = a.query(0.0, 0.0).unwrap();
        let qb = b.query(0.0, 0.0).unwrap();
        assert_eq!(qa.codebook_level, qb.codebook_level);
        assert_eq!(qa.value, qb.value);
        assert!(qa.value.is_finite());
    }

    #[test]
    fn one_sample_is_recalled_at_its_exact_position() {
        let sample = [FieldSample {
            x: 0.25,
            y: 0.75,
            value: 0.37,
        }];
        let memory = SpatialFieldMemory::build(
            &sample,
            spec(4_096, 83),
            FieldRange { min: 0.0, max: 1.0 },
            FieldRange { min: 0.0, max: 1.0 },
            FieldRange { min: 0.0, max: 1.0 },
        )
        .unwrap();

        let result = memory.query(0.25, 0.75).unwrap();
        assert!(result.best_similarity > 0.999);
        assert!((result.value - sample[0].value).abs() <= 0.5 / 256.0);
    }

    #[test]
    fn queries_outside_declared_spatial_domain_fail_closed() {
        let sample = [FieldSample {
            x: 0.5,
            y: 0.5,
            value: 0.5,
        }];
        let memory = SpatialFieldMemory::build(
            &sample,
            spec(2_048, 89),
            FieldRange { min: 0.0, max: 1.0 },
            FieldRange { min: 0.0, max: 1.0 },
            FieldRange { min: 0.0, max: 1.0 },
        )
        .unwrap();
        assert!(matches!(
            memory.query(1.01, 0.5),
            Err(FieldError::SampleOutsideRange)
        ));
    }

    #[test]
    fn field_families_are_bounded_and_report_r2() {
        for family in FieldFamily::ALL {
            for index in 0..=100 {
                let x = index as f64 / 100.0;
                let y = (100 - index) as f64 / 100.0;
                let value = field_value(family, x, y);
                assert!((0.0..=1.0).contains(&value), "{family:?}: {value}");
            }

            let report = run_field_family_benchmark(4, 2_048, 97, family).unwrap();
            assert_eq!(report.field_family, family);
            assert_eq!(report.query_count, 9);
            assert!(report.hdc_r2.is_finite());
            assert!(report.nearest_r2.is_finite());
            assert!(report.bilinear_r2.is_finite());
            assert!(report.hdc_payload_plus_encoder_index_bytes_estimate
                >= report.retained_vector_payload_bytes_estimate);
            assert!(report.reference_grid_value_payload_bytes_estimate > 0);
            assert!(report.hdc_to_grid_value_payload_ratio.is_finite());
        }
    }

    #[test]
    fn r2_is_one_for_exact_predictions_and_zero_for_mean_baseline() {
        assert!((r2_from_residuals(&[0.0, 0.0, 0.0], &[0.0, 1.0, 2.0]) - 1.0).abs() < f64::EPSILON);
        assert!(r2_from_residuals(&[-1.0, 0.0, 1.0], &[0.0, 1.0, 2.0]) < 1.0);
    }

    #[test]
    fn regular_grid_bilinear_baseline_is_measured_separately() {
        let report = run_regular_grid_field_benchmark(8, 2_048, 91).unwrap();
        assert_eq!(report.sample_count, 64);
        assert_eq!(report.query_count, 49);
        assert!(report.hdc_rmse.is_finite());
        assert!(report.bilinear_rmse.is_finite());
        assert!(report.nearest_rmse.is_finite());
        assert!(report.retained_vector_payload_bytes_estimate > 0);
    }
}
