# Decision Assurance Comparator Protocol v0.1

**Status:** research protocol and benchmark targets; no external SOTA win is claimed.  
**Owner:** #6595 / #6598 for typed assurance composition; #6887 for sequential finite-sample evidence; #6958/#6997 for decision feasibility, regret and non-anticipative acquisition; #6996/#7013 for action-universe closure and independent domain falsification.  
**Claim ceiling:** the protocol specifies falsifiable comparisons. Synthetic correctness is not physical qualification, model truth or operational authority.

## 1. Research thesis

The strongest opportunity is not to build another FMEA editor, requirements tracker, robust optimizer or assurance-case notation. It is to connect them through an **evidence execution plane**:

`domain discovery → admissibility → decision quality → evidence acquisition → independent checking → scoped invalidation → requalification`

The plane must answer four questions separately:

1. **What can happen?** Is the candidate action/world universe adequate to a declared, independently challenged domain?
2. **What should be done?** Which feasible policy minimizes decision loss under uncertainty, including worst-case and regime-specific regret?
3. **When should we look or change?** Which observation has decision value, and has operating change made the incumbent decision materially suboptimal?
4. **What remains justified?** Which exact downstream claims are still current after a source, scope, model, policy or authority change?

A change in the world is not automatically a reason to re-optimize. A detectable statistical shift can be decision-irrelevant. Conversely, a subtle shift can change the feasible set or reverse the preferred action. The trigger should be a decision-adequacy test, not a generic drift alarm.

## 2. Paradigm shifts to test

### A. From traceability to executable evidence dependencies

Mature tools already provide requirements traceability, model links, consistency checks, versioning and change-impact navigation. The research target is to add machine-executable, exact-version dependencies between heterogeneous evidence producers and decision claims.

Each edge must pin the consumed producer digest and scope digest. A changed source invalidates its descendants; unrelated branches remain current. An invalidation report must list the exact causal path, not only say “recheck project.” This is an evidence build graph, not just a trace matrix.

### B. From global freshness to bitemporal, scoped currentness

Separate:
- effective time: when the evidence applies in the modeled world;
- knowledge time: when the system could legitimately know the evidence;
- source revision: the exact producer / artifact version;
- scope: subject, configuration, mode, mission profile, environment and validity.

This prevents late-arriving evidence from being silently backdated into an earlier decision and prevents a current timestamp from rescuing an out-of-scope receipt.

### C. From distribution drift to operational suboptimality

Use a RADAR-style baseline: test whether the incumbent decision's optimality gap under the current distribution exceeds a predeclared, decision-relevant margin, accounting for preference uncertainty and switching cost.

Compare against KS / MMD / generic OOD or changepoint alarms. They answer whether data changed; the decision-adequacy test asks whether the change matters to the action. Do not silently replace the exact finite-world / sequential guarantees owned by #6887 with an asymptotic monitor. Treat the methods as complementary layers and state assumptions separately.

### D. From parameter certainty to decision identification

The system need not recover every hidden parameter if all surviving admissible worlds imply the same decision. It should stop acquiring information when the decision is identified, and abstain when the model family is inadequate or the surviving worlds imply incompatible decisions.

Compose #6887's anytime-valid world-survival process with #6835's finite decision-set projection. A mathematical guarantee cannot compensate for an unqualified physical observation channel.

### E. From one “readiness score” to a vector of proof obligations

Never collapse decision quality, safety, calibration, domain adequacy, evidence currentness, physical validity, and authority into one weighted score. Report independent blockers and claim ceilings. Operational authority must remain separately evidenced and never be synthesized by a software evaluator.

## 3. Method comparators

Comparisons must use matched problem instances, observation budgets, optimization solvers, compute, data splits and evaluation code. These methods solve related but non-identical tasks; report their native metrics as well as shared outcomes.

