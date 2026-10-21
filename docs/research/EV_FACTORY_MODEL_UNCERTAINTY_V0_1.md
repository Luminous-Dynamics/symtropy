# EV-FACTORY-MODEL-UNCERTAINTY-001 v0.1

**Issue:** [#2563](https://github.com/Luminous-Dynamics/symtropy/issues/2563)  
**Evidence class:** synthetic coefficient-interval stress case  
**Purpose:** distinguish an optimum for a nominal response surface from an optimum that is defensible under bounded coefficient/model uncertainty

## Why this is a separate problem

The joint-design oracle in [PR #2547](https://github.com/Luminous-Dynamics/symtropy/pull/2547) exhaustively optimizes its supplied response surface. Exact search proves the best answer for that declared model; it does not prove that the model's effect coefficients are physically accurate.

NIST's 2026 manufacturing digital-twin workshop report identifies verification, validation, and uncertainty quantification (VVUQ) among the persistent barriers to credible industrial digital twins: https://doi.org/10.6028/NIST.IR.8620. NIST's October 2026 uncertainty report discusses uncertainty propagation and harmonizing modeling with metrology practice: https://doi.org/10.6028/NIST.AMS.100-85.

## Three different decisions

The benchmark reports three decisions side by side:

1. **Nominal scenario-robust best:** maximize the minimum nominal good-unit gain over the finite scenario catalog.
2. **Interval-robust best:** maximize the minimum lower bound obtained by taking every selected coefficient to its interval's lower endpoint.
3. **Minimax-regret best:** minimize the maximum regret bound against every feasible challenger under the *same unknown coefficient realization* and across every declared scenario.

The third calculation is the important distinction. For challenger D against candidate C, each coefficient's inclusion difference \`delta\` is -1, 0, or 1. With independent intervals centered at \`mean\` and half-width \`u\`, the exact upper bound for that pair/scenario's performance gap is:

\[
\sum_j \left(\delta_j \mu_j + |\delta_j|u_j\right).
\]

The algorithm takes the maximum over feasible challengers and declared scenarios. That preserves the fact that two competing designs share the same unknown coefficient; it does not independently pick the low coefficient for one design and the high coefficient for another when calculating the same-realization gap. For this finite rectangular uncertainty set, the regret calculation is exact.

This contract assumes independent coefficient intervals. Real manufacturing parameters may be correlated, and a calibrated joint uncertainty set could produce materially different results. No probabilities are assigned to scenarios or coefficient values.

## Frozen synthetic fixture

The fixture enumerates all \(2^5=32\) binary designs across four named operating stress scenarios and enforces capital, labor, power, and floor-space constraints. Main-effect coefficient half-widths are 0.12 good units/hour. Pairwise-interaction half-widths are deliberately broad at 0.75 good units/hour, including zero-mean interaction terms. This is a designed sensitivity stress, not a calibrated estimate of uncertainty at a physical factory. The model includes no higher-order interactions.

Expected oracle behavior:

- 28 feasible designs and 4 infeasible designs.
- Nominal scenario-robust best: \`add-body-prep+add-eol-test+improve-yield\`, nominal worst-scenario gain 2.5 units/hour.
- Interval-robust best: \`extend-buffer\`, conservative worst-scenario lower bound 0.18 units/hour.
- Minimax-regret best: \`add-body-prep+add-eol-test+improve-yield\`, with a 6.41 units/hour upper bound on worst-case decision regret under the declared rectangular interval stress.

The disagreement is purposeful. Under these wide interaction intervals, the design with the largest nominal gain has a negative conservative output lower bound. The interval-robust best prefers a modest buffer intervention, while minimax regret retains the high-output combination because avoiding its potential regret is worse under the same interval model. This illustrates that robust optimization, conservative lower-bound optimization and regret minimization are different preferences, not interchangeable labels.

All numbers are deterministic fixture values. They must not be interpreted as forecasts, real factory estimates, ROI or purchasing advice.

## Fail-closed contract

- Strict schema and synthetic evidence class.
- Every option/scenario main-effect pair has a finite mean and non-negative half-width.
- Every option pair in every scenario has an explicit interaction mean and half-width; a missing interaction is not silently assumed to be zero.
- Duplicate/reversed pairs, incomplete catalogs, unknown IDs, invalid resources and negative/non-finite uncertainty widths are rejected.
- Enumerate every binary design for up to 10 options and report hard-constraint reasons. The cap is intentionally conservative because exact interval-regret computation compares every feasible design pair; the next scale-up should use a bounded search or branch-and-bound oracle.
- Exclude infeasible designs from all decision selectors and from the Pareto frontier.
- SHA-256 the exact raw fixture bytes; require byte-identical output for a repeated run.
- Explicitly report no scenario probabilities, expected-value estimate, ROI, field SOTA claim or production authorization.

## Next benchmark steps

1. Replace independent intervals with declared joint uncertainty sets where correlations are measured or justified.
2. Add a model-family generator with misspecified queueing, yield, repair and routing assumptions; separate these model-form stresses from replication noise.
3. Compare nominal, interval-robust, minimax-regret, best-single and greedy methods on frozen held-out generators with exact oracles for small instances.
4. Report nominal regret, uncertainty-set regret, false-feasibility rate, Pareto stability, runtime, and sensitivity to interval widths.
5. Require calibration and a frozen measured holdout before claiming physical predictive accuracy.

Industry baseline framing: AnyLogic documents optimization experiments with constraints and variable replication allocation: https://anylogic.help/anylogic/experiments/optimization.html. The ranking-and-selection survey by Swisher, Jacobson and Yücesan covers statistically principled selection from simulation alternatives: https://doi.org/10.1145/858481.858484. This benchmark does not claim to outperform those commercial or established methods; it creates an exact, auditable comparison task on a synthetic instance.

No plant/MES/PLC data were collected, no vendor or supplier was contacted, and no quotes were requested.
