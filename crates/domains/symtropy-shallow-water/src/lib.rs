// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small, deterministic 1D shallow-water reference kernel.
//!
//! This is a first-order finite-volume method for the inviscid 1D Saint-Venant
//! equations over a fixed, piecewise-constant bed. It uses local Lax-Friedrichs
//! (Rusanov) fluxes with hydrostatic reconstruction and side-specific pressure
//! corrections. It is intended for numerical qualification, not operational
//! flood prediction. Rainfall, friction, sediment, moving beds and 2D flow are
//! deliberately out of scope.

#![forbid(unsafe_code)]

/// One finite-volume cell. Discharge is q = h*u per unit channel width.
///
/// Units: depth and bed elevation in metres; discharge in m²/s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellState {
    pub depth_m: f64,
    pub discharge_m2_s: f64,
    pub bed_elevation_m: f64,
}

impl CellState {
    pub fn still_water(depth_m: f64, bed_elevation_m: f64) -> Self {
        Self {
            depth_m,
            discharge_m2_s: 0.0,
            bed_elevation_m,
        }
    }

    pub fn velocity_m_s(self, dry_depth_m: f64) -> f64 {
        if self.depth_m > dry_depth_m {
            self.discharge_m2_s / self.depth_m
        } else {
            0.0
        }
    }

    pub fn surface_elevation_m(self) -> f64 {
        self.bed_elevation_m + self.depth_m
    }
}

/// Numerical parameters for a uniform 1D finite-volume mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolverConfig {
    pub cell_width_m: f64,
    pub gravity_m_s2: f64,
    /// Courant number. The first-order Rusanov reference is restricted to (0, 0.5].
    pub cfl: f64,
    /// Depth at or below which velocity is treated as undefined and discharge is zero.
    pub dry_depth_m: f64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            cell_width_m: 1.0,
            gravity_m_s2: 9.81,
            cfl: 0.45,
            dry_depth_m: 1.0e-9,
        }
    }
}

impl SolverConfig {
    pub fn validate(self) -> Result<(), SolverError> {
        if !self.cell_width_m.is_finite() || self.cell_width_m <= 0.0 {
            return Err(SolverError::InvalidCellWidth);
        }
        if !self.gravity_m_s2.is_finite() || self.gravity_m_s2 <= 0.0 {
            return Err(SolverError::InvalidGravity);
        }
        if !self.cfl.is_finite() || self.cfl <= 0.0 || self.cfl > 0.5 {
            return Err(SolverError::InvalidCfl);
        }
        if !self.dry_depth_m.is_finite() || self.dry_depth_m < 0.0 {
            return Err(SolverError::InvalidDryDepth);
        }
        Ok(())
    }
}

/// Boundary policy applied to ghost cells at each domain edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryCondition {
    /// Zero normal discharge at the edge (reflective wall).
    Reflective,
    /// Copy the edge cell state into the ghost cell (zero-gradient extrapolation).
    Transmissive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundaryConditions {
    pub left: BoundaryCondition,
    pub right: BoundaryCondition,
}

impl BoundaryConditions {
    pub const CLOSED: Self = Self {
        left: BoundaryCondition::Reflective,
        right: BoundaryCondition::Reflective,
    };

    pub const OPEN: Self = Self {
        left: BoundaryCondition::Transmissive,
        right: BoundaryCondition::Transmissive,
    };
}

#[derive(Clone, Debug, PartialEq)]
pub enum SolverError {
    EmptyDomain,
    InvalidCellWidth,
    InvalidGravity,
    InvalidCfl,
    InvalidDryDepth,
    InvalidTimeStep,
    TimeStepExceedsCfl { requested_s: f64, maximum_s: f64 },
    NonFiniteState { cell: usize },
    NegativeDepth { cell: usize },
    DischargeInDryCell { cell: usize },
    NonFiniteUpdate { cell: usize },
    NegativeUpdatedDepth { cell: usize },
    NonFiniteStorage,
    InvalidAnalyticalInput,
}

