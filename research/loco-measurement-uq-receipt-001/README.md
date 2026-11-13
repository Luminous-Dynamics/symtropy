# LOCO-MEASUREMENT-UQ-RECEIPT-001

Status: initial dependency-free Rust reference for robotics measurement-uncertainty receipt completeness.

Parent: Symtropy [LOCO-JOINT-METROLOGY-001 #4783](https://github.com/Luminous-Dynamics/symtropy/issues/4783). Research child: [#5062](https://github.com/Luminous-Dynamics/symtropy/issues/5062).

## Ownership

This adapter consumes uncertainty-budget and distribution receipts. It does not calculate GUM uncertainty, covariance, Monte Carlo distributions, physical residuals, actuator response, thermal state or energy closure.

Compose with:
- #2995 — evidence-backed uncertainty-budget lineage patterns.
- #3972 — correlated distribution propagation and nonlinear measurement equations.
- #3612 / #3615 / #4044 — physical energy boundary and instrumentation.
- #4179 / #4206 / #4610 — actuator loss, response and thermal evidence.
- #4783 — joint metrology and the actual physical experiment.
- #4889 / PR #4907 — normalized cross-repository evidence packet.

## Implemented contract

- Exact mission, robot, actuator, controller, scenario, test-manifest and measurement-window binding.
- Measurement identity binds raw artifact, instrument configuration, calibration receipt, measurand, unit and scale.
- Every uncertainty receipt binds to exactly one measurement and its exact artifact, instrument, calibration, unit and scope.
- Conservative deterministic bounds and probabilistic coverage intervals are distinct types.
- Monte Carlo coverage requires an explicit convergence receipt.
- Every budget component must be assigned to a dependence group.
- Every dependence group must carry a joint-model receipt, an evidence-backed independence receipt, or remain Unknown. There is no default-to-independence behavior.
- Derived results enumerate exact input measurement and uncertainty-receipt sets.
- Upstream Unknown/Inconclusive/Rejected states are never promoted.
- Report never grants actuation authority or conformity certification.

The reference validates evidence identity/completeness only. It does not validate that a calibration, covariance, independence determination, distribution, model, measurement, or probability statement is physically true.

## Run

    cargo fmt --manifest-path research/loco-measurement-uq-receipt-001/reference/Cargo.toml -- --check
    cargo test --locked --manifest-path research/loco-measurement-uq-receipt-001/reference/Cargo.toml
    cargo clippy --locked --manifest-path research/loco-measurement-uq-receipt-001/reference/Cargo.toml --all-targets -- -D warnings

The 17 deterministic tests are authored known-answer/adversarial cases and remain unqualified until the exact-head Rust 1.96 workflow passes.

## Research basis

- BIPM/JCGM lists JCGM 100:2008 Amendment 1:2026 on nonlinearity, JCGM 101 Monte Carlo distribution propagation, and JCGM 102 multiple-output propagation: https://www.bipm.org/en/web/guest/publications/guides
- JCGM 101:2008: https://www.bipm.org/en/doi/10.59161/jcgm101-2008
- NIST AMS 100-85 (2026-10-05), risk-scaled VVUQ and metrology: https://doi.org/10.6028/NIST.AMS.100-85

## Claim ceiling

No physical result, robot safety claim, conformity assessment, certification, or superiority claim is made. The next evidence upgrade is an instrumented joint run with raw DAQ files, calibration certificates, clock maps, uncertainty budgets, and complete physical boundaries.
