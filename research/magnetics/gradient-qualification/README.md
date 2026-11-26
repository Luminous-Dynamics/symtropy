# Magnetic gradient qualification (v0.1)

A zero-dependency Rust evidence contract for testing gradients produced by differentiable magnetic-field kernels before those gradients are allowed to guide inverse design.

## Research question

A differentiable API, a successful JAX trace, or a finite output is not evidence that a gradient is numerically trustworthy. Inverse design can optimize the wrong geometry very efficiently when a derivative is incorrect, unstable under step-size changes, evaluated for the wrong parameterization, or detached from the exact forward model.

This crate qualifies **directional derivatives** against three separately reported quantities:

1. the derivative supplied by the autodiff implementation;
2. a reference derivative from an independently identified analytic, high-precision, or complex-step implementation;
3. a central-finite-difference estimate computed from a preregistered descending step-size sweep of forward objective evaluations.

It selects a finite-difference plateau using only adjacent-step stability—not by looking at the autodiff or reference answer—then compares all three estimates under explicit absolute-plus-relative tolerances.

## Evidence identity

The kernel record binds package/version, source revision, artifact digest, implementation identity, backend/version, device, precision, model class, parameterization, objective and units, and environment digest.

Each directional trial binds the exact design-input digest, unit direction, forward-value evaluator and record, autodiff result, reference derivative, independent reference implementation/revision/artifact/record, and every plus/minus objective sample.

Reference and kernel implementation identities must differ. Record IDs and digests are provenance links, not cryptographic authentication of the underlying tool or physical model.

## Numerical contract

For direction vector v with unit norm and step h > 0, the recorded central-difference estimate is:

    d_h = (f(x + h v) - f(x - h v)) / (2 h)

The step sweep must be finite, unique, strictly descending, and entirely inside the policy's preregistered step interval. A plateau is the longest contiguous run of adjacent estimates satisfying:

    |d_i - d_(i+1)| <= abs_stability_tol
                             + rel_stability_tol * max(|d_i|, |d_(i+1)|)

The plateau is selected without consulting the reference or autodiff values. Ties are resolved deterministically in favor of the earliest (largest-step) run. A minimum number of estimates is required.

For two estimates a and b, normalized error is:

    |a - b| / (abs_tol + rel_tol * max(|a|, |b|))

A trial is accepted only when:
- a stable finite-difference plateau exists;
- the autodiff derivative agrees with the independent reference;
- the plateau median agrees with the independent reference; and
- the autodiff derivative agrees with the plateau median.

All trial outcomes and error metrics are retained. Structurally invalid or mismatched evidence returns an error; numerically failing but well-formed evidence returns a negative trial disposition rather than disappearing from the report.

## Benchmark protocol

For Magpylib Material Response 0.15.0, keep capabilities separate:
- its face-charge Python kernel supports JAX/PyTorch differentiation and JIT;
- the published field kernels include WebAssembly variants and shared 50-digit reference checks, which can serve as field-value references but are not represented as autodiff backends by this contract.

Compare JAX eager/JIT and PyTorch eager for differentiable face-charge cases; use analytic or high-precision reference cases for independent derivative checks. Compare WASM field values separately. Only compare runtime after field-value and derivative tolerances are matched.

The challenge suite should include:
- analytic dipole/cuboid cases with known directional derivatives;
- face-charge cases with a smooth, non-degenerate geometry and independently regenerated reference values;
- rotations, translations, scaling, and source-superposition metamorphic controls;
- near-axis, far-field, close-surface, thin-panel and near-degenerate geometry strata;
- multiple parameterizations, including source position, magnetization direction, sensor position and material parameters only where the selected solver mathematically supports them.

Do not expose gradients to an optimizer outside the validated smooth/model-applicability domain. A finite-difference plateau is a numerical consistency check, not proof of physical truth or differentiability at a material boundary.

## What this crate does not do

It does not execute Magpylib, JAX, PyTorch, FEM or a physical probe. It validates the supplied derivative evidence and its arithmetic/identity contract. It does not authenticate provenance, establish solver applicability, prove a global optimum, infer manufacturing yield, or qualify an encoder. A passing synthetic unit test is not an upstream backend parity result.

The tests in this crate are authored controls. CI must establish formatting, Clippy, tests and workspace compatibility on the exact PR head before implementation checks are called passed.