| Lane | Baselines to reproduce | What the baseline contributes | What Symtropy must test |
|---|---|---|---|
| Predict-then-optimize | MSE / ERM, two-stage predictor + optimizer | Conventional production baseline | Regret gap vs predictive accuracy; constraint violations |
| Decision-focused learning | SPO+, PFYL, DBB, LAVA, PEAR | Learns predictions for downstream decision quality; PEAR projects prediction error onto active-constraint tangent space for efficient regret gradients | Normalized regret, regret under constraint shift, training time, solver calls, robustness assumptions |
| Robust decision-focused learning | Set-based robust regret (ellipsoid / sparse / hybrid), robust optimization baselines | Worst-case regret under bounded coefficient error | Worst-case and CVaR regret; conservatism; feasibility; ambiguity-set misspecification |
| Distributionally robust DFL | Wasserstein-DRO and GenDFL / distribution-robust baselines | Robustness to specified distributional shift | Held-out regime regret, ambiguity-radius sensitivity, runtime and calibration |
| Decision adequacy monitoring | RADAR-style inverse-optimization / regret monitor | Detects when an incumbent action has become materially suboptimal under shift | Type-I error, power, detection delay, false alarms on decision-irrelevant shift, switching cost |
| Generic shift monitoring | KS / MMD / energy-distance / registered changepoint detector | Detects distribution change without requiring an action model | Alert burden at matched power; decision-irrelevant alerts; missed harmful shifts |
| Sequential evidence | Fixed-sample likelihood tests, EIG-per-cost, fixed grids, maximin KL-per-cost; #6887 anytime-valid policy | Information acquisition and stopping alternatives | Time-uniform coverage, false singleton decisions, probes to decision identification, resource cost |
| Assurance-case structure | SACM / GSN-style argument models and typed graph validation | Explicit claim, context, assumption, argument and evidence structure | Claim-type substitutions, scope/freshness enforcement, seeded-blocker escape rate, reviewer effort |
| Industrial safety engineering | Ansys medini analyze workflows | FMEA/FMECA, FMEDA, HAZOP, FTA, RBD, SysML-linked safety analysis and standards-oriented process | Same requirements-to-evidence change-impact exercises; missed/extra downstream rechecks; reviewer time |
| Industrial lifecycle traceability | Siemens Polarion Requirements workflows | Versioned/global configurations, change requests, traceability from source-code changes to requirements, reports and notifications | Same change-impact corpus; selective invalidation precision/recall; evidence provenance and replayability |

Do not claim to outperform PEAR on gradient efficiency unless reproducing its LP/QP tasks and baselines. Do not claim to replace Ansys medini's domain-specific FMEA/FTA/FMEDA workflows or Polarion's lifecycle/version-control capability. The proposed edge is an interoperable, source-pinned assurance and decision layer across such systems.

## 4. Benchmark design

### Tier 0 — exact finite oracle

Small exhaustive worlds, finite actions, exact rational losses and exhaustive policy trees. Use a clairvoyant selector only as an unattainable upper bound; never expose hidden world IDs to candidate policies.

Required outputs: exact feasible set, optimal action, policy tree, regret, worst-case regret, decision-equivalence classes and exact invalidation closure.

### Tier 1 — adversarial semantic mutations

Mutate one assumption at a time and in correlated groups:
- a new feasible action appears but two enumerators omit it together;
- an upstream capability, authority role, resource envelope or active interval changes;
- evidence payload changes without updating a receipt pin;
- scope changes while content stays byte-identical;
- an omitted dependency edge hides a descendant from invalidation;
- producer revocation or late evidence arrives after a decision cutoff;
- loss model changes while feasibility should remain current;
- unrelated evidence changes and must not invalidate the decision branch;
- a decision-irrelevant distribution shift triggers generic drift monitors but not regret adequacy;
- a small decision-relevant shift reverses the optimum;
- an incumbent action becomes suboptimal only after a delayed regime transition;
- an observation channel has common-cause failure or the true world lies outside the model family.

### Tier 2 — stochastic known-truth benchmark

Use preregistered conditional distributions, multiple regimes, nonstationarity, preference uncertainty, switching costs, constrained optimization and overlapping observation supports. Freeze data-generation code, seeds, training/evaluation split and model family before confirmatory runs.

### Tier 3 — semi-synthetic external data