/// 1D finite-volume state over a uniform cell-centred mesh.
///
/// Bed elevation is fixed during an integration step. All step changes are
/// computed into temporary vectors and committed only after successful checks.
#[derive(Clone, Debug, PartialEq)]
pub struct ShallowWater1D {
    cells: Vec<CellState>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RitterState {
    pub depth_m: f64,
    pub velocity_m_s: f64,
    pub discharge_m2_s: f64,
}

#[derive(Clone, Copy, Debug)]
struct Flux {
    mass: f64,
    momentum: f64,
}

#[derive(Clone, Copy, Debug)]
struct InterfaceFlux {
    /// Flux corrected for the state on the interface's left side.
    left: Flux,
    /// Flux corrected for the state on the interface's right side.
    right: Flux,
}

impl ShallowWater1D {
    pub fn new(cells: Vec<CellState>, config: SolverConfig) -> Result<Self, SolverError> {
        if cells.is_empty() {
            return Err(SolverError::EmptyDomain);
        }
        config.validate()?;
        validate_cells(&cells, config)?;
        Ok(Self { cells })
    }

    pub fn cells(&self) -> &[CellState] {
        &self.cells
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Maximum explicit time step suggested by the current state and CFL rule.
    ///
    /// None denotes a fully dry, motionless state with no propagating wave speed.
    pub fn maximum_stable_timestep_s(
        &self,
        config: SolverConfig,
    ) -> Result<Option<f64>, SolverError> {
        config.validate()?;
        validate_cells(&self.cells, config)?;
        let maximum_speed = self
            .cells
            .iter()
            .map(|cell| wave_speed(*cell, config))
            .fold(0.0_f64, f64::max);
        if maximum_speed <= f64::MIN_POSITIVE {
            return Ok(None);
        }
        let dt = config.cfl * config.cell_width_m / maximum_speed;
        if dt.is_finite() && dt > 0.0 {
            Ok(Some(dt))
        } else {
            Err(SolverError::InvalidTimeStep)
        }
    }

    /// Advance by one SSP-RK2 step, using hydrostatic reconstruction at every face.
    ///
    /// Returns an error, without mutating the state, if the requested step exceeds
    /// the current CFL bound or any intermediate/final state is invalid.
    pub fn step(
        &mut self,
        dt_s: f64,
        config: SolverConfig,
        boundaries: BoundaryConditions,
    ) -> Result<(), SolverError> {
        config.validate()?;
        validate_cells(&self.cells, config)?;
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return Err(SolverError::InvalidTimeStep);
        }
        if let Some(maximum_s) = self.maximum_stable_timestep_s(config)? {
            if dt_s > maximum_s * (1.0 + 1.0e-12) {
                return Err(SolverError::TimeStepExceedsCfl {
                    requested_s: dt_s,
                    maximum_s,
                });
            }
        }

        let initial = self.cells.clone();
        let stage_one = euler_step(&initial, dt_s, config, boundaries)?;
        let stage_two = euler_step(&stage_one, dt_s, config, boundaries)?;

        let mut result = Vec::with_capacity(initial.len());
        for (index, (start, twice_advanced)) in initial.iter().zip(stage_two.iter()).enumerate() {
            let depth_m = 0.5 * start.depth_m + 0.5 * twice_advanced.depth_m;
            let mut discharge_m2_s =
                0.5 * start.discharge_m2_s + 0.5 * twice_advanced.discharge_m2_s;
            if depth_m <= config.dry_depth_m {
                discharge_m2_s = 0.0;
            }
            let cell = CellState {
                depth_m,
                discharge_m2_s,
                bed_elevation_m: start.bed_elevation_m,
            };
            validate_one(cell, index, config)?;
            result.push(cell);
        }

        self.cells = result;
        Ok(())
    }

