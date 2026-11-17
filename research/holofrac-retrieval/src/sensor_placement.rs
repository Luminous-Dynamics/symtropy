//! Reproducible sensor-placement and nearest-sensor virtual-sensing baselines.
//!
//! This experiment isolates geometric placement effects with an analytic,
//! noise-free synthetic field. It is not a measured-building benchmark and
//! nearest-neighbour virtual sensing is a deliberately simple baseline.

use crate::temporal::{synthetic_field_value, SpatialChannel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensorPlacementPolicy {
    RegularGrid,
    Random,
    GreedyMaximin,
}

impl SensorPlacementPolicy {
    pub const ALL: [Self; 3] = [Self::RegularGrid, Self::Random, Self::GreedyMaximin];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RegularGrid => "regular_grid",
            Self::Random => "random",
            Self::GreedyMaximin => "greedy_maximin",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SensorPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SensorPlacementReport {
    pub policy: SensorPlacementPolicy,
    pub seed: u64,
    pub candidate_count: usize,
    pub sensor_count: usize,
    pub query_grid_size: usize,
    pub time_samples: usize,
    pub channel_count: usize,
    pub query_count: usize,
    pub mean_nearest_distance: f64,
    pub max_nearest_distance: f64,
    pub mean_absolute_error: f64,
    pub root_mean_squared_error: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SensorPlacementError {
    InvalidCandidateGrid,
    InvalidSensorCount,
    InvalidQueryGrid,
    InvalidTimeSamples,
}

impl std::fmt::Display for SensorPlacementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCandidateGrid => {
                f.write_str("candidate grid must contain at least two points per axis")
            }
            Self::InvalidSensorCount => {
                f.write_str("sensor count must be non-zero and no greater than candidate count")
            }
            Self::InvalidQueryGrid => {
                f.write_str("query grid must contain at least two points per axis")
            }
            Self::InvalidTimeSamples => f.write_str("time sample count must be non-zero"),
        }
    }
}

impl std::error::Error for SensorPlacementError {}

#[derive(Clone, Debug)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

fn squared_distance(left: SensorPoint, right: SensorPoint) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    dx * dx + dy * dy
}

fn candidate_grid(side: usize) -> Vec<SensorPoint> {
    let mut points = Vec::with_capacity(side * side);
    for y in 0..side {
        for x in 0..side {
            points.push(SensorPoint {
                x: (x as f64 + 0.5) / side as f64,
                y: (y as f64 + 0.5) / side as f64,
            });
        }
    }
    points
}

fn nearest_unselected_candidate(
    candidates: &[SensorPoint],
    selected: &[usize],
    target: SensorPoint,
) -> usize {
    let mut best_index = usize::MAX;
    let mut best_distance = f64::INFINITY;

    for (index, candidate) in candidates.iter().enumerate() {
        if selected.contains(&index) {
            continue;
        }
        let distance = squared_distance(*candidate, target);
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
    }

    best_index
}

fn regular_grid_selection(candidates: &[SensorPoint], sensor_count: usize) -> Vec<usize> {
    let rows = (sensor_count as f64).sqrt().floor().max(1.0) as usize;
    let columns = sensor_count.div_ceil(rows);
    let mut selected = Vec::with_capacity(sensor_count);

    for index in 0..sensor_count {
        let row = index / columns;
        let column = index % columns;
        let target = SensorPoint {
            x: (column as f64 + 0.5) / columns as f64,
            y: (row as f64 + 0.5) / rows as f64,
        };
        let chosen = nearest_unselected_candidate(candidates, &selected, target);
        selected.push(chosen);
    }

    selected
}

fn random_selection(
    candidate_count: usize,
    sensor_count: usize,
    seed: u64,
) -> Vec<usize> {
    let mut rng = DeterministicRng::new(seed);
    let mut indices: Vec<_> = (0..candidate_count).collect();

    // Partial Fisher-Yates: the first sensor_count indices form a uniform
    // deterministic sample without replacement for this PRNG seed.
    for index in 0..sensor_count {
        let remaining = candidate_count - index;
        let offset = (rng.next_u64() % remaining as u64) as usize;
        indices.swap(index, index + offset);
    }

    indices.truncate(sensor_count);
    indices
}

