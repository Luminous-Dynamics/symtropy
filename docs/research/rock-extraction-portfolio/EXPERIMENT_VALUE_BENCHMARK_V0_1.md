# Experiment value and stopping benchmark v0.1

**Status:** finite discrete decision-calculation contract with fabricated conformance data. No excavation campaign, actual experiment cost, prior, likelihood or utility has been qualified.

## 1. Research basis

- NIST AMS 100-85 (published 5 October 2026) harmonizes uncertainty quantification for computational models with metrology and argues for credibility assessment proportional to model influence and consequence. https://doi.org/10.6028/NIST.AMS.100-85
- Cheng & Huan, *Optimal stopping for sequential Bayesian experimental design* (2025; revised June 2026), formalize experiment selection and stopping as coupled sequential decisions. Their numerical demonstrations are simulated; this does not qualify an excavation policy. https://arxiv.org/abs/2509.21734
- Sadatsafavi et al. (2025), *Expected Value of Sample Information Calculations for Risk Prediction Model Validation*, makes the essential point that EVSI is a decision-utility quantity requiring a predictive model for future data, rather than a proxy for information gain alone. https://doi.org/10.1177/0272989X251314010

## 2. Paradigm shift

A research campaign should not collect every possible measurement or optimize for the most convenient benchmark. It should select the next experiment based on expected improvement to a consequential, explicitly owned decision—and stop when the expected decision value no longer justifies the cost.

This joins the physical experiment portfolio with the decision problem. For example, a shape scan is worth its cost only if learning the shape distribution could change the transport/cutter strategy enough to justify the scan. Likewise, a long-hose test is worth its cost only if it could change a real system-design choice.

## 3. Frozen finite one-step EVSI

The evaluator uses the finite state/action/experiment representation:

- `S`: mutually exclusive decision-relevant states;
- `p(s)`: normalized prior;
- `A`: admissible decisions;
- `U(a,s)`: explicitly approved net utility in one declared unit;
- `Z_e`: possible outcomes of experiment e;
- `L(z|s,e)`: likelihood of outcome z conditional on state s and experiment e;
- `C(e)`: experiment cost converted into that same declared utility unit under an explicit boundary.

Current decision value:

`V0 = max_a sum_s p(s) U(a,s)`

Predictive outcome probability:

`P(z|e) = sum_s p(s) L(z|s,e)`

Posterior:

`p(s|z,e) = p(s)L(z|s,e) / P(z|e)`

Expected decision value after experiment:

`V1(e) = sum_z P(z|e) max_a sum_s p(s|z,e) U(a,s)`

`EVSI(e) = V1(e) - V0`

`NetEVSI(e) = EVSI(e) - C(e)`

This is myopic one-step EVSI. It is not EVPI and is not an optimal multi-step experimental policy. It ignores adaptive branching after the next experiment and does not model correlated outcomes across experiments.

## 4. Required evidence and utility declarations

The case must provide:
- prior evidence role, method and source per state distribution;
- likelihood evidence role, method and source for each experiment;
- unique decision state and action identifiers;
- a complete action-by-state utility table;
- named decision owner, decision profile, utility validation reference and utility unit;
- complete experiment outcome × state likelihood table;
- cost amount, unit, cost boundary and source;
- minimum net-EVSI-to-continue threshold;
- remaining budget, maximum experiment count, stop consequence and continuation review authority.

Evidence roles distinguish independent measurement, external calibration, expert elicitation and simulation-only inputs. Expert priors/likelihoods remain explicitly elicited. Simulation-only and synthetic fixtures may exercise the arithmetic but cannot be reported as decision-qualified.

Cost must be expressed in the declared utility unit. Converting currency, labor, rig hours, consumables, energy or opportunity cost into that unit needs an independently justified utility/cost profile. An arbitrary unit conversion is not acceptable.

## 5. Stop rule

A candidate is only surfaced for human review when:
- experiments remain in the declared campaign budget;
- its complete cost is within the remaining budget;
- its net EVSI exceeds the declared stop threshold.

Otherwise, disposition is STOP with an explicit reason. A human review candidate is not a command to execute a test and is not permission to operate equipment.

## 6. Synthetic golden example (not real economics)

The bundled fixture uses two synthetic states:

- transport-limited: prior 0.6;
- fragmentation-limited: prior 0.4.

Three synthetic actions have explicitly fabricated utility values. The current best expected utility is 48 declared benefit points.

The pressure-differential probe has two outcomes with likelihoods 0.8/0.2 and 0.2/0.8 across the states. Its expected post-experiment decision value is 65.2, giving:
- EVSI = 17.2;
- synthetic cost = 5;
- NetEVSI = 12.2.

A weak discriminator produces different observations but does not change the best action under either posterior. Its EVSI is 0 and its NetEVSI is -2. This is a deliberate conformance example showing why information gain and decision value are not the same thing.

**These fabricated utility points do not represent USD, mine economics, actual experiment costs or an actual recommendation.** The case is marked SYNTHETIC_ONLY.

## 7. Conformance gates

- Prior probabilities sum to one.
- Every action has exactly one finite utility per state.
- Every experiment has unique outcomes and one likelihood per state per outcome.
- For each state, probabilities over outcomes sum to one.
- Utility, budget and cost share the exact declared unit.
- Cost, provenance and stopping rules are explicit.
- Impossible zero-predictive-probability outcomes are skipped without division by zero.
- Zero-information experiment produces zero EVSI.
- Perfect observation is bounded by EVPI.
- Splitting a likelihood outcome into identical sub-outcomes preserves EVSI.
- Permuting state/action/outcome order preserves EVSI.
- Increasing cost changes NetEVSI but not EVSI.
- Stop threshold and remaining budget alter stopping disposition.
- Simulation-only/synthetic cases always remain NOT_DECISION_QUALIFIED.

## 8. Future work beyond v0.1

- sequential adaptive design with posterior updates after each observation;
- correlation between sensors/experiments and shared failure modes;
- continuous and interval-valued priors/likelihoods, robust Bayes and ambiguity sets;
- risk-sensitive utility (tail loss, CVaR, catastrophic constraint violation);
- multi-resource schedules for labor, rig time, sample preparation, energy and maintenance;
- model discrepancy and metrology uncertainty inside observation likelihoods;
- value of information under distribution shift / new site;
- preregistered sample-size/power calculations and pilot-based effect sizes;
- immutable decision receipts binding exact data, model, utility, cost, prior and posterior digests.

## 9. Integration and boundaries

This crate composes with #3853's eligibility/Pareto evaluator and consumes domain decisions from:
- #3864/#3876 fragmentation/transport;
- #3891/#3910 morphology;
- #3877/#3913 conveyance-state fusion;
- #3888/#3890 spatial residual/contactability;
- #3812/#3814 full-cycle accepted output;
- #3825 resource accounting.

It does not duplicate their domain measurement authorities. It calculates declared one-step decision value only.

No machine controls, cutting depths, line pressures, airflow setpoints, blockage-clearing actions, geotechnical judgments or safety decisions are produced.
