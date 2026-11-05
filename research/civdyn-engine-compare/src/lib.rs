// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Trace-driven forest-fire cellular automaton reference workload.
//!
//! The random-event tape is generated before the timed stepping loop and is
//! serialized for cross-framework adapters. Engines consume identical initial
//! state and event words; they do not call their own RNG while stepping.
//! This isolates scheduler/transition semantics from RNG implementation.

use core::fmt;
use std::time::{Duration, Instant};

pub const WORKLOAD_ID: &str = "CIVDYN-ENGINE-FOREST-FIRE-TRACE-001";
pub const TAPE_MAGIC: &[u8; 8] = b"CIVDYNFF";
pub const TAPE_VERSION: u16 = 1;
pub const SMALL_WIDTH: usize = 32;
pub const SMALL_HEIGHT: usize = 32;
pub const LARGE_WIDTH: usize = 64;
pub const LARGE_HEIGHT: usize = 64;
pub const DEFAULT_TICKS: usize = 40;
pub const DEFAULT_SEEDS: [u64; 20] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
];
pub const DEFAULT_IGNITION_THRESHOLD: u16 = 8_192;
pub const DEFAULT_REGROWTH_THRESHOLD: u16 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Cell {
    Empty = 0,
    Tree = 1,
    Burning = 2,
    Ash = 3,
}

