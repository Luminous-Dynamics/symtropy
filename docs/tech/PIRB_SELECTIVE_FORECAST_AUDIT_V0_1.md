---
title: PIRB Selective Forecasting, Abstention and Regime-Shift Audit
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB selective forecast audit v0.1

## Goal

An early-warning system should not be forced to emit a probability when its inputs are stale, its target definition has changed, evidence is insufficient, or the observed regime is outside its declared support. The safe alternative is an explicit, reason-coded abstention, visible coverage loss, and a governed escalation path.

**Abstention is not a zero probability, a no-event forecast, permission to suppress warnings, or a reason to withhold aid or essential services.** It means only that this policy did not issue an automated probability for that case.

## Research grounding and comparison boundary

The VIEWS evaluator distinguishes classification/regression tasks and point/sample predictions and defines task-specific metric families; its README notes that some listed metrics are not implemented. PIRB therefore computes this small binary-probability subset directly with formula-level tests, without claiming implementation parity based on metric names: https://github.com/views-platform/views-evaluation.

Selective prediction research evaluates a risk-coverage tradeoff: how error changes as a policy issues forecasts on a larger or smaller fraction of cases. Distribution shift can alter both the reliability and the mix of cases handled, so PIRB reports input/regime strata separately instead of pooling them away. A current early-warning design reference is the Barcelona School of Economics' September 2026 *Dynamic Early Warning and Action Model*, which explicitly separates a conflict forecast module from a decision-making module: https://bse.eu/research/working-papers/dynamic-early-warning-and-action-model.

These references motivate the measurement design. They do not establish that PIRB's selector or its shift labels detect real-world distribution changes correctly.

## Policy contract

The schema is `docs/schemas/pirb-selective-forecast-v0.1.schema.json`; the checked-in example is `docs/fixtures/pirb-selective-forecast-synthetic-v0.1.json`. All rows are invented. Placeholder digests do not identify provider data.

Each registration pins a forecast artifact, outcome snapshot, target ID, forecast/target temporal indices, selection-model digest, pre-outcome feature cutoff, deployed score threshold, preregistered threshold grid and small-group reporting minimum. The deployed threshold must be in a strictly increasing grid; tuning thresholds on held-out labels is outside the contract. Selection features must be cut off strictly before forecast origin; the target period must be later than origin.

The audit derives the expected policy disposition in a deterministic order:
1. Target differs from the registered estimand → `target_mismatch`.
2. Input is stale → `stale_inputs`; input freshness unknown → `unknown_inputs`.
3. Regime is shifted → `out_of_support`; regime unknown → `unknown_regime`.
4. Evidence is insufficient or unknown → `insufficient_evidence`.
5. Selection score is below threshold → `low_selection_score`.
6. Otherwise the forecast may be issued.

The recorded disposition/reason must agree with the registered policy. Issued rows must serve exactly the candidate probability; abstained rows must have a null `served_probability`. The `offline_candidate_probability` is retained only to evaluate the counterfactual full-coverage candidate model, never as a serving value for an abstained row.

An outcome is scored only where the row's target matches the registered target and a mature binary outcome is known. Unknown and immature labels remain null. The receipt reports target mismatch, unknown and immature label counts; no such label is imputed to no-event.

## Measurements

At every preregistered threshold, the aggregate receipt reports:
- case count, issued and abstained counts, overall decision coverage and known-label score coverage;
- Brier score and binary log loss among target-matched, known-outcome cases that would be issued under the policy;
- an explicitly named offline candidate full-coverage reference on all mature, known, target-matched outcomes;
- abstention reason counts; unknown/immature outcome counts among issued and abstained cases;
- opaque group summaries and regime-stratified summaries, suppressing strata below the predeclared minimum group size.

Brier score is the mean of `(p-y)^2`. Log loss is the mean binary negative log likelihood, with probabilities clamped only for numerical evaluation at `1e-15`; the recorded candidate probability is unchanged. The risk-coverage grid is calculated over the thresholds declared before evaluation; no best threshold is picked using test labels.

Coverage has two denominators on purpose. `decision_coverage` is issued cases divided by all supplied cases, so target mismatches and forced abstentions remain visible. `scored_coverage_of_known_target_outcomes` is issued-and-scored mature target-matched rows divided by all mature known target-matched rows. Both are reported so a system cannot look excellent simply by abstaining on hard or unscorable rows.

The output is an aggregate receipt: it includes no per-case forecast probabilities or outcomes. Group metrics are omitted under the preregistered minimum count.

## Acceptance benchmarks

1. **Deterministic policy contract:** 100% of synthetic negative controls have the expected issue/abstain reason.
2. **No hidden predictions:** zero abstained rows expose a served probability; target mismatch always abstains.
3. **No outcome imputation:** unknown and immature outcomes remain null and are never scored as event/non-event.
4. **Pre-registered tradeoff:** every threshold curve point is emitted in declared order, with both coverage denominators, counts and reason codes.
5. **Shift-aware coverage:** report coverage and score separately for known regimes; small groups are suppressed rather than disclosed.
6. **Reproducibility:** repeat evaluation with fixed bytes, schema, selection-model digest and source commit to get identical metrics and receipt commitments.
7. **No winner claim from a curve:** a lower selective risk is meaningful only together with its coverage, eligible-label denominator, abstention reasons, baseline, regime and group behavior.

## Claim ceiling and limits

This is an evaluator for a declared policy, not a distribution-shift detector or an automatic threshold learner. The `regime_status`, `input_status` and `evidence_status` fields are evidence inputs whose external provenance must be separately verified in real research. Synthetic results establish formula and contract behavior only. They do not establish calibration under shift, fairness, safety, skill superiority, response effectiveness or causal impact.

The receipt makes no operational decision and does not authorize use of abstention to limit humanitarian assistance. Live deployment would require local governance, documented human escalation, auditability, disclosure review and a safe fallback.

## Reproduction

```sh
cargo run --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml \
  --bin pirb-selective-forecast-audit -- \
  --input docs/fixtures/pirb-selective-forecast-synthetic-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

## Dependencies

Follows the frozen-outcome, VIEWS offline-import, comparability and hierarchy-coherence work (#2109, #2118, #2160, #2168, #2188, #2191; PRs #2170 and #2190). Issue #2160 is the parent research specification.
