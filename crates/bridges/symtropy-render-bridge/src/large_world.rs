// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Precision-aware world positions for planetary and solar-system scale scenes.
//!
//! Canonical positions use an integer coarse-cell address plus normalized f64
//! offsets inside that cell. Camera-relative rendering subtracts cell addresses
//! before converting coordinates to GPU floats.
//!
//! This module is a coordinate primitive, not an ephemeris, frame-transform,
//! planetary-tile, frustum-culling, logarithmic-depth, or Bevy camera system.
//! Frame IDs are opaque: compare two positions only after a verified transform
//! has put them in the same reference frame.

use std::array;
use std::error::Error;
use std::fmt;

/// Fixed coarse-cell width in metres: 2^20 m (1,048,576 m).
pub const WORLD_CELL_SIZE_METERS: f64 = 1_048_576.0;

/// Beyond this cell quotient, one absolute f64 cannot reliably identify a
/// remainder inside a world cell. Use an explicit cell/local representation.
const MAX_ABSOLUTE_NORMALIZATION_CELLS: f64 = 4_503_599_627_370_496.0; // 2^52

/// Opaque, caller-defined coordinate-frame identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CoordinateFrameId(pub u32);

/// Canonical world position.
///
/// The integer cell and local offset jointly define the location. Do not
/// serialize only approximate_absolute_meters for physics state or replay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LargeWorldPosition {
    frame: CoordinateFrameId,
    cell: [i64; 3],
    local_m: [f64; 3],
}

/// Three-part f32 expansion of a camera-relative coordinate for GPU use.
///
/// Approximate represented value is high + middle + low. Shaders should
/// transform these parts separately through the linear view/projection path
/// and combine transformed results. Do not first add the parts in f32.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuRelativePosition {
    pub high: [f32; 3],
    pub middle: [f32; 3],
    pub low: [f32; 3],
}

/// Fail-closed errors for canonical positions and render-space conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LargeWorldError {
    NonFiniteCoordinate,
    AbsoluteCoordinateNeedsCellAddress,
    OffsetNeedsCellAddress,
    CellAddressOverflow,
    InvalidLocalOffset,
    FrameMismatch,
    NonFiniteRelativePosition,
    InvalidRenderErrorBudget,
    RenderPrecisionBudgetExceeded,
}

impl fmt::Display for LargeWorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for LargeWorldError {}

/// Error-free transformation of two finite f64 values into their rounded
/// sum plus a residual, provided the sum itself does not overflow.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let sum = a + b;
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    let b_roundoff = b - b_virtual;
    let a_roundoff = a - a_virtual;
    (sum, a_roundoff + b_roundoff)
}

impl LargeWorldPosition {
    /// Construct from an explicit cell and normalized local offsets.
    ///
    /// This is the preferred constructor for precise astronomical input.
    pub fn from_cell_local(
        frame: CoordinateFrameId,
        cell: [i64; 3],
        local_m: [f64; 3],
    ) -> Result<Self, LargeWorldError> {
        for local in local_m {
            if !local.is_finite() || !(0.0..WORLD_CELL_SIZE_METERS).contains(&local) {
                return Err(LargeWorldError::InvalidLocalOffset);
            }
        }
        Ok(Self {
            frame,
            cell,
            local_m,
        })
    }

    /// Convenience conversion from one already-rounded absolute f64 position.
    ///
    /// This cannot restore low-order information lost before this call. For
    /// very large coordinates, or whenever small local offsets matter, use
    /// from_cell_local with an explicit cell address.
    pub fn from_absolute_meters(
        frame: CoordinateFrameId,
        absolute_m: [f64; 3],
    ) -> Result<Self, LargeWorldError> {
        let mut cells = [0_i64; 3];
        let mut locals = [0.0_f64; 3];

        for axis in 0..3 {
            let value = absolute_m[axis];
            if !value.is_finite() {
                return Err(LargeWorldError::NonFiniteCoordinate);
            }
            let quotient = (value / WORLD_CELL_SIZE_METERS).floor();
            if !quotient.is_finite()
                || quotient <= -MAX_ABSOLUTE_NORMALIZATION_CELLS
                || quotient >= MAX_ABSOLUTE_NORMALIZATION_CELLS
            {
                return Err(LargeWorldError::AbsoluteCoordinateNeedsCellAddress);
            }

            let mut cell = quotient as i64;
            let mut local = value - quotient * WORLD_CELL_SIZE_METERS;
            if local < 0.0 {
                cell = cell
                    .checked_sub(1)
                    .ok_or(LargeWorldError::CellAddressOverflow)?;
                local += WORLD_CELL_SIZE_METERS;
            } else if local >= WORLD_CELL_SIZE_METERS {
                cell = cell
                    .checked_add(1)
                    .ok_or(LargeWorldError::CellAddressOverflow)?;
                local -= WORLD_CELL_SIZE_METERS;
            }
            if !local.is_finite() || !(0.0..WORLD_CELL_SIZE_METERS).contains(&local) {
                return Err(LargeWorldError::InvalidLocalOffset);
            }
            cells[axis] = cell;
            locals[axis] = local;
        }

        Self::from_cell_local(frame, cells, locals)
    }

