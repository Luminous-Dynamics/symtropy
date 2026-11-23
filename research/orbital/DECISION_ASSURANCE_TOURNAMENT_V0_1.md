# Decision-assurance tournament v0.1

**Status:** benchmark protocol proposal; targets are prospective.  
**Related work:** #6939 joint loss-scenario regret, #6946 per-regime calibration, #6958 scenario-dependent feasibility, #6956 observable-query policy trees, #6959 / PR #6996 scope-bounded action-universe closure.  
**Evidence class:** synthetic algorithmic conformance until the external/physical validation programme is independently conducted.

## Research thesis

The benchmark should evaluate the complete *decision contract*, not just the optimizer or prediction set:

`problem boundary → evidence admissibility → uncertainty set → feasible actions → observable information → non-anticipative policy → consequence model → calibrated claim → assurance/abstention`.

An algorithm may achieve exact regret over the wrong action universe, optimize a query that is unsafe to collect, rely on a hidden state unavailable at decision time, or produce a calibrated statement for the wrong deployment population. A tournament must attack every link, including the assumptions supplied to the algorithm.

This is not a claim that Symtropy is already state of the art. The hypothesis to test is that explicit closure, observation, feasibility, consequence, calibration and evidence boundaries can compose into a stronger *auditable decision system* than methods that report only one component-level guarantee.

## Baseline families

| Family | Representative methods / papers | What the tournament must compare |
|---|---|---|
| Nominal decision-focused learning | Predict-then-optimize, SPO+/decision-focused learning | Nominal regret, decision regret under prediction error, out-of-distribution degradation |
| Robust decision-focused learning | Yamao et al., UAI 2026 | Worst-case regret under coefficient observation error and Wasserstein deployment shift |
| Uncalibrated adversarial regret | Kurtz & van Parys, GARO, 2026 preprint | Worst-case/mean trade-off and absolute/relative regret guarantees across uncertainty-set sizes without assuming a calibrated prediction-error radius |
| Conformal robustness control | Hu et al., ICLR 2026 | Robustness at fixed risk with prediction-set size / conservatism as a separate cost |
| Multi-distribution conformal prediction | Yang & Jin, ICML 2026 | Finite-sample coverage across multiple source distributions and mixtures; set size and worst-regime behavior |
| Split / multi-regime conformal | Split conformal; max-p multi-distribution methods | Exchangeability scope, per-regime false-feasible rate, calibration sample demand, rare-regime behavior |
| Distributionally robust optimization | Wasserstein, phi-divergence, credal / ambiguity-set baselines | Sensitivity to ambiguity-set misspecification and distribution shift |
| Decision-dependent information acquisition | Ley et al., OR Spectrum 2026; ROCU / OCU formulations | Paid query cost, residual uncertainty, budget deflection, and action quality |
| Adaptive recourse | Two-stage robust optimization, K-adaptability, exhaustive policy-tree oracle | Exactness vs policy count, observation timing, constraint uncertainty, non-anticipativity |
| Enumerative closure | Single registry + digest; duplicate code paths; independent registries; grammar-generated enumerators | False-closure rate, missed in-scope action rate, common-cause detection, enumeration cost |
| System assurance | NIST SP 800-160 Vol. 1 Rev. 1 assurance cases; domain-specific standards such as DO-178C/DO-333 | Claim traceability, assumptions, evidence integrity, independent review, lifecycle change control |

**Core literature (2026):**

