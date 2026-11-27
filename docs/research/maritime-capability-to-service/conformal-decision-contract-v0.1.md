# Decision-time split-conformal and robust-feasibility protocol v0.1

Companion to [MCSC-CONFORMAL-ACTION-001 #7855](https://github.com/Luminous-Dynamics/symtropy/issues/7855), and the Durban reproduction programme [#7847](https://github.com/Luminous-Dynamics/symtropy/issues/7847).

## Research distinction

Predictive marginal coverage and decision feasibility are different properties. A split-conformal interval with nominal 90% marginal coverage does not, by itself, guarantee a feasible berth plan, subgroup coverage, or protected receiver service. The decision layer must evaluate adverse interval bounds against explicit resource and service constraints. The calibration layer must establish that labels were available before the decision cutoff.

## Implemented v0.1 contract

The dependency-free Rust module src/conformal.rs provides integer parts-per-million miscoverage, model-fit/calibration/decision clocks, immutable model/schema/calibration SHA-256 identifiers, unique calibration observation identities, prediction-issued and label-available timestamps, exact finite-sample absolute-residual split-conformal rank arithmetic, explicit unbounded output when the order-statistic rank exceeds n, applicability-aware coverage claims, checked interval construction, single-example coverage assessment, and a conservative single-deadline gate.

The finite-sample order statistic is the one-based rank ceil((n + 1) * (1 - alpha)). If that rank exceeds n, the quantile is unbounded. The implementation never substitutes the largest observed residual as a finite quantile when the prescribed rank is unavailable.

## Time and leakage rules

- Model fit must finish before the calibration window begins.
- Calibration predictions must be issued after model fit and inside the declared calibration window.
- Each calibration label must become available after its prediction and no later than the decision cutoff.
- The calibration window must end no later than the decision cutoff.
- Observation IDs must be unique.
- Model, feature-schema and calibration-data digests must have valid lowercase SHA-256 encoding.

The module checks chronology and digest encoding; it does not authenticate the bytes or prove that feature/model commitments are truthful. The evidence-owning adapter must calculate canonical digests and preserve owner provenance.

## Applicability and claim ceiling

Only ExchangeabilitySupported emits a nominal marginal-coverage claim. KnownDistributionShift and Unknown retain the empirical calibration radius for analysis but emit no nominal coverage claim. Their intervals cannot pass the robust-deadline gate; the result is Indeterminate.

This does not prove exchangeability. The caller must justify the calibration protocol, sampling design, temporal dependence assumptions and target regime. Split conformal does not promise distribution-free conditional coverage for every vessel type, terminal, season or disruption regime.

## Decision gate v0.1

Given a finite interval for arrival time, an upper bound on handling duration and a service deadline, the single-deadline robust gate checks whether interval.upper + handling_duration_upper <= deadline.

- Feasible: upper-bound arrival plus upper-bound handling meets the deadline and declared exchangeability applicability is supported.
- Infeasible: the conservative upper bound exceeds the deadline.
- Indeterminate: the conformal rank is unbounded, applicability is shifted/unknown, or endpoint arithmetic cannot be safely represented.

This primitive does not model berth non-overlap, quay-crane capacity, tidal restrictions, yard congestion, channel limits, common-cause outages or multi-stage service state. Those remain responsibilities of the scheduling and MCSC model.

## Required benchmark reporting

- exact calibration sample count and finite-sample rank;
- nominal target coverage only where assumptions are supported;
- empirical marginal coverage and confidence intervals;
- interval width/sharpness and miss severity;
- subgroup coverage by vessel class, call size, terminal, season and disruption regime;
- chronological/blocked split results versus random splits;
- calibration window length, model revision and feature schema;
- drift detection delay and recalibration cost;
- rate of unbounded/indeterminate intervals;
- downstream schedule feasible rate, deadline misses, objective and compute budget.

Never report coverage without sample counts and interval width. Never hide a small-sample unbounded result. Never infer conditional coverage from pooled marginal coverage.

## Next increment

Extend the single-deadline gate to action-specific robust feasibility under multiple resource constraints, then connect it to the finite R1/R2 benchmark. The extension must preserve exact service denominators, common-information non-anticipativity, hard constraints and compute budgets. It must not convert a shifted or unknown calibration state into an automatic pass.

## Claim ceiling

A passing synthetic test establishes rank arithmetic and declared chronology/decision-gate semantics only. It does not establish calibration quality on Durban vessel calls, an actual 90% coverage guarantee, improved scheduling, live port performance or authorization to act.