    pub fn frame(&self) -> CoordinateFrameId {
        self.frame
    }

    pub fn cell(&self) -> [i64; 3] {
        self.cell
    }

    pub fn local_meters(&self) -> [f64; 3] {
        self.local_m
    }

    /// Shift by metres without adding a large displacement directly to the
    /// current local offset. Use explicit cell deltas for larger movements than
    /// one f64 can decompose with a reliable within-cell remainder.
    pub fn offset_by_meters(&self, delta_m: [f64; 3]) -> Result<Self, LargeWorldError> {
        let mut next_cell = self.cell;
        let mut next_local = self.local_m;

        for axis in 0..3 {
            let delta = delta_m[axis];
            if !delta.is_finite() {
                return Err(LargeWorldError::NonFiniteCoordinate);
            }
            let quotient = (delta / WORLD_CELL_SIZE_METERS).floor();
            if !quotient.is_finite()
                || quotient <= -MAX_ABSOLUTE_NORMALIZATION_CELLS
                || quotient >= MAX_ABSOLUTE_NORMALIZATION_CELLS
            {
                return Err(LargeWorldError::OffsetNeedsCellAddress);
            }

            let mut cell_delta = quotient as i128;
            let mut remainder = delta - quotient * WORLD_CELL_SIZE_METERS;
            if remainder < 0.0 {
                cell_delta -= 1;
                remainder += WORLD_CELL_SIZE_METERS;
            } else if remainder >= WORLD_CELL_SIZE_METERS {
                cell_delta += 1;
                remainder -= WORLD_CELL_SIZE_METERS;
            }

            let local_sum = next_local[axis] + remainder;
            let carry = if local_sum >= WORLD_CELL_SIZE_METERS {
                next_local[axis] = local_sum - WORLD_CELL_SIZE_METERS;
                1_i128
            } else {
                next_local[axis] = local_sum;
                0_i128
            };

            let cell_sum = i128::from(next_cell[axis]) + cell_delta + carry;
            next_cell[axis] =
                i64::try_from(cell_sum).map_err(|_| LargeWorldError::CellAddressOverflow)?;
        }

        Self::from_cell_local(self.frame, next_cell, next_local)
    }

    /// Camera-relative metre coordinates, computed from the integer cell delta.
    pub fn relative_to_meters(&self, origin: &Self) -> Result<[f64; 3], LargeWorldError> {
        self.ensure_same_frame(origin)?;
        let mut relative = [0.0; 3];

        for (axis, relative_axis) in relative.iter_mut().enumerate() {
            let cell_delta = i128::from(self.cell[axis]) - i128::from(origin.cell[axis]);
            let coarse = cell_delta as f64 * WORLD_CELL_SIZE_METERS;
            let value = coarse + (self.local_m[axis] - origin.local_m[axis]);
            if !value.is_finite() {
                return Err(LargeWorldError::NonFiniteRelativePosition);
            }
            *relative_axis = value;
        }
        Ok(relative)
    }

    /// Split a camera-relative coordinate into high/middle/low GPU components.
    ///
    /// The cell-distance term is split first, then the local difference is
    /// combined with its residual. This keeps sub-cell detail from being
    /// rounded away when the camera is thousands of millions of metres away.
    pub fn relative_to_gpu_split(
        &self,
        origin: &Self,
    ) -> Result<GpuRelativePosition, LargeWorldError> {
        self.ensure_same_frame(origin)?;
        let mut high = [0.0_f32; 3];
        let mut middle = [0.0_f32; 3];
        let mut low = [0.0_f32; 3];

        for axis in 0..3 {
            let cell_delta = i128::from(self.cell[axis]) - i128::from(origin.cell[axis]);
            let coarse = cell_delta as f64 * WORLD_CELL_SIZE_METERS;
            let high_part = coarse as f32;
            if !high_part.is_finite() {
                return Err(LargeWorldError::NonFiniteRelativePosition);
            }

            let residual =
                (coarse - f64::from(high_part)) + (self.local_m[axis] - origin.local_m[axis]);
            let middle_part = residual as f32;
            let low_part = (residual - f64::from(middle_part)) as f32;
            if !residual.is_finite() || !middle_part.is_finite() || !low_part.is_finite() {
                return Err(LargeWorldError::NonFiniteRelativePosition);
            }

            high[axis] = high_part;
            middle[axis] = middle_part;
            low[axis] = low_part;
        }

        Ok(GpuRelativePosition { high, middle, low })
    }

