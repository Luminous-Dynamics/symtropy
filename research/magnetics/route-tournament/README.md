# Magnetic additive manufacturing route tournament

**Programme:** MAG-AM-ROUTE-TOURNAMENT-001  
**Issue:** #7757  
**Revision:** 0.1-draft  
**Status:** literature-stratified benchmark protocol; no matched physical route result claimed

## Purpose

Compare magnetic additive and adjacent near-net-shape manufacturing routes by the ability to deliver a **manufactured component that meets a target magnetic function**, including process, material, geometry, reliability and economic constraints.

A single remanence leaderboard is insufficient. Route families have different feedstock, alignment, densification, texture, feature-resolution and post-processing constraints.

## Files

- [Route tournament protocol](MAG_AM_ROUTE_TOURNAMENT_2026-11-27.md)
- [Machine-readable route benchmark plan](route-tournament-plan.toml)
- [Literature source-register fixtures](fixtures/route-source-register-v0.1.json)

## Route strata

MEX, LPBF/SLM, SLS, binder jetting, LCM/photopolymer composites, and near-net-shape / conventional controls. Do not pool incomparable material forms or test methods.

## Compose, do not duplicate

- Symthaea #13506: recycled NdFeB process/material science.
- Symthaea #13521 / #13533: target-function inverse design and robust co-design.
- Symtropy #7296 / #7310: LPBF process twin and patterned multi-material magnetic process twin.
- Symtropy #7332: exact-grid magnetic field map metrics.
- Symtropy #7745: specimen-level as-built functional acceptance.
- Symtropy #7770: executable evidence-gated route tournament and robust Pareto evaluator.
- Symtropy #2933: manufacturing genealogy.

## Executable route comparison

The standalone Rust evaluator is in [`evaluator/`](evaluator/README.md). It verifies exact cohort comparability, specimen / lot / build / geometry / material / process / magnetization joins, admissible evidence roles and uncertainty intervals. It emits hard-gated dispositions and a robust Pareto set; overlapping intervals remain incomparable.

- [Evaluator implementation](evaluator/src/lib.rs)
- [Benchmark schema](evaluator/route-tournament-fixture-v0.1.schema.json)
- [Synthetic tournament fixture](evaluator/fixtures/synthetic-route-tournament-v0.1.json)

The fixture is synthetic and must not be interpreted as a MEX, LPBF or LCM performance result.

## Claim boundary

The initial tranche is a literature-stratified protocol. It does not claim matched route superiority, physical specimen manufacture, raw-data reproduction, solver qualification, or product certification.
