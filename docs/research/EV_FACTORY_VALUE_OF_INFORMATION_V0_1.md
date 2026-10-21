# EV factory set-based value of information v0.1

**Issue:** [EV-FACTORY-VOI-001 #2524](https://github.com/Luminous-Dynamics/symtropy/issues/2524)  
**Status:** deterministic decision-support benchmark on synthetic scenarios  
**Evidence class:** synthetic; no real factory observations or quotes

## Question

When the current model does not distinguish between capital interventions, which measurement could most reduce the worst-case cost of making the wrong decision?

The existing paired and scenario-family reports assess candidate interventions. This layer turns uncertainty into an experiment-design question: partition the declared scenario set by each possible observation, re-evaluate the decision inside each outcome group, and see whether the maximum regret decreases.

## Method

For scenario \(s\) and action \(a\), the fixture contains a synthetic incremental-good-output effect \(u(a,s)\). Its units are good units/hour. Regret for selecting action \(a\) in scenario \(s\) is:

\[
r(a,s)=\max_{a'}u(a',s)-u(a,s).
\]

Before new information, select the action minimizing its maximum regret over all declared scenarios:

\[
R_0=\min_a\max_s r(a,s).
\]

A measurement \(m\) maps each scenario to a declared observation \(o_m(s)\). After the observation, a separate minimax-regret action can be selected for each outcome group. The measurement's worst-case residual regret is the maximum of those group-level regrets, \(R_m\), and its **gross worst-case regret reduction** is \(R_0-R_m\).

The engine checks the information-monotonicity invariant \(R_m \le R_0\), allowing a small floating-point tolerance. It reports each measurement's synthetic cost and duration separately. A Pareto frontier maximizes gross regret reduction and minimizes cost and duration; no arbitrary weights combine the axes.

## Why this is set-based

The fixture has no defensible probability distribution over its hand-selected scenarios. Therefore it does **not** report expected value of information, expected regret, scenario probabilities, net financial benefit, ROI, or a payback period. The regret reduction is in good-units/hour and is a model-conditional sensitivity measure, not a production forecast.

A later expected-value-of-sample-information extension must first define and justify probabilities, measurement error, representativeness and a value model. It should be tested on frozen held-out generators before the expected-value fields are enabled.

## Validation contract

The Rust CLI validates the fixture fail-closed:
- schema version, benchmark identity, synthetic evidence labels and claim boundary;
- unique scenario, action and measurement IDs;
- exactly one finite effect for each action in each scenario;
- exactly one non-empty observation for each scenario under each measurement;
- finite, non-negative measurement cost and duration in a single declared currency;
- byte-level SHA-256 of the exact input fixture;
- deterministic report generation;
- information-monotonicity of worst-case regret;
- Pareto dominance requiring no-worse performance on all three objectives and strict improvement on at least one.

Negative controls deliberately tamper with duplicate action rows, incomplete observations, evidence class and negative cost. All release, ROI and production authorization flags remain false.

## Benchmark targets

This v0.1 implementation is a contract and baseline, not an SOTA claim. The next stage should evaluate under held-out synthetic generators:
- regret reduction against random, cheapest-first and highest-uncertainty-first measurement baselines;
- decision accuracy and worst-case regret before/after the selected measurement;
- selection stability under scenario-set expansion and removal;
- Pareto-front stability and dominated-plan detection;
- robustness to imperfect, overlapping and missing observations;
- computation time, fixture/report digests and negative-control rejection.

Only after scenario probabilities and measurement models are defensible should the benchmark compare expected value of information, cost per expected regret reduction or any financial return.

## Research context

- Swisher, Jacobson and Yücesan survey ranking-and-selection and multiple-comparison procedures for discrete-event simulation optimization: https://doi.org/10.1145/858481.858484.
- De Temmerman and Verbeke study uncertainty-aware robust Pareto-front construction in manufacturing: https://www.nature.com/articles/s41598-025-97508-z.
- NIST IR 8620 covers VVUQ, interoperability and cybersecurity priorities for manufacturing digital twins: https://doi.org/10.6028/NIST.IR.8620.

No MES/PLC/plant data was collected; no suppliers, plants, integrators, landlords or agencies were contacted; no quotes were requested.