Reproduce selected published tasks with pinned datasets and baselines (shortest path, knapsack, energy scheduling, resource allocation). Preserve the original method's assumptions and include out-of-regime held-out sets. Publish scripts, hardware, solver versions, licences and raw result files.

### Tier 4 — physical / operational evidence

Only after Tier 0–3 pass: hardware-in-the-loop or field campaigns with calibrated instruments, independently qualified capture, configuration identity, time synchronization, ground truth and a separate authority process. Synthetic receipts cannot enter this tier as physical evidence.

## 5. Primary metrics — no composite score

### Decision and optimization
- normalized mean regret and median regret;
- worst-case regret and CVaR at registered tail levels;
- infeasible action rate and maximum constraint violation;
- Pareto regret across safety, cost, latency and resource use;
- performance under objective, constraint, support and distribution shift;
- regret relative to exact finite oracle and best reproduced comparator.

### Decision adequacy / re-optimization
- family-wise / campaign-wise false re-optimization probability;
- power at a preregistered minimum worthwhile regret gap;
- average and p95 detection delay after harmful shifts;
- false alarms under detectable but decision-irrelevant shifts;
- avoided switching cost and cumulative regret while retaining an incumbent;
- missed harmful-shift rate and duration of stale decisions.

### Sequential observation
- time-uniform true-world survival / decision-set coverage;
- false singleton decision commitment probability over the full campaign;
- expected and worst-case probes to singleton decision;
- cost in physical trials, exposure, time, compute and energy;
- correct abstention, unnecessary abstention and model-family inadequacy detection.

### Evidence graph and invalidation
- downstream stale-claim escape rate (target: zero);
- changed-descendant recall (target: 100% on seeded complete graphs);
- unrelated-branch invalidation rate (target: zero on negative controls);
- dependency graph omission detection under an independent graph oracle;
- precision / recall of the minimal invalidation set;
- invalidation latency, fanout, maximum path and recomputation work avoided;
- exact receipt/source/scope pin mismatch detection;
- deterministic replay and Rust/independent-oracle parity.

### Human and operational cost
- median / p95 reviewer time to find seeded defects;
- reviewer false-negative and false-positive rates;
- evidence authoring and maintenance effort;
- compute, memory, wall time and solver calls at 10 / 100 / 1,000 / 10,000 / 100,000 graph nodes;
- integration effort and data-export / API compatibility with existing engineering systems.

## 6. Prospective benchmark gates

Targets below are research goals, not achieved results.

| Gate | Target |
|---|---:|
| False complete assurance bundles on seeded blockers | 0 |
| Seeded dependency mutation escape on a complete graph | 0 |
| Changed descendants omitted from invalidation | 0 |
| Unrelated-branch invalidation on controls | 0 |
| Source / scope / dependency pin mismatch detection | 100% |
| Independent oracle mismatch on finite fixtures | 0 |
| Action-universe common-mode omission escape on bounded oracle cases | 0 |
| False singleton decision on exact ambiguity / optional-stopping controls | 0 |
| Authority grants from benchmark software | 0 |
| Permutation/replay discrepancies | 0 |
| Harmful-shift power | ≥95% at the registered minimum worthwhile regret gap |
| Sequential false re-optimization probability | ≤ registered campaign-wise α |
| False alerts on decision-irrelevant shift | improve over generic shift baselines at matched power |
| Runtime / memory scaling | publish p50/p95/p99, no hidden asymptotic claims |

For empirical error ceilings, report exact one-sided binomial confidence bounds, not a bare zero count. With zero failures, roughly 299 independent representative trials are needed for a 95% upper bound below 1%; 2,995 for 0.1%; and 29,956 for 0.01%. Independence and representativeness must be justified. Repeated seeds from one generator family are not independent real-world evidence.

## 7. How to compare against industry tools fairly

Use identical, vendor-neutral challenge packages and documented product versions. Ask each system to perform the same tasks:
1. import the same requirement / model / evidence manifest;
2. introduce one source change, one scope-only change, one revoked receipt and one changed decision domain;
3. identify claims and artifacts requiring re-review;
4. preserve unaffected branches;
5. show provenance, history, reviewer-visible rationale and export fidelity;
6. repeat offline from an archived snapshot.

