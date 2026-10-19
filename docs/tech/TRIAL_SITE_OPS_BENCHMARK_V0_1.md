# Clinical-Trial Site Operations Benchmark v0.1

**Status:** synthetic-first benchmark contract, not an operational clinical system.
**Issue:** [TRIAL-SITE-OPS-001](https://github.com/Luminous-Dynamics/symtropy/issues/2192)
**Fixture:** [TRIAL_SITE_OPS_V0_1.json](../../benchmarks/fixtures/TRIAL_SITE_OPS_V0_1.json)

## 1. Research question

Can a site-assessment method make better prospective trial-operations decisions by separating **resources, dynamic execution capability, and outcomes**, while avoiding circular self-reported forecasts, temporal leakage, poorly calibrated enrollment predictions, and hidden access/equity trade-offs?

This is distinct from the existing geospatial site-selection substrate and essential-medicine supply simulation. The v0.1 benchmark evaluates operational readiness, capability diagnosis, support choices, and forecast integrity. It does not evaluate treatment efficacy or individual patient eligibility.

## 2. Industry and research baseline

- Ninh, Vu & Butler, *Beyond prediction: an evidence-based framework for assessing site selection decisions*, published 12 September 2026, identifies circularity in site self-assessed feasibility projections and distinguishes site inputs, dynamic capabilities and outputs: https://link.springer.com/article/10.1007/s43441-026-01043-6
- Gao et al., *Matching clinicians with clinical trials using AI*, *Nature Health* (6 March 2026), presents DocTr for clinician/trial matching with recommendation accuracy, demographic fairness and operational efficiency objectives: https://www.nature.com/articles/s44360-026-00073-6
- ClinicalTrials.gov API v2 offers public study registry data: https://clinicaltrials.gov/data-api/api. Registry records and site recruitment status are observations, not ground truth for current local execution capability, future enrollment, or participant access.
- FDA's December 2025 guidance addresses representative clinical-trial enrollment: https://www.fda.gov/regulatory-information/search-fda-guidance-documents/enhancing-participation-clinical-trials-eligibility-criteria-enrollment-practices-and-trial-designs

The benchmark must not assume public registry data contains site-level staff utilization, follow-up capability, complete screening denominators, intervention exposure, or verified trial outcomes. A later public-data adapter needs field mapping, freshness, applicability, license and provenance checks.

## 3. Paradigm shift: from ranking sites to improving trial execution

A conventional site-selection process may rank sites using prior enrollment, projected enrollment, questionnaires, investigator profile, and proximity. The proposed evaluation separates four decisions:

1. **Readiness:** what can this site execute for this specific protocol at the declared as-of time?
2. **Bottleneck diagnosis:** is the limiting factor staff capacity, workflow coordination, follow-up, access barriers, competing-study burden, missing evidence, or something else?
3. **Support allocation:** which feasible intervention could address a stated bottleneck, at what cost and with what uncertainty?
4. **Monitoring and reassessment:** what has changed since the previous assessment, and what new evidence would alter the decision?

A site-reported enrollment projection may be an input signal, but it cannot be counted as independent proof of that same projection. Post-selection outcomes must not leak into prospective ranking. A site is not inherently “good” or “bad”: its capability is protocol-specific and time-dependent.

## 4. Cross-project composition

- **Sol-Atlas:** consume the existing evidence-first site-selection kernel and provenance/freshness/spatial interfaces ([PR #129](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/129), [issues #130, #131, #135](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/135)). Do not make a second geospatial site-selection engine.
- **Symthaea:** may supply forecasts, competing hypotheses and value-of-information estimates. Confidence must be externally scored against held-out outcomes, not accepted because the model reports it.
- **Symtropy:** owns the deterministic scenario corpus, chronology/leakage checks, operational bottleneck and support-decision comparison, plus replayable evaluator outputs.
- **Mycelix:** can later provide permissioned provenance receipts for authorized evidence and site-support events. v0.1 contains no participant-level records.
- Existing medicine-supply work is only a dependency where investigational-product availability is explicitly part of a scenario; it must not be mistaken for site capability.

## 5. Frozen synthetic scenario set

All sites, protocol identifiers, amounts, forecasts and outcomes are invented for arithmetic and invariant testing only.

| Case | Challenge | Minimum correct behavior |
|---|---|---|
| CAPABILITY-GAP-001 | High resources and self-reported target, weak recruitment workflow and follow-up | Diagnose a workflow/follow-up constraint and keep support selection separate from site ranking |
| CIRCULAR-FORECAST-002 | A high site-reported enrollment estimate conflicts with capability and evidence strength | Do not treat the self-report as verified outcomes |
| DYNAMIC-CHANGE-003 | Staffing and competing-study load change over time | Reassess from the as-of evidence; do not rely on stale prior capability |
| REPRESENTATION-ACCESS-004 | Sites trade projected volume against access barriers | Report separate volume and access/representativeness outcomes; do not hide trade-offs in an opaque scalar |
| STALE-INCOMPLETE-005 | Key readiness evidence is stale, unknown or has a missing denominator | Abstain on a conclusive rank when critical evidence is unresolved |
| OUTCOME-LEAKAGE-006 | A post-selection outcome appears in the feature set | Exclude it from the prospective candidate projection and reject it if marked prospectively admissible |

The reference evaluator now emits two transparent baselines per synthetic case: B1 uses the site-reported enrollment target as an uncalibrated forecast; B3 emits a readiness heuristic from prospectively admissible, verified measurements and abstains when minimum evidence is unavailable. It also reports access-barrier and accessible-contact-channel signals separately, without a composite quality score. This does **not** constitute a validated enrollment-prediction model or establish which intervention will work.

## 6. Comparison baselines

- **B0 — synthetic oracle:** hidden fixture outcomes and declared bottlenecks, available to the evaluator only.
- **B1 — self-reported target baseline (implemented):** uses the site's reported projected enrollment as a deliberately weak transparent comparator; reports mean absolute error against synthetic truth, not a calibrated real-world forecast.
- **B2 — static profile-only baseline (specified, not yet implemented):** uses pre-selection resources and self-reports but ignores evolving execution capability.
- **B3 — capability-aware readiness heuristic (implemented):** computes a within-case score from admissible verified direct measurements (staff capacity and workflow, with available follow-up / competing-load dimensions), returns an explicit abstention when minimum dimensions are unavailable, and keeps access dimensions separate. It is a heuristic readiness signal, not an enrollment forecast or evidence of causal benefit from site support.

Compare all methods on identical as-of cutoffs, protocols, source availability and evaluation denominators. Where real retrospective data becomes available, use site-, sponsor-, protocol-, region- and time-held-out splits; random row splitting may leak repeated site identities and protocol patterns. A method should be compared with the published system most relevant to the task—for example, DocTr for clinician/trial matching—not misrepresented as directly comparable when tasks or inputs differ.

## 7. Metrics

Keep dimensions visible rather than manufacturing one “site quality” score.

- **Forecasting:** B1 reports MAE against the synthetic outcomes. B3 does not emit an enrollment forecast; its pairwise directional concordance with synthetic enrollment is only a small-fixture ranking diagnostic. Brier/log score, calibration curves and interval coverage are reserved for a future method with explicit predictive distributions.
- **Operations:** site activation delay, time to first participant enrolled, enrollment and retention rates, screen-failure burden and protocol deviations with explicit time windows and denominators.
- **Decision utility:** time to identify the binding constraint; fraction of recommendations with admissible evidence; independently adjudicated usefulness of support actions.
- **Access and representation:** gaps relative to an explicitly declared eligible-population frame, participant-facing barriers, and group-specific uncertainty. Aggregate demographic rates do not establish individual eligibility or consent.
- **Cost and feasibility:** support expenditure, coordinator workload, site activation delay and missed targets, reported separately.
- **Robustness:** outcomes by protocol phase, competing-study load, geography, evidence missingness, freshness, and temporal drift.
- **Governance:** post-selection feature leakage, provenance coverage, unexplained abstentions, and false certainty when evidence is unresolved.

Forecast performance and causal benefit from support actions are different claims. A real-world claim that an intervention improves enrollment requires a suitable comparative design, not merely a correlation between intervention and outcome.

## 8. Fail-closed invariants

1. No participant-level data or sensitive medical records in v0.1.
2. Self-reported potential is not verified performance evidence.
3. Recruiting status and geographic proximity do not establish actual capacity or access.
4. Missing follow-up, zero enrollment, suspended recruitment, and absent registry updates remain distinguishable.
5. No prospective candidate projection may include evidence observed after the as-of cutoff.
6. Evidence marked unknown, stale or conflicted cannot silently become a positive capacity measurement.
7. No demographic proxy determines individual eligibility, treatment assignment, or consent.
8. No autonomous eligibility, recruitment, consent, clinical, or regulatory action.
9. No synthetic forecast or support effect is reported as a real trial result.

## 9. Proposed acceptance gates (not results)

- All six frozen fixture IDs validate; any duplicate site/evidence ID or dangling truth reference fails closed.
- 100% of future-dated evidence marked prospectively admissible is rejected.
- 100% of unknown, stale, or conflicted evidence remains visibly non-verified in the candidate projection.
- 100% of candidate projections exclude hidden oracle outcomes and expected-invariant labels.
- Input-order permutations produce identical canonical projection and receipt bytes.
- All denominators are explicit; no dropping sites or unresolved outcomes without a declared protocol rule.
- The B3 heuristic abstains on the frozen stale/incomplete case rather than converting stale/unknown staffing into zero or verified capacity.
- The baseline receipt contains B1 and B3 outputs and separate access signals for all six cases.
- Do not claim industry or SOTA superiority until there is a governed dataset, a protocol-matched benchmark, independent evaluation and uncertainty intervals for the paired comparison.

## 10. Claim ceiling

Passing the v0.1 validator means only that the frozen synthetic contract and its chronology/leakage invariants pass. It does not establish enrollment prediction accuracy, trial acceleration, improved representativeness, better site selection, regulatory compliance or causal benefit. Those require qualified data, trial-operations review, and independent evaluation.