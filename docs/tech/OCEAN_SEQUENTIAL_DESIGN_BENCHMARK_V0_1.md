# OCEAN-SEQUENTIAL-DESIGN-001 — synthetic benchmark v0.1

**Status:** executable, synthetic-only observation-design contract. No real ocean observations are acquired, imported or interpreted.

## Motivation and research grounding

A survey is not just a list of places to visit. It is a sequence of observations under imperfect detection, finite effort, potentially correlated repeats, false positives, method limitations and cumulative disturbance. The latent phenomenon must be distinguished from the observation process. MacKenzie et al. (2002) explain why non-detection is not evidence of absence when detection probability is below one ([USGS publication](https://pubs.usgs.gov/publication/5224176)). Large-scale marine biodiversity collection also needs to account for unbalanced sampling effort and spatial coverage ([Montes et al., Oceanography, 2021](https://tos.org/oceanography/article/optimizing-large-scale-biodiversity-sampling-effort-toward-an-unbalanced-survey-design)).

## Paradigm shift

Replace a static list of high-value locations with an **adaptive, auditable experiment policy**:
1. Start from a declared synthetic prior over a latent target state.
2. Select only actions with complete detection profiles, adequate signal, reviewed rights and explicit use permission, non-contact mode and known exposure.
3. Evaluate possible observations under a declared likelihood, keeping sensitivity distinct from false-positive rate.
4. Update the posterior rather than translating one non-detection into absence.
5. Choose subsequent observations using the updated posterior, subject to cumulative effort and each exposure-channel cap.
6. Compare adaptive heuristics to a small exact decision-tree oracle.

## V0 model contract

Each candidate has sensitivity P(positive | latent target present), false-positive probability P(positive | target absent), minimum detectable signal, expected signal, effort and separate sound/light/sediment exposure. Probabilities and signals are normalized synthetic ppm. Missing parameters do not become zero or certainty. Candidates with expected signal below minimum detectable signal are retained as negative controls but cannot be selected.

A repeated observation that shares the same declared correlation group is treated as perfectly correlated and cannot be counted twice. Different groups are treated as conditionally independent given the latent state. This assumption is deliberately explicit and is a known limitation: real temporal/spatial correlation and changing ecological states need a richer model.

The model computes expected information gain as reduction in Bernoulli entropy under the synthetic prior and likelihood. The exact adaptive oracle enumerates feasible next actions and both possible observation branches up to a bounded horizon. It maximizes expected information gain under the same budgets and eligibility rules as the heuristics. This is a tiny schedule oracle, not a general planning solver.

## Frozen synthetic scenarios

1. Imperfect detection with two candidate visits at the same station/time correlation group and an independent later window.
2. Low prior probability and a high false-positive detector contrasted with a lower-false-positive reference method.
3. A below-threshold signal, a reviewed-but-restricted source, pending rights review, a contact action, unknown exposure and valid passive/active non-contact controls.

All candidate counts, priors, signal levels, sensitivities, false-positive rates, strata and budgets are artificial. No real species or site labels appear.

## Baselines

- fixed-seed pseudo-random sequential policy;
- largest declared coverage gap;
- repeat-dominant-stratum anti-baseline;
- batch-priority information-gain heuristic that does not re-rank by the updated posterior;
- adaptive greedy information-gain heuristic;
- exact adaptive decision-tree oracle over the bounded toy cases.

## Acceptance goals

- 100% deterministic replay for identical corpus bytes, source commit, schema/tool versions and seed.
- Zero selection of contact actions, actions with incomplete rights review, restricted/unresolved use, missing/inadequate detectability, or unknown exposure.
- Zero double-counting of a perfectly correlated repeat group.
- Zero cumulative effort/sound/light/sediment budget violations on any reachable branch.
- Exact adaptive oracle is at least as good as each heuristic on the declared expected-information-gain objective.
- No absence, abundance, ecological condition or safety score is emitted.
- Receipts preserve exact corpus SHA-256 and per-policy expected information values.

## Limitations and next gates

V0 does not model real ocean acoustics, plume transport, changing occupancy, species-specific detectability, spatially correlated noise or ecological impact thresholds. It does not yet compute a real-world Brier score/log loss against held-out biological observations. The next version should add independently specified time-varying latent states, false-positive/negative calibration, correlated-noise truth generators, held-out synthetic calibration metrics, sensitivity analysis and Pareto comparisons. A real-data stage requires reviewed terms, provenance, sensor calibration, ecological expertise and independent scientific review.

The benchmark must remain separate from decisions to actuate vehicles, contact the seabed, collect specimens, disturb habitat or extract minerals.
