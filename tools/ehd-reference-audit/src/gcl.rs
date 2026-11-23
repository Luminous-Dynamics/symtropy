// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reference geometric-conservation-law (GCL) oracle for linearly moving
//! axis-aligned cells.
//!
//! This module is deliberately narrower than a production ALE solver. It
//! computes the time-integrated grid-volume flux through the faces of a
//! moving axis-aligned box and verifies
//!
//!     V(t1) - V(t0) = integral[t0,t1] integral[boundary] w · n dA dt.
//!
//! Face areas are products of linearly varying widths. Their time integrals
//! are evaluated exactly by integrating the resulting polynomial, rather
//! than by time stepping or numerical quadrature. The result is an analytic
//! manufactured-solution oracle for GCL adapters; it is not a general moving
//! mesh, curved-cell, polyhedral, or fluid solver.

use std::error::Error;
use std::fmt;

/// An axis-aligned spatial cell whose lower and upper bounds move linearly
/// from the start state to the end state over duration_seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovingAabb<const N: usize> {
    pub id: u64,
    pub start_lower: [f64; N],
    pub start_upper: [f64; N],
    pub end_lower: [f64; N],
    pub end_upper: [f64; N],
    pub duration_seconds: f64,
}

/// Absolute-plus-relative tolerance for the integrated GCL volume residual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GclTolerance {
    pub absolute_volume: f64,
    pub relative: f64,
}

/// Auditable integrated GCL result. Face fluxes use the outward-normal sign
/// convention: lower faces have negative coordinate normals, upper faces
/// have positive coordinate normals. Each flux has units of spatial volume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GclReport<const N: usize> {
    pub cell_id: u64,
    pub duration_seconds: f64,
    pub initial_volume: f64,
    pub final_volume: f64,
    pub volume_change: f64,
    pub lower_face_flux: [f64; N],
    pub upper_face_flux: [f64; N],
    pub integrated_grid_flux: f64,
    /// Integrated grid flux minus the observed cell-volume change.
    pub residual: f64,
    pub allowed_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GclError {
    ZeroDimensions,
    InvalidDuration,
    InvalidTolerance,
    InvalidBounds(u64),
    NonFiniteVolume(u64),
    NonFiniteFaceIntegral {
        cell_id: u64,
        axis: usize,
    },
    NonFiniteFlux(u64),
    ResidualExceeded {
        cell_id: u64,
        residual: f64,
        allowed: f64,
    },
    InvalidUniformState,
}

impl fmt::Display for GclError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimensions => {
                write!(f, "moving cell must have at least one spatial dimension")
            }
            Self::InvalidDuration => write!(f, "moving-cell duration must be finite and positive"),
            Self::InvalidTolerance => write!(f, "GCL tolerances must be finite and non-negative"),
            Self::InvalidBounds(id) => {
                write!(f, "moving cell {id} has invalid or non-finite bounds")
            }
            Self::NonFiniteVolume(id) => write!(
                f,
                "moving cell {id} has a non-finite or non-positive volume"
            ),
            Self::NonFiniteFaceIntegral { cell_id, axis } => {
                write!(
                    f,
                    "moving cell {cell_id} has a non-finite face integral on axis {axis}"
                )
            }
            Self::NonFiniteFlux(id) => {
                write!(f, "moving cell {id} has a non-finite integrated grid flux")
            }
            Self::ResidualExceeded {
                cell_id,
                residual,
                allowed,
            } => write!(
                f,
                "moving cell {cell_id} GCL residual {residual} exceeds allowed {allowed}"
            ),
            Self::InvalidUniformState => write!(
                f,
                "uniform state and GCL volumes must be finite; final volume must be positive"
            ),
        }
    }
}

impl Error for GclError {}

fn validate_cell<const N: usize>(cell: &MovingAabb<N>) -> Result<(f64, f64), GclError> {
    if N == 0 {
        return Err(GclError::ZeroDimensions);
    }
    if !cell.duration_seconds.is_finite() || cell.duration_seconds <= 0.0 {
        return Err(GclError::InvalidDuration);
    }

    let mut initial_volume = 1.0;
    let mut final_volume = 1.0;
    for axis in 0..N {
        let sl = cell.start_lower[axis];
        let su = cell.start_upper[axis];
        let el = cell.end_lower[axis];
        let eu = cell.end_upper[axis];

        if !sl.is_finite()
            || !su.is_finite()
            || !el.is_finite()
            || !eu.is_finite()
            || su <= sl
            || eu <= el
        {
            return Err(GclError::InvalidBounds(cell.id));
        }

        initial_volume *= su - sl;
        final_volume *= eu - el;
        if !initial_volume.is_finite()
            || !final_volume.is_finite()
            || initial_volume <= 0.0
            || final_volume <= 0.0
        {
            return Err(GclError::NonFiniteVolume(cell.id));
        }
    }
    Ok((initial_volume, final_volume))
}

