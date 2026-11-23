// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact polynomial-moment integration over axis-aligned space-time boxes.
//!
//! This is a reference oracle for P0/P1/P2 conservative projection tests. It
//! integrates an explicitly supplied polynomial over source/destination
//! intersections, then checks local destination integrals and the global
//! integral. It does not reconstruct polynomial coefficients from solver
//! samples, build a high-order production remapper, or implement the full
//! transport-polynomial-exactness (TPE) property of an ALE time integrator.

use crate::geometry::{GeometryError, MeasureTolerance, SpaceTimeBox, build_overlap_weights};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// One monomial term: coefficient * product_i(x_i ^ powers_i).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolynomialTerm<const N: usize> {
    pub coefficient: f64,
    pub powers: [u8; N],
}

/// Absolute-plus-relative tolerance for the integrated polynomial quantity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolynomialTolerance {
    pub absolute_integral: f64,
    pub relative: f64,
}

/// Exact-integral reference result for a polynomial distributed over a
/// validated source-to-destination axis-aligned partition.
#[derive(Clone, Debug, PartialEq)]
pub struct PolynomialRemapReport {
    pub source_integral: f64,
    pub destination_integrals: BTreeMap<u64, f64>,
    pub destination_total: f64,
    pub conservation_residual: f64,
    pub allowed_residual: f64,
    pub maximum_total_degree: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PolynomialError {
    ZeroDimensions,
    EmptyPolynomial,
    InvalidBox(u64),
    InvalidTolerance,
    NonFiniteCoefficient,
    DegreeExceeded { degree: u32, maximum: u32 },
    UnsupportedMaximumDegree { requested: u32, supported: u32 },
    NonFiniteIntegral,
    ConservationFailure { residual: f64, allowed: f64 },
    Geometry(GeometryError),
}

impl fmt::Display for PolynomialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimensions => {
                write!(f, "polynomial integration requires at least one dimension")
            }
            Self::EmptyPolynomial => write!(f, "polynomial must contain at least one term"),
            Self::InvalidBox(id) => write!(f, "polynomial integration box {id} has invalid bounds"),
            Self::InvalidTolerance => write!(
                f,
                "polynomial integral tolerances must be finite and non-negative"
            ),
            Self::NonFiniteCoefficient => write!(f, "polynomial contains a non-finite coefficient"),
            Self::DegreeExceeded { degree, maximum } => {
                write!(
                    f,
                    "polynomial term degree {degree} exceeds declared maximum {maximum}"
                )
            }
            Self::UnsupportedMaximumDegree {
                requested,
                supported,
            } => write!(
                f,
                "requested maximum polynomial degree {requested} exceeds supported reference degree {supported}"
            ),
            Self::NonFiniteIntegral => write!(f, "polynomial integral became non-finite"),
            Self::ConservationFailure { residual, allowed } => write!(
                f,
                "polynomial remap residual {residual} exceeds allowed {allowed}"
            ),
            Self::Geometry(error) => write!(f, "space-time partition rejected: {error}"),
        }
    }
}

impl Error for PolynomialError {}

impl From<GeometryError> for PolynomialError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum
    }
}

fn validate_box<const N: usize>(cell: &SpaceTimeBox<N>) -> Result<(), PolynomialError> {
    if N == 0 {
        return Err(PolynomialError::ZeroDimensions);
    }
    for axis in 0..N {
        let lower = cell.lower[axis];
        let upper = cell.upper[axis];
        if !lower.is_finite() || !upper.is_finite() || upper <= lower {
            return Err(PolynomialError::InvalidBox(cell.id));
        }
    }
    Ok(())
}

/// Analytically integrate a single monomial over an axis-aligned box.
pub fn integrate_monomial<const N: usize>(
    cell: &SpaceTimeBox<N>,
    powers: [u8; N],
) -> Result<f64, PolynomialError> {
    validate_box(cell)?;
    let mut integral = 1.0_f64;

    for axis in 0..N {
        let exponent = powers[axis] as i32 + 1;
        let lower = cell.lower[axis];
        let upper = cell.upper[axis];
        let moment = (upper.powi(exponent) - lower.powi(exponent)) / exponent as f64;
        integral *= moment;
        if !integral.is_finite() {
            return Err(PolynomialError::NonFiniteIntegral);
        }
    }
    Ok(integral)
}

