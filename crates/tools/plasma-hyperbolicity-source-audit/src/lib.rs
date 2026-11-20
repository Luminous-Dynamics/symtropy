//! Fail-closed audit for characteristic-polynomial source consistency.
//!
//! For the convention det(lambda I - A), an n×n matrix has a monic
//! characteristic polynomial of degree n, with coefficient of lambda^(n-1)
//! equal to -trace(A). This checker validates only those necessary identities;
//! it does not compute a determinant or authenticate a source transcription.

pub const SCHEMA_VERSION: &str = "luminous.plasma-hyperbolicity-source-audit.v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacteristicConvention {
    LambdaIdentityMinusMatrix,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourcePairRequest {
    pub schema_version: String,
    pub source_title: String,
    pub source_reference: String,
    pub matrix_equation_id: String,
    pub polynomial_equation_id: String,
    pub convention: CharacteristicConvention,
    pub matrix_dimension: usize,
    pub polynomial_degree: usize,
    pub leading_coefficient: f64,
    pub matrix_trace: f64,
    /// Printed coefficient of lambda^(matrix_dimension - 1).
    pub coefficient_lambda_n_minus_1: f64,
    /// Absolute tolerance in the source's normalized coefficient units.
    pub absolute_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    UnsupportedSchemaVersion,
    EmptySourceIdentity(&'static str),
    ZeroMatrixDimension,
    ZeroPolynomialDegree,
    InvalidTolerance,
    NonFiniteCoefficient(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditViolation {
    PolynomialDegreeDiffersFromMatrixDimension {
        matrix_dimension: usize,
        polynomial_degree: usize,
    },
    CharacteristicPolynomialNotMonic {
        leading_coefficient: f64,
        expected: f64,
        residual: f64,
    },
    TraceCoefficientMismatch {
        printed_coefficient: f64,
        expected_coefficient: f64,
        residual: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditDisposition {
    Consistent,
    Inconsistent { violations: Vec<AuditViolation> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourcePairAuditReport {
    pub schema_version: String,
    pub source_title: String,
    pub source_reference: String,
    pub matrix_equation_id: String,
    pub polynomial_equation_id: String,
    pub convention: CharacteristicConvention,
    pub matrix_dimension: usize,
    pub polynomial_degree: usize,
    pub leading_coefficient: f64,
    pub matrix_trace: f64,
    pub printed_coefficient_lambda_n_minus_1: f64,
    pub expected_coefficient_lambda_n_minus_1: f64,
    pub trace_coefficient_residual: f64,
    pub absolute_tolerance: f64,
    pub disposition: AuditDisposition,
}

/// Check necessary dimension / monicity / trace coefficient identities.
///
/// Under det(lambda I - A), the characteristic polynomial is monic of degree n
/// and its next coefficient is -trace(A). A failure is a contradiction in the
/// supplied source pair, not proof of which printed equation should be edited.
pub fn audit_source_pair(
    request: &SourcePairRequest,
) -> Result<SourcePairAuditReport, InputError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(InputError::UnsupportedSchemaVersion);
    }
    for (name, value) in [
        ("source_title", request.source_title.as_str()),
        ("source_reference", request.source_reference.as_str()),
        ("matrix_equation_id", request.matrix_equation_id.as_str()),
        ("polynomial_equation_id", request.polynomial_equation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(InputError::EmptySourceIdentity(name));
        }
    }
    if request.matrix_dimension == 0 {
        return Err(InputError::ZeroMatrixDimension);
    }
    if request.polynomial_degree == 0 {
        return Err(InputError::ZeroPolynomialDegree);
    }
    if !request.absolute_tolerance.is_finite() || request.absolute_tolerance < 0.0 {
        return Err(InputError::InvalidTolerance);
    }
    for (name, value) in [
        ("leading_coefficient", request.leading_coefficient),
        ("matrix_trace", request.matrix_trace),
        (
            "coefficient_lambda_n_minus_1",
            request.coefficient_lambda_n_minus_1,
        ),
    ] {
        if !value.is_finite() {
            return Err(InputError::NonFiniteCoefficient(name));
        }
    }

    let expected_coefficient_lambda_n_minus_1 = -request.matrix_trace;
    let trace_coefficient_residual =
        request.coefficient_lambda_n_minus_1 - expected_coefficient_lambda_n_minus_1;
    let monic_residual = request.leading_coefficient - 1.0;

    let mut violations = Vec::new();
    if request.polynomial_degree != request.matrix_dimension {
        violations.push(
            AuditViolation::PolynomialDegreeDiffersFromMatrixDimension {
                matrix_dimension: request.matrix_dimension,
                polynomial_degree: request.polynomial_degree,
            },
        );
    }
    if monic_residual.abs() > request.absolute_tolerance {
        violations.push(AuditViolation::CharacteristicPolynomialNotMonic {
            leading_coefficient: request.leading_coefficient,
            expected: 1.0,
            residual: monic_residual,
        });
    }
    if trace_coefficient_residual.abs() > request.absolute_tolerance {
        violations.push(AuditViolation::TraceCoefficientMismatch {
            printed_coefficient: request.coefficient_lambda_n_minus_1,
            expected_coefficient: expected_coefficient_lambda_n_minus_1,
            residual: trace_coefficient_residual,
        });
    }

    let disposition = if violations.is_empty() {
        AuditDisposition::Consistent
    } else {
        AuditDisposition::Inconsistent { violations }
    };

    Ok(SourcePairAuditReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        source_title: request.source_title.clone(),
        source_reference: request.source_reference.clone(),
        matrix_equation_id: request.matrix_equation_id.clone(),
        polynomial_equation_id: request.polynomial_equation_id.clone(),
        convention: request.convention,
        matrix_dimension: request.matrix_dimension,
        polynomial_degree: request.polynomial_degree,
        leading_coefficient: request.leading_coefficient,
        matrix_trace: request.matrix_trace,
        printed_coefficient_lambda_n_minus_1: request.coefficient_lambda_n_minus_1,
        expected_coefficient_lambda_n_minus_1,
        trace_coefficient_residual,
        absolute_tolerance: request.absolute_tolerance,
        disposition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        matrix_dimension: usize,
        polynomial_degree: usize,
        trace: f64,
        next_coefficient: f64,
    ) -> SourcePairRequest {
        SourcePairRequest {
            schema_version: SCHEMA_VERSION.into(),
            source_title: "Synthetic characteristic identity fixture".into(),
            source_reference: "synthetic://source-pair/v1".into(),
            matrix_equation_id: "matrix-A".into(),
            polynomial_equation_id: "poly-det-lambdaI-minus-A".into(),
            convention: CharacteristicConvention::LambdaIdentityMinusMatrix,
            matrix_dimension,
            polynomial_degree,
            leading_coefficient: 1.0,
            matrix_trace: trace,
            coefficient_lambda_n_minus_1: next_coefficient,
            absolute_tolerance: 1.0e-12,
        }
    }

    #[test]
    fn two_by_two_known_answer_satisfies_trace_identity() {
        // A = diag(2, -1), det(lambda I - A) = lambda^2 - lambda - 2.
        let report = audit_source_pair(&request(2, 2, 1.0, -1.0)).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Consistent);
        assert_eq!(report.expected_coefficient_lambda_n_minus_1, -1.0);
    }

    #[test]
    fn five_by_five_zero_trace_known_answer_is_consistent() {
        let report = audit_source_pair(&request(5, 5, 0.0, 0.0)).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Consistent);
    }

    #[test]
    fn degree_six_cannot_be_characteristic_polynomial_of_five_by_five_matrix() {
        let report = audit_source_pair(&request(5, 6, 0.0, 0.0)).unwrap();
        let AuditDisposition::Inconsistent { violations } = report.disposition else {
            panic!("degree mismatch must be rejected");
        };
        assert!(violations.iter().any(|violation| matches!(
            violation,
            AuditViolation::PolynomialDegreeDiffersFromMatrixDimension {
                matrix_dimension: 5,
                polynomial_degree: 6,
            }
        )));
    }

    #[test]
    fn nonzero_lambda_four_coefficient_conflicts_with_zero_trace_five_by_five_matrix() {
        let report = audit_source_pair(&request(5, 5, 0.0, -3.25)).unwrap();
        let AuditDisposition::Inconsistent { violations } = report.disposition else {
            panic!("trace coefficient mismatch must be rejected");
        };
        assert!(violations.iter().any(|violation| matches!(
            violation,
            AuditViolation::TraceCoefficientMismatch {
                printed_coefficient,
                expected_coefficient,
                ..
            } if *printed_coefficient == -3.25 && *expected_coefficient == 0.0
        )));
    }

    #[test]
    fn article_eq_64_and_eq_66_printed_pair_is_inconsistent() {
        // Eq. (64) as printed is 5x5 with zero diagonal. Eq. (66) as printed
        // gives the lambda^4 coefficient -2(1 + 9 sqrt(2) + sqrt(3)).
        let lambda_four = -2.0 * (1.0 + 9.0 * 2.0_f64.sqrt() + 3.0_f64.sqrt());
        let report = audit_source_pair(&request(5, 6, 0.0, lambda_four)).unwrap();
        let AuditDisposition::Inconsistent { violations } = report.disposition else {
            panic!("journal Eq. 64 / Eq. 66 pair is inconsistent as printed");
        };
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn article_eq_65_and_eq_67_printed_pair_is_inconsistent() {
        // Eq. (65) as printed is 5x5 with zero diagonal. Eq. (67) as printed
        // gives the lambda^4 coefficient -4(1 + 7 sqrt(2)).
        let lambda_four = -4.0 * (1.0 + 7.0 * 2.0_f64.sqrt());
        let report = audit_source_pair(&request(5, 6, 0.0, lambda_four)).unwrap();
        let AuditDisposition::Inconsistent { violations } = report.disposition else {
            panic!("journal Eq. 65 / Eq. 67 pair is inconsistent as printed");
        };
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn non_monic_determinant_expression_is_rejected() {
        let mut req = request(3, 3, 0.0, 0.0);
        req.leading_coefficient = 2.0;
        let report = audit_source_pair(&req).unwrap();
        assert!(matches!(
            report.disposition,
            AuditDisposition::Inconsistent { ref violations }
                if violations.iter().any(|item| matches!(
                    item,
                    AuditViolation::CharacteristicPolynomialNotMonic { .. }
                ))
        ));
    }

    #[test]
    fn nonzero_trace_requires_negative_trace_as_next_coefficient() {
        let report = audit_source_pair(&request(4, 4, 2.5, -2.5)).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Consistent);
    }

    #[test]
    fn coefficient_residual_within_absolute_tolerance_is_accepted() {
        let mut req = request(3, 3, 0.0, 0.5e-12);
        req.absolute_tolerance = 1.0e-12;
        assert_eq!(
            audit_source_pair(&req).unwrap().disposition,
            AuditDisposition::Consistent
        );
    }

    #[test]
    fn coefficient_residual_above_tolerance_is_rejected() {
        let mut req = request(3, 3, 0.0, 2.0e-12);
        req.absolute_tolerance = 1.0e-12;
        assert!(matches!(
            audit_source_pair(&req).unwrap().disposition,
            AuditDisposition::Inconsistent { .. }
        ));
    }

    #[test]
    fn source_equation_identity_is_required() {
        let mut req = request(3, 3, 0.0, 0.0);
        req.matrix_equation_id.clear();
        assert_eq!(
            audit_source_pair(&req),
            Err(InputError::EmptySourceIdentity("matrix_equation_id"))
        );
    }

    #[test]
    fn zero_dimension_or_degree_is_rejected() {
        assert_eq!(
            audit_source_pair(&request(0, 1, 0.0, 0.0)),
            Err(InputError::ZeroMatrixDimension)
        );
        assert_eq!(
            audit_source_pair(&request(1, 0, 0.0, 0.0)),
            Err(InputError::ZeroPolynomialDegree)
        );
    }

    #[test]
    fn non_finite_coefficients_and_tolerance_are_rejected() {
        let mut non_finite = request(2, 2, 0.0, 0.0);
        non_finite.coefficient_lambda_n_minus_1 = f64::NAN;
        assert_eq!(
            audit_source_pair(&non_finite),
            Err(InputError::NonFiniteCoefficient(
                "coefficient_lambda_n_minus_1"
            ))
        );
        let mut invalid_tolerance = request(2, 2, 0.0, 0.0);
        invalid_tolerance.absolute_tolerance = f64::INFINITY;
        assert_eq!(
            audit_source_pair(&invalid_tolerance),
            Err(InputError::InvalidTolerance)
        );
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let mut req = request(2, 2, 0.0, 0.0);
        req.schema_version = "future".into();
        assert_eq!(
            audit_source_pair(&req),
            Err(InputError::UnsupportedSchemaVersion)
        );
    }

    #[test]
    fn non_finite_trace_and_leading_coefficient_are_rejected() {
        let mut bad_trace = request(2, 2, f64::NAN, 0.0);
        assert_eq!(
            audit_source_pair(&bad_trace),
            Err(InputError::NonFiniteCoefficient("matrix_trace"))
        );
        bad_trace.matrix_trace = 0.0;
        bad_trace.leading_coefficient = f64::INFINITY;
        assert_eq!(
            audit_source_pair(&bad_trace),
            Err(InputError::NonFiniteCoefficient("leading_coefficient"))
        );
    }
}
