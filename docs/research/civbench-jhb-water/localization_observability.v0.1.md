# CivBench observability-qualified localization evaluation v0.1

**Status:** engineering contract / synthetic test coverage; no claim of measured utility performance  
**Owner:** Symtropy / CivBench  
**Physical observability and hydraulic evidence:** Sol-Atlas  
**Decision closure and resource evaluation:** Symtropy  
**Authority, privacy, appeals and institutional governance:** Mycelix

## Research motivation

CivBench must separate event detection from spatial localization. A detector can reliably identify that a leak-like event occurred while the installed sensor layout cannot distinguish the responsible pipe. Scoring every such case as an ordinary pipe-classification failure encourages algorithms to overclaim precision that the physical evidence does not support.

A 2026 calibrated Moroccan district-network study illustrates the gap. It evaluated 648 scenarios with ten pressure sensors and five flow sensors across 171 candidate pipes. The study reports detection AUPRC of 0.901, but Top-5 pipe localization hit rate of 0.122. A zone-oracle intervention increased Top-5 pipe localization only to 0.170. The authors' diagnostics attribute a substantial part of the loss to sparse sensing and sensor coverage, not solely to classifier choice. This is a study-specific result, not a universal benchmark ceiling.

A separate 2026 Net3 EPANET/WNTR benchmark reports greater than 95% detection across more than 47,000 generated scenarios. Its synthetic scenario scale is useful for reproducibility and stress testing, but is not directly comparable to the calibrated Moroccan network or to prospective utility operations. It reinforces the need for split integrity and explicit provenance.

## Paradigm shift

Before asking whether a model can identify a pipe, ask what the physical measurement design can identify:

`sensor geometry + hydraulic sensitivity + model discrepancy + operating regime
  -> observability evidence
  -> supported localization level
  -> model localization / abstention
  -> resource-constrained intervention
  -> independently verified service outcome`

The physical owner supplies an event-level observability assessment and candidate universe. Symtropy evaluates whether the model respects that boundary and how well it performs *within* it. No second hydraulic solver or sensor placement optimizer is introduced here.

## Rust contract

The standalone Rust evaluator consumes three aligned collections:

1. `LocalizationTruth`: event identity and independently labelled zone/pipe truth.
2. `ObservabilityAssessment`: maximum supported level (`DetectionOnly`, `Zone`, `Pipe`), candidate universe at each supported level, sensor layout reference, hydraulic model/calibration reference and caller-computed assessment digest.
3. `LocalizationPrediction`: one policy decision per event, either an explicit abstention or ranked candidates at zone/pipe level, with top-1 confidence expressed in parts per million.

It validates exact event correspondence, unique IDs, digest syntax, known-truth membership in the physical candidate universe, and model candidate membership in that universe. A prediction finer than the supplied ceiling is retained as an unsupported-granularity claim and receives no localization credit. It is not silently accepted because the model emitted a high confidence value.

The report binds a single policy reference and a sorted list of observability source references. A single evaluation cannot mix model/policy versions. Assessment digests and external references are syntax-checked and recorded, not cryptographically authenticated or recomputed from source payloads.

## Epistemic handling

Ground-truth labels use separate states:

- Known: `Observed`, `Derived` or `Modelled`.
- Unscorable: `NotMeasured`, `Unsupported`, `Withheld` or `NotApplicable`.

Each known truth evidence class receives a separate metric slice. The evaluator never pools observed field truth with modelled/synthetic truth. Non-known states have their own counters and do not become false negatives or zero-valued locations.

## Metrics

The evaluator produces separate zone and pipe reports with:

- eligible events under the physical observability ceiling;
- supported localized predictions and explicit abstentions;
- coarse predictions for pipe-level evaluation and more-specific predictions for zone-level evaluation;
- unsupported-granularity claims;
- Top-1 and Top-5 hits;
- coverage, conditional Top-1/Top-5 accuracy, and end-to-end Top-1/Top-5 accuracy;
- confidence-threshold risk–coverage points, grouping tied confidence values into indivisible thresholds;
- observability-ceiling counts and per-event source bindings.

Conditional accuracy alone is insufficient. A model that localizes only easy cases can have excellent conditional precision but poor system coverage. End-to-end hit rate, coverage and the full risk–coverage curve make that trade-off visible. Confidence values are model assertions: the risk–coverage curve measures empirical error as the confidence threshold rises, but does not by itself establish probabilistic calibration. Calibration curves, Brier/log scores and interval coverage remain additional evaluation requirements.

## Experimental design and baselines

Every localization experiment should freeze the same event set, split receipt, hydraulic model version, sensor layout, noise assumptions, model tuning budget and candidate universe for each comparator.

Baselines should include:

- nearest-sensor and nearest-instrument heuristics;
- hydraulic residual/sensitivity ranking and a zone-oracle diagnostic;
- a classical tree model;
- a representative GNN;
- a temporal/Transformer localizer;
- a dense-sensor upper-bound reference;
- random sensor placement and observability-driven placement under the same sensor count.

