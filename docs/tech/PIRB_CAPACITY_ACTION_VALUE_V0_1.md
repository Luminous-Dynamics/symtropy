---
title: PIRB Capacity-Constrained Action Value Benchmark
version: 0.1
status: research-stage, synthetic-only
authority: MeasurementOnly
---

# PIRB capacity-constrained action value benchmark v0.1

## Purpose and research frame

A forecast can rank risks accurately while still supporting a poor allocation policy under finite response capacity. Conversely, a policy that appears useful under an assumed action-effectiveness model has not proved that it prevents harm in reality. PIRB therefore evaluates the model-to-policy step separately from forecast skill, operational execution, and causal impact.

This design is consistent with the split between forecast and decision modules in the Barcelona School of Economics' Dynamic Early Warning and Action Model and the multi-stage prevention problem in *If You Only Have a Hammer: Optimal Dynamic Prevention Policy*. They are methodology comparators, not directly comparable performance tables: their assumptions, estimands, response costs and modeled consequences must be aligned before numeric comparisons are meaningful.
- https://bse.eu/research/working-papers/dynamic-early-warning-and-action-model
- https://bse.eu/research/working-papers/if-you-only-have-hammer-optimal-dynamic-prevention-policy

The benchmark is a transparent research calculator, not a deployment authorization.

## Registered policy family

The Draft 2020-12 schema is docs/schemas/pirb-capacity-action-value-v0.1.schema.json. The public fixture is docs/fixtures/pirb-capacity-action-value-synthetic-v0.1.json and contains twelve invented, opaque aggregate records. It is synthetic-only and aggregate-only.

The evaluator compares three predeclared policies over the exact same candidate pool, capacity and scenario assumptions:

1. **No action:** the transparent baseline, with zero actions, costs and assumed avoided harm.
2. **Risk-ranked top-K:** selects up to K rows in descending forecast probability, with record ID as deterministic tie-break.
3. **Expected-net-benefit top-K:** ranks by modeled avoided harm minus action cost, selects only rows with strictly positive modeled net benefit, and never exceeds the same capacity K.

A scenario grid is preregistered over action-cost multipliers, intervention-effectiveness multipliers and capacity levels. Every combination is reported; the evaluator does not choose a favorable scenario after the results are visible.

## Model quantities and algebra

For record \(i\), the benchmark defines:

\[
H_i = p_i \, h_i
\]

where \(p_i\) is the registered forecast probability and \(h_i\) is expected harm per event in a synthetic common value unit. Under assumed effectiveness \(e_i\) multiplied by scenario multiplier \(m_e\), the modeled avoided-harm amount for an action is:

\[
A_i = p_i \, h_i \, e_i \, m_e
\]

and its modeled cost under cost multiplier \(m_c\) is:

\[
C_i = c_i \, m_c
\]

For selected set \(S\), the receipt reports baseline expected harm \(\sum_i H_i\), assumed avoided harm \(\sum_{i\in S} A_i\), action cost \(\sum_{i\in S} C_i\), modeled net benefit \(\sum_{i\in S}(A_i-C_i)\), and residual expected harm \(\sum_i H_i-\sum_{i\in S}A_i\).

These are outputs of the registered assumptions, **not observed prevented events or causal effects**. The evaluator must not upgrade modeled avoidance into real-world impact. A future study needs separately measured action delivery and a credible counterfactual design.

## Observed labels remain a separate lane

Mature known labels are used only for descriptive selected-case diagnostics: known event count, known no-event count, unknown count, precision among selected known labels, and event capture among all known event labels. The metrics are not used to tune the registered policy. Unknown labels remain unknown; abstention, missing labels and modelled harm are never automatically converted to no-event.

A policy may select rows whose outcomes are unknown. Those rows count toward action capacity and cost, appear in the unknown-selected count, and are excluded from known-label precision's denominator. Their unknown status must remain visible.

## Fairness, disclosure and authority limits

Groups are opaque aggregate strata. Group-allocation summaries are suppressed below the registered minimum denominator, and the receipt reports how many groups were suppressed. This threshold is a disclosure control, not a guarantee of privacy or fairness.

The output is advisory and synthetic. It cannot dispatch real responses, identify or target people, or gate access to essential services. A locally legitimate response authority would need its own governance, affected-community review, safety safeguards, data-use permission, operational accountability and independent validation.

## Acceptance gates

- 100% of synthetic schema and semantic negative controls reject invalid target, duplicate IDs, nonmonotone scenario grid, capacity overflow and effectiveness above one.
- Deterministic policy tie-breaking and replay produce identical receipts for the same input/schema/implementation/source commit.
- Every policy uses the same candidate pool, target, forecast vintage and capacity level.
- All cost/effectiveness scenarios are present in the report, including cases where expected-net-benefit policy uses less than the available capacity.
- For every scenario, baseline expected harm, assumed avoided harm, cost, net benefit and residual expected harm satisfy the defined arithmetic identities.
- Unknown labels remain explicitly unknown; observed event capture is separated from model-expected harm and modeled intervention value.
- A passing synthetic receipt cannot claim forecast superiority, action effectiveness, lives/harm prevented, causal impact or deployment authorization.

## Limitations and comparison to industry

The benchmark does not evaluate whether the probability forecasts are calibrated—that belongs to the separate proper-score evaluation. It assumes costs, expected harm and intervention effectiveness are registered constants; it does not estimate them from evidence. It also does not model spillovers, adversarial adaptation, changes in event reporting, intervention delivery failures, interactions between actions, or resource constraints beyond the registered top-K capacity.

For fair industry comparison, the forecast estimand, candidate rows, outcome maturity, capacity, cost/effectiveness assumptions and policy objectives must be identical. A VIEWS binary country-month probability and an ACLED CAST rolling count forecast are not comparable merely because they address related phenomena. The result is a decision-value diagnostic under assumptions—not a SOTA ranking or a causal estimate.

## References and dependencies

- PIRB preregistered evidence ladder: issue #2159
- Selective forecast coverage/abstention: issue #2160 / PR #2293
- Cross-provider estimand comparability: issue #2168 / PR #2170
- Prevention value and impact qualification: issue #2248 / PR #2253
- Capacity-constrained action value and value of information: issue #2275

Public CI remains synthetic-only and aggregate-only. Real evaluation requires authorized data, frozen mature outcomes, locally governed cost and action assumptions, disclosure review, and a separately reviewed impact design.
