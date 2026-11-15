# Differentiable simulation benchmark contract v0.1

This package is the first machine-readable contract slice of [PHYS-DIFFSIM-MATCHED-BENCHMARK-001 #5300](https://github.com/Luminous-Dynamics/symtropy/issues/5300).

## Two axes of derivative identity

Every run records the derivative **referent** (finite computation, converged discrete equations, or continuous flow) separately from the backward **realization** (AD, finite difference, equation-level adjoint, custom adjoint, analytic derivative). These are not interchangeable mathematical objects.

## Run identity

The schema binds the case fixture and semantics, state/control input, units, integration method, timestep/horizon, contact regime, engine version/source/build/compiler/hardware, derivative settings, numerical outputs, covariance results, abstentions, and resource costs.

## Fail-closed evidence rules

Missing or empirical curvature evidence cannot emit a total upper-bound claim. A bound requires an analytic or interval evidence class, a bound-artifact digest, and an explicitly conditional claim. Production authority is structurally prohibited.

The schema does not verify the referenced bytes, compute metrics, or prove physics. Engine adapters and an independent validator must verify actual source and fixture identities.

## Synthetic fixture

`fixtures/synthetic-run-v0.1.json` is shape-only: placeholder digests, no measured timings (unmeasured metrics are null), inconclusive results, missing curvature evidence, no upper bound, and no production authority.

## Baseline families

The benchmark protocol names MuJoCo, Pinocchio, Drake MultibodyPlant and SUNDIALS CVODES. Exact engine releases and commits must be pinned for each measured run. It does not claim one universal ranking across unlike physics or derivative referents.

The error-envelope record binds the numeric upper-bound value separately from its evidence class; a conditional upper-bound claim is rejected unless the emitted value, declared curvature class, artifact digest, and claim label agree.