/// Analytically integrate a polynomial whose terms have total degree no
/// greater than maximum_total_degree.
pub fn integrate_polynomial<const N: usize>(
    cell: &SpaceTimeBox<N>,
    terms: &[PolynomialTerm<N>],
    maximum_total_degree: u32,
) -> Result<f64, PolynomialError> {
    validate_box(cell)?;
    if maximum_total_degree > 2 {
        return Err(PolynomialError::UnsupportedMaximumDegree {
            requested: maximum_total_degree,
            supported: 2,
        });
    }
    if terms.is_empty() {
        return Err(PolynomialError::EmptyPolynomial);
    }

    let mut total = CompensatedSum::default();
    for term in terms {
        if !term.coefficient.is_finite() {
            return Err(PolynomialError::NonFiniteCoefficient);
        }
        let degree: u32 = term.powers.iter().map(|power| *power as u32).sum();
        if degree > maximum_total_degree {
            return Err(PolynomialError::DegreeExceeded {
                degree,
                maximum: maximum_total_degree,
            });
        }
        let moment = integrate_monomial(cell, term.powers)?;
        total.add(term.coefficient * moment);
    }

    if !total.value().is_finite() {
        return Err(PolynomialError::NonFiniteIntegral);
    }
    Ok(total.value())
}

fn intersection<const N: usize>(
    source: &SpaceTimeBox<N>,
    destination: &SpaceTimeBox<N>,
) -> Option<SpaceTimeBox<N>> {
    let lower = std::array::from_fn(|axis| source.lower[axis].max(destination.lower[axis]));
    let upper = std::array::from_fn(|axis| source.upper[axis].min(destination.upper[axis]));
    if (0..N).any(|axis| upper[axis] <= lower[axis]) {
        return None;
    }
    Some(SpaceTimeBox {
        id: destination.id,
        lower,
        upper,
    })
}

/// Integrate a known P0/P1/P2 polynomial over a source cell and its
/// destination intersections. Geometry is validated by the independent
/// space-time partition backend before polynomial integration is attempted.
///
/// This is an exact-moment reference oracle, not a reconstruction operator:
/// the polynomial coefficients are supplied explicitly by the caller.
pub fn remap_polynomial_integral<const N: usize>(
    source: &SpaceTimeBox<N>,
    destinations: &[SpaceTimeBox<N>],
    terms: &[PolynomialTerm<N>],
    maximum_total_degree: u32,
    geometry_tolerance: MeasureTolerance,
    integral_tolerance: PolynomialTolerance,
) -> Result<PolynomialRemapReport, PolynomialError> {
    if !integral_tolerance.absolute_integral.is_finite()
        || !integral_tolerance.relative.is_finite()
        || integral_tolerance.absolute_integral < 0.0
        || integral_tolerance.relative < 0.0
    {
        return Err(PolynomialError::InvalidTolerance);
    }

    // This validates geometric non-overlap and complete source coverage.
    let weights = build_overlap_weights(source, destinations, geometry_tolerance)?;
    let source_integral = integrate_polynomial(source, terms, maximum_total_degree)?;

    let destination_by_id: BTreeMap<u64, SpaceTimeBox<N>> = destinations
        .iter()
        .copied()
        .map(|cell| (cell.id, cell))
        .collect();
    let mut destination_integrals = BTreeMap::new();
    let mut destination_total = CompensatedSum::default();
    let mut destination_absolute_total = CompensatedSum::default();

    for weight in weights {
        let destination = destination_by_id
            .get(&weight.destination_cell_id)
            .ok_or(PolynomialError::InvalidBox(weight.destination_cell_id))?;
        let overlap = intersection(source, destination)
            .ok_or(PolynomialError::InvalidBox(weight.destination_cell_id))?;
        let integral = integrate_polynomial(&overlap, terms, maximum_total_degree)?;
        destination_total.add(integral);
        destination_absolute_total.add(integral.abs());
        destination_integrals.insert(weight.destination_cell_id, integral);
    }

    let destination_total = destination_total.value();
    let residual = destination_total - source_integral;
    let allowed = integral_tolerance.absolute_integral
        + integral_tolerance.relative
            * source_integral
                .abs()
                .max(destination_absolute_total.value());
    if !destination_total.is_finite() || !residual.is_finite() || !allowed.is_finite() {
        return Err(PolynomialError::NonFiniteIntegral);
    }
    if residual.abs() > allowed {
        return Err(PolynomialError::ConservationFailure { residual, allowed });
    }

    Ok(PolynomialRemapReport {
        source_integral,
        destination_integrals,
        destination_total,
        conservation_residual: residual,
        allowed_residual: allowed,
        maximum_total_degree,
    })
}

