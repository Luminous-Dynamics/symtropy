# Living World Observatory V0

Status: normative Living World validation contract. Documentation only.

## Purpose

Define a reproducible validation harness for Living World ecology, flora, fauna, multiscale authority, and presentation so claims are supported by exact scenario/evidence rather than screenshots alone.

## Core principle

> Canonical correctness, ecological plausibility, causal legibility, presentation quality, and performance are separate evidence dimensions.

Passing one dimension does not imply the others.

## Evidence capsule

Every qualified Observatory run should record enough information to reproduce the result, including at least:

- exact code/product head;
- Cargo.lock/content dependency identity;
- toolchain version;
- scenario/schema version;
- world/species/content hashes;
- deterministic seed/key material where applicable;
- authoritative starting tick and ending tick;
- process/fidelity policy versions;
- renderer quality/profile when visual evidence is collected;
- canonical output hashes/metrics;
- capture identifiers/paths where applicable;
- qualification script/harness version.

Do not mix evidence from different environment/content lineages under one qualification result without an explicit new root.

## Validation layers

### Q0 Static contract

Types/config/schema/lints encode the intended authority boundaries.

### Q1 Exact deterministic execution

Same exact inputs produce the same required canonical observables.

### Q2 Differential/invariance

Alternative execution paths that are supposed to be semantically equivalent are compared directly.

Examples: fine vs qualified coarse execution, save/reload vs uninterrupted, headless vs rendered, one region vs partitioned regions.

### Q3 Ecological/causal behavior

Controlled interventions produce directionally expected biological/ecological responses.

### Q4 Presentation legibility

Rendered output makes important canonical causes visible without inventing biology.

### Q5 Human validation

Owner/tester review assesses readability, realism, motion quality, artistic coherence, and whether automated metrics miss obvious failures.

No Q5 opinion rewrites failed Q1/Q2 canonical evidence.

## Canonical benchmark families

The Observatory should eventually include versioned fixtures for:

- population conservation/materialization;
- projection non-mutation;
- Level-A realization exclusivity;
- exact stock/reaction settlement;
- scarcity arbitration;
- developmental exposure/reaction norms;
- structural flora growth/damage/phenology;
- fauna perception/behavior/locomotion;
- group dynamics and niche construction;
- mortality/decomposition;
- succession/disturbance recovery;
- save/reload and region migration;
- presentation/LOD invariance.

## Metrics are tied to claims

A metric must state what claim it supports and what it does **not** prove.

Examples:

- foot-slip distance supports locomotion presentation/contact quality; it does not prove animal intelligence;
- biomass conservation supports accounting correctness; it does not prove ecological realism;
- causal-history classification supports visual legibility; it does not prove artistic superiority;
- deterministic replay supports reproducibility; it does not prove model validity.

## Failure discipline

Failed/queued/cancelled evidence is not PASS.

A source audit prediction is not runtime evidence.

A newer head invalidates qualification of an older head unless the evidence claim is explicitly about the older immutable product.

## Performance evidence

Record performance separately by fidelity/process tier. At minimum consider:

- canonical CPU time per ecological tick/process;
- active-organism cost;
- population/coarse-region cost;
- memory per persistent/active/coarse entity;
- projection/materialization cost vs N and requested k;
- rendering cost by presentation tier;
- promotion/demotion cost;
- save/load size/time for ecology regions.

Performance optimization may not silently weaken required canonical observables.

## First validation scene

Maintain a small deterministic Living World validation region rather than relying only on a full game world.

The scene should eventually contain:

- mature + juvenile flora;
- ground vegetation;
- deadwood/detritus;
- mycelium/decomposer seam;
- one mobile animal;
- small collective/cohort fauna;
- light/shade gradient;
- moisture/nutrient gradient;
- wind/mechanical exposure;
- disturbance/damage;
- death and regrowth path.

The purpose is to exercise the complete causal loop cheaply and repeatedly.

## Non-goals

The Observatory is not a benchmark designed to maximize one leaderboard number. It is an evidence framework for keeping increasingly sophisticated simulation/rendering layers honest.