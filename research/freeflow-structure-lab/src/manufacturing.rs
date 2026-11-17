//! Dimensional fit and tolerance-stack contracts for modular connectors.
//!
//! These are geometric manufacturing-planning checks only. They do not
//! establish fastener strength, fatigue, preload, wear, thermal expansion,
//! material process capability or assembly qualification.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DimensionalTolerance {
    pub nominal_mm: f64,
    pub lower_deviation_mm: f64,
    pub upper_deviation_mm: f64,
}

impl DimensionalTolerance {
    pub fn validate(self) -> Result<(), ManufacturingError> {
        if !self.nominal_mm.is_finite()
            || !self.lower_deviation_mm.is_finite()
            || !self.upper_deviation_mm.is_finite()
            || self.nominal_mm <= 0.0
            || self.lower_deviation_mm < 0.0
            || self.upper_deviation_mm < 0.0
            || self.minimum_mm() <= 0.0
        {
            return Err(ManufacturingError::InvalidDimension);
        }
        Ok(())
    }

    pub fn minimum_mm(self) -> f64 {
        self.nominal_mm - self.lower_deviation_mm
    }

    pub fn maximum_mm(self) -> f64 {
        self.nominal_mm + self.upper_deviation_mm
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectorFitSpec {
    pub pin_diameter: DimensionalTolerance,
    pub bore_diameter: DimensionalTolerance,
    pub minimum_radial_clearance_mm: f64,
    pub maximum_radial_clearance_mm: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectorFitEnvelope {
    pub minimum_diametral_clearance_mm: f64,
    pub maximum_diametral_clearance_mm: f64,
    pub minimum_radial_clearance_mm: f64,
    pub maximum_radial_clearance_mm: f64,
}

impl ConnectorFitSpec {
    pub fn analyze(self) -> Result<ConnectorFitEnvelope, ManufacturingError> {
        self.pin_diameter.validate()?;
        self.bore_diameter.validate()?;
        if !self.minimum_radial_clearance_mm.is_finite()
            || !self.maximum_radial_clearance_mm.is_finite()
            || self.minimum_radial_clearance_mm < 0.0
            || self.maximum_radial_clearance_mm < self.minimum_radial_clearance_mm
        {
            return Err(ManufacturingError::InvalidClearanceBounds);
        }

        let minimum_diametral = self.bore_diameter.minimum_mm() - self.pin_diameter.maximum_mm();
        let maximum_diametral = self.bore_diameter.maximum_mm() - self.pin_diameter.minimum_mm();
        let minimum_radial = minimum_diametral * 0.5;
        let maximum_radial = maximum_diametral * 0.5;

        if !minimum_radial.is_finite() || !maximum_radial.is_finite() {
            return Err(ManufacturingError::NonFiniteResult);
        }
        if minimum_radial < self.minimum_radial_clearance_mm {
            return Err(ManufacturingError::InterferenceOrInsufficientClearance {
                minimum_radial_clearance_mm: minimum_radial,
                required_mm: self.minimum_radial_clearance_mm,
            });
        }
        if maximum_radial > self.maximum_radial_clearance_mm {
            return Err(ManufacturingError::ExcessiveClearance {
                maximum_radial_clearance_mm: maximum_radial,
                allowed_mm: self.maximum_radial_clearance_mm,
            });
        }

        Ok(ConnectorFitEnvelope {
            minimum_diametral_clearance_mm: minimum_diametral,
            maximum_diametral_clearance_mm: maximum_diametral,
            minimum_radial_clearance_mm: minimum_radial,
            maximum_radial_clearance_mm: maximum_radial,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToleranceStackResult {
    pub interface_count: usize,
    /// Conservative arithmetic sum of all symmetric half-width tolerances.
    pub worst_case_absolute_error_mm: f64,
    /// Root-sum-square estimate, assuming independent centered contributions.
    /// This is not a guaranteed bound or a probability statement.
    pub root_sum_square_mm: f64,
}

pub fn analyze_linear_tolerance_stack(
    symmetric_half_widths_mm: &[f64],
) -> Result<ToleranceStackResult, ManufacturingError> {
    if symmetric_half_widths_mm.is_empty() {
        return Err(ManufacturingError::EmptyToleranceStack);
    }

    let mut worst_case = 0.0;
    let mut sum_squares = 0.0;
    for tolerance in symmetric_half_widths_mm {
        if !tolerance.is_finite() || *tolerance < 0.0 {
            return Err(ManufacturingError::InvalidTolerance);
        }
        worst_case += tolerance;
        sum_squares += tolerance * tolerance;
    }
    let root_sum_square = sum_squares.sqrt();
    if !worst_case.is_finite() || !root_sum_square.is_finite() {
        return Err(ManufacturingError::NonFiniteResult);
    }

    Ok(ToleranceStackResult {
        interface_count: symmetric_half_widths_mm.len(),
        worst_case_absolute_error_mm: worst_case,
        root_sum_square_mm: root_sum_square,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum ManufacturingError {
    InvalidDimension,
    InvalidClearanceBounds,
    InterferenceOrInsufficientClearance {
        minimum_radial_clearance_mm: f64,
        required_mm: f64,
    },
    ExcessiveClearance {
        maximum_radial_clearance_mm: f64,
        allowed_mm: f64,
    },
    EmptyToleranceStack,
    InvalidTolerance,
    NonFiniteResult,
}

impl fmt::Display for ManufacturingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(
                f,
                "connector dimensions and tolerances must be finite and positive"
            ),
            Self::InvalidClearanceBounds => write!(f, "radial clearance bounds are invalid"),
            Self::InterferenceOrInsufficientClearance {
                minimum_radial_clearance_mm,
                required_mm,
            } => write!(
                f,
                "minimum radial clearance {minimum_radial_clearance_mm} mm is below required {required_mm} mm"
            ),
            Self::ExcessiveClearance {
                maximum_radial_clearance_mm,
                allowed_mm,
            } => write!(
                f,
                "maximum radial clearance {maximum_radial_clearance_mm} mm exceeds allowed {allowed_mm} mm"
            ),
            Self::EmptyToleranceStack => {
                write!(f, "tolerance stack must contain at least one interface")
            }
            Self::InvalidTolerance => {
                write!(f, "tolerance half-width must be finite and non-negative")
            }
            Self::NonFiniteResult => {
                write!(f, "manufacturing analysis produced a non-finite value")
            }
        }
    }
}
impl Error for ManufacturingError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance(nominal_mm: f64, symmetric_mm: f64) -> DimensionalTolerance {
        DimensionalTolerance {
            nominal_mm,
            lower_deviation_mm: symmetric_mm,
            upper_deviation_mm: symmetric_mm,
        }
    }

    #[test]
    fn pin_bore_fit_reports_worst_case_radial_clearance() {
        let spec = ConnectorFitSpec {
            pin_diameter: tolerance(8.0, 0.05),
            bore_diameter: tolerance(8.2, 0.05),
            minimum_radial_clearance_mm: 0.04,
            maximum_radial_clearance_mm: 0.16,
        };
        let fit = spec.analyze().unwrap();
        assert!((fit.minimum_diametral_clearance_mm - 0.10).abs() < 1e-12);
        assert!((fit.maximum_diametral_clearance_mm - 0.30).abs() < 1e-12);
        assert!((fit.minimum_radial_clearance_mm - 0.05).abs() < 1e-12);
        assert!((fit.maximum_radial_clearance_mm - 0.15).abs() < 1e-12);
    }

    #[test]
    fn interference_is_rejected_before_a_connector_is_declared_assemblable() {
        let spec = ConnectorFitSpec {
            pin_diameter: tolerance(8.05, 0.02),
            bore_diameter: tolerance(8.0, 0.02),
            minimum_radial_clearance_mm: 0.0,
            maximum_radial_clearance_mm: 0.2,
        };
        assert!(matches!(
            spec.analyze(),
            Err(ManufacturingError::InterferenceOrInsufficientClearance { .. })
        ));
    }

    #[test]
    fn excessive_clearance_is_rejected() {
        let spec = ConnectorFitSpec {
            pin_diameter: tolerance(8.0, 0.01),
            bore_diameter: tolerance(8.5, 0.01),
            minimum_radial_clearance_mm: 0.0,
            maximum_radial_clearance_mm: 0.1,
        };
        assert!(matches!(
            spec.analyze(),
            Err(ManufacturingError::ExcessiveClearance { .. })
        ));
    }

    #[test]
    fn linear_stack_reports_worst_case_and_rss_separately() {
        let result = analyze_linear_tolerance_stack(&[0.1, 0.2, 0.05]).unwrap();
        assert_eq!(result.interface_count, 3);
        assert!((result.worst_case_absolute_error_mm - 0.35).abs() < 1e-12);
        assert!((result.root_sum_square_mm - (0.0525_f64).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn invalid_or_empty_tolerance_stacks_fail_closed() {
        assert_eq!(
            analyze_linear_tolerance_stack(&[]),
            Err(ManufacturingError::EmptyToleranceStack)
        );
        assert_eq!(
            analyze_linear_tolerance_stack(&[0.1, f64::NAN]),
            Err(ManufacturingError::InvalidTolerance)
        );
        assert_eq!(
            analyze_linear_tolerance_stack(&[0.1, -0.01]),
            Err(ManufacturingError::InvalidTolerance)
        );
    }
}
