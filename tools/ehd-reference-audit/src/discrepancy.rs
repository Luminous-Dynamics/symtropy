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
    InvalidRankTolerance,
    InvalidConditionLimit,
    NonFiniteCovariance { row: usize, column: usize },
    AsymmetricCovariance { row: usize, column: usize },
    NonPositiveDefiniteCovariance { pivot: usize },
    RankDeficientSignatureBasis,
    IllConditionedSignatureBasis { condition_proxy: f64 },
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
            Self::InvalidRankTolerance => {
                write!(f, "D0 relative rank tolerance must be finite and in (0, 1)")
            }
            Self::InvalidConditionLimit => {
                write!(f, "D0 QR condition-proxy limit must be finite and greater than one")
            }
            Self::NonFiniteCovariance { row, column } => {
                write!(f, "D0 covariance component ({row}, {column}) is non-finite")
            }
            Self::AsymmetricCovariance { row, column } => {
                write!(f, "D0 covariance is not symmetric at ({row}, {column})")
            }
            Self::NonPositiveDefiniteCovariance { pivot } => {
                write!(f, "D0 covariance is not positive definite at pivot {pivot}")
            }
            Self::RankDeficientSignatureBasis => {
                write!(f, "D0 source signature basis is rank-deficient")
            }
            Self::IllConditionedSignatureBasis { condition_proxy } => {
                write!(f, "D0 source signature basis exceeds QR diagonal condition proxy ({condition_proxy})")
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


// ---------------------------------------------------------------------------
// D0-QR-001: pivoted QR attribution for correlated-but-identifiable source
// signatures under a full 8x8 observation covariance.
// ---------------------------------------------------------------------------

pub const D0_GENERAL_SOURCE_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct D0GeneralSignatures {
    pub source_columns: [[f64; D0_VECTOR_LENGTH]; D0_GENERAL_SOURCE_COUNT],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct D0GeneralCaseInput {
    pub residual: [f64; D0_VECTOR_LENGTH],
    pub signatures: D0GeneralSignatures,
    pub observation_covariance: [[f64; D0_VECTOR_LENGTH]; D0_VECTOR_LENGTH],
    pub measurement_noise_l2_bound: f64,
    pub minimum_detectable_amplitude: f64,
    pub relative_rank_tolerance: f64,
    /// Bound on max(R_kk)/min(R_kk) after column-pivoted QR; this is a
    /// conditioning proxy, not a spectral condition number.
    pub maximum_qr_diagonal_condition_proxy: f64,
    pub within_validation_envelope: bool,
    pub conservation_ledgers_pass: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct D0GeneralDecompositionReport {
    /// Coefficients in original source order, restored after QR pivoting.
    pub source_coefficients: [f64; D0_GENERAL_SOURCE_COUNT],
    pub active_sources: Vec<ErrorSource>,
    /// Residuals are in whitened coordinates; L2 is therefore a Mahalanobis norm.
    pub reconstructed_systematic_residual_whitened: [f64; D0_VECTOR_LENGTH],
    pub unexplained_residual_whitened: [f64; D0_VECTOR_LENGTH],
    pub unexplained_residual_l2: f64,
    pub qr_diagonal_condition_proxy: f64,
    pub measurement_noise_l2_bound: f64,
    pub within_validation_envelope: bool,
    pub conservation_ledgers_pass: bool,
    pub disposition: D0Disposition,
}

fn dot_eight(left: &[f64; D0_VECTOR_LENGTH], right: &[f64; D0_VECTOR_LENGTH]) -> f64 {
    let mut value = 0.0_f64;
    for i in 0..D0_VECTOR_LENGTH {
        value = left[i].mul_add(right[i], value);
    }
    value
}

fn cholesky_covariance_8x8(
    covariance: &[[f64; D0_VECTOR_LENGTH]; D0_VECTOR_LENGTH],
) -> Result<[[f64; D0_VECTOR_LENGTH]; D0_VECTOR_LENGTH], D0Error> {
    for row in 0..D0_VECTOR_LENGTH {
        for column in 0..D0_VECTOR_LENGTH {
            if !covariance[row][column].is_finite() {
                return Err(D0Error::NonFiniteCovariance { row, column });
            }
            let scale = covariance[row][column].abs()
                .max(covariance[column][row].abs())
                .max(1.0);
            if (covariance[row][column] - covariance[column][row]).abs() > 1.0e-12 * scale {
                return Err(D0Error::AsymmetricCovariance { row, column });
            }
        }
    }
    let mut lower = [[0.0_f64; D0_VECTOR_LENGTH]; D0_VECTOR_LENGTH];
    for row in 0..D0_VECTOR_LENGTH {
        for column in 0..=row {
            let mut value = covariance[row][column];
            for k in 0..column {
                value -= lower[row][k] * lower[column][k];
            }
            if !value.is_finite() {
                return Err(D0Error::NonFiniteDerivedValue);
            }
            if row == column {
                if value <= 0.0 {
                    return Err(D0Error::NonPositiveDefiniteCovariance { pivot: row });
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn forward_solve_eight(
    lower: &[[f64; D0_VECTOR_LENGTH]; D0_VECTOR_LENGTH],
    input: &[f64; D0_VECTOR_LENGTH],
) -> Result<[f64; D0_VECTOR_LENGTH], D0Error> {
    let mut output = [0.0_f64; D0_VECTOR_LENGTH];
    for row in 0..D0_VECTOR_LENGTH {
        let mut value = input[row];
        for column in 0..row {
            value -= lower[row][column] * output[column];
        }
        output[row] = value / lower[row][row];
        if !output[row].is_finite() {
            return Err(D0Error::NonFiniteDerivedValue);
        }
    }
    Ok(output)
}

/// Modified Gram-Schmidt with column pivoting and one re-orthogonalization
/// pass. Coefficients are solved in pivoted order and later restored.
fn pivoted_qr_four_sources(
    columns: &[[f64; D0_VECTOR_LENGTH]; D0_GENERAL_SOURCE_COUNT],
    relative_rank_tolerance: f64,
) -> Result<
    (
        [[f64; D0_VECTOR_LENGTH]; D0_GENERAL_SOURCE_COUNT],
        [[f64; D0_GENERAL_SOURCE_COUNT]; D0_GENERAL_SOURCE_COUNT],
        [usize; D0_GENERAL_SOURCE_COUNT],
        f64,
    ),
    D0Error,
> {
    let mut residual_columns = *columns;
    let mut permutation = [0_usize, 1, 2, 3];
    let mut orthonormal = [[0.0_f64; D0_VECTOR_LENGTH]; D0_GENERAL_SOURCE_COUNT];
    let mut upper = [[0.0_f64; D0_GENERAL_SOURCE_COUNT]; D0_GENERAL_SOURCE_COUNT];
    let maximum_initial_norm = residual_columns
        .iter()
        .map(|column| dot_eight(column, column).sqrt())
        .fold(0.0_f64, f64::max);
    if !maximum_initial_norm.is_finite() || maximum_initial_norm <= 0.0 {
        return Err(D0Error::RankDeficientSignatureBasis);
    }

    for k in 0..D0_GENERAL_SOURCE_COUNT {
        let mut pivot = k;
        let mut pivot_norm_squared = -1.0_f64;
        for column in k..D0_GENERAL_SOURCE_COUNT {
            let norm_squared = dot_eight(&residual_columns[column], &residual_columns[column]);
            if !norm_squared.is_finite() {
                return Err(D0Error::NonFiniteDerivedValue);
            }
            if norm_squared > pivot_norm_squared {
                pivot_norm_squared = norm_squared;
                pivot = column;
            }
        }
        if pivot != k {
            residual_columns.swap(k, pivot);
            permutation.swap(k, pivot);
            for prior_row in 0..k {
                let value = upper[prior_row][k];
                upper[prior_row][k] = upper[prior_row][pivot];
                upper[prior_row][pivot] = value;
            }
        }

        let diagonal = dot_eight(&residual_columns[k], &residual_columns[k]).sqrt();
        if !diagonal.is_finite() {
            return Err(D0Error::NonFiniteDerivedValue);
        }
        if diagonal <= relative_rank_tolerance * maximum_initial_norm {
            return Err(D0Error::RankDeficientSignatureBasis);
        }
        upper[k][k] = diagonal;
        for row in 0..D0_VECTOR_LENGTH {
            orthonormal[k][row] = residual_columns[k][row] / diagonal;
        }

        for column in (k + 1)..D0_GENERAL_SOURCE_COUNT {
            let mut projection = dot_eight(&orthonormal[k], &residual_columns[column]);
            for row in 0..D0_VECTOR_LENGTH {
                residual_columns[column][row] -= projection * orthonormal[k][row];
            }
            let correction = dot_eight(&orthonormal[k], &residual_columns[column]);
            for row in 0..D0_VECTOR_LENGTH {
                residual_columns[column][row] -= correction * orthonormal[k][row];
            }
            projection += correction;
            upper[k][column] = projection;
        }
    }

    let maximum_diagonal = (0..D0_GENERAL_SOURCE_COUNT)
        .map(|i| upper[i][i])
        .fold(0.0_f64, f64::max);
    let minimum_diagonal = (0..D0_GENERAL_SOURCE_COUNT)
        .map(|i| upper[i][i])
        .fold(f64::INFINITY, f64::min);
    if !minimum_diagonal.is_finite() || minimum_diagonal <= 0.0 {
        return Err(D0Error::RankDeficientSignatureBasis);
    }
    let condition_proxy = maximum_diagonal / minimum_diagonal;
    if !condition_proxy.is_finite() {
        return Err(D0Error::NonFiniteDerivedValue);
    }
    Ok((orthonormal, upper, permutation, condition_proxy))
}

fn solve_upper_four(
    upper: &[[f64; D0_GENERAL_SOURCE_COUNT]; D0_GENERAL_SOURCE_COUNT],
    rhs: &[f64; D0_GENERAL_SOURCE_COUNT],
) -> Result<[f64; D0_GENERAL_SOURCE_COUNT], D0Error> {
    let mut solution = [0.0_f64; D0_GENERAL_SOURCE_COUNT];
    for row in (0..D0_GENERAL_SOURCE_COUNT).rev() {
        let mut value = rhs[row];
        for column in (row + 1)..D0_GENERAL_SOURCE_COUNT {
            value -= upper[row][column] * solution[column];
        }
        solution[row] = value / upper[row][row];
        if !solution[row].is_finite() {
            return Err(D0Error::NonFiniteDerivedValue);
        }
    }
    Ok(solution)
}

/// Decompose residuals against arbitrary well-conditioned, non-orthogonal
/// source signatures using full-covariance whitening and pivoted QR.
pub fn analyse_d0_general_case(
    input: &D0GeneralCaseInput,
) -> Result<D0GeneralDecompositionReport, D0Error> {
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
    if !input.relative_rank_tolerance.is_finite()
        || input.relative_rank_tolerance <= 0.0
        || input.relative_rank_tolerance >= 1.0
    {
        return Err(D0Error::InvalidRankTolerance);
    }
    if !input.maximum_qr_diagonal_condition_proxy.is_finite()
        || input.maximum_qr_diagonal_condition_proxy <= 1.0
    {
        return Err(D0Error::InvalidConditionLimit);
    }
    for (index, value) in input.residual.iter().enumerate() {
        if !value.is_finite() {
            return Err(D0Error::NonFiniteResidual { index });
        }
    }

    let lower = cholesky_covariance_8x8(&input.observation_covariance)?;
    let whitened_residual = forward_solve_eight(&lower, &input.residual)?;
    let mut whitened_signatures = [[0.0_f64; D0_VECTOR_LENGTH]; D0_GENERAL_SOURCE_COUNT];
    for source in 0..D0_GENERAL_SOURCE_COUNT {
        for index in 0..D0_VECTOR_LENGTH {
            if !input.signatures.source_columns[source][index].is_finite() {
                return Err(D0Error::NonFiniteSignature { source, index });
            }
        }
        whitened_signatures[source] =
            forward_solve_eight(&lower, &input.signatures.source_columns[source])?;
    }

    let (q, r, permutation, condition_proxy) =
        pivoted_qr_four_sources(&whitened_signatures, input.relative_rank_tolerance)?;
    if condition_proxy > input.maximum_qr_diagonal_condition_proxy {
        return Err(D0Error::IllConditionedSignatureBasis { condition_proxy });
    }

    let mut projected_residual = [0.0_f64; D0_GENERAL_SOURCE_COUNT];
    for source in 0..D0_GENERAL_SOURCE_COUNT {
        projected_residual[source] = dot_eight(&q[source], &whitened_residual);
    }
    let pivoted_coefficients = solve_upper_four(&r, &projected_residual)?;
    let mut source_coefficients = [0.0_f64; D0_GENERAL_SOURCE_COUNT];
    for pivoted_index in 0..D0_GENERAL_SOURCE_COUNT {
        source_coefficients[permutation[pivoted_index]] = pivoted_coefficients[pivoted_index];
    }

    let mut reconstructed = [0.0_f64; D0_VECTOR_LENGTH];
    for source in 0..D0_GENERAL_SOURCE_COUNT {
        for index in 0..D0_VECTOR_LENGTH {
            reconstructed[index] = source_coefficients[source]
                .mul_add(whitened_signatures[source][index], reconstructed[index]);
        }
    }
    let mut unexplained = [0.0_f64; D0_VECTOR_LENGTH];
    for index in 0..D0_VECTOR_LENGTH {
        unexplained[index] = whitened_residual[index] - reconstructed[index];
    }
    let unexplained_l2 = dot_eight(&unexplained, &unexplained).sqrt();
    if !unexplained_l2.is_finite() {
        return Err(D0Error::NonFiniteDerivedValue);
    }

    let mut active_sources = Vec::new();
    for source in 0..D0_GENERAL_SOURCE_COUNT {
        if source_coefficients[source].abs() > input.minimum_detectable_amplitude {
            active_sources.push(match source {
                0 => ErrorSource::ParameterPerturbation,
                1 => ErrorSource::NumericalBias,
                2 => ErrorSource::MeasurementArtifact,
                _ => ErrorSource::ModelFormDiscrepancy,
            });
        }
    }
    let disposition = if !input.conservation_ledgers_pass {
        D0Disposition::PhysicalLedgerFailure
    } else if !input.within_validation_envelope {
        D0Disposition::OutOfEnvelope
    } else if unexplained_l2 > input.measurement_noise_l2_bound {
        D0Disposition::NotEvaluable
    } else if source_coefficients[0].abs() > input.minimum_detectable_amplitude
        || source_coefficients[1].abs() > input.minimum_detectable_amplitude
    {
        D0Disposition::ParameterOrNumericalErrorDetected
    } else if source_coefficients[2].abs() > input.minimum_detectable_amplitude {
        D0Disposition::MeasurementArtifactUnresolved
    } else if source_coefficients[3].abs() > input.minimum_detectable_amplitude {
        D0Disposition::ModelFormDiscrepancyDetected
    } else {
        D0Disposition::NoSystematicSourceDetected
    };

    Ok(D0GeneralDecompositionReport {
        source_coefficients,
        active_sources,
        reconstructed_systematic_residual_whitened: reconstructed,
        unexplained_residual_whitened: unexplained,
        unexplained_residual_l2: unexplained_l2,
        qr_diagonal_condition_proxy: condition_proxy,
        measurement_noise_l2_bound: input.measurement_noise_l2_bound,
        within_validation_envelope: input.within_validation_envelope,
        conservation_ledgers_pass: input.conservation_ledgers_pass,
        disposition,
    })
}

/// Non-orthogonal source signatures formed by mixing the original four
/// Hadamard directions. The basis is independent and intentionally correlated.
pub fn d0_general_reference_signatures() -> D0GeneralSignatures {
    let basis = reference_signatures();
    let source = [
        basis.parameter_perturbation,
        basis.numerical_bias,
        basis.measurement_artifact,
        basis.model_form_discrepancy,
    ];
    let mixes = [[1.0, 0.2, 0.0, 0.0], [0.15, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.3], [0.0, 0.0, 0.1, 1.0]];
    let mut columns = [[0.0_f64; D0_VECTOR_LENGTH]; D0_GENERAL_SOURCE_COUNT];
    for target in 0..D0_GENERAL_SOURCE_COUNT {
        for component in 0..D0_VECTOR_LENGTH {
            for original in 0..D0_GENERAL_SOURCE_COUNT {
                columns[target][component] =
                    mixes[target][original].mul_add(source[original][component], columns[target][component]);
            }
        }
    }
    D0GeneralSignatures { source_columns: columns }
}

pub fn d0_general_reference_covariance() -> [[f64; D0_VECTOR_LENGTH]; D0_VECTOR_LENGTH] {
    let rho = 0.25_f64;
    let mut covariance = [[0.0_f64; D0_VECTOR_LENGTH]; D0_VECTOR_LENGTH];
    for row in 0..D0_VECTOR_LENGTH {
        for column in 0..D0_VECTOR_LENGTH {
            covariance[row][column] = rho.powi(row.abs_diff(column) as i32);
        }
    }
    covariance
}

fn d0_general_reference_input() -> D0GeneralCaseInput {
    let signatures = d0_general_reference_signatures();
    let coefficients = [1.5, -0.75, 0.5, 2.0];
    let mut residual = [0.0_f64; D0_VECTOR_LENGTH];
    for source in 0..D0_GENERAL_SOURCE_COUNT {
        for component in 0..D0_VECTOR_LENGTH {
            residual[component] = coefficients[source]
                .mul_add(signatures.source_columns[source][component], residual[component]);
        }
    }
    D0GeneralCaseInput {
        residual,
        signatures,
        observation_covariance: d0_general_reference_covariance(),
        measurement_noise_l2_bound: 1.0e-10,
        minimum_detectable_amplitude: 1.0e-10,
        relative_rank_tolerance: 1.0e-12,
        maximum_qr_diagonal_condition_proxy: 1.0e8,
        within_validation_envelope: true,
        conservation_ledgers_pass: true,
    }
}

pub fn d0_general_reference_self_check() -> Result<(), D0Error> {
    let report = analyse_d0_general_case(&d0_general_reference_input())?;
    if (report.source_coefficients[0] - 1.5).abs() > 1.0e-10
        || (report.source_coefficients[1] + 0.75).abs() > 1.0e-10
        || (report.source_coefficients[2] - 0.5).abs() > 1.0e-10
        || (report.source_coefficients[3] - 2.0).abs() > 1.0e-10
        || report.unexplained_residual_l2 > 1.0e-10
        || (report.qr_diagonal_condition_proxy - 1.5236673968369188).abs() > 1.0e-9
    {
        return Err(D0Error::NonFiniteDerivedValue);
    }
    let mut confounded = d0_general_reference_input();
    confounded.signatures.source_columns[1] = confounded.signatures.source_columns[0];
    if !matches!(
        analyse_d0_general_case(&confounded),
        Err(D0Error::RankDeficientSignatureBasis)
    ) {
        return Err(D0Error::NonFiniteDerivedValue);
    }
    Ok(())
}

#[cfg(test)]
mod d0_pivoted_qr_tests {
    use super::*;

    #[test]
    fn correlated_nonorthogonal_signatures_recover_mixed_source_coefficients() {
        let report = analyse_d0_general_case(&d0_general_reference_input()).unwrap();
        assert!((report.source_coefficients[0] - 1.5).abs() < 1.0e-10);
        assert!((report.source_coefficients[1] + 0.75).abs() < 1.0e-10);
        assert!((report.source_coefficients[2] - 0.5).abs() < 1.0e-10);
        assert!((report.source_coefficients[3] - 2.0).abs() < 1.0e-10);
        assert_eq!(report.active_sources.len(), 4);
        assert!(report.unexplained_residual_l2 < 1.0e-10);
    }

    #[test]
    fn full_covariance_whitening_uses_correlated_ar1_measurement_model() {
        let input = d0_general_reference_input();
        let report = analyse_d0_general_case(&input).unwrap();
        assert!(report.qr_diagonal_condition_proxy > 1.0);
        assert!(report.unexplained_residual_l2 < 1.0e-10);
    }

    #[test]
    fn exact_source_confounding_is_rejected_as_rank_deficient() {
        let mut input = d0_general_reference_input();
        input.signatures.source_columns[1] = input.signatures.source_columns[0];
        assert_eq!(
            analyse_d0_general_case(&input).unwrap_err(),
            D0Error::RankDeficientSignatureBasis
        );
    }

    #[test]
    fn near_confounded_source_basis_abstains_above_condition_proxy_limit() {
        let mut input = d0_general_reference_input();
        let original = input.signatures.source_columns[0];
        for i in 0..D0_VECTOR_LENGTH {
            input.signatures.source_columns[1][i] =
                original[i] + 1.0e-9 * input.signatures.source_columns[1][i];
        }
        assert!(matches!(
            analyse_d0_general_case(&input),
            Err(D0Error::IllConditionedSignatureBasis { .. })
        ));
    }

    #[test]
    fn asymmetric_and_non_positive_definite_covariances_are_rejected() {
        let mut asymmetric = d0_general_reference_input();
        asymmetric.observation_covariance[0][1] += 0.1;
        assert!(matches!(
            analyse_d0_general_case(&asymmetric),
            Err(D0Error::AsymmetricCovariance { .. })
        ));
        let mut indefinite = d0_general_reference_input();
        indefinite.observation_covariance[7][7] = 0.0;
        assert!(matches!(
            analyse_d0_general_case(&indefinite),
            Err(D0Error::NonPositiveDefiniteCovariance { .. })
        ));
    }

    #[test]
    fn failed_physical_ledger_cannot_be_repaired_by_correlated_discrepancy_fit() {
        let mut input = d0_general_reference_input();
        input.conservation_ledgers_pass = false;
        let report = analyse_d0_general_case(&input).unwrap();
        assert_eq!(report.disposition, D0Disposition::PhysicalLedgerFailure);
    }

    #[test]
    fn excessive_unexplained_residual_is_not_evaluable() {
        let mut input = d0_general_reference_input();
        input.residual[7] += 0.1;
        input.measurement_noise_l2_bound = 1.0e-4;
        let report = analyse_d0_general_case(&input).unwrap();
        assert_eq!(report.disposition, D0Disposition::NotEvaluable);
    }
}