    /// Advance by at most the requested maximum time step, respecting the current CFL limit.
    ///
    /// Returns the actual time step taken. A fully dry stationary state takes the
    /// requested finite step because its flux operator is zero.
    pub fn advance(
        &mut self,
        requested_max_dt_s: f64,
        config: SolverConfig,
        boundaries: BoundaryConditions,
    ) -> Result<f64, SolverError> {
        if !requested_max_dt_s.is_finite() || requested_max_dt_s <= 0.0 {
            return Err(SolverError::InvalidTimeStep);
        }
        let dt_s = self
            .maximum_stable_timestep_s(config)?
            .map_or(requested_max_dt_s, |stable| requested_max_dt_s.min(stable));
        self.step(dt_s, config, boundaries)?;
        Ok(dt_s)
    }

    /// Water storage per unit channel width (m²), for a uniform cell width.
    pub fn water_storage_per_unit_width_m2(
        &self,
        config: SolverConfig,
    ) -> Result<f64, SolverError> {
        config.validate()?;
        validate_cells(&self.cells, config)?;
        let storage = self.cells.iter().map(|cell| cell.depth_m).sum::<f64>() * config.cell_width_m;
        if storage.is_finite() {
            Ok(storage)
        } else {
            Err(SolverError::NonFiniteStorage)
        }
    }
}

fn validate_cells(cells: &[CellState], config: SolverConfig) -> Result<(), SolverError> {
    if cells.is_empty() {
        return Err(SolverError::EmptyDomain);
    }
    for (index, cell) in cells.iter().copied().enumerate() {
        validate_one(cell, index, config)?;
    }
    Ok(())
}

fn validate_one(cell: CellState, index: usize, config: SolverConfig) -> Result<(), SolverError> {
    if !cell.depth_m.is_finite()
        || !cell.discharge_m2_s.is_finite()
        || !cell.bed_elevation_m.is_finite()
    {
        return Err(SolverError::NonFiniteState { cell: index });
    }
    if cell.depth_m < 0.0 {
        return Err(SolverError::NegativeDepth { cell: index });
    }
    if cell.depth_m <= config.dry_depth_m && cell.discharge_m2_s != 0.0 {
        return Err(SolverError::DischargeInDryCell { cell: index });
    }
    Ok(())
}

fn wave_speed(cell: CellState, config: SolverConfig) -> f64 {
    let velocity = cell.velocity_m_s(config.dry_depth_m);
    velocity.abs() + (config.gravity_m_s2 * cell.depth_m).sqrt()
}

fn euler_step(
    cells: &[CellState],
    dt_s: f64,
    config: SolverConfig,
    boundaries: BoundaryConditions,
) -> Result<Vec<CellState>, SolverError> {
    let interfaces = interface_fluxes(cells, config, boundaries);
    let mut next = Vec::with_capacity(cells.len());

    for index in 0..cells.len() {
        // Each cell uses its own corrected flux at each adjacent interface.
        // The shared mass flux stays conservative; pressure corrections balance
        // the fixed-bed source term in a still-water equilibrium.
        let left_flux = interfaces[index].right;
        let right_flux = interfaces[index + 1].left;
        let depth_m = cells[index].depth_m
            - (dt_s / config.cell_width_m) * (right_flux.mass - left_flux.mass);
        let mut discharge_m2_s = cells[index].discharge_m2_s
            - (dt_s / config.cell_width_m) * (right_flux.momentum - left_flux.momentum);

        if !depth_m.is_finite() || !discharge_m2_s.is_finite() {
            return Err(SolverError::NonFiniteUpdate { cell: index });
        }
        if depth_m < 0.0 {
            return Err(SolverError::NegativeUpdatedDepth { cell: index });
        }
        if depth_m <= config.dry_depth_m {
            discharge_m2_s = 0.0;
        }
        next.push(CellState {
            depth_m,
            discharge_m2_s,
            bed_elevation_m: cells[index].bed_elevation_m,
        });
    }

    validate_cells(&next, config)?;
    Ok(next)
}

fn interface_fluxes(
    cells: &[CellState],
    config: SolverConfig,
    boundaries: BoundaryConditions,
) -> Vec<InterfaceFlux> {
    let mut interfaces = Vec::with_capacity(cells.len() + 1);
    let first = cells[0];
    let last = cells[cells.len() - 1];
    let left_ghost = ghost_cell(first, boundaries.left);
    interfaces.push(hydrostatic_rusanov_flux(left_ghost, first, config));

    for pair in cells.windows(2) {
        interfaces.push(hydrostatic_rusanov_flux(pair[0], pair[1], config));
    }

    let right_ghost = ghost_cell(last, boundaries.right);
    interfaces.push(hydrostatic_rusanov_flux(last, right_ghost, config));
    interfaces
}

fn ghost_cell(edge: CellState, condition: BoundaryCondition) -> CellState {
    match condition {
        BoundaryCondition::Reflective => CellState {
            discharge_m2_s: -edge.discharge_m2_s,
            ..edge
        },
        BoundaryCondition::Transmissive => edge,
    }
}

fn hydrostatic_rusanov_flux(
    left: CellState,
    right: CellState,
    config: SolverConfig,
) -> InterfaceFlux {
    let interface_bed_m = left.bed_elevation_m.max(right.bed_elevation_m);
    let reconstructed_left = reconstruct_to_bed(left, interface_bed_m, config.dry_depth_m);
    let reconstructed_right = reconstruct_to_bed(right, interface_bed_m, config.dry_depth_m);

    let flux_left = physical_flux(reconstructed_left, config);
    let flux_right = physical_flux(reconstructed_right, config);
    let speed = wave_speed(reconstructed_left, config).max(wave_speed(reconstructed_right, config));

    let shared = Flux {
        mass: 0.5 * (flux_left.mass + flux_right.mass)
            - 0.5 * speed * (reconstructed_right.depth_m - reconstructed_left.depth_m),
        momentum: 0.5 * (flux_left.momentum + flux_right.momentum)
            - 0.5
                * speed
                * (reconstructed_right.discharge_m2_s - reconstructed_left.discharge_m2_s),
    };

    let left_correction =
        0.5 * config.gravity_m_s2 * (left.depth_m.powi(2) - reconstructed_left.depth_m.powi(2));
    let right_correction =
        0.5 * config.gravity_m_s2 * (right.depth_m.powi(2) - reconstructed_right.depth_m.powi(2));

    InterfaceFlux {
        left: Flux {
            momentum: shared.momentum + left_correction,
            ..shared
        },
        right: Flux {
            momentum: shared.momentum + right_correction,
            ..shared
        },
    }
}

fn reconstruct_to_bed(cell: CellState, interface_bed_m: f64, dry_depth_m: f64) -> CellState {
    let depth_m = (cell.surface_elevation_m() - interface_bed_m).max(0.0);
    let discharge_m2_s = if cell.depth_m > dry_depth_m {
        cell.discharge_m2_s * (depth_m / cell.depth_m)
    } else {
        0.0
    };
    CellState {
        depth_m,
        discharge_m2_s,
        bed_elevation_m: interface_bed_m,
    }
}

fn physical_flux(cell: CellState, config: SolverConfig) -> Flux {
    let pressure = 0.5 * config.gravity_m_s2 * cell.depth_m.powi(2);
    let advective_momentum = if cell.depth_m > config.dry_depth_m {
        cell.discharge_m2_s * cell.discharge_m2_s / cell.depth_m
    } else {
        0.0
    };
    Flux {
        mass: cell.discharge_m2_s,
        momentum: advective_momentum + pressure,
    }
}

/// Exact Ritter rarefaction solution for an idealized flat-bed dam break.
///
/// Initial data: still water of left_depth_m for x < 0 and dry bed for x > 0.
/// This is an analytical reference for an inviscid semi-infinite reservoir, not
/// a model of a finite dam, roughness, structures or real terrain.
pub fn ritter_dam_break(
    x_m: f64,
    time_s: f64,
    left_depth_m: f64,
    gravity_m_s2: f64,
) -> Result<RitterState, SolverError> {
    if !x_m.is_finite()
        || !time_s.is_finite()
        || !left_depth_m.is_finite()
        || !gravity_m_s2.is_finite()
        || time_s <= 0.0
        || left_depth_m <= 0.0
        || gravity_m_s2 <= 0.0
    {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    let c0 = (gravity_m_s2 * left_depth_m).sqrt();
    let similarity = x_m / time_s;
    let (depth_m, velocity_m_s) = if similarity <= -c0 {
        (left_depth_m, 0.0)
    } else if similarity >= 2.0 * c0 {
        (0.0, 0.0)
    } else {
        let depth = ((2.0 * c0 - similarity).powi(2) / (9.0 * gravity_m_s2)).max(0.0);
        let velocity = (2.0 / 3.0) * (c0 + similarity);
        (depth, velocity)
    };

    Ok(RitterState {
        depth_m,
        velocity_m_s,
        discharge_m2_s: depth_m * velocity_m_s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(dx: f64) -> SolverConfig {
        SolverConfig {
            cell_width_m: dx,
            cfl: 0.45,
            ..SolverConfig::default()
        }
    }

    #[test]
    fn lake_at_rest_over_non_flat_bed_is_well_balanced() {
        let config = config(1.0);
        let cells = (0..80)
            .map(|index| {
                let x = index as f64;
                let bed = 0.4 * (1.0 + (x * 0.17).sin());
                CellState::still_water((2.0 - bed).max(0.0), bed)
            })
            .collect::<Vec<_>>();
        let initial = cells.clone();
        let mut solver = ShallowWater1D::new(cells, config).unwrap();

        for _ in 0..50 {
            solver
                .step(0.01, config, BoundaryConditions::CLOSED)
                .unwrap();
        }

        for (actual, expected) in solver.cells().iter().zip(initial.iter()) {
            assert!((actual.depth_m - expected.depth_m).abs() < 1.0e-12);
            assert!(actual.discharge_m2_s.abs() < 1.0e-12);
        }
    }

    #[test]
    fn fully_dry_bed_stays_dry() {
        let config = config(0.5);
        let cells = (0..32)
            .map(|index| CellState::still_water(0.0, (index as f64 * 0.3).sin()))
            .collect();
        let mut solver = ShallowWater1D::new(cells, config).unwrap();

        for _ in 0..5 {
            solver
                .advance(0.1, config, BoundaryConditions::CLOSED)
                .unwrap();
        }
        assert!(
            solver
                .cells()
                .iter()
                .all(|cell| { cell.depth_m == 0.0 && cell.discharge_m2_s == 0.0 })
        );
    }

    #[test]
    fn reflective_boundaries_conserve_water_during_dam_break() {
        let config = config(0.1);
        let cells = (0..100)
            .map(|index| CellState::still_water(if index < 50 { 1.0 } else { 0.0 }, 0.0))
            .collect();
        let mut solver = ShallowWater1D::new(cells, config).unwrap();
        let initial_storage = solver.water_storage_per_unit_width_m2(config).unwrap();

        for _ in 0..250 {
            solver
                .advance(0.01, config, BoundaryConditions::CLOSED)
                .unwrap();
        }

        let final_storage = solver.water_storage_per_unit_width_m2(config).unwrap();
        assert!((final_storage - initial_storage).abs() < 1.0e-10);
        assert!(solver.cells().iter().all(|cell| {
            cell.depth_m.is_finite() && cell.depth_m >= 0.0 && cell.discharge_m2_s.is_finite()
        }));
    }

    #[test]
    fn ritter_solution_has_correct_tail_origin_and_front() {
        let h0: f64 = 2.0;
        let g: f64 = 9.81;
        let t: f64 = 0.7;
        let c0 = (g * h0).sqrt();

        let tail = ritter_dam_break(-c0 * t, t, h0, g).unwrap();
        assert!((tail.depth_m - h0).abs() < 1.0e-12);
        assert!(tail.velocity_m_s.abs() < 1.0e-12);

        let origin = ritter_dam_break(0.0, t, h0, g).unwrap();
        assert!((origin.depth_m - 4.0 * h0 / 9.0).abs() < 1.0e-12);
        assert!((origin.velocity_m_s - (2.0 / 3.0) * c0).abs() < 1.0e-12);

        let front = ritter_dam_break(2.0 * c0 * t, t, h0, g).unwrap();
        assert_eq!(front.depth_m, 0.0);
        assert_eq!(front.discharge_m2_s, 0.0);
    }

    #[test]
    fn numerical_dam_break_converges_toward_ritter_depth_profile() {
        let dx: f64 = 0.05;
        let cfg = config(dx);
        let x_min = -5.0;
        let x_max = 13.0;
        let t_end = 0.4;
        let h0 = 1.0;
        let count = ((x_max - x_min) / dx).round() as usize;
        let cells = (0..count)
            .map(|index| {
                let x = x_min + (index as f64 + 0.5) * dx;
                CellState::still_water(if x < 0.0 { h0 } else { 0.0 }, 0.0)
            })
            .collect();
        let mut solver = ShallowWater1D::new(cells, cfg).unwrap();
        let mut time = 0.0;
        while time < t_end {
            let dt = solver
                .advance(t_end - time, cfg, BoundaryConditions::OPEN)
                .unwrap();
            time += dt;
            assert!(time <= t_end + 1.0e-12);
        }

        let squared_error = solver
            .cells()
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let x = x_min + (index as f64 + 0.5) * dx;
                let expected = ritter_dam_break(x, t_end, h0, cfg.gravity_m_s2).unwrap();
                (cell.depth_m - expected.depth_m).powi(2)
            })
            .sum::<f64>();
        let rmse = (squared_error / count as f64).sqrt();

        assert!(rmse < 0.06, "dam-break depth RMSE too high: {rmse}");
    }

    #[test]
    fn invalid_inputs_and_unsafe_timestep_fail_closed() {
        let cfg = config(1.0);
        assert_eq!(
            ShallowWater1D::new(vec![], cfg).unwrap_err(),
            SolverError::EmptyDomain
        );
        assert_eq!(
            ShallowWater1D::new(vec![CellState::still_water(-1.0, 0.0)], cfg).unwrap_err(),
            SolverError::NegativeDepth { cell: 0 }
        );
        assert_eq!(
            ShallowWater1D::new(vec![CellState::still_water(f64::NAN, 0.0)], cfg).unwrap_err(),
            SolverError::NonFiniteState { cell: 0 }
        );

        let mut solver =
            ShallowWater1D::new(vec![CellState::still_water(1.0, 0.0); 4], cfg).unwrap();
        let before = solver.clone();
        assert!(matches!(
            solver.step(10.0, cfg, BoundaryConditions::CLOSED),
            Err(SolverError::TimeStepExceedsCfl { .. })
        ));
        assert_eq!(solver, before);

        let invalid_cfg = SolverConfig { cfl: 0.9, ..cfg };
        assert_eq!(invalid_cfg.validate(), Err(SolverError::InvalidCfl));
    }

    #[test]
    fn ritter_rejects_invalid_parameters() {
        assert_eq!(
            ritter_dam_break(0.0, 0.0, 1.0, 9.81),
            Err(SolverError::InvalidAnalyticalInput)
        );
        assert_eq!(
            ritter_dam_break(f64::INFINITY, 1.0, 1.0, 9.81),
            Err(SolverError::InvalidAnalyticalInput)
        );
    }
}