/// Exactly integrate the product of all linearly varying cell widths except
/// the width normal to one face. The integrand is a polynomial of degree
/// at most N-1 on normalized time tau in [0, 1].
fn integrated_cross_section<const N: usize>(
    cell: &MovingAabb<N>,
    normal_axis: usize,
) -> Result<f64, GclError> {
    let mut coefficients = [0.0_f64; N];
    coefficients[0] = 1.0;
    let mut degree = 0_usize;

    for axis in 0..N {
        if axis == normal_axis {
            continue;
        }

        let start_width = cell.start_upper[axis] - cell.start_lower[axis];
        let end_width = cell.end_upper[axis] - cell.end_lower[axis];
        let width_delta = end_width - start_width;
        let mut next = [0.0_f64; N];

        for power in 0..=degree {
            next[power] += coefficients[power] * start_width;
            next[power + 1] += coefficients[power] * width_delta;
        }

        if !next.iter().all(|coefficient| coefficient.is_finite()) {
            return Err(GclError::NonFiniteFaceIntegral {
                cell_id: cell.id,
                axis: normal_axis,
            });
        }
        coefficients = next;
        degree += 1;
    }

    // Integral_0^1 sum_k c_k tau^k d tau = sum_k c_k/(k+1).
    let mut integral = 0.0;
    for power in 0..=degree {
        integral += coefficients[power] / (power as f64 + 1.0);
    }
    if !integral.is_finite() || integral <= 0.0 {
        return Err(GclError::NonFiniteFaceIntegral {
            cell_id: cell.id,
            axis: normal_axis,
        });
    }
    Ok(integral)
}

/// Audit the discrete geometric conservation identity for a linearly moving
/// axis-aligned cell. The time-integrated face flux is exact for this motion
/// model, since the face-area polynomial is integrated analytically.
pub fn audit_moving_aabb<const N: usize>(
    cell: &MovingAabb<N>,
    tolerance: GclTolerance,
) -> Result<GclReport<N>, GclError> {
    if !tolerance.absolute_volume.is_finite()
        || !tolerance.relative.is_finite()
        || tolerance.absolute_volume < 0.0
        || tolerance.relative < 0.0
    {
        return Err(GclError::InvalidTolerance);
    }

    let (initial_volume, final_volume) = validate_cell(cell)?;
    let volume_change = final_volume - initial_volume;
    let allowed_residual = tolerance.absolute_volume
        + tolerance.relative * initial_volume.abs().max(final_volume.abs());
    if !allowed_residual.is_finite() {
        return Err(GclError::InvalidTolerance);
    }

    let mut lower_face_flux = [0.0_f64; N];
    let mut upper_face_flux = [0.0_f64; N];
    let mut integrated_grid_flux = 0.0_f64;

    for axis in 0..N {
        let integrated_area = integrated_cross_section(cell, axis)?;
        let lower_displacement = cell.end_lower[axis] - cell.start_lower[axis];
        let upper_displacement = cell.end_upper[axis] - cell.start_upper[axis];

        // w_face * dt is the face displacement. The lower face's outward
        // normal is -e_axis, while the upper face's outward normal is +e_axis.
        lower_face_flux[axis] = -lower_displacement * integrated_area;
        upper_face_flux[axis] = upper_displacement * integrated_area;
        if !lower_face_flux[axis].is_finite() || !upper_face_flux[axis].is_finite() {
            return Err(GclError::NonFiniteFlux(cell.id));
        }
        integrated_grid_flux += lower_face_flux[axis] + upper_face_flux[axis];
    }

    let residual = integrated_grid_flux - volume_change;
    if !integrated_grid_flux.is_finite() || !residual.is_finite() {
        return Err(GclError::NonFiniteFlux(cell.id));
    }
    if residual.abs() > allowed_residual {
        return Err(GclError::ResidualExceeded {
            cell_id: cell.id,
            residual,
            allowed: allowed_residual,
        });
    }

    Ok(GclReport {
        cell_id: cell.id,
        duration_seconds: cell.duration_seconds,
        initial_volume,
        final_volume,
        volume_change,
        lower_face_flux,
        upper_face_flux,
        integrated_grid_flux,
        residual,
        allowed_residual,
    })
}

/// ALE free-stream manufactured update for a uniform scalar q with no
/// physical flux/source: q1*V1 = q0*(V0 + integrated_grid_flux).
/// A valid GCL report therefore leaves q unchanged up to roundoff.
pub fn uniform_state_after_mesh_motion<const N: usize>(
    initial_value: f64,
    report: &GclReport<N>,
) -> Result<f64, GclError> {
    if !initial_value.is_finite()
        || !report.initial_volume.is_finite()
        || !report.final_volume.is_finite()
        || report.final_volume <= 0.0
        || !report.integrated_grid_flux.is_finite()
    {
        return Err(GclError::InvalidUniformState);
    }

    let value =
        initial_value * (report.initial_volume + report.integrated_grid_flux) / report.final_volume;
    if !value.is_finite() {
        return Err(GclError::InvalidUniformState);
    }
    Ok(value)
}