    /// Convert to f32 only when its per-axis round-trip error meets the declared
    /// metre budget. This is intended for near-field transforms, not AU-scale
    /// coordinates.
    pub fn relative_to_f32_with_error_budget(
        &self,
        origin: &Self,
        max_abs_error_m: f64,
    ) -> Result<[f32; 3], LargeWorldError> {
        if !max_abs_error_m.is_finite() || max_abs_error_m < 0.0 {
            return Err(LargeWorldError::InvalidRenderErrorBudget);
        }

        self.ensure_same_frame(origin)?;
        let mut output = [0.0_f32; 3];
        const MAX_EXACT_F64_INTEGER: u128 = 9_007_199_254_740_992;

        for (((self_cell, origin_cell), (self_local, origin_local)), output_axis) in self
            .cell
            .iter()
            .zip(origin.cell.iter())
            .zip(self.local_m.iter().zip(origin.local_m.iter()))
            .zip(output.iter_mut())
        {
            let cell_delta = i128::from(*self_cell) - i128::from(*origin_cell);
            if cell_delta.unsigned_abs() > MAX_EXACT_F64_INTEGER {
                return Err(LargeWorldError::RenderPrecisionBudgetExceeded);
            }

            let coarse = cell_delta as f64 * WORLD_CELL_SIZE_METERS;
            let local_delta = *self_local - *origin_local;
            let (relative_sum, residual) = two_sum(coarse, local_delta);
            let rounded = relative_sum as f32;
            let round_trip_error = (f64::from(rounded) - relative_sum) - residual;

            if !rounded.is_finite() || round_trip_error.abs() > max_abs_error_m {
                return Err(LargeWorldError::RenderPrecisionBudgetExceeded);
            }
            *output_axis = rounded;
        }
        Ok(output)
    }

    /// Approximate absolute metres for display/debug output only.
    pub fn approximate_absolute_meters(&self) -> [f64; 3] {
        array::from_fn(|axis| {
            (self.cell[axis] as f64) * WORLD_CELL_SIZE_METERS + self.local_m[axis]
        })
    }

