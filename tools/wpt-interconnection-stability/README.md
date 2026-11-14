# WPT interconnection stability analytic oracle v0

**Status:** executable mathematical known-truth control  
**Research programme:** [#3715 WPT-GRID-STABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/3715)  
**Specification:** [#3716](https://github.com/Luminous-Dynamics/symtropy/pull/3716)

## Purpose

This crate evaluates the eigenvalues of a real, continuous-time 2x2 system:

x_dot = A x

It is the first executable negative-control lane for the WPT network-stability programme. It establishes a simple but essential fact: independently stable subsystems do not guarantee a stable interconnected system.

## The central known-truth counterexample

Each isolated subsystem has pole -1:

A1 = [-1], A2 = [-1].

When coupled, the joint matrix is:

A = [[-1, 3], [3, -1]]

Its exact eigenvalues are +2 and -4. The interconnected system is unstable despite both isolated diagonal subsystems being stable.

A stability evaluator that infers network stability from isolated-unit stability fails this negative control.

## Method

For a real 2x2 matrix, the characteristic polynomial is:

lambda^2 - trace(A) * lambda + det(A) = 0.

The evaluator uses the analytic discriminant and eigenvalue formula. A caller-declared positive margin classifies a system as:

- STABLE_WITHIN_MARGIN: maximum eigenvalue real part is strictly below minus the margin;
- UNSTABLE: maximum eigenvalue real part is strictly above the margin;
- MARGINAL_OR_UNRESOLVED: the maximum real part lies within the margin band.

Non-finite inputs, invalid margins and arithmetic overflow fail closed.

## Known-truth and negative-control tests

The Rust test suite covers:
- decoupled stable subsystems;
- individually stable diagonal terms with an unstable coupled system;
- stable coupled real poles;
- stable complex-conjugate poles;
- near-axis and exact-axis poles;
- a non-normal stable matrix (asymptotic stability is not a transient-amplification bound);
- non-finite input, invalid margin and intermediate overflow.

The crate uses only the Rust standard library. CI runs cargo test --locked -p wpt-interconnection-stability.

## Explicit limitations

This is not a general eigenvalue solver for arbitrary order, a WPT converter model, a multiport impedance/admittance solver, a generalized-Nyquist implementation or a robust-stability proof. It contains no PLL, current controller, LCL filter, resonant network, grid impedance, delay, switching model, nonlinear limiter or parameter uncertainty.

Its classification is for the supplied linear matrix only. The margin is a classification deadband, not a guaranteed robust-stability radius. It does not establish transient performance, large-signal stability, protection adequacy, hardware safety, grid-code compliance, electromagnetic exposure or standards compliance.

## Next research increments

1. Independently validate the analytic fixture against a second implementation.
2. Add n-state state-space eigenanalysis with conditioning and residual evidence.
3. Add explicit port/sign conventions and MIMO return-ratio representations.
4. Implement generalized Nyquist with open-loop right-half-plane pole count and contour assumptions.
5. Add converter/feeder impedance models and declared operating-point linearizations.
6. Add uncertainty sweeps and an independent EMT cross-check.
7. Only then connect stability qualification to dispatch feasibility.

## Claim ceiling

The tests establish the correctness of this small analytic oracle for the supplied fixtures. They do not demonstrate stability of any WPT hardware, controller, grid interconnection or roadway installation.
