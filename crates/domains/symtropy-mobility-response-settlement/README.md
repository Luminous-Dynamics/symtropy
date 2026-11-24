# Symtropy Mobility Response Settlement Evidence Bridge

A read-only reconciler for four independently produced reports: dispatch tracking, PCC metrology, counterfactual baseline/rebound, and mobility-to-network deliverability.

## What this bridge does

This crate consumes serialized reports from the existing owners. It does not duplicate their calculations:

- PR #3649: product-specific response latency, signed delivered reduction, sustained tracking, overshoot, ramp and underdelivery.
- PR #3772: covariance-aware corrected PCC powers, clock alignment, expanded uncertainty and signed energy.
- PR #3807: no-dispatch baseline qualification, interval lower bounds, dispatch impact, full recovery horizon and rebound.
- PR #7313: mobility offer identity, reserve/lease/journal receipts and locational DC network screen.

It checks exact identities, source repository path/commit shape, canonical report-content digests, timestamps, PCC/baseline interval alignment, dispatch evidence IDs, recovery coverage, arithmetic consistency and the exact upstream gate set. It preserves all four source reports as opaque inputs and returns one cross-report reconciliation summary.

## Four quantities that must not be collapsed

1. **Requested** response: the offer's requested MW/MWh.
2. **Instructed / tracked** response: product command and actual signed import reduction relative to the declared baseline trace.
3. **Measured PCC** response: calibrated meter result with covariance, clock and uncertainty evidence.
4. **Counterfactual net impact**: lower no-dispatch baseline minus measured dispatched power over the complete event and recovery horizon, including rebound.

Locational DC deliverability is a separate cap and is not a measurement. Actual post-event service and energy-settlement values still require the owners' qualified reports and external authorization.

## Run

    cargo run -p symtropy-mobility-response-settlement --bin mobility_response_settlement_screen -- crates/domains/symtropy-mobility-response-settlement/fixtures/mobility_response_settlement_v0_1.json

Tests and lint:

    cargo test -p symtropy-mobility-response-settlement --locked
    cargo clippy -p symtropy-mobility-response-settlement --all-targets --locked -- -D warnings

## Frozen synthetic known answer

The four-report fixture represents a 10 kW two-second instruction with two subsequent recovery intervals:

| Quantity | Expected |
|---|---:|
| Requested energy | 0.0055556 kWh |
| Signed dispatch-tracking delivery | 0.0055556 kWh |
| PCC point net after rebound | 0.0054167 kWh |
| Counterfactual net after recovery | 0.0053056 kWh |
| Conservative technical lower bound | 0.0051944 kWh |
| Lower bound is a joint event probability | false |
| Source artifact signatures verified by this bridge | false |
| Market / settlement / payment / dispatch authority | all false |

These values exercise report reconciliation and arithmetic only. All source report values are synthetic; no physical delivery or market eligibility is implied.

## Claim and trust boundaries

The bridge validates declared source PR numbers, source paths, commit-SHA format and canonical JSON report-content digests. It does not authenticate GitHub, verify cryptographic signatures, or prove that a producer generated a claimed report or digest. It also does not independently reproduce the source solvers, establish causal identification, qualify AC power flow, compute ELCC, or issue settlement/payment authority.

Interval-summed lower bounds are not a joint confidence bound unless the source methodology separately qualifies cross-interval dependence. The bridge preserves the baseline owner's joint-probability flag and the sub-interval transient caveats from metrology and response tracking.
