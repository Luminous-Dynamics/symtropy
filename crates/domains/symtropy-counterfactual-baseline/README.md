# Symtropy Counterfactual Baseline Qualification

A Rust assurance layer for no-dispatch load baseline validation and conservative event-impact estimation.

## Paradigm

Demand-response impact is counterfactual. The no-dispatch load during a dispatch event is not observed. A model that overpredicts this counterfactual can manufacture a positive reduction even when the site did nothing.

This package therefore requires disjoint no-dispatch calibration and evaluation groups. It computes a lower residual quantile from the calibration partition, then tests lower-bound coverage, error bias and false-positive reductions on the independent evaluation groups. An unqualified model cannot be used to produce an event report.

## v0.1 guarantees

- Forecasts must be issued before the target interval begins.
- Dispatched observations are rejected from baseline calibration/evaluation.
- Calibration and evaluation independence-group IDs cannot overlap.
- Calibration and evaluation source digests cannot be identical.
- One-sided lower residual quantile is calibrated separately by declared operating regime.
- Holdout bias, normalized MAE, RMSE, lower-bound coverage and apparent false-positive reduction rate are reported per regime.
- Every regime must pass the pre-registered contract before event evaluation is enabled.
- Event evaluation uses the lower baseline bound and upper PCC measured-load bound (including the metrology uncertainty supplied by #3772).
- Event and recovery intervals are chronological and contiguous; gross reduction, rebound and net energy remain separate.
- The event receipt binds the exact qualified model identity, regime, forecast issue time, interval, metrology and evidence ID.

## Method and assumptions

For each regime, calibration residuals are `r = observed no-dispatch power − predicted no-dispatch power`. The caller declares a lower-tail alpha. The selected empirical lower residual quantile is applied as:

`baseline lower bound = predicted no-dispatch power + lower residual quantile`

This is a split-calibration-style empirical bound. The independent evaluation fold measures actual coverage; it does not prove exchangeability in future regimes. If the regime, model, hardware, workload mix, cooling or forecast process changes materially, requalify or abstain.

The event response lower bound is:

`baseline lower bound − (measured dispatched power + PCC expanded uncertainty)`

Signed rebound is retained. Summed event energy is reported separately from any capacity-credit or settlement claim.

## Run

```sh
cargo test -p symtropy-counterfactual-baseline
cargo clippy -p symtropy-counterfactual-baseline --all-targets -- -D warnings
```

All current integration fixtures are synthetic known truth. This crate makes model-based M&V estimates; it is not causal identification, market accreditation or ELCC.

## Independence and measurement uncertainty

The calibration tail is estimated from **independence groups**, not raw interval count. Within each calibration group, v0.1 uses the worst conservative residual across its intervals, where the observed no-dispatch power is reduced by its supplied expanded measurement uncertainty. This block-minimum residual prevents a long, autocorrelated session from masquerading as hundreds of independent calibration observations.

The evaluation audit is also grouped: a group passes lower-bound coverage only when every interval in that group is covered; a group is counted as a false-positive reduction if any interval would have minted the contracted minimum response with no dispatch. This is intentionally more conservative than row-wise binomial scoring.

Every observation source digest must match the pinned calibration or evaluation partition digest. Evaluation regimes without a matching calibration regime are rejected rather than silently omitted. The report is an immutable read-only receipt (it is not deserializable into a caller-forged qualification token).

## Event-horizon interpretation

The event report includes point-estimate net energy and a separate sum of conservative interval lower-bound energy contributions. The latter is **not** a joint horizon-wide probability bound: serial and cross-interval dependence must be modeled or validated before attaching a whole-event coverage probability. The supplied event trace must include the full recovery horizon to make rebound accounting meaningful.

## Group-weighted holdout metrics

Bias, normalized bias, MAE, RMSE and normalization scale are now computed by first calculating each independent evaluation group's metric and then weighting the groups equally. A long site-day or session cannot dominate a short site-day merely because it contributes more samples. Coverage is stricter still: every interval in a group must be covered for the group to count as covered; any interval that would mint the contracted response marks the group as a false-positive group.

## Evidence-bearing event and recovery horizon

Event points carry an explicit `Dispatch` or `Recovery` phase. The event-evaluation contract consumes an `EventRecoveryEvidence` object binding:
- exact event start, dispatch end and observation-horizon end timestamps;
- a recovery evidence ID and source digest;
- explicit reconciliation of useful-work obligations; and
- explicit reconciliation of physical resource states.

The trace must start and end at the pinned boundaries, be contiguous, contain at least one dispatch interval and one post-dispatch recovery interval, and never straddle the dispatch-end boundary. A dispatch interval cannot appear after recovery begins. The contracted minimum-response fraction is evaluated over dispatch intervals only, while gross reduction, gross rebound, point-estimate net energy and conservative signed interval energy include both dispatch and recovery. The event cannot qualify unless work and resource-state reconciliation are both true.

This does not establish a joint probability bound over the full horizon. The report explicitly marks event lower bounds as non-joint until cross-interval dependence is separately qualified.