impl TryFrom<u8> for Cell {
    type Error = WorkloadError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Empty),
            1 => Ok(Self::Tree),
            2 => Ok(Self::Burning),
            3 => Ok(Self::Ash),
            _ => Err(WorkloadError::InvalidCellCode(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadSpec {
    pub workload_id: String,
    pub width: usize,
    pub height: usize,
    pub ticks: usize,
    pub seed: u64,
    /// A tree catches fire when an adjacent cell is burning and tape_word < threshold.
    pub ignition_threshold: u16,
    /// An ash cell regrows when tape_word < threshold.
    pub regrowth_threshold: u16,
    pub boundary: BoundaryCondition,
    pub scheduler: SchedulerSemantics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryCondition {
    FixedEmptyOutside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerSemantics {
    SynchronousDoubleBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub spec: WorkloadSpec,
    pub initial: Vec<Cell>,
    /// Row-major, tick-major event tape: ticks * width * height u16 words.
    pub event_tape: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepMetrics {
    pub tick: usize,
    pub empty: usize,
    pub tree: usize,
    pub burning: usize,
    pub ash: usize,
    pub total: usize,
}

impl StepMetrics {
    pub fn validate(&self) -> Result<(), WorkloadError> {
        let sum = self
            .empty
            .checked_add(self.tree)
            .and_then(|value| value.checked_add(self.burning))
            .and_then(|value| value.checked_add(self.ash))
            .ok_or(WorkloadError::CountOverflow)?;
        if sum != self.total {
            return Err(WorkloadError::CellCountMismatch {
                expected: self.total,
                actual: sum,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub spec: WorkloadSpec,
    pub trajectory: Vec<StepMetrics>,
    pub elapsed: Duration,
    /// FNV-1a 64-bit fingerprint for quick deterministic comparison only.
    /// It is not cryptographic provenance; CI computes SHA-256 of artifacts.
    pub initial_fingerprint_fnv64: u64,
    pub tape_fingerprint_fnv64: u64,
    pub trajectory_fingerprint_fnv64: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadError {
    EmptyWorkloadId,
    DimensionsTooSmall { width: usize, height: usize },
    ZeroTicks,
    DimensionOverflow,
    InitialLengthMismatch { expected: usize, actual: usize },
    TapeLengthMismatch { expected: usize, actual: usize },
    InvalidCellCode(u8),
    CellCountMismatch { expected: usize, actual: usize },
    CountOverflow,
    InvalidTapeHeader,
    TruncatedTape,
    TrailingTapeBytes,
    UnsupportedTapeVersion(u16),
    RecordCountMismatch { expected: usize, actual: usize },
}

impl fmt::Display for WorkloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyWorkloadId => write!(f, "workload ID must not be empty"),
            Self::DimensionsTooSmall { width, height } => {
                write!(f, "grid dimensions must be at least 2x2, got {width}x{height}")
            }
            Self::ZeroTicks => write!(f, "workload ticks must be greater than zero"),
            Self::DimensionOverflow => write!(f, "grid or tape dimensions overflow usize"),
            Self::InitialLengthMismatch { expected, actual } => {
                write!(f, "initial state has {actual} cells; expected {expected}")
            }
            Self::TapeLengthMismatch { expected, actual } => {
                write!(f, "event tape has {actual} words; expected {expected}")
            }
            Self::InvalidCellCode(code) => write!(f, "invalid cell code: {code}"),
            Self::CellCountMismatch { expected, actual } => {
                write!(f, "cell counts sum to {actual}; expected {expected}")
            }
            Self::CountOverflow => write!(f, "cell count overflow"),
            Self::InvalidTapeHeader => write!(f, "invalid forest-fire tape header"),
            Self::TruncatedTape => write!(f, "forest-fire tape is truncated"),
            Self::TrailingTapeBytes => write!(f, "forest-fire tape has trailing bytes"),
            Self::UnsupportedTapeVersion(version) => {
                write!(f, "unsupported tape version {version}")
            }
            Self::RecordCountMismatch { expected, actual } => {
                write!(f, "tape has {actual} records; expected {expected}")
            }
        }
    }
}

impl std::error::Error for WorkloadError {}

impl Scenario {
    pub fn validate(&self) -> Result<(), WorkloadError> {
        validate_spec(&self.spec)?;
        let cells = cell_count(&self.spec)?;
        if self.initial.len() != cells {
            return Err(WorkloadError::InitialLengthMismatch {
                expected: cells,
                actual: self.initial.len(),
            });
        }
        let tape_words = self
            .spec
            .ticks
            .checked_mul(cells)
            .ok_or(WorkloadError::DimensionOverflow)?;
        if self.event_tape.len() != tape_words {
            return Err(WorkloadError::TapeLengthMismatch {
                expected: tape_words,
                actual: self.event_tape.len(),
            });
        }
        Ok(())
    }

    pub fn encode_record(&self) -> Result<Vec<u8>, WorkloadError> {
        self.validate()?;
        let mut bytes = Vec::with_capacity(
            44 + self.initial.len() + self.event_tape.len() * 2,
        );
        bytes.extend_from_slice(&self.spec.width.to_le_bytes_u32());
        bytes.extend_from_slice(&self.spec.height.to_le_bytes_u32());
        bytes.extend_from_slice(&self.spec.ticks.to_le_bytes_u32());
        bytes.extend_from_slice(&self.spec.seed.to_le_bytes());
        bytes.extend_from_slice(&self.spec.ignition_threshold.to_le_bytes());
        bytes.extend_from_slice(&self.spec.regrowth_threshold.to_le_bytes());
        bytes.push(match self.spec.boundary {
            BoundaryCondition::FixedEmptyOutside => 0,
        });
        bytes.push(match self.spec.scheduler {
            SchedulerSemantics::SynchronousDoubleBuffer => 0,
        });
        bytes.extend(self.initial.iter().map(|cell| *cell as u8));
        for word in &self.event_tape {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        Ok(bytes)
    }

    pub fn decode_record(workload_id: &str, bytes: &[u8]) -> Result<Self, WorkloadError> {
        if bytes.len() < 32 {
            return Err(WorkloadError::TruncatedTape);
        }
        let mut cursor = 0usize;
        let width = take_u32(bytes, &mut cursor)? as usize;
        let height = take_u32(bytes, &mut cursor)? as usize;
        let ticks = take_u32(bytes, &mut cursor)? as usize;
        let seed = take_u64(bytes, &mut cursor)?;
        let ignition_threshold = take_u16(bytes, &mut cursor)?;
        let regrowth_threshold = take_u16(bytes, &mut cursor)?;
        let boundary = match take_u8(bytes, &mut cursor)? {
            0 => BoundaryCondition::FixedEmptyOutside,
            _ => return Err(WorkloadError::InvalidTapeHeader),
        };
        let scheduler = match take_u8(bytes, &mut cursor)? {
            0 => SchedulerSemantics::SynchronousDoubleBuffer,
            _ => return Err(WorkloadError::InvalidTapeHeader),
        };
        let spec = WorkloadSpec {
            workload_id: workload_id.into(),
            width,
            height,
            ticks,
            seed,
            ignition_threshold,
            regrowth_threshold,
            boundary,
            scheduler,
        };
        validate_spec(&spec)?;
        let cells = cell_count(&spec)?;
        let tape_words = ticks.checked_mul(cells).ok_or(WorkloadError::DimensionOverflow)?;
        let expected_len = cursor
            .checked_add(cells)
            .and_then(|value| value.checked_add(tape_words.checked_mul(2)?))
            .ok_or(WorkloadError::DimensionOverflow)?;
        if bytes.len() < expected_len {
            return Err(WorkloadError::TruncatedTape);
        }
        if bytes.len() > expected_len {
            return Err(WorkloadError::TrailingTapeBytes);
        }
        let initial = bytes[cursor..cursor + cells]
            .iter()
            .copied()
            .map(Cell::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        cursor += cells;
        let mut event_tape = Vec::with_capacity(tape_words);
        for _ in 0..tape_words {
            event_tape.push(take_u16(bytes, &mut cursor)?);
        }
        let scenario = Self {
            spec,
            initial,
            event_tape,
        };
        scenario.validate()?;
        Ok(scenario)
    }
}

trait U32Bytes {
    fn to_le_bytes_u32(self) -> [u8; 4];
}

impl U32Bytes for usize {
    fn to_le_bytes_u32(self) -> [u8; 4] {
        u32::try_from(self).expect("validated dimensions fit u32").to_le_bytes()
    }
}

fn take<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], WorkloadError> {
    let end = cursor.checked_add(N).ok_or(WorkloadError::TruncatedTape)?;
    let slice = bytes.get(*cursor..end).ok_or(WorkloadError::TruncatedTape)?;
    let mut result = [0u8; N];
    result.copy_from_slice(slice);
    *cursor = end;
    Ok(result)
}

fn take_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, WorkloadError> {
    Ok(take::<1>(bytes, cursor)?[0])
}
fn take_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, WorkloadError> {
    Ok(u16::from_le_bytes(take::<2>(bytes, cursor)?))
}
fn take_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, WorkloadError> {
    Ok(u32::from_le_bytes(take::<4>(bytes, cursor)?))
}
fn take_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, WorkloadError> {
    Ok(u64::from_le_bytes(take::<8>(bytes, cursor)?))
}

fn validate_spec(spec: &WorkloadSpec) -> Result<(), WorkloadError> {
    if spec.workload_id.trim().is_empty() {
        return Err(WorkloadError::EmptyWorkloadId);
    }
    if spec.width < 2 || spec.height < 2 {
        return Err(WorkloadError::DimensionsTooSmall {
            width: spec.width,
            height: spec.height,
        });
    }
    if spec.ticks == 0 {
        return Err(WorkloadError::ZeroTicks);
    }
    let _ = cell_count(spec)?;
    u32::try_from(spec.width).map_err(|_| WorkloadError::DimensionOverflow)?;
    u32::try_from(spec.height).map_err(|_| WorkloadError::DimensionOverflow)?;
    u32::try_from(spec.ticks).map_err(|_| WorkloadError::DimensionOverflow)?;
    Ok(())
}

fn cell_count(spec: &WorkloadSpec) -> Result<usize, WorkloadError> {
    spec.width
        .checked_mul(spec.height)
        .ok_or(WorkloadError::DimensionOverflow)
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^ (value >> 31)
    }

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
}

pub fn generate_scenario(
    width: usize,
    height: usize,
    ticks: usize,
    seed: u64,
) -> Result<Scenario, WorkloadError> {
    let spec = WorkloadSpec {
        workload_id: WORKLOAD_ID.into(),
        width,
        height,
        ticks,
        seed,
        ignition_threshold: DEFAULT_IGNITION_THRESHOLD,
        regrowth_threshold: DEFAULT_REGROWTH_THRESHOLD,
        boundary: BoundaryCondition::FixedEmptyOutside,
        scheduler: SchedulerSemantics::SynchronousDoubleBuffer,
    };
    validate_spec(&spec)?;
    let cells = cell_count(&spec)?;
    let tape_words = ticks.checked_mul(cells).ok_or(WorkloadError::DimensionOverflow)?;
    let mut random = SplitMix64::new(seed);
    let mut initial = Vec::with_capacity(cells);
    for _ in 0..cells {
        let roll = random.next_u16();
        initial.push(if roll < 45_875 {
            Cell::Tree
        } else if roll < 46_203 {
            Cell::Burning
        } else {
            Cell::Empty
        });
    }
    let event_tape = (0..tape_words).map(|_| random.next_u16()).collect();
    let scenario = Scenario {
        spec,
        initial,
        event_tape,
    };
    scenario.validate()?;
    Ok(scenario)
}

pub fn run_reference(scenario: &Scenario) -> Result<RunResult, WorkloadError> {
    scenario.validate()?;
    let initial_fingerprint_fnv64 = fnv1a64(
        &scenario.initial.iter().map(|cell| *cell as u8).collect::<Vec<_>>(),
    );
    let tape_bytes = scenario
        .event_tape
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<_>>();
    let tape_fingerprint_fnv64 = fnv1a64(&tape_bytes);

    let start = Instant::now();
    let cells = scenario.initial.len();
    let mut current = scenario.initial.clone();
    let mut next = vec![Cell::Empty; cells];
    let mut trajectory = Vec::with_capacity(scenario.spec.ticks + 1);
    trajectory.push(count_cells(0, &current)?);

    for tick in 0..scenario.spec.ticks {
        let tape_start = tick * cells;
        let tape = &scenario.event_tape[tape_start..tape_start + cells];
        for index in 0..cells {
            let state = current[index];
            next[index] = match state {
                Cell::Empty => Cell::Empty,
                Cell::Burning => Cell::Ash,
                Cell::Ash => {
                    if tape[index] < scenario.spec.regrowth_threshold {
                        Cell::Tree
                    } else {
                        Cell::Ash
                    }
                }
                Cell::Tree => {
                    if has_burning_neighbor(index, scenario.spec.width, scenario.spec.height, &current)
                        && tape[index] < scenario.spec.ignition_threshold
                    {
                        Cell::Burning
                    } else {
                        Cell::Tree
                    }
                }
            };
        }
        std::mem::swap(&mut current, &mut next);
        trajectory.push(count_cells(tick + 1, &current)?);
    }
    let elapsed = start.elapsed();
    let trajectory_bytes = trajectory
        .iter()
        .flat_map(|metric| {
            [
                metric.tick as u64,
                metric.empty as u64,
                metric.tree as u64,
                metric.burning as u64,
                metric.ash as u64,
                metric.total as u64,
            ]
            .into_iter()
            .flat_map(u64::to_le_bytes)
        })
        .collect::<Vec<_>>();
    let trajectory_fingerprint_fnv64 = fnv1a64(&trajectory_bytes);

    Ok(RunResult {
        spec: scenario.spec.clone(),
        trajectory,
        elapsed,
        initial_fingerprint_fnv64,
        tape_fingerprint_fnv64,
        trajectory_fingerprint_fnv64,
    })
}

fn has_burning_neighbor(
    index: usize,
    width: usize,
    height: usize,
    cells: &[Cell],
) -> bool {
    let x = index % width;
    let y = index / width;
    let y_start = y.saturating_sub(1);
    let y_end = (y + 1).min(height - 1);
    let x_start = x.saturating_sub(1);
    let x_end = (x + 1).min(width - 1);
    for neighbor_y in y_start..=y_end {
        for neighbor_x in x_start..=x_end {
            if neighbor_x == x && neighbor_y == y {
                continue;
            }
            if cells[neighbor_y * width + neighbor_x] == Cell::Burning {
                return true;
            }
        }
    }
    false
}

fn count_cells(tick: usize, cells: &[Cell]) -> Result<StepMetrics, WorkloadError> {
    let mut metrics = StepMetrics {
        tick,
        empty: 0,
        tree: 0,
        burning: 0,
        ash: 0,
        total: cells.len(),
    };
    for cell in cells {
        match cell {
            Cell::Empty => metrics.empty += 1,
            Cell::Tree => metrics.tree += 1,
            Cell::Burning => metrics.burning += 1,
            Cell::Ash => metrics.ash += 1,
        }
    }
    metrics.validate()?;
    Ok(metrics)
}

pub fn encode_tape_bundle(scenarios: &[Scenario]) -> Result<Vec<u8>, WorkloadError> {
    let count = u32::try_from(scenarios.len()).map_err(|_| WorkloadError::DimensionOverflow)?;
    let mut records = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        records.push(scenario.encode_record()?);
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(TAPE_MAGIC);
    bytes.extend_from_slice(&TAPE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    for record in records {
        let length = u64::try_from(record.len()).map_err(|_| WorkloadError::DimensionOverflow)?;
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(&record);
    }
    Ok(bytes)
}

pub fn decode_tape_bundle(bytes: &[u8]) -> Result<Vec<Scenario>, WorkloadError> {
    if bytes.len() < 14 || &bytes[..8] != TAPE_MAGIC {
        return Err(WorkloadError::InvalidTapeHeader);
    }
    let mut cursor = 8usize;
    let version = take_u16(bytes, &mut cursor)?;
    if version != TAPE_VERSION {
        return Err(WorkloadError::UnsupportedTapeVersion(version));
    }
    let count = take_u32(bytes, &mut cursor)? as usize;
    let mut scenarios = Vec::with_capacity(count);
    for _ in 0..count {
        let length = usize::try_from(take_u64(bytes, &mut cursor)?)
            .map_err(|_| WorkloadError::DimensionOverflow)?;
        let end = cursor.checked_add(length).ok_or(WorkloadError::TruncatedTape)?;
        let record = bytes.get(cursor..end).ok_or(WorkloadError::TruncatedTape)?;
        scenarios.push(Scenario::decode_record(WORKLOAD_ID, record)?);
        cursor = end;
    }
    if cursor != bytes.len() {
        return Err(WorkloadError::TrailingTapeBytes);
    }
    Ok(scenarios)
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

pub fn default_scenarios() -> Result<Vec<Scenario>, WorkloadError> {
    let mut scenarios = Vec::new();
    for seed in DEFAULT_SEEDS {
        scenarios.push(generate_scenario(SMALL_WIDTH, SMALL_HEIGHT, DEFAULT_TICKS, seed)?);
        scenarios.push(generate_scenario(LARGE_WIDTH, LARGE_HEIGHT, DEFAULT_TICKS, seed)?);
    }
    Ok(scenarios)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_seed_scenarios_have_identical_inputs() {
        let first = generate_scenario(32, 32, 10, 42).unwrap();
        let second = generate_scenario(32, 32, 10, 42).unwrap();
        assert_eq!(first, second);
        assert_ne!(first.event_tape, generate_scenario(32, 32, 10, 43).unwrap().event_tape);
    }

    #[test]
    fn tape_bundle_round_trips_all_initial_and_event_bytes() {
        let scenarios = vec![
            generate_scenario(8, 8, 4, 1).unwrap(),
            generate_scenario(8, 8, 4, 2).unwrap(),
        ];
        let encoded = encode_tape_bundle(&scenarios).unwrap();
        let decoded = decode_tape_bundle(&encoded).unwrap();
        assert_eq!(decoded, scenarios);
    }

    #[test]
    fn tape_bundle_rejects_truncation_and_trailing_bytes() {
        let scenario = generate_scenario(8, 8, 2, 1).unwrap();
        let mut encoded = encode_tape_bundle(&[scenario]).unwrap();
        let truncated = encoded.pop().unwrap();
        assert!(matches!(decode_tape_bundle(&encoded), Err(WorkloadError::TruncatedTape)));
        encoded.push(truncated);
        encoded.push(0);
        assert!(matches!(decode_tape_bundle(&encoded), Err(WorkloadError::TrailingTapeBytes)));
    }

    #[test]
    fn no_ignition_control_does_not_spread_a_fire() {
        let mut scenario = generate_scenario(8, 8, 5, 1).unwrap();
        scenario.spec.ignition_threshold = 0;
        let result = run_reference(&scenario).unwrap();
        assert!(result.trajectory.iter().skip(1).all(|row| row.burning == 0));
    }

    #[test]
    fn synchronous_transition_does_not_burn_a_cell_in_the_same_tick_as_ignition() {
        let spec = WorkloadSpec {
            workload_id: WORKLOAD_ID.into(),
            width: 3,
            height: 3,
            ticks: 1,
            seed: 0,
            ignition_threshold: u16::MAX,
            regrowth_threshold: 0,
            boundary: BoundaryCondition::FixedEmptyOutside,
            scheduler: SchedulerSemantics::SynchronousDoubleBuffer,
        };
        let mut initial = vec![Cell::Empty; 9];
        initial[4] = Cell::Burning;
        initial[1] = Cell::Tree;
        initial[7] = Cell::Tree;
        let scenario = Scenario {
            spec,
            initial,
            event_tape: vec![0; 9],
        };
        let result = run_reference(&scenario).unwrap();
        assert_eq!(result.trajectory[1].burning, 2);
        assert_eq!(result.trajectory[1].ash, 1);
    }

    #[test]
    fn every_trajectory_row_has_exactly_one_state_per_cell() {
        let scenario = generate_scenario(16, 12, 9, 8).unwrap();
        let result = run_reference(&scenario).unwrap();
        assert_eq!(result.trajectory.len(), 10);
        assert!(result.trajectory.iter().all(|row| row.total == 192));
    }

    #[test]
    fn reference_replay_is_identical_for_the_same_tape() {
        let scenario = generate_scenario(32, 32, 20, 19).unwrap();
        assert_eq!(run_reference(&scenario).unwrap(), run_reference(&scenario).unwrap());
    }

    #[test]
    fn invalid_scenarios_fail_closed() {
        let mut scenario = generate_scenario(8, 8, 2, 1).unwrap();
        scenario.event_tape.pop();
        assert!(matches!(scenario.validate(), Err(WorkloadError::TapeLengthMismatch { .. })));
        let mut empty = generate_scenario(8, 8, 2, 1).unwrap();
        empty.spec.workload_id.clear();
        assert!(matches!(empty.validate(), Err(WorkloadError::EmptyWorkloadId)));
    }

    #[test]
    fn all_cell_codes_are_exhaustive() {
        for value in 0..=3 {
            assert!(Cell::try_from(value).is_ok());
        }
        assert!(matches!(Cell::try_from(4), Err(WorkloadError::InvalidCellCode(4))));
    }
}