- [Yamao et al., “Robust Decision-Focused Learning via Worst-Case Regret Minimization,” UAI 2026](https://proceedings.mlr.press/v337/yamao26a.html). It explicitly addresses coefficient-observation error and deployment distribution shift with uncertainty and Wasserstein ambiguity sets.
- [Kurtz & van Parys, “Globalized Adversarial Regret Optimization: Robust Decisions with Uncalibrated Predictions,” 2026 preprint](https://arxiv.org/abs/2603.25948). It targets adversarial regret relative to an oracle that knows prediction error, uniformly across uncertainty-set sizes, without requiring a probabilistically calibrated uncertainty radius.
- [Hu et al., “Conformal Robustness Control: A New Strategy for Robust Decision,” ICLR 2026](https://proceedings.iclr.cc/paper_files/paper/2026/hash/a80188f9b1246d4b95d396165cf99207-Abstract-Conference.html). It directly optimizes prediction-set construction under explicit robustness constraints, noting that coverage is sufficient but not necessary for robustness.
- [Yang & Jin, “Multi-Distribution Robust Conformal Prediction,” ICML 2026](https://proceedings.mlr.press/v306/yang26ad.html). It provides finite-sample coverage across heterogeneous source distributions and arbitrary source mixtures using max-p aggregation.
- [Ley et al., “Decision-dependent robust optimization with elementwise controllable uncertainty sets,” OR Spectrum, 2 June 2026](https://link.springer.com/article/10.1007/s00291-026-00856-z). It internalizes paid information acquisition and robust optimization, including the budget-deflection phenomenon.
- [Kurtz, “Bounding the optimal number of policies for robust K-Adaptability,” Mathematical Programming, 29 January 2026](https://link.springer.com/article/10.1007/s10107-026-02329-1). For convex objective uncertainty the exactness bound can be (n_\xi+1); under constraint uncertainty, recourse-stable regions affect the policy bound. Finite-set minimum-k selection is NP-hard in general.
- [NIST SP 800-160 Vol. 1 Rev. 1, Engineering Trustworthy Secure Systems](https://csrc.nist.gov/pubs/sp/800/160/v1/r1/final) and [NIST assurance-case definition](https://csrc.nist.gov/glossary/term/assurance_case). An assurance case is a reasoned, auditable structure of claims, arguments, evidence and explicit assumptions—not a scalar score or a passing CI job.

## Experimental design

### A. Separate the axes

Each run must report native metrics rather than a single weighted score:

1. **Decision quality:** worst-case loss, expected/mean loss, joint regret, calibrated regret, and regret to the correct comparator.
2. **Feasibility and safety:** false-feasible rate by regime, infeasible action comparator contamination, unsafe-policy rate, and abstention rate.
3. **Information value:** query cost, query delay, observation error, residual uncertainty, branch cost and regret reduction.
4. **Universe closure:** false-closure rate, missed in-scope actions, partition omissions, invalid exclusions, stale-snapshot escapes and common-cause miss rate.
5. **Calibration:** marginal and per-regime coverage, worst-group coverage, threshold width, calibration sample size, exchangeability diagnostics and shift performance.
6. **Computation:** solve time, memory, number of enumerated policies, oracle complexity, receipt generation/verification time.
7. **Evidence quality:** receipt provenance, source independence evidence, signature verification, reproducibility, exact-head linkage and stale-evidence rejection.
8. **Operational burden:** measurement cost, human review burden, rollback/reconciliation cost and availability under abstention.

Plot the Pareto frontier over these dimensions. A method does not win by hiding more risk inside a scalarized utility.

### B. Build matched scenario strata

Every baseline sees identical train/calibration/test splits, action payloads, costs and scenarios. Hold out by **entire regime, source, time window and generator seed**, not just random rows. Include:

- correlated action-loss shifts vs rectangularized extremes;
- objective-coefficient error and Wasserstein deployment shift;
- uncalibrated predictor radii (including severe undercoverage);
- source-distribution heterogeneity and arbitrary mixtures;
- adaptive and decision-dependent uncertainty;
- query measurement error, unavailable queries, cost overruns and stale observations;
- scenario-dependent action infeasibility and disjoint robust feasible sets;
- adversarial observations that merge worlds with different feasible recourse;
- unseen but in-scope actions, missing partitions, and two enumerators sharing an upstream dependency;
- domain drift after enumerator execution, late-arriving catalogue entries and revoked evidence;
- common-cause source failures, correlated sensor errors and deliberately misleading receipt references.

### C. Use three truth oracles

1. **Finite mathematical oracle:** exhaustively enumerates all worlds, action sets and policy trees within small bounded instances. Required for 100% exact semantic agreement.
2. **Independent domain oracle:** independently generates the scope grammar and valid actions, then plants actions omitted from both enumerators. This is necessary to test *scope truth*, not just agreement with a supplied manifest.
3. **External empirical oracle:** independently adjudicated field/reference outcomes from the actual target domain, with preregistered protocol and immutable provenance. Synthetic tests cannot substitute for this.

Do not train, calibrate or adjudicate against the same generator/registry used by the method under evaluation. A separately written oracle that consumes the same incomplete action source is not independent evidence of completeness.

## Prospective acceptance targets

These are test goals, **not achieved results**:

| Measure | Target |
|---|---:|
| Rust/reference-oracle agreement on bounded finite domains | 100% |
| Infeasible actions entering any regret comparator | 0 |
| Deployed policy branching on hidden world/loss-model IDs | 0 |
| Numeric frontier emitted with unqualified loss/feasibility evidence | 0 |
| False closure with a planted omitted in-scope action | 0 |
| Closure accepted with omitted planned partitions / stale snapshot / unresolved declared common cause | 0 |
| Accepted confidence-budget overrun or duplicate evidence receipt | 0 |
| False-feasible rate | report per regime with exact confidence intervals |
| Calibration/reference-trial reduction | ≥20% vs strongest matched baseline at equal risk, coverage and shift profile |
| Query improvement | lower regret at equal or lower query/measurement cost, shown on Pareto frontier |
| Policy efficiency | compare regret gap vs k policies against exact exhaustive policy-tree optimum |

The zero-miss closure target must be tested against the independently generated domain oracle—not against the closure package's own planned inventory.

For a binomial zero-failure sample, the one-sided 95% exact upper bound is (p_U=1-0.05^{1/n}). Thus 299 independent zero-failure trials place the unadjusted upper bound just below 1%; about 2,995 are needed for 0.1%, and about 29,956 for 0.01%. These bounds depend on a representative target population and independent trials; correlated repeated runs, synthetic fixtures and repeated measurements of one unit do not count as independent field trials.

## Failure taxonomy and reporting

Record at least: `MODEL_MISSPECIFICATION`, `ACTION_OMISSION`, `SCOPE_MISSPECIFICATION`, `COMMON_CAUSE`, `OBSERVATION_LEAKAGE`, `QUERY_UNSAFE`, `NONANTICIPATIVE_VIOLATION`, `INFEASIBLE_COMPARATOR`, `CALIBRATION_SHIFT`, `RECEIPT_STALE`, `ASSUMPTION_UNBOUND`, `ORACLE_DISAGREEMENT`, and `UNRESOLVED`.

Publish every failed case, seed, generator version, exact source commit, baseline configuration and oracle result. A benchmark that publishes only the solved subset is not admissible.

## Route to competitive research

1. Reproduce and match each baseline on its native assumptions before combining results.
2. Establish exact finite-domain policy and receipt oracles.
3. Add independently generated omitted-action challenges and source common-cause dependency graphs.
4. Evaluate GARO, robust DFL, CRC and multi-distribution conformal on their native uncertainty/calibration regimes; do not force a single method into an incompatible mathematical formulation.
5. Compare K-adaptability against exhaustive policy trees and measure the actual k needed under objective and constraint uncertainty.
6. Add external reference datasets and field trials only after preregistration, independent adjudication and provenance controls.
7. Report where Symtropy loses. A credible claim to leadership requires repeatable improvements on matched baselines plus stronger evidence integrity—not more benchmark dimensions or more passing synthetic tests.

## Claim ceiling

This protocol specifies a research tournament. It does not itself demonstrate SOTA performance, statistically valid field guarantees, physical reliability, regulatory compliance or execution authority. NIST / FAA / domain-specific assurance expectations remain separate acceptance obligations.