/// Deterministic offline reference fixture: exact linear polynomial integral
/// over a two-destination space-time partition.
pub fn reference_self_check() -> Result<(), PolynomialError> {
    let source = SpaceTimeBox {
        id: 1,
        lower: [0.0, -1.0],
        upper: [2.0, 1.0],
    };
    let destinations = [
        SpaceTimeBox {
            id: 10,
            lower: [0.0, -1.0],
            upper: [1.0, 1.0],
        },
        SpaceTimeBox {
            id: 20,
            lower: [1.0, -1.0],
            upper: [2.0, 1.0],
        },
    ];
    let polynomial = [
        PolynomialTerm {
            coefficient: 1.0,
            powers: [0, 0],
        },
        PolynomialTerm {
            coefficient: 2.0,
            powers: [1, 0],
        },
        PolynomialTerm {
            coefficient: -3.0,
            powers: [0, 1],
        },
    ];
    let report = remap_polynomial_integral(
        &source,
        &destinations,
        &polynomial,
        1,
        MeasureTolerance {
            absolute: 1.0e-12,
            relative: 1.0e-12,
        },
        PolynomialTolerance {
            absolute_integral: 1.0e-12,
            relative: 1.0e-12,
        },
    )?;
    if (report.source_integral - 12.0).abs() > 1.0e-12
        || (report.destination_integrals[&10] - 4.0).abs() > 1.0e-12
        || (report.destination_integrals[&20] - 8.0).abs() > 1.0e-12
    {
        return Err(PolynomialError::ConservationFailure {
            residual: report.conservation_residual,
            allowed: 1.0e-12,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry_tolerance() -> MeasureTolerance {
        MeasureTolerance {
            absolute: 1.0e-12,
            relative: 1.0e-12,
        }
    }

    fn integral_tolerance() -> PolynomialTolerance {
        PolynomialTolerance {
            absolute_integral: 1.0e-12,
            relative: 1.0e-12,
        }
    }

    #[test]
    fn constant_polynomial_integral_is_conservative_under_split() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 1.0],
        };
        let destinations = [
            SpaceTimeBox {
                id: 10,
                lower: [0.0, 0.0],
                upper: [1.0, 1.0],
            },
            SpaceTimeBox {
                id: 20,
                lower: [1.0, 0.0],
                upper: [2.0, 1.0],
            },
        ];
        let report = remap_polynomial_integral(
            &source,
            &destinations,
            &[PolynomialTerm {
                coefficient: 2.0,
                powers: [0, 0],
            }],
            0,
            geometry_tolerance(),
            integral_tolerance(),
        )
        .unwrap();
        assert_eq!(report.source_integral, 4.0);
        assert_eq!(report.destination_integrals[&10], 2.0);
        assert_eq!(report.destination_integrals[&20], 2.0);
    }

    #[test]
    fn linear_polynomial_integrals_are_exact_on_each_destination() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, -1.0],
            upper: [2.0, 1.0],
        };
        let destinations = [
            SpaceTimeBox {
                id: 10,
                lower: [0.0, -1.0],
                upper: [1.0, 1.0],
            },
            SpaceTimeBox {
                id: 20,
                lower: [1.0, -1.0],
                upper: [2.0, 1.0],
            },
        ];
        let terms = [
            PolynomialTerm {
                coefficient: 1.0,
                powers: [0, 0],
            },
            PolynomialTerm {
                coefficient: 2.0,
                powers: [1, 0],
            },
            PolynomialTerm {
                coefficient: -3.0,
                powers: [0, 1],
            },
        ];
        let report = remap_polynomial_integral(
            &source,
            &destinations,
            &terms,
            1,
            geometry_tolerance(),
            integral_tolerance(),
        )
        .unwrap();
        assert_eq!(report.source_integral, 12.0);
        assert_eq!(report.destination_integrals[&10], 4.0);
        assert_eq!(report.destination_integrals[&20], 8.0);
    }

    #[test]
    fn quadratic_polynomial_integrals_are_exact_on_each_destination() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 1.0],
        };
        let destinations = [
            SpaceTimeBox {
                id: 10,
                lower: [0.0, 0.0],
                upper: [1.0, 1.0],
            },
            SpaceTimeBox {
                id: 20,
                lower: [1.0, 0.0],
                upper: [2.0, 1.0],
            },
        ];
        // p(x,y) = x^2 + x*y + 2*y^2
        let terms = [
            PolynomialTerm {
                coefficient: 1.0,
                powers: [2, 0],
            },
            PolynomialTerm {
                coefficient: 1.0,
                powers: [1, 1],
            },
            PolynomialTerm {
                coefficient: 2.0,
                powers: [0, 2],
            },
        ];
        let report = remap_polynomial_integral(
            &source,
            &destinations,
            &terms,
            2,
            geometry_tolerance(),
            integral_tolerance(),
        )
        .unwrap();
        assert_eq!(report.source_integral, 5.0);
        assert_eq!(report.destination_integrals[&10], 1.25);
        assert_eq!(report.destination_integrals[&20], 3.75);
    }

    #[test]
    fn space_time_linear_polynomial_integral_is_conservative() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0, 0.0],
            upper: [1.0, 1.0, 2.0],
        };
        let destinations = [
            SpaceTimeBox {
                id: 10,
                lower: [0.0, 0.0, 0.0],
                upper: [1.0, 1.0, 1.0],
            },
            SpaceTimeBox {
                id: 20,
                lower: [0.0, 0.0, 1.0],
                upper: [1.0, 1.0, 2.0],
            },
        ];
        // p(x,y,t) = 1 + 2t - x*y
        let terms = [
            PolynomialTerm {
                coefficient: 1.0,
                powers: [0, 0, 0],
            },
            PolynomialTerm {
                coefficient: 2.0,
                powers: [0, 0, 1],
            },
            PolynomialTerm {
                coefficient: -1.0,
                powers: [1, 1, 0],
            },
        ];
        let report = remap_polynomial_integral(
            &source,
            &destinations,
            &terms,
            2,
            geometry_tolerance(),
            integral_tolerance(),
        )
        .unwrap();
        assert!((report.source_integral - 5.5).abs() < 1.0e-12);
        assert!((report.destination_total - 5.5).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_terms_above_the_declared_polynomial_degree() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0],
            upper: [1.0],
        };
        let error = integrate_polynomial(
            &source,
            &[PolynomialTerm {
                coefficient: 1.0,
                powers: [3],
            }],
            2,
        )
        .unwrap_err();
        assert_eq!(
            error,
            PolynomialError::DegreeExceeded {
                degree: 3,
                maximum: 2
            }
        );
    }

    #[test]
    fn rejects_requested_degree_above_p2_reference_scope() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0],
            upper: [1.0],
        };
        assert_eq!(
            integrate_polynomial(
                &source,
                &[PolynomialTerm {
                    coefficient: 1.0,
                    powers: [1]
                }],
                3,
            )
            .unwrap_err(),
            PolynomialError::UnsupportedMaximumDegree {
                requested: 3,
                supported: 2,
            }
        );
    }

    #[test]
    fn rejects_non_finite_polynomial_coefficients() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0],
            upper: [1.0],
        };
        assert_eq!(
            integrate_polynomial(
                &source,
                &[PolynomialTerm {
                    coefficient: f64::NAN,
                    powers: [0]
                }],
                0,
            )
            .unwrap_err(),
            PolynomialError::NonFiniteCoefficient
        );
    }

    #[test]
    fn rejects_geometric_gap_before_polynomial_transfer() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 1.0],
        };
        let destinations = [
            SpaceTimeBox {
                id: 10,
                lower: [0.0, 0.0],
                upper: [0.9, 1.0],
            },
            SpaceTimeBox {
                id: 20,
                lower: [1.1, 0.0],
                upper: [2.0, 1.0],
            },
        ];
        let error = remap_polynomial_integral(
            &source,
            &destinations,
            &[PolynomialTerm {
                coefficient: 1.0,
                powers: [0, 0],
            }],
            0,
            geometry_tolerance(),
            integral_tolerance(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            PolynomialError::Geometry(GeometryError::CoverageMismatch { .. })
        ));
    }

    #[test]
    fn remap_is_deterministic_under_destination_input_permutation() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 1.0],
        };
        let a = SpaceTimeBox {
            id: 10,
            lower: [0.0, 0.0],
            upper: [1.0, 1.0],
        };
        let b = SpaceTimeBox {
            id: 20,
            lower: [1.0, 0.0],
            upper: [2.0, 1.0],
        };
        let terms = [
            PolynomialTerm {
                coefficient: 1.0,
                powers: [0, 0],
            },
            PolynomialTerm {
                coefficient: 0.5,
                powers: [2, 0],
            },
        ];
        let forward = remap_polynomial_integral(
            &source,
            &[a, b],
            &terms,
            2,
            geometry_tolerance(),
            integral_tolerance(),
        )
        .unwrap();
        let reverse = remap_polynomial_integral(
            &source,
            &[b, a],
            &terms,
            2,
            geometry_tolerance(),
            integral_tolerance(),
        )
        .unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn reference_self_check_passes() {
        reference_self_check().unwrap();
    }
}
