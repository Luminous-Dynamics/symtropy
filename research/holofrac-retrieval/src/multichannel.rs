//! Shared multi-channel continuous spatial-field experiment.
//!
//! A channel identity is bound alongside spatial position and scalar value.
//! This tests whether one shared positional memory can amortize codebook and
//! coordinate-encoder storage across several fields. It remains an estimate,
//! never an authoritative physical field.

use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use symtropy_hdc_core::{BipolarBundle, BipolarHV, EncoderSpec, HdcError, ItemMemory, LevelEncoder};

use crate::field::{FieldFamily, FieldRange, SpatialFieldMemory, field_value};

#[derive(Clone, Debug, PartialEq)]
pub struct MultiChannelFieldSample {
    pub channel: String,
    pub x: f64,
    pub y: f64,
    pub value: f64,
}

#[derive(Debug)]
pub enum MultiChannelError {
    EmptySamples,
    EmptyChannel,
    InvalidRange,
    SampleOutsideRange,
    DuplicateEncodedPosition,
    UnknownChannel(String),
    InvalidGrid,
    Hdc(HdcError),
}

impl std::fmt::Display for MultiChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySamples => f.write_str("multi-channel field requires at least one sample"),
            Self::EmptyChannel => f.write_str("field channel identifier must not be empty or contain control characters"),
            Self::InvalidRange => f.write_str("field coordinate/value range must be finite with max > min"),
            Self::SampleOutsideRange => f.write_str("field sample or query lies outside declared coordinate/value ranges"),
            Self::DuplicateEncodedPosition => f.write_str("two samples for one channel quantize to the same encoded spatial position"),
            Self::UnknownChannel(channel) => write!(f, "unknown field channel: {channel}"),
            Self::InvalidGrid => f.write_str("multi-channel field grid requires at least two samples per axis"),
            Self::Hdc(error) => write!(f, "multi-channel spatial VSA error: {error}"),
        }
    }
}

impl std::error::Error for MultiChannelError {}

impl From<HdcError> for MultiChannelError {
    fn from(value: HdcError) -> Self {
        Self::Hdc(value)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MultiChannelQueryResult {
    pub value: f64,
    pub codebook_level: usize,
    pub best_similarity: f32,
    pub coordinate_channel_encode_micros: f64,
    pub unbind_micros: f64,
    pub cleanup_micros: f64,
}

#[derive(Clone, Debug)]
pub struct MultiChannelFieldMemory {
    spec: EncoderSpec,
    x_encoder: LevelEncoder,
    y_encoder: LevelEncoder,
    x_role: BipolarHV,
    y_role: BipolarHV,
    channel_roles: BTreeMap<String, BipolarHV>,
    memory: BipolarHV,
    value_codebook: Vec<BipolarHV>,
    x_range: FieldRange,
    y_range: FieldRange,
    value_range: FieldRange,
    sample_count: usize,
    build_micros: f64,
}

impl MultiChannelFieldMemory {
    pub fn build(
        samples: &[MultiChannelFieldSample],
        spec: EncoderSpec,
        x_range: FieldRange,
        y_range: FieldRange,
        value_range: FieldRange,
    ) -> Result<Self, MultiChannelError> {
        if samples.is_empty() {
            return Err(MultiChannelError::EmptySamples);
        }
        validate_range(x_range)?;
        validate_range(y_range)?;
        validate_range(value_range)?;

        let started = Instant::now();
        let item_memory = ItemMemory::new(spec.clone())?;
        let x_encoder = item_memory.scalar_encoder("continuous-field-x");
        let y_encoder = item_memory.scalar_encoder("continuous-field-y");
        let value_encoder = item_memory.scalar_encoder("continuous-field-value");
        let x_role = item_memory.role("continuous-field-x-role");
        let y_role = item_memory.role("continuous-field-y-role");

        let mut channel_roles = BTreeMap::new();
        for sample in samples {
            validate_channel(&sample.channel)?;
            channel_roles.entry(sample.channel.clone()).or_insert_with(|| {
                item_memory.bind_role_value(
                    "continuous-field-channel",
                    "channel",
                    &sample.channel,
                )
            });
        }

        let mut bundle = BipolarBundle::new(spec.dimension);
        let mut encoded_positions = HashSet::with_capacity(samples.len());
        for sample in samples {
            if !in_range(x_range, sample.x)
                || !in_range(y_range, sample.y)
                || !in_range(value_range, sample.value)
            {
                return Err(MultiChannelError::SampleOutsideRange);
            }

            let position_key = (
                sample.channel.clone(),
                x_encoder.quantize(sample.x, x_range.min, x_range.max)?,
                y_encoder.quantize(sample.y, y_range.min, y_range.max)?,
            );
            if !encoded_positions.insert(position_key) {
                return Err(MultiChannelError::DuplicateEncodedPosition);
            }

            let position = encode_position(
                &x_encoder,
                &y_encoder,
                &x_role,
                &y_role,
                x_range,
                y_range,
                sample.x,
                sample.y,
            )?;
            let channel_role = channel_roles
                .get(&sample.channel)
                .ok_or_else(|| MultiChannelError::UnknownChannel(sample.channel.clone()))?;
            let position_channel = position.bind(channel_role)?;
            let value = value_encoder.encode(sample.value, value_range.min, value_range.max)?;
            bundle.add(&position_channel.bind(&value)?)?;
        }

        let memory = bundle.finish(&item_memory.tie_breaker("multi-channel-field-memory"))?;
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
            channel_roles,
            memory,
            value_codebook,
            x_range,
            y_range,
            value_range,
            sample_count: samples.len(),
            build_micros,
        })
    }