Score change-impact recall and precision, stale-evidence escape, unrelated invalidation, auditability, export/API interoperability, reviewer time, setup effort and runtime. Separate native supported features from custom scripting and external services. Do not equate product marketing estimates with independently measured productivity gains.

Standards / frameworks are interoperability targets, not competitors to defeat:
- OMG SACM 2.3 is formal; SACM 2.4 beta was published September 2026. Map concepts and export/import where practical.
- Goal Structuring Notation (GSN) remains a structured assurance argument method.
- NASA-STD-7009B supplies model/simulation V&V and credibility context.
- Domain-specific standards and competent engineering authorities retain their own approval roles.

## 8. Literature and product anchors

- Yamao et al. (UAI 2026), [Robust Decision-Focused Learning via Worst-Case Regret Minimization](https://proceedings.mlr.press/v337/yamao26a.html): observation-error uncertainty sets and Wasserstein ambiguity for distributional shift; official [implementation](https://github.com/isct-nakatalab/UAI2026_Robust_Decision-Focused_Learning_via_Worst-Case_Regret_Minimization).
- Lee, Jin & Lee (ICML 2026), [Decision-Focused Learning via Tangent-Space Projection of Prediction Error (PEAR)](https://proceedings.mlr.press/v306/lee26q.html): regret-gradient geometry under locally stable active constraints; compare its assumptions and solver efficiency on matched LP/QP tasks.
- Zheng, Wiberg & Zhu (29 August 2026 preprint), [Deciding When to Decide: Testing Operational Suboptimality Under Distributional Shift (RADAR)](https://arxiv.org/abs/2608.29465): inverse-optimization preference inference and regret-based testing for when to re-optimize; use as a major comparator, not an assumed identical method.
- OMG, [Structured Assurance Case Metamodel (SACM)](https://www.omg.org/spec/SACM): SACM 2.4 beta (September 2026) plus formal 2.3.
- Ansys, [medini analyze](https://ansys.synopsys.com/products/safety-analysis/ansys-medini-analyze) and [2026 R1 user guide](https://ansyshelp.ansys.com/public/Views/Secured/medini/v261/en/pdf/Ansys_medini_analyze_User_Guide.pdf): model-based safety analysis, FMEA/FMECA, FMEDA, HAZOP, FTA, RBD and traceability.
- Siemens, [Polarion Requirements](https://www.siemens.com/en-us/products/polarion/requirements/): versioning/global configurations, change requests, requirement-to-source traceability, reports and notifications.

## 9. Integration ownership

- #6595 / #6598 owns typed assurance claims, scope matching and assurance-bundle composition.
- The new invalidation adapter in this branch owns only version-pinned dependency invalidation; it does not authenticate upstream claims or prove graph completeness.
- #6996 / #7013 own bounded action-universe closure and independent domain falsification.
- #6958 / #6997 own scenario regret and non-anticipative feasibility discovery.
- #6887 owns the anytime-valid sequential evidence / decision-set bridge.
- #7105 owns finite-world incumbent retention/re-optimization adequacy under explicit regret, materiality and switching-cost thresholds; it consumes #6887/#6958/#7001 and does not replace RADAR's stochastic guarantees.
- The stacked #7105 loss-model bridge composes the finite compatible-world receipt (#6835) with #6939's coherent joint loss-model scenario manifest. It checks planned/observed scenario inventory and exact rational-to-integer projection; it does not claim #6887's sequential guarantee until a separate adapter exists.
- #3588 / #6127 / #6129 remain reusable sequential-process and policy owners.
- #6519 owns trust roots, credentials, signer roles and external authority semantics.

The next integration milestone is a receipt adapter that turns each producer's exact source commit, artifact digest, scope digest, validity interval and typed claim ID into dependency pins. A graph completeness challenge must be separate from graph invalidation: perfect invalidation on an incomplete graph still misses stale descendants.
