# Origin-Aware Neutral Reference Sampler V1

## Purpose

Qualification-only Oracle B for POPGEN-05E / #718.

This oracle compares the POPGEN-05C production provenance sampler against an independently implemented weighted sampler. It does not alter production code and it does not require per-replicate trajectory equality.

## Independence boundary

The reference sampler deliberately does **not** use:

- the production SHA-256 semantic draw domain;
- `OriginAwarePopulationStateDigest` as random input;
- `PopulationTransitionId` as its RNG construction;
- production provenance-draw helpers;
- shared random variates with the production lane.

Instead V1 uses a qualification-local SplitMix64 generator, frozen case/replicate seeds, unbiased bounded integer rejection sampling, and independent cumulative weighted-category selection.

Sharing the same probability law is required. Sharing the same trajectory is not.

## Frozen ensemble

Each case uses `16,384` independent production semantic transition IDs and `16,384` independent reference-sampler seeds.

Cases:

1. two active origins, source weights 1:1, destination count 2;
2. two active origins, source weights 1:2, destination count 3;
3. modeled baseline + one active origin, source weights 1:1, destination count 2;
4. three active origins, source weights 1:1:1, destination count 3.

The allele is fixed in each case. The qualification therefore isolates conditional provenance sampling rather than mixing provenance statistics with a changing allele marginal.

## Frozen comparison metrics

- total-variation distance between production and reference outcome histograms: `<= 0.05`;
- absolute difference in selected extinction/fixation event probabilities: `<= 0.04`;
- reference-sampler 50/50 `n=2` probabilities must independently remain within `0.025` of `1/4, 1/2, 1/4`.

These thresholds are frozen before any exact-head execution result is observed and must not be widened after seeing a result.

## Outcome representation

Outcome vectors are ordered as:

`[modeled_baseline_count, origin_1_count, ..., origin_k_count]`.

Every outcome must sum exactly to the already-fixed destination allele count.

## Interpretation

Agreement supports the statement that two independently implemented samplers produce compatible neutral provenance distributions for the frozen cases.

Disagreement must be classified. Candidate classes include:

- production implementation defect;
- reference-sampler defect;
- deterministic-RNG/domain-separation defect;
- finite-ensemble fluctuation;
- incorrect test-state construction;
- wrong claimed probability law.

Do not tune the production model to the particular reference RNG, and do not demand trajectory-by-trajectory equality.

## Non-claims

A PASS does not establish:

- explicit linked-organism equivalence;
- real-world population structure;
- mutation-rate realism;
- genotype/linkage preservation;
- ecological validity;
- fitness or selection;
- adaptation or speciation.

Those remain separate evidence layers.

## Evidence rule

Source presence is not execution evidence. This oracle becomes evidence only after its exact product head executes under a captured subject/toolchain lane.
