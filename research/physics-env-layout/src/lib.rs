//! Exact-equivalence microbenchmark for independent simulation state layouts.
//!
//! The kernel intentionally contains no contacts, constraints, reductions,
//! random-number generation, or cross-environment communication. It measures
//! data-layout and scheduling effects for the same scalar semi-implicit
//! constant-acceleration update. It is not a rigid-body contact benchmark,
//! GPU benchmark, or physical validation result.

use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentState {
    pub id: u64,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub acceleration: [f64; 3],
}

impl EnvironmentState {
    /// Deterministic, ID-derived initial state. No RNG or shared sequence state.
    pub fn from_id(id: u64) -> Self {
        let a = id.wrapping_mul(13).wrapping_add(7);
        let b = id.wrapping_mul(17).wrapping_add(29);
        let c = id.wrapping_mul(23).wrapping_add(11);
        let d = id.wrapping_mul(31).wrapping_add(5);

        Self {
            id,
            position: [
                (a % 997) as f64 * 0.001,
                (b % 991) as f64 * 0.002,
                (c % 983) as f64 * 0.003,
            ],
            velocity: [
                (b % 97) as f64 * 0.01 - 0.4,
                (c % 89) as f64 * 0.01 - 0.3,
                (d % 83) as f64 * 0.01 - 0.2,
            ],
            acceleration: [
                (c % 37) as f64 * 0.001 - 0.018,
                (d % 31) as f64 * 0.001 - 0.015,
                (a % 29) as f64 * 0.001 - 0.014,
            ],
        }
    }