    fn ensure_same_frame(&self, other: &Self) -> Result<(), LargeWorldError> {
        if self.frame != other.frame {
            return Err(LargeWorldError::FrameMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: CoordinateFrameId = CoordinateFrameId(1);

    fn pos(cell: [i64; 3], local_m: [f64; 3]) -> LargeWorldPosition {
        LargeWorldPosition::from_cell_local(FRAME, cell, local_m).expect("valid cell/local")
    }

    fn reconstruct(split: GpuRelativePosition, axis: usize) -> f64 {
        f64::from(split.high[axis]) + f64::from(split.middle[axis]) + f64::from(split.low[axis])
    }

    #[test]
    fn rejects_invalid_local_offsets() {
        assert_eq!(
            LargeWorldPosition::from_cell_local(FRAME, [0; 3], [-0.1, 0.0, 0.0]),
            Err(LargeWorldError::InvalidLocalOffset)
        );
        assert_eq!(
            LargeWorldPosition::from_cell_local(FRAME, [0; 3], [0.0, WORLD_CELL_SIZE_METERS, 0.0]),
            Err(LargeWorldError::InvalidLocalOffset)
        );
        assert_eq!(
            LargeWorldPosition::from_cell_local(FRAME, [0; 3], [f64::NAN, 0.0, 0.0]),
            Err(LargeWorldError::InvalidLocalOffset)
        );
    }

    #[test]
    fn normalizes_negative_and_positive_absolute_coordinates() {
        let value = LargeWorldPosition::from_absolute_meters(
            FRAME,
            [-0.25, -WORLD_CELL_SIZE_METERS, WORLD_CELL_SIZE_METERS + 0.5],
        )
        .expect("moderate absolute coordinates normalize");
        assert_eq!(value.cell(), [-1, -1, 1]);
        assert!((value.local_meters()[0] - (WORLD_CELL_SIZE_METERS - 0.25)).abs() < 1e-9);
        assert_eq!(value.local_meters()[1], 0.0);
        assert!((value.local_meters()[2] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn camera_relative_coordinates_subtract_cells_before_local_offsets() {
        let object = pos([142_665, -4, 8], [0.125, 90.0, 12.0]);
        let origin = pos([142_665, -4, 8], [0.120, 91.5, 10.0]);
        let relative = object.relative_to_meters(&origin).expect("same frame");
        assert!((relative[0] - 0.005).abs() < 1e-9);
        assert!((relative[1] + 1.5).abs() < 1e-9);
        assert!((relative[2] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn preserves_small_local_offset_when_adding_au_scale_displacement() {
        let origin = pos([0; 3], [0.001, 0.002, 0.003]);
        let moved = origin
            .offset_by_meters([149_597_870_700.0, 0.0, 0.0])
            .expect("one astronomical unit fits the cell address");
        assert_eq!(moved.local_meters()[1], origin.local_meters()[1]);
        assert_eq!(moved.local_meters()[2], origin.local_meters()[2]);
        let relative = moved.relative_to_meters(&origin).expect("same frame");
        assert!((relative[0] - 149_597_870_700.0).abs() < 0.001);
        assert_eq!(relative[1], 0.0);
        assert_eq!(relative[2], 0.0);
    }

    #[test]
    fn gpu_split_retains_more_precision_at_au_scale_than_single_f32() {
        let origin = pos([0, 0, 0], [700_000.0, 400_000.0, 200_000.0]);
        let object = pos([142_666, 0, 0], [700_000.001, 400_000.0, 200_000.0]);
        let relative = object.relative_to_meters(&origin).expect("same frame");
        let split = object.relative_to_gpu_split(&origin).expect("GPU split");

        for (axis, relative_axis) in relative.iter().enumerate() {
            let error = (reconstruct(split, axis) - relative_axis).abs();
            assert!(error < 0.02, "axis {axis} error was {error} m");
        }
        let plain_error = (f64::from(relative[0] as f32) - relative[0]).abs();
        let split_error = (reconstruct(split, 0) - relative[0]).abs();
        assert!(split_error < plain_error);
    }

    #[test]
    fn refuses_f32_conversion_when_precision_budget_is_exceeded() {
        let origin = pos([0; 3], [0.0; 3]);
        // Half a cell is exactly f32-representable here; add 123 m so the
        // expected rejection measures real precision loss against the 1 m budget.
        let one_au = pos(
            [142_665, 0, 0],
            [WORLD_CELL_SIZE_METERS * 0.5 + 123.0, 0.0, 0.0],
        );
        assert_eq!(
            one_au.relative_to_f32_with_error_budget(&origin, 1.0),
            Err(LargeWorldError::RenderPrecisionBudgetExceeded)
        );
        assert_eq!(
            one_au.relative_to_f32_with_error_budget(&origin, f64::NAN),
            Err(LargeWorldError::InvalidRenderErrorBudget)
        );
    }

    #[test]
    fn f32_error_budget_includes_small_offsets_lost_by_f64_addition() {
        let origin = pos([0; 3], [0.0; 3]);
        let target = pos([1_i64 << 30, 0, 0], [0.001, 0.0, 0.0]);

        assert_eq!(
            target.relative_to_f32_with_error_budget(&origin, 0.0005),
            Err(LargeWorldError::RenderPrecisionBudgetExceeded)
        );
        assert!(
            target
                .relative_to_f32_with_error_budget(&origin, 0.002)
                .is_ok()
        );
    }

    #[test]
    fn refuses_comparisons_between_untransformed_reference_frames() {
        let object = pos([1, 0, 0], [0.0; 3]);
        let other = LargeWorldPosition::from_cell_local(CoordinateFrameId(2), [0; 3], [0.0; 3])
            .expect("other frame");
        assert_eq!(
            object.relative_to_meters(&other),
            Err(LargeWorldError::FrameMismatch)
        );
        assert_eq!(
            object.relative_to_gpu_split(&other),
            Err(LargeWorldError::FrameMismatch)
        );
    }

    #[test]
    fn explicit_cell_addresses_cover_solar_system_scale() {
        let sun = pos([0, 0, 0], [0.0; 3]);
        let neptune = pos([4_284_000, 0, 0], [524_288.0, 0.0, 0.0]);
        let relative = neptune.relative_to_meters(&sun).expect("same frame");
        assert!(relative[0] > 4.4e12);
        assert!(relative[0] < 4.6e12);
        assert_eq!(relative[1], 0.0);
        assert_eq!(relative[2], 0.0);
    }

    #[test]
    fn rejects_absolute_floats_that_cannot_recover_cell_local_parts() {
        assert_eq!(
            LargeWorldPosition::from_absolute_meters(FRAME, [1.0e23, 0.0, 0.0]),
            Err(LargeWorldError::AbsoluteCoordinateNeedsCellAddress)
        );
        assert_eq!(
            LargeWorldPosition::from_absolute_meters(FRAME, [f64::INFINITY, 0.0, 0.0]),
            Err(LargeWorldError::NonFiniteCoordinate)
        );
    }

    #[test]
    fn cell_address_overflow_fails_closed() {
        let far_edge = pos([i64::MAX, 0, 0], [WORLD_CELL_SIZE_METERS - 0.25, 0.0, 0.0]);
        assert_eq!(
            far_edge.offset_by_meters([1.0, 0.0, 0.0]),
            Err(LargeWorldError::CellAddressOverflow)
        );
    }
}
