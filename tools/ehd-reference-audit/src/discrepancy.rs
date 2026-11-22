//! D0 manufactured error-source separation and conservation-firewall reference.
//!
//! This kernel decomposes a residual only against a frozen, orthogonal synthetic
//! signature basis. Correlated signatures cause abstention. It is not a general
//! statistical discrepancy model, physical calibration, or OOD detector.

use std::error::Error;
use std::fmt;

pub const D0_VECTOR_LENGTH: usize = 8;
const SIGNATURE_ORTHOGONALITY_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorSource {
    ParameterPerturbation,
    NumericalBias,
    MeasurementArtifact,
    ModelFormDiscrepancy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum D0Disposition {
    NoSystematicSourceDetected,
    ParameterOrNumericalErrorDetected,
    MeasurementArtifactUnresolved,
    ModelFormDiscrepancyDetected,
    NotEvaluable,
    OutOfEnvelope,
    PhysicalLedgerFailure,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorSourceSignatures {
    pub parameter_perturbation: [f64; D0_VECTOR_LENGTH],
    pub numerical_bias: [f64; D0_VECTOR_LENGTH],
    pub measurement_artifact: [f64; D0_VECTOR_LENGTH],
    pub model_form_discrepancy: [f64; D0_VECTOR_LENGTH],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct D0CaseInput {
    /// Observation minus baseline-model prediction at a frozen observable boundary.
    pub residual: [f64; D0_VECTOR_LENGTH],
    pub signatures: ErrorSourceSignatures,
    /// Declared upper bound for the unexplained measurement-noise remainder.
    pub measurement_noise_l2_bound: f64,
    /// Minimum absolute coefficient reported as an active injected source.
    pub minimum_detectable_amplitude: f64,
    /// Supplied by the independent envelope auditor, not inferred from residual fit.
    pub within_validation_envelope: bool,
    /// Supplied by independent charge/mass/momentum/energy ledger gates.
    pub conservation_ledgers_pass: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorSourceCoefficients {
    pub parameter_perturbation: f64,
    pub numerical_bias: f64,
    pub measurement_artifact: f64,
    pub model_form_discrepancy: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct D0DecompositionReport {
    pub coefficients: ErrorSourceCoefficients,
    pub active_sources: Vec<ErrorSource>,
    pub reconstructed_systematic_residual: [f64; D0_VECTOR_LENGTH],
    pub unexplained_residual: [f64; D0_VECTOR_LENGTH],
    pub unexplained_residual_l2: f64,
    pub measurement_noise_l2_bound: f64,
    pub within_validation_envelope: bool,
    pub conservation_ledgers_pass: bool,
    pub disposition: D0Disposition,
}

#[derive(Clone, Debug, PartialEq)]
pub enum D0Error {
    NonFiniteResidual { index: usize },
    NonFiniteSignature { source: usize, index: usize },
    EmptySignature { source: usize },
    ConfoundedSignatures { first: usize, second: usize, absolute_correlation: f64 },
    InvalidMeasurementNoiseBound,
    InvalidDetectionThreshold,
    NonFiniteDerivedValue,
}

impl fmt::Display for D0Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteResidual { index } => {
                write!(f, "D0 residual component {index} is non-finite")
            }
            Self::NonFiniteSignature { source, index } => {
                write!(f, "D0 source signature {source} component {index} is non-finite")
            }
            Self::EmptySignature { source } => {
                write!(f, "D0 source signature {source} has zero norm")
            }
            Self::ConfoundedSignatures {
                first,
                second,
                absolute_correlation,
            } => write!(
                f,
                "D0 source signatures {first} and {second} are confounded (absolute correlation {absolute_correlation})"
            ),
            Self::InvalidMeasurementNoiseBound => {
                write!(f, "D0 measurement-noise bound must be finite and non-negative")
            }
            Self::InvalidDetectionThreshold => {
                write!(f, "D0 detection threshold must be finite and non-negative")
            }
            Self::NonFiniteDerivedValue => {
                write!(f, "D0 decomposition produced a non-finite derived value")
            }
        }
    }
}

impl Error for D0Error {}

fn dot(left: &[f64; D0_VECTOR_LENGTH], right: &[f64; D0_VECTOR_LENGTH]) -> f64 {
    left.iter()
        .zip(right.iter())
        .fold(0.0, |sum, (a, b)| a.mul_add(*b, sum))
}

fn l2_norm(values: &[f64; D0_VECTOR_LENGTH]) -> f64 {
    dot(values, values).sqrt()
}

fn signature_at(
    signatures: &ErrorSourceSignatures,
    source: usize,
) -> &[f64; D0_VECTOR_LENGTH] {
    match source {
        0 => &signatures.parameter_perturbation,
        1 => &signatures.numerical_bias,
        2 => &signatures.measurement_artifact,
        _ => &signatures.model_form_discrepancy,
    }
}

/// Frozen Hadamard signatures make the manufactured source classes exactly
/// separable. The unused fifth Hadamard direction is reserved for bounded
/// measurement-noise residuals, not fitted as a physical source.
pub fn reference_signatures() -> ErrorSourceSignatures {
    ErrorSourceSignatures {
        parameter_perturbation: [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
        numerical_bias: [1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0],
        measurement_artifact: [1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0],
        model_form_discrepancy: [1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0],
    }
}

/// Decompose a manufactured residual against four declared systematic source
/// signatures. Non-orthogonal signatures are rejected rather than regularized
/// into a potentially misleading source attribution.
pub fn analyse_d0_case(input: &D0CaseInput) -> Result<D0DecompositionReport, D0Error> {
    if !input.measurement_noise_l2_bound.is_finite()
        || input.measurement_noise_l2_bound < 0.0
    {
        return Err(D0Error::InvalidMeasurementNoiseBound);
    }
    if !input.minimum_detectable_amplitude.is_finite()
        || input.minimum_detectable_amplitude < 0.0
    {
        return Err(D0Error::InvalidDetectionThreshold);
    }
    for (index, value) in input.residual.iter().enumerate() {
        if !value.is_finite() {
            return Err(D0Error::NonFiniteResidual { index });
        }
    }

    let mut signatures = [[0.0_f64; D0_VECTOR_LENGTH]; 4];
    let mut signature_norm_squared = [0.0_f64; 4];
    for source in 0..4 {
        let signature = signature_at(&input.signatures, source);
        for (index, value) in signature.iter().enumerate() {
            if !value.is_finite() {
                return Err(D0Error::NonFiniteSignature { source, index });
            }
        }
        let norm_squared = dot(signature, signature);
        if !norm_squared.is_finite() {
            return Err(D0Error::NonFiniteDerivedValue);
        }
        if norm_squared <= 0.0 {
            return Err(D0Error::EmptySignature { source });
        }
        signatures[source] = *signature;
        signature_norm_squared[source] = norm_squared;
    }

    for first in 0..4 {
        for second in (first + 1)..4 {
            let denominator =
                (signature_norm_squared[first] * signature_norm_squared[second]).sqrt();
            if !denominator.is_finite() || denominator <= 0.0 {
                return Err(D0Error::NonFiniteDerivedValue);
            }
            let correlation = dot(&signatures[first], &signatures[second]) / denominator;
            if !correlation.is_finite() {
                return Err(D0Error::NonFiniteDerivedValue);
            }
            if correlation.abs() > SIGNATURE_ORTHOGONALITY_TOLERANCE {
                return Err(D0Error::ConfoundedSignatures {
                    first,
                    second,
                    absolute_correlation: correlation.abs(),
                });
            }
        }
    }

    let mut coefficients = [0.0_f64; 4];
    let mut reconstructed = [0.0_f64; D0_VECTOR_LENGTH];
    for source in 0..4 {
        coefficients[source] = dot(&input.residual, &signatures[source])
            / signature_norm_squared[source];
        if !coefficients[source].is_finite() {
            return Err(D0Error::NonFiniteDerivedValue);
        }
        for component in 0..D0_VECTOR_LENGTH {
            reconstructed[component] = coefficients[source]
                .mul_add(signatures[source][component], reconstructed[component]);
        }
    }

    let mut unexplained = [0.0_f64; D0_VECTOR_LENGTH];
    for component in 0..D0_VECTOR_LENGTH {
        unexplained[component] = input.residual[component] - reconstructed[component];
    }
    let unexplained_l2 = l2_norm(&unexplained);
    if !unexplained_l2.is_finite() {
        return Err(D0Error::NonFiniteDerivedValue);
    }

    let mut active_sources = Vec::new();
    if coefficients[0].abs() > input.minimum_detectable_amplitude {
        active_sources.push(ErrorSource::ParameterPerturbation);
    }
    if coefficients[1].abs() > input.minimum_detectable_amplitude {
        active_sources.push(ErrorSource::NumericalBias);
    }
    if coefficients[2].abs() > input.minimum_detectable_amplitude {
        active_sources.push(ErrorSource::MeasurementArtifact);
    }
    if coefficients[3].abs() > input.minimum_detectable_amplitude {
        active_sources.push(ErrorSource::ModelFormDiscrepancy);
    }

    // The ledger is an independent hard gate. A fitted discrepancy coefficient
    // never changes its supplied pass/fail status.
    let disposition = if !input.conservation_ledgers_pass {
        D0Disposition::PhysicalLedgerFailure
    } else if !input.within_validation_envelope {
        D0Disposition::OutOfEnvelope
    } else if unexplained_l2 > input.measurement_noise_l2_bound {
        D0Disposition::NotEvaluable
    } else if coefficients[0].abs() > input.minimum_detectable_amplitude
        || coefficients[1].abs() > input.minimum_detectable_amplitude
    {
        D0Disposition::ParameterOrNumericalErrorDetected
    } else if coefficients[2].abs() > input.minimum_detectable_amplitude {
        D0Disposition::MeasurementArtifactUnresolved
    } else if coefficients[3].abs() > input.minimum_detectable_amplitude {
        D0Disposition::ModelFormDiscrepancyDetected
    } else {
        D0Disposition::NoSystematicSourceDetected
    };

    Ok(D0DecompositionReport {
        coefficients: ErrorSourceCoefficients {
            parameter_perturbation: coefficients[0],
            numerical_bias: coefficients[1],
            measurement_artifact: coefficients[2],
            model_form_discrepancy: coefficients[3],
        },
        active_sources,
        reconstructed_systematic_residual: reconstructed,
        unexplained_residual: unexplained,
        unexplained_residual_l2: unexplained_l2,
        measurement_noise_l2_bound: input.measurement_noise_l2_bound,
        within_validation_envelope: input.within_validation_envelope,
        conservation_ledgers_pass: input.conservation_ledgers_pass,
        disposition,
    })
}

fn manufactured_residual(
    signatures: &ErrorSourceSignatures,
    coefficients: [f64; 4],
    noise_amplitude: f64,
) -> [f64; D0_VECTOR_LENGTH] {
    // Orthogonal fifth Hadamard row reserved as one deterministic noise
    // realization in this manufactured reference.
    let noise = [1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0];
    let mut residual = [0.0_f64; D0_VECTOR_LENGTH];
    for source in 0..4 {
        let signature = signature_at(signatures, source);
        for component in 0..D0_VECTOR_LENGTH {
            residual[component] =
                coefficients[source].mul_add(signature[component], residual[component]);
        }
    }
    for component in 0..D0_VECTOR_LENGTH {
        residual[component] = noise_amplitude.mul_add(noise[component], residual[component]);
    }
    residual
}

/// Manufactured mixed-error, noise-only, confounding and conservation-firewall
/// references. These vectors have no EHD physical or calibration interpretation.
pub fn reference_self_check() -> Result<(), D0Error> {
    let signatures = reference_signatures();
    let mixed = D0CaseInput {
        residual: manufactured_residual(&signatures, [1.5, -0.75, 0.5, 2.0], 0.05),
        signatures,
        measurement_noise_l2_bound: 0.15,
        minimum_detectable_amplitude: 1.0e-10,
        within_validation_envelope: true,
        conservation_ledgers_pass: true,
    };
    let report = analyse_d0_case(&mixed)?;
    if (report.coefficients.parameter_perturbation - 1.5).abs() > 1.0e-12
        || (report.coefficients.numerical_bias + 0.75).abs() > 1.0e-12
        || (report.coefficients.measurement_artifact - 0.5).abs() > 1.0e-12
        || (report.coefficients.model_form_discrepancy - 2.0).abs() > 1.0e-12
        || (report.unexplained_residual_l2 - 8.0_f64.sqrt() * 0.05).abs() > 1.0e-12
        || report.disposition != D0Disposition::ParameterOrNumericalErrorDetected
    {
        return Err(D0Error::NonFiniteDerivedValue);
    }

    let discrepancy_with_failed_ledger = D0CaseInput {
        residual: manufactured_residual(&signatures, [0.0, 0.0, 0.0, 3.0], 0.0),
        signatures,
        measurement_noise_l2_bound: 0.0,
        minimum_detectable_amplitude: 1.0e-10,
        within_validation_envelope: true,
        conservation_ledgers_pass: false,
    };
    let failed = analyse_d0_case(&discrepancy_with_failed_ledger)?;
    if (failed.coefficients.model_form_discrepancy - 3.0).abs() > 1.0e-12
        || failed.disposition != D0Disposition::PhysicalLedgerFailure
    {
        return Err(D0Error::NonFiniteDerivedValue);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case_with(
        coefficients: [f64; 4],
        noise_amplitude: f64,
        noise_bound: f64,
        within_validation_envelope: bool,
        conservation_ledgers_pass: bool,
    ) -> D0CaseInput {
        let signatures = reference_signatures();
        D0CaseInput {
            residual: manufactured_residual(&signatures, coefficients, noise_amplitude),
            signatures,
            measurement_noise_l2_bound: noise_bound,
            minimum_detectable_amplitude: 1.0e-10,
            within_validation_envelope,
            conservation_ledgers_pass,
        }
    }

    #[test]
    fn mixed_parameter_numerical_artifact_and_model_errors_are_separated() {
        let report = analyse_d0_case(&case_with(
            [1.5, -0.75, 0.5, 2.0],
            0.05,
            0.15,
            true,
            true,
        )).unwrap();
        assert!((report.coefficients.parameter_perturbation - 1.5).abs() < 1.0e-12);
        assert!((report.coefficients.numerical_bias + 0.75).abs() < 1.0e-12);
        assert!((report.coefficients.measurement_artifact - 0.5).abs() < 1.0e-12);
        assert!((report.coefficients.model_form_discrepancy - 2.0).abs() < 1.0e-12);
        assert!((report.unexplained_residual_l2 - 8.0_f64.sqrt() * 0.05).abs() < 1.0e-12);
        assert_eq!(report.disposition, D0Disposition::ParameterOrNumericalErrorDetected);
        assert_eq!(report.active_sources.len(), 4);
    }

    #[test]
    fn measurement_noise_below_bound_is_not_fitted_as_a_systematic_source() {
        let report = analyse_d0_case(&case_with([0.0; 4], 0.1, 0.3, true, true)).unwrap();
        assert!(report.active_sources.is_empty());
        assert_eq!(report.disposition, D0Disposition::NoSystematicSourceDetected);
    }

    #[test]
    fn correlated_parameter_and_numerical_signatures_abstain() {
        let mut input = case_with([1.0, 0.0, 0.0, 0.0], 0.0, 0.0, true, true);
        input.signatures.numerical_bias = input.signatures.parameter_perturbation;
        assert!(matches!(
            analyse_d0_case(&input),
            Err(D0Error::ConfoundedSignatures { first: 0, second: 1, .. })
        ));
    }

    #[test]
    fn discrepancy_fit_never_repairs_failed_conservation_ledger() {
        let report = analyse_d0_case(&case_with([0.0, 0.0, 0.0, 3.0], 0.0, 0.0, true, false))
            .unwrap();
        assert!((report.coefficients.model_form_discrepancy - 3.0).abs() < 1.0e-12);
        assert_eq!(report.disposition, D0Disposition::PhysicalLedgerFailure);
        assert!(!report.conservation_ledgers_pass);
    }

    #[test]
    fn out_of_envelope_case_abstains_even_with_small_residual() {
        let report = analyse_d0_case(&case_with([0.0; 4], 0.0, 0.0, false, true)).unwrap();
        assert_eq!(report.disposition, D0Disposition::OutOfEnvelope);
    }

    #[test]
    fn unexplained_residual_above_measurement_noise_bound_is_not_evaluable() {
        let report = analyse_d0_case(&case_with([0.0; 4], 0.2, 0.1, true, true)).unwrap();
        assert_eq!(report.disposition, D0Disposition::NotEvaluable);
        assert!(report.unexplained_residual_l2 > report.measurement_noise_l2_bound);
    }

    #[test]
    fn negative_noise_bound_is_rejected() {
        let mut input = case_with([0.0; 4], 0.0, 0.0, true, true);
        input.measurement_noise_l2_bound = -0.1;
        assert_eq!(
            analyse_d0_case(&input).unwrap_err(),
            D0Error::InvalidMeasurementNoiseBound
        );
    }
}
