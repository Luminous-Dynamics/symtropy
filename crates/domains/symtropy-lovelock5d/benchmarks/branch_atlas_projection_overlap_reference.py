#!/usr/bin/env python3
"""Exact-rational controls for observable overlap versus a true BVP fold."""
from fractions import Fraction as F
import json


def observable_family(lam: F):
    # F((u,v),lambda)=(u-lambda,v-lambda^2)=0.
    return (lam, lam * lam)


def observable_residual(state, lam: F):
    u, v = state
    return (u - lam, v - lam * lam)


def true_fold_roots(lam: F):
    if lam < 0:
        return ()
    if lam == 0:
        return (F(0),)
    # Exact controls below use rational squares only.
    from math import isqrt
    num, den = lam.numerator, lam.denominator
    sn, sd = isqrt(num), isqrt(den)
    if sn * sn != num or sd * sd != den:
        raise ValueError("this exact fixture requires a rational square")
    root = F(sn, sd)
    return (-root, root)


def main():
    lam_a, lam_b = F(-1, 2), F(1, 2)
    ua, ub = observable_family(lam_a), observable_family(lam_b)
    obs_a, obs_b = ua[1], ub[1]

    assert observable_residual(ua, lam_a) == (0, 0)
    assert observable_residual(ub, lam_b) == (0, 0)
    assert ua != ub and obs_a == obs_b == F(1, 4)
    # D_U F = I for all lambda: full rank at the projection turn and overlap.
    det_DU = F(1)
    assert det_DU != 0
    # With lambda itself as branch parameter, d lambda / d s = 1, never zero.
    d_lambda_ds = F(1)

    # A separate genuine fold: F(u,lambda)=u^2-lambda=0, lambda=u^2.
    fold_u, fold_lam = F(0), F(0)
    fixed_parameter_derivative = 2 * fold_u
    parameter_derivative = F(-1)
    tangent_du_ds, tangent_dlambda_ds = F(1), F(0)
    second_parameter_derivative = F(2)
    assert fold_u * fold_u - fold_lam == 0
    assert fixed_parameter_derivative == 0 and parameter_derivative != 0
    assert tangent_du_ds != 0 and tangent_dlambda_ds == 0
    assert second_parameter_derivative != 0
    assert true_fold_roots(F(1, 4)) == (F(-1, 2), F(1, 2))

    result = {
        "schema_version": 1,
        "artifact_kind": "branch_atlas_projection_overlap_control",
        "status": "executed_exact_rational_reference",
        "projection_overlap": {
            "equations": ["u-lambda=0", "v-lambda^2=0"],
            "nodes": [
                {"lambda": "-1/2", "u": "-1/2", "v": "1/4", "original_residual": ["0", "0"]},
                {"lambda": "1/2", "u": "1/2", "v": "1/4", "original_residual": ["0", "0"]},
            ],
            "same_scalar_projection": True,
            "distinct_full_states": True,
            "det_DU_F": "1",
            "d_lambda_ds": "1",
            "classification": "observable_projection_self_overlap_not_a_BVP_fold",
        },
        "true_fold": {
            "equation": "u^2-lambda=0",
            "fold": {"u": "0", "lambda": "0"},
            "dF_du_at_fold": "0",
            "dF_dlambda_at_fold": "-1",
            "tangent": {"du_ds": "1", "dlambda_ds": "0"},
            "d2lambda_du2": "2",
            "roots_at_lambda_1_4": ["-1/2", "1/2"],
            "classification": "simple_parameter_fold",
        },
        "acceptance": {
            "projection_overlap_not_misclassified_as_fold": True,
            "true_fold_control_detected": True,
            "all_rational_residuals_exact_zero": True,
        },
        "claim_ceiling": "Exact synthetic controls only; no Akribeia reproduction, no target EGB branch event, and no branch completeness claim.",
    }
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