    fn bits(self) -> EnvironmentBits {
        EnvironmentBits {
            id: self.id,
            position: self.position.map(f64::to_bits),
            velocity: self.velocity.map(f64::to_bits),
            acceleration: self.acceleration.map(f64::to_bits),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvironmentBits {
    pub id: u64,
    pub position: [u64; 3],
    pub velocity: [u64; 3],
    pub acceleration: [u64; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutError {
    EmptyBatch,
    InvalidTimestep,
    InvalidLaneWidth,
    ShapeMismatch,
    UnexpectedEnvironmentId { id: u64, expected_count: usize },
    DuplicateEnvironmentId { id: u64 },
    MissingEnvironmentId { id: u64 },
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBatch => write!(f, "environment batch must not be empty"),
            Self::InvalidTimestep => write!(f, "dt must be finite and greater than zero"),
            Self::InvalidLaneWidth => write!(f, "AoSoA lane width must be greater than zero"),
            Self::ShapeMismatch => write!(f, "SoA field arrays must all have the same length as IDs"),
            Self::UnexpectedEnvironmentId { id, expected_count } => {
                write!(f, "unexpected environment id {id}; expected ids 0..{expected_count}")
            }
            Self::DuplicateEnvironmentId { id } => {
                write!(f, "duplicate environment id: {id}")
            }
            Self::MissingEnvironmentId { id } => write!(f, "missing environment id: {id}"),
        }
    }
}

impl std::error::Error for LayoutError {}

/// Array of Structs: one full record per independent environment.
#[derive(Clone, Debug)]
pub struct AosLayout {
    pub states: Vec<EnvironmentState>,
}

impl AosLayout {
    pub fn from_count(count: usize) -> Self {
        Self {
            states: (0..count as u64).map(EnvironmentState::from_id).collect(),
        }
    }

    pub fn from_states(states: Vec<EnvironmentState>) -> Self {
        Self { states }
    }

    pub fn step_n(&mut self, steps: usize, dt: f64) -> Result<(), LayoutError> {
        validate_dt(dt)?;
        for _ in 0..steps {
            for state in &mut self.states {
                step_state(state, dt);
            }
        }
        Ok(())
    }

    pub fn step_n_parallel(&mut self, steps: usize, dt: f64) -> Result<(), LayoutError> {
        validate_dt(dt)?;
        for _ in 0..steps {
            self.states.par_iter_mut().for_each(|state| step_state(state, dt));
        }
        Ok(())
    }

    pub fn canonical_snapshot(&self, expected_count: usize) -> Result<Vec<EnvironmentBits>, LayoutError> {
        canonicalize(
            self.states.iter().copied().map(EnvironmentState::bits).collect(),
            expected_count,
        )
    }

    pub fn digest(&self, expected_count: usize) -> Result<String, LayoutError> {
        digest_snapshot(&self.canonical_snapshot(expected_count)?)
    }
}

/// Structure of Arrays: each field is stored in its own contiguous vector.
#[derive(Clone, Debug)]
pub struct SoaLayout {
    pub ids: Vec<u64>,
    pub position: [Vec<f64>; 3],
    pub velocity: [Vec<f64>; 3],
    pub acceleration: [Vec<f64>; 3],
}

impl SoaLayout {
    pub fn from_aos(source: &AosLayout) -> Self {
        let len = source.states.len();
        let mut output = Self {
            ids: Vec::with_capacity(len),
            position: std::array::from_fn(|_| Vec::with_capacity(len)),
            velocity: std::array::from_fn(|_| Vec::with_capacity(len)),
            acceleration: std::array::from_fn(|_| Vec::with_capacity(len)),
        };
        for state in &source.states {
            output.ids.push(state.id);
            for axis in 0..3 {
                output.position[axis].push(state.position[axis]);
                output.velocity[axis].push(state.velocity[axis]);
                output.acceleration[axis].push(state.acceleration[axis]);
            }
        }
        output
    }

    fn validate_shape(&self) -> Result<(), LayoutError> {
        let expected = self.ids.len();
        if self.position.iter().any(|field| field.len() != expected)
            || self.velocity.iter().any(|field| field.len() != expected)
            || self.acceleration.iter().any(|field| field.len() != expected)
        {
            return Err(LayoutError::ShapeMismatch);
        }
        Ok(())
    }

    pub fn step_n(&mut self, steps: usize, dt: f64) -> Result<(), LayoutError> {
        validate_dt(dt)?;
        self.validate_shape()?;
        for _ in 0..steps {
            for i in 0..self.ids.len() {
                for axis in 0..3 {
                    step_component(
                        &mut self.position[axis][i],
                        &mut self.velocity[axis][i],
                        self.acceleration[axis][i],
                        dt,
                    );
                }
            }
        }
        Ok(())
    }

    pub fn step_n_parallel(&mut self, steps: usize, dt: f64) -> Result<(), LayoutError> {
        validate_dt(dt)?;
        self.validate_shape()?;
        let [px, py, pz] = &mut self.position;
        let [vx, vy, vz] = &mut self.velocity;
        let [ax, ay, az] = &self.acceleration;

        for _ in 0..steps {
            px.par_iter_mut()
                .zip(py.par_iter_mut())
                .zip(pz.par_iter_mut())
                .zip(vx.par_iter_mut())
                .zip(vy.par_iter_mut())
                .zip(vz.par_iter_mut())
                .zip(ax.par_iter())
                .zip(ay.par_iter())
                .zip(az.par_iter())
                .for_each(|((((((((px, py), pz), vx), vy), vz), ax), ay), az)| {
                    step_component(px, vx, *ax, dt);
                    step_component(py, vy, *ay, dt);
                    step_component(pz, vz, *az, dt);
                });
        }
        Ok(())
    }

    pub fn canonical_snapshot(&self, expected_count: usize) -> Result<Vec<EnvironmentBits>, LayoutError> {
        self.validate_shape()?;
        let len = self.ids.len();
        let mut states = Vec::with_capacity(len);
        for i in 0..len {
            states.push(EnvironmentBits {
                id: self.ids[i],
                position: [
                    self.position[0][i].to_bits(),
                    self.position[1][i].to_bits(),
                    self.position[2][i].to_bits(),
                ],
                velocity: [
                    self.velocity[0][i].to_bits(),
                    self.velocity[1][i].to_bits(),
                    self.velocity[2][i].to_bits(),
                ],
                acceleration: [
                    self.acceleration[0][i].to_bits(),
                    self.acceleration[1][i].to_bits(),
                    self.acceleration[2][i].to_bits(),
                ],
            });
        }
        canonicalize(states, expected_count)
    }

    pub fn digest(&self, expected_count: usize) -> Result<String, LayoutError> {
        digest_snapshot(&self.canonical_snapshot(expected_count)?)
    }
}

#[derive(Clone, Debug)]
struct AoSoABlock<const LANES: usize> {
    len: usize,
    ids: [u64; LANES],
    position: [[f64; LANES]; 3],
    velocity: [[f64; LANES]; 3],
    acceleration: [[f64; LANES]; 3],
}

/// Array of Structs of Arrays: each tile stores fixed-width field arrays.
#[derive(Clone, Debug)]
pub struct AosoaLayout<const LANES: usize> {
    blocks: Vec<AoSoABlock<LANES>>,
    len: usize,
}

impl<const LANES: usize> AosoaLayout<LANES> {
    pub fn from_aos(source: &AosLayout) -> Result<Self, LayoutError> {
        if LANES == 0 {
            return Err(LayoutError::InvalidLaneWidth);
        }
        let mut blocks = Vec::with_capacity(source.states.len().div_ceil(LANES));
        for chunk in source.states.chunks(LANES) {
            let mut block = AoSoABlock {
                len: chunk.len(),
                ids: [0; LANES],
                position: [[0.0; LANES]; 3],
                velocity: [[0.0; LANES]; 3],
                acceleration: [[0.0; LANES]; 3],
            };
            for (lane, state) in chunk.iter().enumerate() {
                block.ids[lane] = state.id;
                for axis in 0..3 {
                    block.position[axis][lane] = state.position[axis];
                    block.velocity[axis][lane] = state.velocity[axis];
                    block.acceleration[axis][lane] = state.acceleration[axis];
                }
            }
            blocks.push(block);
        }
        Ok(Self {
            blocks,
            len: source.states.len(),
        })
    }

    pub fn step_n(&mut self, steps: usize, dt: f64) -> Result<(), LayoutError> {
        validate_dt(dt)?;
        for _ in 0..steps {
            for block in &mut self.blocks {
                for lane in 0..block.len {
                    for axis in 0..3 {
                        step_component(
                            &mut block.position[axis][lane],
                            &mut block.velocity[axis][lane],
                            block.acceleration[axis][lane],
                            dt,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub fn step_n_parallel(&mut self, steps: usize, dt: f64) -> Result<(), LayoutError> {
        validate_dt(dt)?;
        for _ in 0..steps {
            self.blocks.par_iter_mut().for_each(|block| {
                for lane in 0..block.len {
                    for axis in 0..3 {
                        step_component(
                            &mut block.position[axis][lane],
                            &mut block.velocity[axis][lane],
                            block.acceleration[axis][lane],
                            dt,
                        );
                    }
                }
            });
        }
        Ok(())
    }

    pub fn canonical_snapshot(&self, expected_count: usize) -> Result<Vec<EnvironmentBits>, LayoutError> {
        let mut states = Vec::with_capacity(self.len);
        for block in &self.blocks {
            for lane in 0..block.len {
                states.push(EnvironmentBits {
                    id: block.ids[lane],
                    position: std::array::from_fn(|axis| block.position[axis][lane].to_bits()),
                    velocity: std::array::from_fn(|axis| block.velocity[axis][lane].to_bits()),
                    acceleration: std::array::from_fn(|axis| block.acceleration[axis][lane].to_bits()),
                });
            }
        }
        canonicalize(states, expected_count)
    }

    pub fn digest(&self, expected_count: usize) -> Result<String, LayoutError> {
        digest_snapshot(&self.canonical_snapshot(expected_count)?)
    }
}

fn step_state(state: &mut EnvironmentState, dt: f64) {
    for axis in 0..3 {
        step_component(
            &mut state.position[axis],
            &mut state.velocity[axis],
            state.acceleration[axis],
            dt,
        );
    }
}

#[inline]
fn step_component(position: &mut f64, velocity: &mut f64, acceleration: f64, dt: f64) {
    *velocity += acceleration * dt;
    *position += *velocity * dt;
}

fn validate_dt(dt: f64) -> Result<(), LayoutError> {
    if !dt.is_finite() || dt <= 0.0 {
        Err(LayoutError::InvalidTimestep)
    } else {
        Ok(())
    }
}

fn canonicalize(
    mut states: Vec<EnvironmentBits>,
    expected_count: usize,
) -> Result<Vec<EnvironmentBits>, LayoutError> {
    if expected_count == 0 {
        return Err(LayoutError::EmptyBatch);
    }
    let mut seen = vec![false; expected_count];
    for state in &states {
        let index = usize::try_from(state.id).map_err(|_| LayoutError::UnexpectedEnvironmentId {
            id: state.id,
            expected_count,
        })?;
        if index >= expected_count {
            return Err(LayoutError::UnexpectedEnvironmentId {
                id: state.id,
                expected_count,
            });
        }
        if seen[index] {
            return Err(LayoutError::DuplicateEnvironmentId { id: state.id });
        }
        seen[index] = true;
    }
    for (id, found) in seen.into_iter().enumerate() {
        if !found {
            return Err(LayoutError::MissingEnvironmentId { id: id as u64 });
        }
    }
    states.sort_by_key(|state| state.id);
    Ok(states)
}

fn digest_snapshot(states: &[EnvironmentBits]) -> Result<String, LayoutError> {
    let mut hasher = Sha256::new();
    for state in states {
        hasher.update(state.id.to_le_bytes());
        for value in state.position.iter().chain(state.velocity.iter()).chain(state.acceleration.iter()) {
            hasher.update(value.to_le_bytes());
        }
    }
    let digest = hasher.finalize();
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    Ok(output)
}

/// Build a fixed deterministic input set from stable IDs.
pub fn initial_aos(count: usize) -> AosLayout {
    AosLayout::from_count(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEPS: usize = 120;
    const DT: f64 = 1.0 / 120.0;

    fn equivalent_layouts(count: usize) -> (AosLayout, SoaLayout, AosoaLayout<4>, AosoaLayout<8>, AosoaLayout<16>) {
        let aos = initial_aos(count);
        let soa = SoaLayout::from_aos(&aos);
        let aosoa4 = AosoaLayout::from_aos(&aos).expect("lanes 4");
        let aosoa8 = AosoaLayout::from_aos(&aos).expect("lanes 8");
        let aosoa16 = AosoaLayout::from_aos(&aos).expect("lanes 16");
        (aos, soa, aosoa4, aosoa8, aosoa16)
    }

    #[test]
    fn all_layouts_produce_bit_identical_results() {
        let (mut aos, mut soa, mut aosoa4, mut aosoa8, mut aosoa16) = equivalent_layouts(1031);
        aos.step_n(STEPS, DT).expect("AoS");
        soa.step_n(STEPS, DT).expect("SoA");
        aosoa4.step_n(STEPS, DT).expect("AoSoA4");
        aosoa8.step_n(STEPS, DT).expect("AoSoA8");
        aosoa16.step_n(STEPS, DT).expect("AoSoA16");
        let expected = aos.digest(1031).expect("AoS digest");
        assert_eq!(soa.digest(1031).expect("SoA digest"), expected);
        assert_eq!(aosoa4.digest(1031).expect("AoSoA4 digest"), expected);
        assert_eq!(aosoa8.digest(1031).expect("AoSoA8 digest"), expected);
        assert_eq!(aosoa16.digest(1031).expect("AoSoA16 digest"), expected);
    }

    #[test]
    fn all_parallel_layouts_match_scalar_output() {
        let (
            mut aos_scalar,
            mut soa_scalar,
            mut aosoa4_scalar,
            mut aosoa8_scalar,
            mut aosoa16_scalar,
        ) = equivalent_layouts(1003);
        aos_scalar.step_n(STEPS, DT).expect("AoS scalar");
        soa_scalar.step_n(STEPS, DT).expect("SoA scalar");
        aosoa4_scalar.step_n(STEPS, DT).expect("AoSoA4 scalar");
        aosoa8_scalar.step_n(STEPS, DT).expect("AoSoA8 scalar");
        aosoa16_scalar.step_n(STEPS, DT).expect("AoSoA16 scalar");
        let expected = aos_scalar.digest(1003).expect("AoS scalar digest");

        let (
            mut aos_parallel,
            mut soa_parallel,
            mut aosoa4_parallel,
            mut aosoa8_parallel,
            mut aosoa16_parallel,
        ) = equivalent_layouts(1003);
        aos_parallel.step_n_parallel(STEPS, DT).expect("AoS parallel");
        soa_parallel.step_n_parallel(STEPS, DT).expect("SoA parallel");
        aosoa4_parallel.step_n_parallel(STEPS, DT).expect("AoSoA4 parallel");
        aosoa8_parallel.step_n_parallel(STEPS, DT).expect("AoSoA8 parallel");
        aosoa16_parallel.step_n_parallel(STEPS, DT).expect("AoSoA16 parallel");

        assert_eq!(soa_scalar.digest(1003).expect("SoA scalar"), expected);
        assert_eq!(aosoa4_scalar.digest(1003).expect("AoSoA4 scalar"), expected);
        assert_eq!(aosoa8_scalar.digest(1003).expect("AoSoA8 scalar"), expected);
        assert_eq!(aosoa16_scalar.digest(1003).expect("AoSoA16 scalar"), expected);
        assert_eq!(aos_parallel.digest(1003).expect("AoS parallel digest"), expected);
        assert_eq!(soa_parallel.digest(1003).expect("SoA parallel digest"), expected);
        assert_eq!(aosoa4_parallel.digest(1003).expect("AoSoA4 parallel digest"), expected);
        assert_eq!(aosoa8_parallel.digest(1003).expect("AoSoA8 parallel digest"), expected);
        assert_eq!(aosoa16_parallel.digest(1003).expect("AoSoA16 parallel digest"), expected);
    }

    #[test]
    fn input_order_does_not_change_canonical_digest() {
        let mut states = initial_aos(257).states;
        states.reverse();
        let reversed = AosLayout::from_states(states);
        let canonical = initial_aos(257);
        assert_eq!(
            reversed.digest(257).expect("reversed digest"),
            canonical.digest(257).expect("canonical digest")
        );
    }

    #[test]
    fn duplicate_and_missing_ids_fail_closed() {
        let mut duplicate = initial_aos(10);
        duplicate.states[1].id = duplicate.states[0].id;
        assert_eq!(
            duplicate.digest(10).unwrap_err(),
            LayoutError::DuplicateEnvironmentId { id: 0 }
        );

        let mut missing = initial_aos(10);
        missing.states.pop();
        assert_eq!(
            missing.digest(10).unwrap_err(),
            LayoutError::MissingEnvironmentId { id: 9 }
        );
    }

    #[test]
    fn soa_mismatched_field_lengths_fail_closed() {
        let mut soa = SoaLayout::from_aos(&initial_aos(8));
        soa.velocity[1].pop();
        assert_eq!(soa.step_n(1, DT), Err(LayoutError::ShapeMismatch));
        assert_eq!(soa.digest(8).unwrap_err(), LayoutError::ShapeMismatch);
    }

    #[test]
    fn unexpected_ids_fail_closed() {
        let mut states = initial_aos(10).states;
        states[0].id = 10;
        let invalid = AosLayout::from_states(states);
        assert_eq!(
            invalid.digest(10).unwrap_err(),
            LayoutError::UnexpectedEnvironmentId {
                id: 10,
                expected_count: 10
            }
        );
    }

    #[test]
    fn non_finite_or_non_positive_dt_fails_closed() {
        let mut states = initial_aos(1);
        assert_eq!(states.step_n(1, 0.0), Err(LayoutError::InvalidTimestep));
        assert_eq!(states.step_n(1, f64::NAN), Err(LayoutError::InvalidTimestep));
        assert_eq!(states.step_n(1, f64::INFINITY), Err(LayoutError::InvalidTimestep));
    }

    #[test]
    fn empty_campaign_is_not_a_valid_comparable_result() {
        let empty = initial_aos(0);
        assert_eq!(empty.digest(0).unwrap_err(), LayoutError::EmptyBatch);
    }

    #[test]
    fn aosoa_rejects_zero_lane_width() {
        let source = initial_aos(10);
        assert_eq!(
            AosoaLayout::<0>::from_aos(&source).unwrap_err(),
            LayoutError::InvalidLaneWidth
        );
    }
}