fn greedy_maximin_selection(
    candidates: &[SensorPoint],
    sensor_count: usize,
) -> Vec<usize> {
    let centre = SensorPoint { x: 0.5, y: 0.5 };
    let first = candidates
        .iter()
        .enumerate()
        .min_by(|left, right| {
            squared_distance(*left.1, centre)
                .total_cmp(&squared_distance(*right.1, centre))
        })
        .map(|(index, _)| index)
        .unwrap_or(0);

    let mut selected = vec![first];
    let mut chosen = vec![false; candidates.len()];
    chosen[first] = true;

    while selected.len() < sensor_count {
        let mut best_index = usize::MAX;
        let mut best_minimum_distance = f64::NEG_INFINITY;

        for (candidate_index, candidate) in candidates.iter().enumerate() {
            if chosen[candidate_index] {
                continue;
            }

            let minimum_distance = selected
                .iter()
                .map(|selected_index| {
                    squared_distance(*candidate, candidates[*selected_index])
                })
                .fold(f64::INFINITY, f64::min);

            // Strict comparison preserves candidate-order tie breaking.
            if minimum_distance > best_minimum_distance {
                best_minimum_distance = minimum_distance;
                best_index = candidate_index;
            }
        }

        if best_index == usize::MAX {
            break;
        }
        chosen[best_index] = true;
        selected.push(best_index);
    }

    selected
}

fn choose_sensor_indices(
    candidates: &[SensorPoint],
    sensor_count: usize,
    policy: SensorPlacementPolicy,
    seed: u64,
) -> Vec<usize> {
    match policy {
        SensorPlacementPolicy::RegularGrid => {
            regular_grid_selection(candidates, sensor_count)
        }
        SensorPlacementPolicy::Random => {
            random_selection(candidates.len(), sensor_count, seed)
        }
        SensorPlacementPolicy::GreedyMaximin => {
            greedy_maximin_selection(candidates, sensor_count)
        }
    }
}

