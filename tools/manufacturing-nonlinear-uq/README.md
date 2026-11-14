# Manufacturing nonlinear measurement-uncertainty benchmark

This benchmark compares first-order linearized uncertainty propagation, second-order Taylor approximation and deterministic distribution propagation for synthetic nonlinear measurement models.

The distribution quadrature is deterministic equal-probability midpoint quadrature over truncated Normal components. It is **not random Monte Carlo** and is not claimed to be a complete JCGM 101 implementation.

## Why this exists

ISO/IEC Guide 98-3:2008/Amd.1:2026 (published July 2026) addresses nonlinearity in measurement models. The 2026 amendment makes clear that higher-order terms may be needed when nonlinearity is significant. BIPM lists the GUM amendment, JCGM 101 Monte Carlo propagation, JCGM 102 multiple outputs and JCGM 106 conformity assessment as distinct guidance documents.

The important engineering question is not only whether a first-order approximation predicts the output mean accurately. It is whether the approximation changes a guarded conformity decision or hides tail/domain probability that a full distribution calculation preserves.

## Methods

### First-order comparator

For input mean mu and standard deviation u, it reports:

    y_1 = f(mu)
    u_1 = abs(f'(mu)) * u

It uses a local Normal output approximation for a directly comparable diagnostic conformity probability. On mixture inputs this comparator deliberately collapses the first two input moments and is expected to be vulnerable to multimodality.

### Second-order comparator

For a single Normal input, it reports:

    y_2 = f(mu) + 0.5 * f''(mu) * u^2
    u_2^2 = f'(mu)^2 * u^2 + 0.5 * f''(mu)^2 * u^4

This is the second-order Taylor variance approximation for the declared scalar normal fixtures. It is not applied to mixture inputs.

### Deterministic distribution propagation

For each Normal component, it uses deterministic equal-probability midpoint quantiles over [-8 sigma, +8 sigma], with a declared residual tail mass. It evaluates the full nonlinear function at every node and retains probability weights.

Invalid-domain nodes are not removed and renormalized. Their probability is retained as unresolved mass. The output histogram integrates exactly to valid-domain mass; represented plus unresolved mass must equal one.

For a closed specification [LSL,USL], it reports:

    p_lower = represented valid mass inside specification
    p_upper = min(1, p_lower + invalid-domain mass + residual input-tail mass)

A decision uses externally frozen synthetic thresholds (accept >= 0.95, reject <= 0.05). These are benchmark policy values only, not real engineering acceptance thresholds.

## Frozen synthetic models

- Linear control: f(x)=2x+1, X~Normal(1,0.2), specification [2,4].
- Curvature: f(x)=x^2, X~Normal(1,0.2), with exact moments E[Y]=1.04 and Var[Y]=0.1632.
- Exponential/lognormal: f(x)=exp(x), X~Normal(0,0.5), with analytic lognormal moment and quantile references.
- Reciprocal domain: f(x)=1/(1-x), X~Normal(0.8,0.2); x>=1 is invalid. Its probability must remain unresolved.
- Bimodal exponential: X is an equal mixture of Normal(-1,0.1) and Normal(1,0.1), transformed by exp(x). The first-two-moment Gaussian approximation is expected to hide the central low-density gap and can change a specification decision.
- Correlated two-input product: exact moments for X*Y under a declared bivariate Normal covariance; includes a positive-semidefinite covariance gate.

All cases are numerical fixtures, not actual instruments, materials or manufactured products.

## Refinement and metrics

Every scalar scenario runs at 256, 1024, 4096 and 16384 quadrature nodes per mixture component. Report:
- valid, invalid-domain, tail and total unresolved mass;
- conditional valid-domain output mean and standard deviation;
- conditional valid-domain 2.5/5/50/95/97.5 percentiles;
- lower/upper conformance probability;
- first-/second-order approximation values and decisions;
- whether the linearized decision diverges from the distribution-propagated decision;
- a 32-bin output histogram whose unnormalized masses sum to valid-domain mass.

## Build

    rustc --edition=2024 -D warnings --test tools/manufacturing-nonlinear-uq/nonlinear_uq.rs -o /tmp/nonlinear-uq-tests
    /tmp/nonlinear-uq-tests
    rustc --edition=2024 -D warnings tools/manufacturing-nonlinear-uq/nonlinear_uq.rs -o /tmp/nonlinear-uq
    /tmp/nonlinear-uq --verify
    /tmp/nonlinear-uq --csv

## Claim ceiling

This is a deterministic synthetic numerical benchmark. It does not qualify an instrument, provide an empirical measurement likelihood, establish posterior calibration, validate actual product conformity or authorize release. Its outputs are research fixtures for the existing Symtropy uncertainty and posterior/conformity owners.
