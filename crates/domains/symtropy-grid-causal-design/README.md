# Symtropy Grid Causal Design Audit

A fail-closed pre-registration and cluster-randomization auditor for grid-flexibility causal M&V experiments.

## Purpose

This package audits the **experiment design before outcome data are observed**. It is the executable design layer for [GRID-CAUSAL-MV-001 #7347](https://github.com/Luminous-Dynamics/symtropy/issues/7347).

It does not estimate an effect, turn a baseline forecast into a causal claim, infer treatment-on-treated effects, compute ELCC, issue dispatch permissions or authorize settlement/payment. The report distinguishes checks over declared values from authentication: source timestamps, the pre-registration document, the interference mapping and the public randomness provider are not authenticated by this crate.

## v0.1 contract

- Cluster-randomized parallel design with intention-to-treat as the primary estimand.
- Pre-registration digest, covariate digest, baseline partition digest and interference-map digest.
- Seed commitment recorded before the plan freeze, followed by public-randomness publication and assignment issuance.
- Deterministic SHA-256 rank assignment within frozen strata; each stratum's treated/control allocation is predeclared.
- Each randomized cluster binds its sites, assets and one exclusive interference domain (for example, a depot-feeder island). Shared feeder/depot interference domains must be merged into one randomization cluster before assignment.
- Baseline, pretrend and placebo time windows are pre-treatment, frozen and pairwise disjoint.
- Negative-control outcomes are preregistered and may not reuse the primary outcome.
- Analysis is explicitly cluster-level; the audit does not treat asset rows as IID replicates.

## Reproducible assignment algorithm

For each cluster in a stratum the auditor hashes:

    experiment_id | seed_reveal | public_randomness_value | stratum_id | cluster_id

It sorts clusters by the resulting SHA-256 rank and assigns the first preregistered `treated_cluster_count` to treatment; the remainder are controls. The revealed seed must match its pre-freeze commitment digest, and the randomness value must match its declared digest. The audit checks declared timestamp ordering and uniqueness of interference-domain IDs in the manifest; it does not authenticate those claims, the pre-registration document or the external randomness provider. Source attestation remains an external trust-fabric responsibility.

## Run

    cargo run -p symtropy-grid-causal-design --bin grid_causal_design_audit -- crates/domains/symtropy-grid-causal-design/fixtures/grid_causal_design_v0_1.json

Tests and lint:

    cargo test -p symtropy-grid-causal-design --locked
    cargo clippy -p symtropy-grid-causal-design --all-targets --locked -- -D warnings

## Frozen synthetic fixture

The fixture has eight clusters across two strata, four treatment and four control clusters. It checks deterministic assignment, temporal ordering, exclusive interference domains and the false authority/effect flags. All identities, digests and timestamps are synthetic.

## Claim ceiling

A passing report says the declared assignment plan is internally consistent and reproducible under the included seed and public-randomness value. It does not authenticate the randomness provider, prove the interference map is complete, establish sample-size power, identify a causal effect, show transportability, or qualify a grid service. Final power/sample-size justification must account for cluster count, intracluster correlation, expected non-compliance, outcome variance, attrition and spillovers.
