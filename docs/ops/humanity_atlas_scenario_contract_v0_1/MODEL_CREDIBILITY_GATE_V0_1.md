# Humanity Atlas model credibility gate v0.1

**Status:** proposed qualification process. This is an internal engineering gate inspired by established modeling-and-simulation verification, validation, and uncertainty-quantification practice. It is not a certification and does not confer scientific credibility merely by being completed.

## Principle: qualify a model for a specific use

"Valid model" is too broad to be useful. A model must be evaluated for a named intended use, relevant variables, time horizon, geographic/domain scope, decision stakes, and known limits. A model suitable for illustrating a possible long-term energy transition may be unsuitable for an infrastructure investment forecast.

NASA-STD-7009B describes uniform practices for model and simulation design, use, acceptance criteria, and credibility assessment, including verification, validation, uncertainty, and communication of results. Use its structure as a process reference, not as a claim that Humanity Atlas is NASA-certified:
- https://standards.nasa.gov/standard/NASA/NASA-STD-7009
- NASA software engineering guidance on model/simulation verification, validation data, domain, metrics, and results: https://swehb.nasa.gov/spaces/SWEHBVC/pages/50888929/SWE-070%2B-%2BModels%2BSimulations%2BTools

## Required model card

Every non-toy model proposed for Atlas use should have a versioned model card with:

1. **Intended use and exclusions:** the decision or question supported, target variables, horizon, spatial scope, prohibited extrapolations, and consequences of error.
2. **Conceptual model:** state variables, causal assumptions, equations, parameter meaning, units, boundary conditions, mechanisms omitted, and intervention semantics.
3. **Input pedigree:** source, collection and publication dates, measurement/estimate status, missingness, transformations, license, exact snapshot digest, and known source biases.
4. **Implementation identity:** source revision, build/toolchain identity where relevant, configuration digest, dependencies, numeric precision, and deterministic/stochastic behavior.
5. **Verification evidence:** tests that the implementation conforms to equations/specification; dimensional checks; invariants and conservation laws where applicable; numerical convergence/error analysis where applicable; regression and property-based tests.
6. **Validation design:** independent referent data, validation domain, train/calibration/evaluation separation, time cutoff, metric definitions, simple baselines, and results. The data available after the historical cutoff must not leak into the simulated forecast origin.
7. **Uncertainty and robustness:** parameter uncertainty, observational uncertainty, stochastic variability, structural-model alternatives, sensitivity analysis, failure conditions, and a plain statement when uncertainty is not quantified.
8. **Review and history:** reviewer identity/role, review scope, model/data/version hashes, unresolved issues, superseded evidence, and date. A governance vote does not establish empirical truth.

Acceptance thresholds must be selected and justified **before** inspecting the final holdout results and must reflect the intended use. There is no universally appropriate forecast error, calibration, or conservation tolerance across all domains.

## Result-classification gates

The run envelope is intentionally more conservative than a single "confidence" score.

| Output class | Minimum meaning | What it does not imply |
|---|---|---|
| **synthetic_fixture** | A contract or algorithm example with generated inputs | Real-world validity |
| **illustrative_scenario** | A conditional story or parameterized scenario, possibly not empirically calibrated | A probabilistic forecast |
| **unvalidated_simulation** | An engine executed a stated model/configuration, but intended-use validation is incomplete | Predictive accuracy |
| **calibrated_simulation** | A retrospective evaluation artifact exists for the stated use and scope, with documented metrics and evaluation design | General validity outside that domain, causal identification, or correct tail risks |
| **prospective_forecast** | A forecast receipt was frozen before resolution, with explicit target, probability/output, deadline and resolution procedure | That the forecast has already proved accurate |

**calibrated_simulation** therefore requires a **retrospective_evaluation** evidence item, not just a status field or a generic review. Independent review may accompany the evaluation but is not a substitute for it. **prospective_forecast** requires a **prospective_forecast** receipt.

The v0.1 schema's SHA-256 fields bind artifact bytes by reference. They do **not** authenticate who published those bytes, prove that an artifact exists at an external service, or prove the report is sound. A later Mycelix-backed trust layer should add signer identity, key status at signing time, verifier identity, exact artifact digest, review scope, revocation/supersession, and a non-circular receipt chain. Until then, the evidence fields are traceability metadata only.

## Retrospective evaluation protocol

For historical forecasts and quantitative models:

1. Freeze the model revision, data snapshot, evaluation period and metrics before running the holdout.
2. Use a cutoff-specific snapshot to reproduce the information available at the forecast origin. Record missing or revised data and avoid future leakage.
3. Compare with simple baselines and, when appropriate, a domain-relevant competing model.
4. Report aggregate and slice-level metrics, sample counts, uncertainty intervals, calibration diagnostics where probabilities are produced, and failure cases—not just a single headline score.
5. Perform sensitivity analyses across plausible parameter ranges and at least one meaningfully different structural assumption when the structure is uncertain.
6. Record out-of-domain behavior and conditions under which the model must refuse to extrapolate.
7. Hash and preserve the report, input snapshots, configuration, and machine-readable metrics. Re-running the same deterministic configuration should reproduce declared outputs within a declared numerical tolerance.

Counterfactual history needs a distinct statement: the unrealized outcome cannot be directly observed. Evaluation can test causal subclaims, model components, and robustness across alternative causal graphs; it cannot conclusively score the complete timeline that never happened.

## Prospective forecast receipt

Before an event resolves, archive:

- stable forecast ID and issue time;
- unambiguous outcome/metric definition, target, units and time/geographic scope;
- probability for a binary event or a distribution/quantiles for continuous outcomes;
- close time/deadline and named resolution source/procedure;
- model, inputs, scenario, and forecast-run digests;
- scoring rule chosen in advance and benchmark/baseline forecast;
- updates as new records linked to the prior receipt (never a silent overwrite);
- resolution record and score when available, including adjudication if ambiguous.

For binary forecasts, a Brier score is a useful proper scoring rule; probabilistic continuous forecasts may be evaluated with CRPS and calibration diagnostics. Score choices depend on the question and forecast format. Simulated-world forecasting benchmarks such as ForecastBench-Sim demonstrate a useful complementary loop: freeze a world state, forecast hidden future states, let the simulator advance, then score the forecasts. Such performance establishes competence within that simulator, not automatic transfer to real-world questions:
- https://arxiv.org/abs/2606.18686

## Process gate checklist

The model owner must be able to answer "yes" or attach a justified "not applicable" for each row.

- [ ] Intended use, domain, outcome, and prohibited uses are explicit.
- [ ] Equations, assumptions, units, boundaries, and intervention semantics are versioned.
- [ ] Input/model/source digests are checked; provenance and licensing are recorded.
- [ ] Implementation verification and any numerical-error analysis are documented.
- [ ] Independent evaluation data and a predeclared protocol exist for calibrated claims.
- [ ] Baselines, error metrics, calibration (if probabilities), and failure slices are reported.
- [ ] Parameter and structural sensitivity are reported, or uncertainty limitations are explicit.
- [ ] Forecast receipts can be resolved and scored without revising the original forecast.
- [ ] The evidence artifacts are addressable and hashed; publisher authenticity is separately stated.
- [ ] The consumer displays the result class and caveats; no downstream component strips them.

This checklist is a process artifact. Completing boxes alone does not prove validity; the evidence and acceptance rationale must be reviewable for the named intended use.
