use metric_induced_error_envelope_reference::rectangular_operator::RectangularTangentToTangentOperator;
use metric_induced_error_envelope_reference::{
    CartesianQ, Cotangent, DenseY, Metric, MetricProfileV1,
    nonlinear_sensitivity::{
        CurvatureEvidenceV1, NonlinearSensitivityStatusV1, observable_sensitivity,
        propagate_output_error,
    },
    operator::OperatorConditionProfileV1,
};

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    derivative_at_origin: f64,
    second_derivative: f64,
    observable_gradient: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metric_profile = MetricProfileV1::default();
    let operator_profile = OperatorConditionProfileV1 {
        relative_jacobi_tolerance: 1.0e-13,
        relative_rank_tolerance: 1.0e-12,
        maximum_condition_estimate: 1.0e12,
        maximum_jacobi_sweeps: 64,
    };
    let input_metric = Metric::<CartesianQ, 1>::admit([[1.0]], metric_profile)?;
    let output_metric = Metric::<DenseY, 1>::admit([[1.0]], metric_profile)?;

    let cases = [
        Case {
            id: "affine",
            derivative_at_origin: 0.25,
            second_derivative: 0.0,
            observable_gradient: 2.0,
        },
        Case {
            id: "curvature_dominates",
            derivative_at_origin: 0.05,
            second_derivative: 10.0,
            observable_gradient: 0.5,
        },
        Case {
            id: "stiff_nonlinear",
            derivative_at_origin: 2.0,
            second_derivative: 4.0,
            observable_gradient: 1.5,
        },
        Case {
            id: "sign_changing",
            derivative_at_origin: -1.5,
            second_derivative: 6.0,
            observable_gradient: -0.75,
        },
        Case {
            id: "near_flat",
            derivative_at_origin: 0.01,
            second_derivative: 0.4,
            observable_gradient: 3.0,
        },
    ];
    let radii = [1.0e-4, 1.0e-3, 1.0e-2, 2.5e-2, 5.0e-2, 1.0e-1];
    let sample_intervals = 2_000usize;

    println!(
        "case,radius,observed_state_max,state_upper,state_coverage,state_tightness,observed_observable_max,observable_upper,observable_coverage,observable_tightness,status"
    );

    for case in cases {
        let jacobian = RectangularTangentToTangentOperator::<CartesianQ, DenseY, 1, 1>::new([[
            case.derivative_at_origin,
        ]])?;
        let output_curvature = CurvatureEvidenceV1::AnalyticUpperBound {
            second_derivative_norm_bound: case.second_derivative.abs(),
            valid_radius: 1.0e-1,
            evidence_id: format!("analytic-HF-{}", case.id),
        };
        let output_gradient = Cotangent::<DenseY, 1>::new([case.observable_gradient])?;
        let observable_curvature = CurvatureEvidenceV1::AnalyticUpperBound {
            second_derivative_norm_bound: (case.observable_gradient * case.second_derivative).abs(),
            valid_radius: 1.0e-1,
            evidence_id: format!("analytic-Hphi-composed-{}", case.id),
        };

        for radius in radii {
            let output = propagate_output_error(
                &jacobian,
                &input_metric,
                &output_metric,
                radius,
                &output_curvature,
                operator_profile,
            )?;
            let observable = observable_sensitivity(
                &jacobian,
                &input_metric,
                &output_metric,
                &output_gradient,
                radius,
                &observable_curvature,
                operator_profile,
            )?;

            let mut observed_state_max = 0.0_f64;
            let mut observed_observable_max = 0.0_f64;
            for sample in 0..=sample_intervals {
                let normalized = -1.0 + 2.0 * sample as f64 / sample_intervals as f64;
                let x = radius * normalized;
                // Exact synthetic one-dimensional map: F(x) = a*x + 0.5*h*x^2.
                let next = case.derivative_at_origin * x + 0.5 * case.second_derivative * x * x;
                observed_state_max = observed_state_max.max(next.abs());
                observed_observable_max =
                    observed_observable_max.max((case.observable_gradient * next).abs());
            }

            let state_upper = output
                .total_output_error_upper_estimate
                .ok_or("analytic curvature did not produce a state upper estimate")?;
            let observable_upper = observable.total_observable_change_upper_estimate.ok_or(
                "analytic composite curvature did not produce an observable upper estimate",
            )?;

            let state_coverage = observed_state_max <= state_upper + 1.0e-12;
            let observable_coverage = observed_observable_max <= observable_upper + 1.0e-12;
            let state_tightness = if observed_state_max == 0.0 {
                1.0
            } else {
                state_upper / observed_state_max
            };
            let observable_tightness = if observed_observable_max == 0.0 {
                1.0
            } else {
                observable_upper / observed_observable_max
            };
            let status = match output.status {
                NonlinearSensitivityStatusV1::DeclaredAnalyticRemainderBound => "declared_analytic",
                NonlinearSensitivityStatusV1::DeclaredIntervalRemainderBound => "declared_interval",
                NonlinearSensitivityStatusV1::EmpiricalEstimateOnly => "empirical_only",
                NonlinearSensitivityStatusV1::FirstOrderOnly => "first_order_only",
            };

            println!(
                "{},{:.8e},{:.12e},{:.12e},{},{:.8e},{:.12e},{:.12e},{},{:.8e},{}",
                case.id,
                radius,
                observed_state_max,
                state_upper,
                state_coverage,
                state_tightness,
                observed_observable_max,
                observable_upper,
                observable_coverage,
                observable_tightness,
                status
            );

            assert!(
                state_coverage,
                "state envelope failed sampled coverage for {} at radius {}",
                case.id, radius
            );
            assert!(
                observable_coverage,
                "observable envelope failed sampled coverage for {} at radius {}",
                case.id, radius
            );
            assert!(state_tightness.is_finite() && state_tightness >= 1.0 - 1.0e-10);
            assert!(observable_tightness.is_finite() && observable_tightness >= 1.0 - 1.0e-10);
        }
    }

    Ok(())
}
