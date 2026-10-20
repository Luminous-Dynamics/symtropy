# EV factory fixed-vs-adaptive lifecycle policies v0.1

**Issue:** [EV-FACTORY-OPTIONS-001 #2448](https://github.com/Luminous-Dynamics/symtropy/issues/2448)  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_adaptive_lifecycle_policies.rs`  
**Input:** the exact synthetic lifecycle catalog and its lifecycle-cost report  
**Status:** value-of-flexibility experiment, not a real procurement recommendation

## Paradigm shift: don't confuse adaptive recourse with perfect foresight

A fixed equipment strategy commits to purchase, lease, rent, or refurbish/redeploy regardless of what happens to the product family. An adaptive strategy waits for an explicitly defined operating signal, then selects among available acquisition routes. The question becomes: **does the value of adapting exceed the costs of sensing, deciding and changing course?**

This evaluator consumes the catalog and the lifecycle report, hashes the exact bytes of both, and rejects them if the report's recorded catalog digest does not match the supplied catalog. The per-family costs come from the already-tested lifecycle cash-flow engine; the adaptive selector itself never reads cost or regret columns.

## Strategies and rules

Four fixed strategies provide the baseline. Four simple adaptive rules select a strategy from a declared profile signal:

| Policy | Explicit signal rule | Decision/measurement overhead |
|---|---|---:|
| Fixed purchase | Purchase new in every family | 0 |
| Fixed lease | Lease in every family | 0 |
| Fixed rent | Short-term rent in every family | 0 |
| Fixed refurbish | Refurbish/redeploy in every family | 0 |
| Adaptive low-activity rent | Rent if observed annual active fraction is below 0.30; otherwise refurbish/redeploy | 1,500 synthetic cost units per family |
| Adaptive high-change lease | Lease if observed product changes are at least 0.70/year; otherwise refurbish/redeploy | 1,200 units per family |
| Adaptive reuse confidence | Refurbish/redeploy if observed reuse success is at least 0.90 and redeployability at least 0.80; otherwise lease | 1,800 units per family |
| Adaptive combined | Low activity → rent; else high change rate → lease; else high reuse/redeployability → refurbish; otherwise lease | 2,500 units per family |
| Perfect-information oracle | Choose the lowest-cost strategy after seeing each family’s realised cost results | 0; **non-deployable lower bound only** |

Thresholds and overheads are declared benchmark assumptions, not measured sensor performance, industrial decision costs or standard thresholds. The family signals stand in for evidence available at a lifecycle decision point. They are not a claim that a future event is perfectly known at the start of plant design.

## Metrics

For every policy/family combination, the report records the selected strategy, selector rule, signal values, lifecycle NPV before decision overhead, overhead, total NPV and regret relative to the cheapest strategy in that same family.

Across families it reports equal-weight mean cost, worst-family cost, maximum regret, average decision overhead and strategy-selection counts. The actionable three-objective Pareto frontier compares only deployable policies over mean cost, worst-family cost and maximum regret; the perfect-information oracle is deliberately excluded so it cannot trivially dominate every practical option. Its lower-bound cost is reported separately.

`value_of_adaptivity_vs_best_fixed_npv_cost_reduction` is signed: positive means the best threshold policy costs less on the declared family set; negative means flexibility and its decision overhead cost more than the best fixed strategy. The perfect-information gap is separate from this realistic adaptivity comparison.

It also perturbs selector observations—not the underlying realised family cash flows—to measure policy fragility:
- active-fraction signal lower by 0.05;
- observed product-change rate higher by 0.15/year;
- observed reuse success and redeployability each lower by 0.05.

Each sensitivity case reports how many policy-family choices change and whether the best deployable policy by mean cost changes. These perturbations are deterministic stress tests, not probabilistic measurement-error models.

## Benchmark protocol

1. Verify the lifecycle catalog is versioned, synthetic and has five unique family profiles plus four strategy types.
2. Generate the lifecycle report from that catalog using `ev_factory_lifecycle_frontier`.
3. Feed both exact files to `ev_factory_adaptive_lifecycle_policies`; validate source digest equality and one cost result for every `(family, strategy)` pair.
4. Compare fixed and adaptive policies with the same family set, same source report and explicitly charged decision overhead. No family ID or per-family outcome is read by adaptive selector logic.
5. Re-run identical inputs and require byte-identical JSON; mutate source digest and catalog inputs as negative controls.
6. Report the perfect-information oracle only as a theoretical lower bound. Never call it an operational policy, SOTA, or a deployable option.
7. Run the same rules against additional scenario families or measured, time-separated observations only after the signal meaning and costs have evidence provenance commensurate with the decision risk.

## Research comparison

The October 2026 paper *Adaptive robust optimization for sustainable assembly line design under product evolution* uses scenario-based robust resource selection/replacement and examines reuse-oriented equipment arrangements such as renting, leasing and salvaging. It reports savings for its own model; these are not targets this prototype claims to reproduce: https://doi.org/10.1016/j.ijpe.2026.110136. Earlier manufacturing real-options work frames flexibility as an option whose value depends on uncertainty and the actions genuinely available later: https://doi.org/10.1016/S0925-5273(01)00128-1 and https://doi.org/10.1016/S0925-5273(01)00143-8.

NIST's manufacturing digital-twin workshop report highlights interoperability, verification/validation/uncertainty quantification, cybersecurity and workforce readiness: https://doi.org/10.6028/NIST.IR.8620. That motivates digest-linked inputs and fail-closed schema checks, but does not mean this tool has achieved validated real-world predictive credibility.

## Limits and claim ceiling

- All lifecycle costs and policy overheads remain `synthetic_cost_unit`; no market quote, currency cost, tax treatment, lease offer or salvage record is represented.
- The policy set is a few hand-specified threshold rules. It is not a full adaptive robust MILP, C&CG algorithm, reinforcement-learning controller or proof of optimal adaptive policy.
- The family profiles are scenario assumptions, not an estimated distribution of product evolution.
- The observation-perturbation tests are one-at-a-time and do not model a calibrated sensor confusion matrix, correlated forecast error or data latency.
- Policy rules should be pre-registered before evaluating any measured holdout; do not tune thresholds on test families and then report the same families as unbiased evidence.
- Economic recommendation authorization is false; production, product-quality, safety, electrical/battery and local regulatory gates remain separate and false/unknown.

No supplier, manufacturer, plant, landlord, integrator or agency was contacted; no quotes were requested.