Report detection, zone, nearest-sensor and pipe-level results independently. For pipe localization, publish Top-1/3/5 hit rates, candidate-set cardinality, spatial distance error where coordinates are trustworthy, coverage, abstention, unsupported-granularity rate and risk–coverage. Report every metric by network, sensor layout, operating regime, leak magnitude and truth evidence class.

For transfer experiments include target-only training, same-architecture random initialization, source-trained/no-adaptation and explicit adaptation baselines. A cross-network claim requires a cross-network split receipt; cross-topology requires topology and scenario-template holdout. Use the registered CivBench split presets rather than arbitrary names.

## Proposed research gates

These are proposed targets, not achieved results:

- Improve Top-5 pipe hit rate by at least 20% relative to the strongest matched baseline **at the same coverage and abstention rate**, or improve the complete risk–coverage curve at matched coverage.
- Report the physical observability ceiling and candidate-set cardinality before comparing model families. The model may not claim a finer level than the owner-supplied ceiling.
- When the physical assessment supports only a zone, zone-level abstention/localization is preferred over unsupported pipe-level confidence.
- Improve the attainable observability ceiling by at least 20% relative to random sensor placement under the same sensor-count and installation-cost envelope, measured across held-out scenarios and networks.
- Require calibration assessment by network/layout/regime before treating confidence thresholds as transferable.
- For every claimed gain, preserve the same event-level split, truth provenance, compute/tuning budget and sensor layout, and provide negative controls.

The Moroccan study's Top-5 result (0.122) and zone-oracle result (0.170) are diagnostic reference points only. They are not pass/fail thresholds for a different network. A claim exceeding the study's zone-oracle result is only meaningful if the sensor layout, observable candidate set and protocol are genuinely comparable or an explicitly changed physical design is reported.

## Industry comparison and field use

Machine-learning localization scores do not directly compare with operational leak-survey productivity. The Affinity Water/SUEZ/ASTERRA provider case reports 902 field-confirmed leaks after satellite screening across 9,045 km, with less than 4% of the network walked and a reported 7.2 confirmed leaks per crew-day. Those are provider-published field workflow metrics, not an independent controlled trial and not a Top-5 localization result.

Johannesburg Water's public active leak-detection programme reports over 10,500 km inspected annually, 15 daily technical teams and approximately 8,100 ML/year of estimated savings. Those operator-reported programme totals are not an AI model benchmark, event-level test set or causal randomized result. They should inform an eventual prospective trial, not be ranked next to a synthetic AUPRC.

A future Johannesburg pilot should begin in shadow mode, with independent event/pipe confirmation and no automated PLC/SCADA control. Only after a safe and representative prospective dataset exists should policy comparisons move to matched or randomized area/time evaluation. Report confirmed leads per crew-day, fraction of prioritized leads confirmed, false investigations, time-to-confirmation, repair closure, verified water recovered, essential-service impact and worst-group service floors.

## Explicit limitations

This v0.1 evaluator does not:

- compute hydraulic sensitivity, Jacobians, observability or sensor placement;
- calculate causal effects or service restoration;
- detect near-duplicate samples or perform data splitting;
- authenticate signatures or independently verify the truth/assessment sources;
- compute calibration curves, Brier score or AURC;
- prove chronological order for time-based splits;
- operate field sensors, SCADA or PLCs.

The evaluator is an integrity layer between the hydraulic owner's observability evidence and the benchmark's localization claims. It is not a new hydraulic model or a field deployment.

## Sources

- Sparse-sensor localization study (2026), DOI: https://doi.org/10.2166/aqua.2026.300 — 648 scenarios, ten pressure plus five flow sensors, 171 candidate pipes; detection AUPRC 0.901, pipe Top-5 0.122 and zone-oracle Top-5 0.170.
- Ranganathan et al. (2026), "Benchmarking Diverse Learning Architectures for Leak Detection and Localization in Water Distribution Systems", *Journal of Water Process Engineering* 87, 110114. DOI: https://doi.org/10.1016/j.jwpe.2026.110114 — Net3 EPANET/WNTR benchmark with 47,000+ scenarios.
- Farah & Shahrour (2026), "Standardization and Benchmarking of Datasets and Evaluation Protocols for Machine Learning-Based Water Leak Detection: A Systematic Review", *Water* 18(18), 2267. DOI: https://doi.org/10.3390/w18182267.
- SUEZ / ASTERRA / Affinity Water, "Affinity Water Uses Satellite Intelligence to Accelerate Leak Detection": https://www.suez.com/en/uk/case-studies/satellite-leak-detection-delivers-results-affinity-water.
- Johannesburg Water, "Active Leak Detection Programme": https://www.johannesburgwater.co.za/about-us/campaigns/active-leak-detection-programme/.
