# WPT Pareto Safety v0

An independent, dependency-free Rust reference for engineering design ranking
where safety and operating limits are hard constraints and measurements have
explicit uncertainty intervals.

## Core rule

A design with excellent efficiency is not allowed to compensate for a proven
exposure or thermal-limit violation. Constraint evaluation happens first.
Only designs proven feasible can enter the Pareto front.

Candidate dispositions:

- `Feasible`: all declared hard constraints are proven satisfied and all
  required objective/constraint measurements are present.
- `Infeasible`: at least one hard constraint is proven violated.
- `Uncertain`: no violation is proven, but at least one interval overlaps a
  hard limit.
- `NotEvaluable`: a required objective or hard-constraint metric is missing
  and no hard violation has already been proven.

A missing measurement is never zero and never safe.

## Interval semantics

For an upper-bound constraint x <= L:

- Safe if interval.upper <= L.
- Violated if interval.lower > L.
- Uncertain otherwise.

For a lower-bound constraint x >= L:

- Safe if interval.lower >= L.
- Violated if interval.upper < L.
- Uncertain otherwise.

Intervals must be finite and ordered. An exact reported value can be
represented as a zero-width interval. Unknown uncertainty must not be replaced
with a zero-width interval because that would manufacture precision.

## Pareto semantics

Objectives declare either Maximize or Minimize. There is no weighted sum.

A robust dominance claim requires the full intervals to establish no-worse
performance on every objective and strict improvement on at least one:

- Maximize: A.lower >= B.upper.
- Minimize: A.upper <= B.lower.

Overlapping intervals do not justify a forced ranking. Only Feasible
candidates are compared for the feasible Pareto front. The remaining candidate
dispositions remain visible in the portfolio result.

This conservative interval rule can leave many candidates non-dominated. That
is deliberate: uncertain evidence should not be turned into a false ordering.

## WPT example metrics

Possible objectives:

- useful load power (maximize);
- DC-to-load efficiency (maximize);
- shielding loss (minimize);
- coupler mass (minimize);
- thermal rise (minimize);
- misalignment tolerance and availability (maximize).

Possible hard constraints:

- electric- and magnetic-field exposure;
- temperature and component current/voltage ratings;
- EMC and minimum service requirements.

No universal numeric safety limit is built in. A caller must supply an
applicable standard's value, measurement procedure, frequency weighting,
geometry and units. This package does not determine regulatory compliance.

## Run

From this directory:

    cargo fmt --all -- --check
    cargo test --locked
    cargo clippy --all-targets --locked -- -D warnings
    cargo run --release --locked --bin front_demo

The demonstration uses synthetic illustrative values only. Its thresholds are
not regulatory values, and no physical WPT system is being evaluated.

## Qualification status

Twelve Rust unit tests are authored, including non-compensatory safety
violations, interval overlap, missingness, inclusive boundaries, robust
dominance, overlap non-dominance, feasible-only fronts, deterministic ordering,
duplicate identifiers and invalid intervals.

The tests have not yet been compiled or executed in this authoring session.
No CI or physical safety qualification is claimed.
