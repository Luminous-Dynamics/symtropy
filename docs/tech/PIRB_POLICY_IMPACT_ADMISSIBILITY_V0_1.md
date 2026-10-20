---
title: PIRB Forecast-to-Action Policy Impact Admissibility Contract
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB forecast-to-action policy impact admissibility v0.1

## Motivation and paradigm shift

Forecast verification, operational execution, intervention value and causal effect are separate questions. A successful early action may prevent an event that a forecast predicted; scoring the prevented event as an ordinary false positive may penalize useful warnings. Conversely, a decline after intervention is not proof the intervention caused the decline.

The [ACLED CAST methodology](https://acleddata.com/methodology/cast-methodology) is a strong public forecasting/reconciliation reference, while current action-oriented early warning work increasingly joins prediction with a decision layer. PIRB should compare forecast skill, policy value and causal impact only as separate evidence lanes.

Research on early-warning valuation emphasizes costs, consequences, timing and stakeholder objectives—not just accuracy metrics. A useful additional context is the [Dynamic Early Warning and Action Model](https://bse.eu/research/working-papers/dynamic-early-warning-and-action-model) (Barcelona School of Economics, 2026) and the [WFP/OCHA multi-agency impact-evaluation window for anticipatory action](https://www.wfp.org/publications/multi-agency-impact-evaluation-window-anticipatory-action).

## Contract and tool

Schema: `docs/schemas/pirb-policy-impact-registration-v0.1.schema.json`

Synthetic negative-control fixture: `docs/fixtures/pirb-policy-impact-before-after-not-identified-synthetic-v0.1.json`

Rust command: `pirb-policy-impact-admissibility`

This validator determines whether declarations are structurally valid and whether a historical registration contains the minimum design/governance prerequisites to be **eligible for independent review**. It does not validate identification assumptions as true, approve a study, estimate a treatment effect, or prove reduced violence. Even an eligible receipt keeps `causal_effect_estimated=false`, `effect_estimate=null`, `causal_impact_established=false`, and `live_decision_authority=false`.

## Required evidence lanes

- **Forecast lane:** pinned forecast registration, target definition and forecast artifact; held-out forecast scoring stays separate.
- **Action lane:** policy/action definition; records of offer/assignment, delivery, receipt/adherence, cost and no-action baseline.
- **Outcome lane:** primary outcome and measurement process, frozen outcome-release digest, follow-up period, missingness rule, and explicit assessment of reporting shift.
- **Design/counterfactual lane:** named trial or quasi-experimental design, assignment mechanism, comparison strategy, preregistered analysis plan and digest, independent review, counterfactual/estimand, diagnostics and sensitivity plans, treatment timing, and interference/spillover boundaries.
- **Policy and governance lane:** fixed resource capacity, declared policy alternatives including `no_action`, preregistered thresholds, subgroup harm review, data authority, disclosure review and publication review.

A historical registration is eligible for independent review only if it uses a supported randomized or named quasi-experimental design and satisfies those declared prerequisites. Randomized trials require a randomized assignment mechanism. Difference-in-differences, synthetic-control and interrupted-time-series registrations may declare as-if-random or observational assignment but still require a reviewed analysis plan and design-specific assumptions.

A before/after comparison or an observational association is not converted into an identified effect by filling in metadata. If any required gate is absent, the validator rejects a registration claiming independent-review eligibility. It can also accept a conservative `not_identified` status even if some declarations are present; independent design review retains discretion.

## Synthetic negative control

The fixture deliberately describes a synthetic before/after comparison with incomplete adherence evidence and no counterfactual identification. It validates as a **not-identified** registration. It records no effect estimate and makes no external claim. Public CI contains only invented aggregate values and placeholder digests.

Additional Rust negative controls reject:
- claiming causal review eligibility for synthetic evidence;
- promoting an unreviewed before/after design to eligibility;
- an eligible declaration without adherence/delivery and outcome-observation gates;
- missing `no_action` policy comparison;
- follow-up that does not extend beyond forecast issue;
- user-supplied causal estimates (this tool accepts none).

## Benchmark ladder

1. **Contract:** 100% of invalid design, assignment, timing, missingness, counterfactual and governance negative controls fail closed.
2. **Known-truth simulation:** only a future separate estimator benchmark can test bias, interval coverage and policy regret against generated ground truth. This contract tool does not run that simulation.
3. **Policy value:** compare preregistered policies at fixed resource capacity against no-action/current-practice baselines, including costs, delivery, harms, lead time and uncertainty.
4. **External causal evidence:** authorized data, adequate follow-up, identified comparison design, interference analysis, disclosure review and independent reviewers before any causal assertion.
5. **Prospective learning:** record non-delivery, reporting changes, uncertainty, subgroup harms and spillovers; no forecast score authorizes a live intervention automatically.

## Claim boundary

The admissibility receipt is not a causal estimator. It does not prove that design assumptions hold, that outcomes are correctly measured, or that an intervention saved lives or reduced violence. A synthetic test pass proves only contract behavior. When a counterfactual is not identified, impact remains **unestimated**, not zero and not positive.

## References

- [ACLED CAST methodology](https://acleddata.com/methodology/cast-methodology)
- [Dynamic Early Warning and Action Model, BSE Working Paper 1355 (2026)](https://bse.eu/research/working-papers/dynamic-early-warning-and-action-model)
- [WFP/OCHA impact-evaluation window for anticipatory action](https://www.wfp.org/publications/multi-agency-impact-evaluation-window-anticipatory-action)
- [Forecast valuation for action-oriented early warning](https://doi.org/10.1016/j.wace.2018.03.006)
- Tracking issue: #2248
