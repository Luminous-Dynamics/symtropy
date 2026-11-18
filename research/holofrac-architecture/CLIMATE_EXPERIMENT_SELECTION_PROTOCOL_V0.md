# HoloFrac Climate Experiment-Selection Protocol v0

**Status:** finite synthetic probability-grounded value-of-information fixture  
**Implementation:** research/holofrac-retrieval/src/climate_experiment_selection.rs  
**Executable:** research/holofrac-retrieval/src/bin/climate_experiment_selection_sweep.rs  
**Workflow output:** climate-experiment-selection.csv  
**Related:** [#5961 — budgeted experiment selection](https://github.com/Luminous-Dynamics/symtropy/issues/5961), [#6025 — cross-scenario robustness](https://github.com/Luminous-Dynamics/symtropy/issues/6025)

## 1. Objective

Connect HoloFrac's climate-scenario robustness work with decision-value experiment selection. The system should propose the next observation/stress experiment that can change a declared building decision, rather than selecting solely by entropy reduction.

There is a critical semantic boundary:

- Analytical scenario weights express decision-maker priorities across futures. They are not probabilities.
- Expected value of information requires a probability distribution over latent states and a defensible source/basis for that distribution.
- A scalar decision utility requires an explicit decision objective and an identified authority that owns the trade-off.

The adapter rejects analytical scenario weights when asked to calculate expected value of information. It does not silently normalize preference weights into a probability prior.

## 2. Request contract

A climate experiment-selection request binds:
- exact scenario IDs;
- scenario probability priors;
- a probability-basis/provenance identifier;
- an explicit decision-objective ID;
- an explicit decision-authority ID;
- candidate decisions and their utility by world state;
- experiment observation likelihood matrices;
- experiment cost and total budget.

The initial probability basis is explicitly synthetic and equally weighted. It is a controlled demonstration only; it is not a climate forecast or empirically justified probability model.

In a real system, SSP narratives are not probabilities by default. Probability priors must be independently justified for the particular decision and population of futures. If that is not possible, use analytical robust/maximin/regret comparison rather than expected-value language.

## 3. Synthetic counterexample

Four latent states combine two dimensions:
- moderate vs hot local exposure;
- dry vs wet conditions.

Two interventions are compared:
- passive shading;
- active cooling.

The declared utilities depend on thermal exposure, not humidity. Two experiments are available:
- measure_humidity_exact: exactly observes dry/wet state and yields 1 bit of mutual information, but does not change the best intervention;
- measure_heat_exposure_70pct: imperfectly distinguishes moderate from hot exposure, yields less mutual information, but can change the preferred intervention and reduce expected decision regret.

The experiment costs are equal and the budget allows either. This is designed to show why maximum information gain is not the same as maximum decision value.

## 4. Policies and metrics

Compare:
- no-test;
- seeded-random;
- maximum mutual information;
- expected decision value (EVSI minus experiment cost).

Report by policy and experiment:
- mutual information bits;
- baseline expected decision value;
- expected value after observing the experiment;
- expected value improvement;
- net expected decision value after cost;
- expected decision regret;
- selected experiment, if any;
- scenario probability basis, decision-objective ID and authority ID.

The selector may choose no test when all affordable experiments have non-positive net expected decision value.

## 5. Acceptance targets

- Analytical decision weights used as expected-value priors: zero.
- Missing probability-basis provenance: rejected.
- Missing decision-objective or authority ID: rejected.
- Higher information gain but no decision improvement: must not outrank a lower-information experiment with higher positive net decision value under the EVSI policy.
- No-test must be selected when every affordable experiment has non-positive net expected value.
- Exact finite sums and deterministic replay for the frozen synthetic probability/likelihood model.

## 6. Boundaries and follow-on work

This fixture does not validate a climate probability distribution, the utility function, likelihood calibration or solver costs. The probability basis and utility contract are synthetic, explicitly named, and must not be reused as real project assumptions.

Follow-on work should:
1. bind probability models to versioned weather/occupancy/supply datasets and their uncertainty;
2. calibrate likelihoods on held-out climates and operational observations;
3. keep multi-objective Pareto decisions separate where no scalar utility is authorized;
4. bind every requested experiment to an exact candidate/source snapshot, registered solver, budget admission and immutable result receipt through #5959/#5961;
5. feed realized outcomes back to Symthaea for calibration without allowing it to self-authorize results.

## 7. Claim ceiling

Passing establishes only finite arithmetic and request-semantics correctness for the synthetic model. It does not establish forecast calibration, climate probabilities, real value of information, building energy savings, comfort, resilience or engineering qualification.
