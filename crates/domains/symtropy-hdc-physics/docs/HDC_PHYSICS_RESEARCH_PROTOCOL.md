# HDC Physics Episode Research Protocol

## Claim boundary

`symtropy-hdc-physics` creates a semantic representation of exact simulation
state. It does not replace collision detection, numerical integration,
conservation diagnostics, replay hashes, cryptographic evidence, or game-state
authority.

A result may claim retrieval, classification, novelty detection, or prediction
performance. It may not claim improved physical accuracy unless a separate,
pre-registered control intervention is evaluated against exact physical metrics.

## Reproducibility record

Every published run must retain:

- source commit and dirty-tree status;
- Rust compiler, target triple, CPU architecture, and enabled features;
- `PhysicsEncoderConfig` serialized in full;
- encoder fingerprint;
- scenario generator version and deterministic seed;
- timestep, frame count, solver settings, and body construction parameters;
- train, validation, and test episode identifiers;
- exact state digests for every encoded frame;
- raw retrieval results, not only aggregate accuracy;
- hardware, wall time, peak memory, and vector dimension.

Changing any semantic role, scalar range, quantization rule, reference frame,
item-memory algorithm, temporal binding, or tie-breaking rule requires an
encoder schema-version increment.

## Required baselines

At minimum, compare against:

1. raw standardized-feature Euclidean nearest neighbor;
2. random projection with the same output dimension;
3. summary-statistic nearest neighbor;
4. HDC without temporal permutation;
5. HDC without shape features;
6. HDC without invariant features;
7. HDC with world and center-of-mass reference frames.

Approximate nearest-neighbor indexes must also report recall relative to the
included exact linear-scan memory.

## Dataset splits

Do not randomly split adjacent frames from one trajectory across training and
test sets. Split by complete episode, parameter regime, and where relevant
scenario family. Strong generalization studies should reserve at least one of:

- unseen masses or size ratios;
- unseen orientations;
- unseen translations;
- unseen speeds or energies;
- unseen body counts;
- unseen solver settings;
- unseen random seeds;
- unseen collision topology.

## Metrics

Retrieval:

- top-1 and top-k accuracy;
- mean reciprocal rank;
- normalized discounted cumulative gain when graded relevance exists;
- class-balanced accuracy;
- exact-index query latency and memory use.

Novelty and anomaly detection:

- AUROC and AUPRC;
- false-positive rate at a pre-registered true-positive rate;
- calibration error;
- detection lead time before an exact failure condition.

Prediction:

- accuracy by prediction horizon;
- Brier score or log loss;
- confidence intervals across seeds;
- comparison with temporal and non-temporal baselines.

## Translation and identity controls

`CenterOfDynamicMass` and anchored reference frames intentionally discard global
translation. Always report whether this invariance is desired. Likewise,
`IdentityPolicy::None` improves structural matching but cannot answer questions
about a particular persistent entity.

The exact state digest must still differ when exact source states differ, even
when their semantic vectors are intentionally identical under an invariance.

## Negative results

Publish failed regimes, similarity collisions, unstable thresholds, and encoder
changes that did not improve held-out results. The project should retain a
machine-readable failure corpus rather than deleting inconvenient scenarios.