    pub fn channel_count(&self) -> usize {
        self.channel_roles.len()
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    pub fn build_micros(&self) -> f64 {
        self.build_micros
    }

    pub fn query(
        &self,
        channel: &str,
        x: f64,
        y: f64,
    ) -> Result<MultiChannelQueryResult, MultiChannelError> {
        if !in_range(self.x_range, x) || !in_range(self.y_range, y) {
            return Err(MultiChannelError::SampleOutsideRange);
        }
        let channel_role = self
            .channel_roles
            .get(channel)
            .ok_or_else(|| MultiChannelError::UnknownChannel(channel.to_owned()))?;

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
        let position_channel = position.bind(channel_role)?;
        let coordinate_channel_encode_micros =
            encode_started.elapsed().as_secs_f64() * 1_000_000.0;

        let unbind_started = Instant::now();
        let noisy_value = self.memory.bind(&position_channel)?;
        let unbind_micros = unbind_started.elapsed().as_secs_f64() * 1_000_000.0;

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

        Ok(MultiChannelQueryResult {
            value,
            codebook_level: best_level,
            best_similarity,
            coordinate_channel_encode_micros,
            unbind_micros,
            cleanup_micros,
        })
    }

    /// Vector payload includes one shared memory, one scalar codebook,
    /// coordinate encoder bases and roles, and one role vector per channel.
    pub fn retained_vector_payload_bytes_estimate(&self) -> usize {
        let dimension_bytes = self.spec.dimension * std::mem::size_of::<i8>();
        dimension_bytes
            + self.value_codebook.len() * dimension_bytes
            + 2 * dimension_bytes
            + 2 * dimension_bytes
            + self.channel_roles.len() * dimension_bytes
    }

    pub fn encoder_index_bytes_estimate(&self) -> usize {
        2 * self.spec.dimension * std::mem::size_of::<usize>()
    }

    pub fn channel_name_bytes_estimate(&self) -> usize {
        self.channel_roles.keys().map(String::len).sum()
    }
}

fn validate_channel(channel: &str) -> Result<(), MultiChannelError> {
    if channel.trim().is_empty() || channel.chars().any(char::is_control) {
        return Err(MultiChannelError::EmptyChannel);
    }
    Ok(())
}

fn validate_range(range: FieldRange) -> Result<(), MultiChannelError> {
    if !range.min.is_finite() || !range.max.is_finite() || range.max <= range.min {
        return Err(MultiChannelError::InvalidRange);
    }
    Ok(())
}

fn in_range(range: FieldRange, value: f64) -> bool {
    value.is_finite() && value >= range.min && value <= range.max
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
pub struct MultiChannelBenchmarkReport {
    pub channels: usize,
    pub grid_size: usize,
    pub dimension: usize,
    pub samples_per_channel: usize,
    pub held_out_queries: usize,
    pub shared_mae: f64,
    pub shared_rmse: f64,
    pub independent_mae: f64,
    pub independent_rmse: f64,
    pub bilinear_mae: f64,
    pub bilinear_rmse: f64,
    pub shared_build_micros: f64,
    pub independent_build_micros: f64,
    pub shared_query_p95_micros: f64,
    pub independent_query_p95_micros: f64,
    pub shared_vector_payload_bytes: usize,
    pub shared_encoder_index_bytes: usize,
    pub shared_channel_name_bytes: usize,
    pub independent_vector_payload_bytes: usize,
    pub independent_encoder_index_bytes: usize,
    pub scalar_grid_value_bytes: usize,
    pub shared_total_payload_bytes: usize,
    pub independent_total_payload_bytes: usize,
    pub shared_to_grid_payload_ratio: f64,
    pub shared_to_independent_payload_ratio: f64,
}

pub fn run_multi_channel_benchmark(
    grid_size: usize,
    dimension: usize,
    channel_count: usize,
    seed: u64,
) -> Result<MultiChannelBenchmarkReport, MultiChannelError> {
    if grid_size < 2 || channel_count == 0 || channel_count > CHANNELS.len() {
        return Err(MultiChannelError::InvalidGrid);
    }
    let sample_capacity = grid_size
        .checked_mul(grid_size)
        .and_then(|value| value.checked_mul(channel_count))
        .ok_or(MultiChannelError::InvalidGrid)?;
    let mut all_samples = Vec::with_capacity(sample_capacity);
    let mut independent_samples = Vec::with_capacity(channel_count);
    let channel_names = &CHANNELS[..channel_count];

    for (channel_index, channel) in channel_names.iter().enumerate() {
        let mut samples = Vec::with_capacity(grid_size * grid_size);
        let amplitude = 1.0 - channel_index as f64 * 0.08;
        for y_index in 0..grid_size {
            for x_index in 0..grid_size {
                let x = x_index as f64 / (grid_size - 1) as f64;
                let y = y_index as f64 / (grid_size - 1) as f64;
                let base = field_value(FieldFamily::Smooth, x, y);
                let value = (0.5 + amplitude * (base - 0.5)).clamp(0.0, 1.0);
                let sample = MultiChannelFieldSample {
                    channel: (*channel).to_owned(),
                    x,
                    y,
                    value,
                };
                all_samples.push(sample.clone());
                samples.push(crate::field::FieldSample { x, y, value });
            }
        }
        independent_samples.push(samples);
    }

    let range = FieldRange { min: 0.0, max: 1.0 };
    let spec = EncoderSpec {
        schema_version: 1,
        dimension,
        seed,
        scalar_levels: 257,
    };

    let shared_memory = MultiChannelFieldMemory::build(
        &all_samples,
        spec.clone(),
        range,
        range,
        range,
    )?;

    let independent_started = Instant::now();
    let mut independent_memories = Vec::with_capacity(channel_count);
    for samples in &independent_samples {
        independent_memories.push(SpatialFieldMemory::build(
            samples,
            spec.clone(),
            range,
            range,
            range,
        )?);
    }
    let independent_build_micros =
        independent_started.elapsed().as_secs_f64() * 1_000_000.0;

    let mut shared_errors = Vec::with_capacity(sample_capacity);
    let mut independent_errors = Vec::with_capacity(sample_capacity);
    let mut bilinear_errors = Vec::with_capacity(sample_capacity);
    let mut shared_query_times = Vec::with_capacity(sample_capacity);
    let mut independent_query_times = Vec::with_capacity(sample_capacity);

    for (channel_index, channel) in channel_names.iter().enumerate() {
        for y_index in 0..(grid_size - 1) {
            for x_index in 0..(grid_size - 1) {
                let x = (x_index as f64 + 0.5) / (grid_size - 1) as f64;
                let y = (y_index as f64 + 0.5) / (grid_size - 1) as f64;
                let base = field_value(FieldFamily::Smooth, x, y);
                let truth = (0.5 + (1.0 - channel_index as f64 * 0.08) * (base - 0.5))
                    .clamp(0.0, 1.0);

                let shared_started = Instant::now();
                let shared_result = shared_memory.query(channel, x, y)?;
                shared_query_times.push(
                    shared_started.elapsed().as_secs_f64() * 1_000_000.0,
                );
                shared_errors.push(shared_result.value - truth);

                let independent_started = Instant::now();
                let independent_result =
                    independent_memories[channel_index].query(x, y)?;
                independent_query_times.push(
                    independent_started.elapsed().as_secs_f64() * 1_000_000.0,
                );
                independent_errors.push(independent_result.value - truth);

                let bilinear = bilinear_channel_value(
                    &independent_samples[channel_index],
                    grid_size,
                    x,
                    y,
                );
                bilinear_errors.push(bilinear - truth);
            }
        }
    }

    let samples_per_channel = grid_size * grid_size;
    let held_out_queries = (grid_size - 1) * (grid_size - 1);
    let shared_vector_payload_bytes = shared_memory.retained_vector_payload_bytes_estimate();
    let shared_encoder_index_bytes = shared_memory.encoder_index_bytes_estimate();
    let shared_channel_name_bytes = shared_memory.channel_name_bytes_estimate();
    let independent_vector_payload_bytes = independent_memories
        .iter()
        .map(SpatialFieldMemory::retained_vector_payload_bytes_estimate)
        .sum();
    let independent_encoder_index_bytes = independent_memories
        .iter()
        .map(SpatialFieldMemory::encoder_index_bytes_estimate)
        .sum();
    let scalar_grid_value_bytes = sample_capacity * std::mem::size_of::<f64>();
    let shared_total_payload_bytes = shared_vector_payload_bytes
        + shared_encoder_index_bytes
        + shared_channel_name_bytes;
    let independent_total_payload_bytes =
        independent_vector_payload_bytes + independent_encoder_index_bytes;

    Ok(MultiChannelBenchmarkReport {
        channels: channel_count,
        grid_size,
        dimension,
        samples_per_channel,
        held_out_queries: held_out_queries * channel_count,
        shared_mae: mean_abs(&shared_errors),
        shared_rmse: rmse(&shared_errors),
        independent_mae: mean_abs(&independent_errors),
        independent_rmse: rmse(&independent_errors),
        bilinear_mae: mean_abs(&bilinear_errors),
        bilinear_rmse: rmse(&bilinear_errors),
        shared_build_micros: shared_memory.build_micros(),
        independent_build_micros,
        shared_query_p95_micros: percentile(&mut shared_query_times, 0.95),
        independent_query_p95_micros: percentile(&mut independent_query_times, 0.95),
        shared_vector_payload_bytes,
        shared_encoder_index_bytes,
        shared_channel_name_bytes,
        independent_vector_payload_bytes,
        independent_encoder_index_bytes,
        scalar_grid_value_bytes,
        shared_total_payload_bytes,
        independent_total_payload_bytes,
        shared_to_grid_payload_ratio: shared_total_payload_bytes as f64
            / scalar_grid_value_bytes as f64,
        shared_to_independent_payload_ratio: shared_total_payload_bytes as f64
            / independent_total_payload_bytes as f64,
    })
}

const CHANNELS: [&str; 6] = [
    "temperature",
    "illuminance",
    "humidity",
    "moisture",
    "airflow",
    "occupancy_density",
];

fn bilinear_channel_value(
    samples: &[crate::field::FieldSample],
    grid_size: usize,
    x: f64,
    y: f64,
) -> f64 {
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

fn mean_abs(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().map(|value| value.abs()).sum::<f64>() / values.len() as f64
    }
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
    fn channel_keying_separates_values_at_the_same_spatial_position() {
        let samples = [
            MultiChannelFieldSample {
                channel: "temperature".to_owned(),
                x: 0.25,
                y: 0.75,
                value: 0.25,
            },
            MultiChannelFieldSample {
                channel: "humidity".to_owned(),
                x: 0.25,
                y: 0.75,
                value: 0.75,
            },
        ];
        let memory = MultiChannelFieldMemory::build(
            &samples,
            spec(8_192, 101),
            FieldRange { min: 0.0, max: 1.0 },
            FieldRange { min: 0.0, max: 1.0 },
            FieldRange { min: 0.0, max: 1.0 },
        )
        .unwrap();
        let temperature = memory.query("temperature", 0.25, 0.75).unwrap();
        let humidity = memory.query("humidity", 0.25, 0.75).unwrap();
        assert!((temperature.value - 0.25).abs() <= 0.5 / 256.0);
        assert!((humidity.value - 0.75).abs() <= 0.5 / 256.0);
    }

    #[test]
    fn unknown_and_empty_channels_fail_closed() {
        let sample = [MultiChannelFieldSample {
            channel: "temperature".to_owned(),
            x: 0.5,
            y: 0.5,
            value: 0.5,
        }];
        let memory = MultiChannelFieldMemory::build(
            &sample,
            spec(2_048, 103),
            FieldRange { min: 0.0, max: 1.0 },
            FieldRange { min: 0.0, max: 1.0 },
            FieldRange { min: 0.0, max: 1.0 },
        )
        .unwrap();
        assert!(matches!(
            memory.query("humidity", 0.5, 0.5),
            Err(MultiChannelError::UnknownChannel(_))
        ));
        let empty_name = [MultiChannelFieldSample {
            channel: "".to_owned(),
            x: 0.5,
            y: 0.5,
            value: 0.5,
        }];
        assert!(matches!(
            MultiChannelFieldMemory::build(
                &empty_name,
                spec(2_048, 103),
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
                FieldRange { min: 0.0, max: 1.0 },
            ),
            Err(MultiChannelError::EmptyChannel)
        ));
    }

    #[test]
    fn shared_multi_channel_storage_is_compared_to_independent_memories() {
        let report = run_multi_channel_benchmark(4, 4_096, 3, 107).unwrap();
        assert_eq!(report.channels, 3);
        assert_eq!(report.held_out_queries, 27);
        assert!(report.shared_mae.is_finite());
        assert!(report.independent_mae.is_finite());
        assert!(report.shared_to_independent_payload_ratio > 0.0);
        assert!(report.shared_to_grid_payload_ratio > 0.0);
        assert!(report.shared_vector_payload_bytes < report.independent_vector_payload_bytes);
    }
}