/// Small executable manufactured-solution check used by the offline audit
/// CLI. It validates a 3D linearly moving box and ALE free-stream preservation;
/// it is not a moving-mesh fluid simulation.
pub fn reference_self_check() -> Result<(), GclError> {
    let cell = MovingAabb {
        id: 7,
        start_lower: [0.0, 0.0, 0.0],
        start_upper: [2.0, 3.0, 4.0],
        end_lower: [0.5, -0.25, 1.0],
        end_upper: [3.5, 5.75, 7.0],
        duration_seconds: 0.25,
    };
    let report = audit_moving_aabb(
        &cell,
        GclTolerance {
            absolute_volume: 1.0e-12,
            relative: 1.0e-12,
        },
    )?;
    let before = 1.25;
    let after = uniform_state_after_mesh_motion(before, &report)?;
    if (after - before).abs() > 1.0e-11 {
        return Err(GclError::ResidualExceeded {
            cell_id: cell.id,
            residual: after - before,
            allowed: 1.0e-11,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance() -> GclTolerance {
        GclTolerance {
            absolute_volume: 1.0e-12,
            relative: 1.0e-12,
        }
    }

    #[test]
    fn static_mesh_has_zero_grid_flux() {
        let cell = MovingAabb {
            id: 1,
            start_lower: [0.0, -1.0, 2.0],
            start_upper: [2.0, 3.0, 5.0],
            end_lower: [0.0, -1.0, 2.0],
            end_upper: [2.0, 3.0, 5.0],
            duration_seconds: 1.0,
        };
        let report = audit_moving_aabb(&cell, tolerance()).unwrap();
        assert_eq!(report.volume_change, 0.0);
        assert_eq!(report.integrated_grid_flux, 0.0);
        assert_eq!(report.residual, 0.0);
    }

    #[test]
    fn one_dimensional_translation_and_expansion_satisfy_gcl() {
        let cell = MovingAabb {
            id: 2,
            start_lower: [0.0],
            start_upper: [2.0],
            end_lower: [1.0],
            end_upper: [4.0],
            duration_seconds: 0.5,
        };
        let report = audit_moving_aabb(&cell, tolerance()).unwrap();
        assert_eq!(report.initial_volume, 2.0);
        assert_eq!(report.final_volume, 3.0);
        assert_eq!(report.lower_face_flux, [-1.0]);
        assert_eq!(report.upper_face_flux, [2.0]);
        assert_eq!(report.integrated_grid_flux, 1.0);
    }

    #[test]
    fn two_dimensional_translation_and_anisotropic_expansion_satisfy_gcl() {
        let cell = MovingAabb {
            id: 3,
            start_lower: [0.0, 0.0],
            start_upper: [2.0, 3.0],
            end_lower: [1.0, -1.0],
            end_upper: [4.0, 5.0],
            duration_seconds: 2.0,
        };
        let report = audit_moving_aabb(&cell, tolerance()).unwrap();
        assert_eq!(report.initial_volume, 6.0);
        assert_eq!(report.final_volume, 18.0);
        assert!((report.integrated_grid_flux - 12.0).abs() < 1.0e-12);
        assert!(report.residual.abs() < 1.0e-12);
    }

    #[test]
    fn three_dimensional_anisotropic_motion_satisfies_gcl() {
        let cell = MovingAabb {
            id: 4,
            start_lower: [-1.0, 2.0, 0.0],
            start_upper: [1.0, 5.0, 4.0],
            end_lower: [-0.5, 1.0, 1.0],
            end_upper: [2.5, 7.0, 7.0],
            duration_seconds: 0.125,
        };
        let report = audit_moving_aabb(&cell, tolerance()).unwrap();
        assert!((report.integrated_grid_flux - report.volume_change).abs() < 1.0e-11);
    }

    #[test]
    fn reversing_mesh_motion_reverses_integrated_grid_flux() {
        let forward = MovingAabb {
            id: 5,
            start_lower: [0.0, 0.0],
            start_upper: [2.0, 3.0],
            end_lower: [0.5, -1.0],
            end_upper: [3.5, 5.0],
            duration_seconds: 1.0,
        };
        let reverse = MovingAabb {
            id: 5,
            start_lower: forward.end_lower,
            start_upper: forward.end_upper,
            end_lower: forward.start_lower,
            end_upper: forward.start_upper,
            duration_seconds: 1.0,
        };
        let f = audit_moving_aabb(&forward, tolerance()).unwrap();
        let r = audit_moving_aabb(&reverse, tolerance()).unwrap();
        assert!((f.integrated_grid_flux + r.integrated_grid_flux).abs() < 1.0e-11);
    }

    #[test]
    fn independent_temporal_subdivision_preserves_total_swept_measure() {
        let whole = MovingAabb {
            id: 6,
            start_lower: [0.0, 0.0, 0.0],
            start_upper: [2.0, 3.0, 4.0],
            end_lower: [0.5, -0.25, 1.0],
            end_upper: [3.5, 5.75, 7.0],
            duration_seconds: 1.0,
        };
        let fraction = 0.37;
        let blend = |a: f64, b: f64| a + (b - a) * fraction;
        let middle_lower =
            std::array::from_fn(|axis| blend(whole.start_lower[axis], whole.end_lower[axis]));
        let middle_upper =
            std::array::from_fn(|axis| blend(whole.start_upper[axis], whole.end_upper[axis]));

        let first = MovingAabb {
            id: whole.id,
            start_lower: whole.start_lower,
            start_upper: whole.start_upper,
            end_lower: middle_lower,
            end_upper: middle_upper,
            duration_seconds: whole.duration_seconds * fraction,
        };
        let second = MovingAabb {
            id: whole.id,
            start_lower: middle_lower,
            start_upper: middle_upper,
            end_lower: whole.end_lower,
            end_upper: whole.end_upper,
            duration_seconds: whole.duration_seconds * (1.0 - fraction),
        };

        let whole_report = audit_moving_aabb(&whole, tolerance()).unwrap();
        let first_report = audit_moving_aabb(&first, tolerance()).unwrap();
        let second_report = audit_moving_aabb(&second, tolerance()).unwrap();
        assert!(
            (whole_report.integrated_grid_flux
                - first_report.integrated_grid_flux
                - second_report.integrated_grid_flux)
                .abs()
                < 1.0e-10
        );
    }

    #[test]
    fn uniform_scalar_free_stream_is_preserved() {
        let cell = MovingAabb {
            id: 8,
            start_lower: [0.0, 0.0, 0.0],
            start_upper: [1.0, 2.0, 3.0],
            end_lower: [-0.2, 0.5, 0.25],
            end_upper: [2.8, 4.5, 5.25],
            duration_seconds: 0.75,
        };
        let report = audit_moving_aabb(&cell, tolerance()).unwrap();
        let updated = uniform_state_after_mesh_motion(3.125, &report).unwrap();
        assert!((updated - 3.125).abs() < 1.0e-11);
    }

    #[test]
    fn rejects_non_positive_or_non_finite_duration() {
        let mut cell = MovingAabb {
            id: 9,
            start_lower: [0.0],
            start_upper: [1.0],
            end_lower: [0.0],
            end_upper: [2.0],
            duration_seconds: 0.0,
        };
        assert_eq!(
            audit_moving_aabb(&cell, tolerance()).unwrap_err(),
            GclError::InvalidDuration
        );
        cell.duration_seconds = f64::INFINITY;
        assert_eq!(
            audit_moving_aabb(&cell, tolerance()).unwrap_err(),
            GclError::InvalidDuration
        );
    }

    #[test]
    fn rejects_collapsed_or_non_finite_cell_bounds() {
        let collapsed = MovingAabb {
            id: 10,
            start_lower: [0.0, 0.0],
            start_upper: [1.0, 1.0],
            end_lower: [0.0, 0.0],
            end_upper: [1.0, 0.0],
            duration_seconds: 1.0,
        };
        assert_eq!(
            audit_moving_aabb(&collapsed, tolerance()).unwrap_err(),
            GclError::InvalidBounds(10)
        );
        let non_finite = MovingAabb {
            id: 11,
            start_lower: [0.0],
            start_upper: [1.0],
            end_lower: [0.0],
            end_upper: [f64::INFINITY],
            duration_seconds: 1.0,
        };
        assert_eq!(
            audit_moving_aabb(&non_finite, tolerance()).unwrap_err(),
            GclError::InvalidBounds(11)
        );
    }

    #[test]
    fn rejects_invalid_tolerances() {
        let cell = MovingAabb {
            id: 12,
            start_lower: [0.0],
            start_upper: [1.0],
            end_lower: [0.0],
            end_upper: [2.0],
            duration_seconds: 1.0,
        };
        let invalid = GclTolerance {
            absolute_volume: -1.0,
            relative: 0.0,
        };
        assert_eq!(
            audit_moving_aabb(&cell, invalid).unwrap_err(),
            GclError::InvalidTolerance
        );
    }

    #[test]
    fn reference_self_check_passes() {
        reference_self_check().unwrap();
    }
}