pub fn run_sensor_placement_benchmark(
    candidate_grid_size: usize,
    sensor_count: usize,
    query_grid_size: usize,
    time_samples: usize,
    seed: u64,
) -> Result<Vec<SensorPlacementReport>, SensorPlacementError> {
    if candidate_grid_size < 2 {
        return Err(SensorPlacementError::InvalidCandidateGrid);
    }
    if query_grid_size < 2 {
        return Err(SensorPlacementError::InvalidQueryGrid);
    }
    if time_samples == 0 {
        return Err(SensorPlacementError::InvalidTimeSamples);
    }

    let candidates = candidate_grid(candidate_grid_size);
    if sensor_count == 0 || sensor_count > candidates.len() {
        return Err(SensorPlacementError::InvalidSensorCount);
    }

    let query_points: Vec<_> = (0..query_grid_size)
        .flat_map(|y| {
            (0..query_grid_size).map(move |x| SensorPoint {
                x: x as f64 / (query_grid_size - 1) as f64,
                y: y as f64 / (query_grid_size - 1) as f64,
            })
        })
        .collect();

    let mut reports = Vec::with_capacity(SensorPlacementPolicy::ALL.len());

    for policy in SensorPlacementPolicy::ALL {
        let selected_indices =
            choose_sensor_indices(&candidates, sensor_count, policy, seed);
        let sensors: Vec<_> = selected_indices
            .iter()
            .map(|index| candidates[*index])
            .collect();

        let mut distance_sum = 0.0_f64;
        let mut maximum_distance = 0.0_f64;
        let mut absolute_error_sum = 0.0_f64;
        let mut squared_error_sum = 0.0_f64;
        let mut query_count = 0_usize;

        for channel in SpatialChannel::ALL {
            for time_index in 0..time_samples {
                // Sample a full day and then continue across day boundaries;
                // this is a synthetic temporal coverage test only.
                let time_seconds = time_index as f64 * (86_400.0 / time_samples as f64);
                let sensor_values: Vec<_> = sensors
                    .iter()
                    .map(|sensor| {
                        synthetic_field_value(
                            channel,
                            sensor.x,
                            sensor.y,
                            time_seconds,
                        )
                    })
                    .collect();

                for query in &query_points {
                    let mut nearest_index = 0_usize;
                    let mut nearest_squared_distance = f64::INFINITY;

                    for (index, sensor) in sensors.iter().enumerate() {
                        let distance = squared_distance(*query, *sensor);
                        if distance < nearest_squared_distance {
                            nearest_squared_distance = distance;
                            nearest_index = index;
                        }
                    }

                    let estimate = sensor_values[nearest_index];
                    let truth =
                        synthetic_field_value(channel, query.x, query.y, time_seconds);
                    let error = estimate - truth;
                    let distance = nearest_squared_distance.sqrt();

                    distance_sum += distance;
                    maximum_distance = maximum_distance.max(distance);
                    absolute_error_sum += error.abs();
                    squared_error_sum += error * error;
                    query_count += 1;
                }
            }
        }

        let denominator = query_count as f64;
        reports.push(SensorPlacementReport {
            policy,
            seed,
            candidate_count: candidates.len(),
            sensor_count,
            query_grid_size,
            time_samples,
            channel_count: SpatialChannel::ALL.len(),
            query_count,
            mean_nearest_distance: distance_sum / denominator,
            max_nearest_distance: maximum_distance,
            mean_absolute_error: absolute_error_sum / denominator,
            root_mean_squared_error: (squared_error_sum / denominator).sqrt(),
        });
    }

    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_benchmark_is_deterministic_for_a_fixed_seed() {
        let first = run_sensor_placement_benchmark(16, 8, 7, 6, 42).unwrap();
        let second = run_sensor_placement_benchmark(16, 8, 7, 6, 42).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn every_policy_returns_finite_metrics_for_all_channels() {
        let reports = run_sensor_placement_benchmark(12, 6, 6, 4, 77).unwrap();
        assert_eq!(reports.len(), SensorPlacementPolicy::ALL.len());

        for report in reports {
            assert_eq!(report.sensor_count, 6);
            assert_eq!(report.channel_count, SpatialChannel::ALL.len());
            assert!(report.query_count > 0);
            assert!(report.mean_nearest_distance.is_finite());
            assert!(report.max_nearest_distance.is_finite());
            assert!(report.mean_absolute_error.is_finite());
            assert!(report.root_mean_squared_error.is_finite());
        }
    }

    #[test]
    fn additional_greedy_maximin_sensors_do_not_reduce_coverage_on_the_fixture_grid() {
        let small = run_sensor_placement_benchmark(16, 4, 12, 4, 5).unwrap();
        let large = run_sensor_placement_benchmark(16, 12, 12, 4, 5).unwrap();
        let small = small
            .iter()
            .find(|report| report.policy == SensorPlacementPolicy::GreedyMaximin)
            .unwrap();
        let large = large
            .iter()
            .find(|report| report.policy == SensorPlacementPolicy::GreedyMaximin)
            .unwrap();

        assert!(large.max_nearest_distance <= small.max_nearest_distance);
    }

    #[test]
    fn invalid_sensor_and_grid_budgets_are_rejected() {
        assert_eq!(
            run_sensor_placement_benchmark(1, 1, 4, 2, 1).unwrap_err(),
            SensorPlacementError::InvalidCandidateGrid
        );
        assert_eq!(
            run_sensor_placement_benchmark(4, 0, 4, 2, 1).unwrap_err(),
            SensorPlacementError::InvalidSensorCount
        );
        assert_eq!(
            run_sensor_placement_benchmark(4, 17, 4, 2, 1).unwrap_err(),
            SensorPlacementError::InvalidSensorCount
        );
    }
}
