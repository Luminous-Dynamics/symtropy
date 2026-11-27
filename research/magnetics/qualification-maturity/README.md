# Magnetic qualification maturity evaluator

**Programme:** MAG-AM-QUALIFICATION-MATURITY-001  
**Issue:** #7792  
**Status:** standalone Rust 2024 reference evaluator; synthetic evidence controls only

## Purpose

Derive the highest supportable MOEM M1–M5 evidence maturity from typed, scope-bound evidence receipts. A caller-supplied label is never accepted as proof.

## Maturity meaning

- M1 — concept/model and frozen target.
- M2 — measured, method-scoped coupon/material properties.
- M3 — controlled process–structure–property (PSPP) linkage.
- M4 — as-built functional component and acceptance under application conditions.
- M5 — repeated builds/lots, variability and durability evidence, production-site process-quality evidence, applicable equipment qualification, and laboratory method-scope evidence.

The MOEM ladder is a 2026 research reporting heuristic, not a formal certification or industry standard.

## Industry standards crosswalk

The evaluator references, but does not certify conformity with:
- ISO/ASTM 52920:2023 — industrial AM process and production-site quality assurance.
- ISO/ASTM TS 52930:2021 — PBF-LB equipment IQ/OQ/PQ; not a feedstock or complete post-processing qualification.
- IEC 60404-5:2015 and IEC 60404-18:2025 — permanent-magnet material-property measurement methods.
- ISO/IEC 17025:2017 — testing/calibration laboratory competence and method scope.

These are complementary layers. None substitutes for the others.

## Files

- [Rust evaluator](evaluator/src/lib.rs)
- [Evidence schema](evaluator/qualification-maturity-v0.1.schema.json)
- [Synthetic M1–M5 fixture](evaluator/fixtures/synthetic-qualification-maturity-v0.1.json)
- [Benchmark plan](benchmark-plan.toml)
- [Source register](fixtures/qualification-maturity-source-register-v0.1.json)

## Claim boundary

The evaluator checks receipt identity, evidence class, scope joins and maturity-gate semantics. It does not authenticate issuers, verify a certification registry, confer ISO/ASTM/IEC conformity, qualify a real process, or prove route superiority. Invalid or mismatched evidence cannot promote a candidate.


## Stronger specimen and process evidence gates

M2 requires measured remanence (`Br`) and intrinsic coercivity (`Hcj`) under method-scoped measurement evidence and a matching laboratory-method-scope receipt. M3 requires same-specimen/same-lot/same-build process–microstructure–property linkage plus phase, texture/orientation and porosity/defect characterization. Empty evidence returns `Unestablished`, not M1.

M4 binds geometry inspection, functional response and component acceptance to the same component and specimen. M5 binds variability analysis to the exact cohort summary, site quality to a production-site ID, and PBF-LB IQ/OQ/PQ records to the exact machine ID.
