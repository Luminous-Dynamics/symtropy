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

## Evidence maturity

Every result should be labelled by its highest demonstrated MOEM maturity: M1 concept/model, M2 coupon material evidence, M3 controlled process–structure–property validation, M4 application-relevant functional component, M5 reproducible qualification-grade evidence. This is a research reporting framework, not a formal standard. Route results at different maturity levels must be stratified, not treated as equivalent.

## Route strata

MEX, LPBF/SLM, SLS, binder jetting, LCM/photopolymer composites, and near-net-shape / conventional controls. Do not pool incomparable material forms or test methods.

## Compose, do not duplicate

- Symthaea #13506: recycled NdFeB process/material science.
- Symthaea #13521 / #13533: target-function inverse design and robust co-design.
- Symtropy #7296 / #7310: LPBF process twin and patterned multi-material magnetic process twin.
- Symtropy #7332: exact-grid magnetic field map metrics.
- Symtropy #7745: specimen-level as-built functional acceptance.
- Symtropy #2933: manufacturing genealogy.

## Claim boundary

The initial tranche is a literature-stratified protocol. It does not claim matched route superiority, physical specimen manufacture, raw-data reproduction, solver qualification, or product certification.


## Current performance envelope and industrial comparator

The September 2026 NdFeB AM review reports literature remanence generally around 0.6–1.0 T; MEX is the most competitive surveyed printed route for bonded NdFeB; LPBF density is promising but magnetic texture remains a limiting gap; SLS/binder jet are porosity-limited. Conventional anisotropic sintered NdFeB is reported around 1.4–1.5 T. These are literature aggregates, not common-condition matched trials.

The route tournament therefore compares functionally equivalent components against optimized conventional sintered-PM, bonded-PM and PM-assisted reluctance baselines. The AM hypothesis is architecture value—topology-specific placement, integrated retention, segmentation, lower PM mass/inertia/scrap or improved cost per torque—not a raw-Br contest. All hard constraints, including Hcj(T), demagnetization margin, retention and durability, remain non-compensatory.

## Current digital-thread standards references

The 2025–2026 crosswalk now includes ISO/ASTM 52951:2026 (part data packages), 52953:2025 (process-monitoring/QC data registration for metallic PBF-LB), 52948:2026 (metallic PBF imperfection classification; no acceptance criteria), TR 52958:2026 (coaxial photodiode LOF monitoring in PBF-LB), and 52941:2026 (aerospace PBF-LB/M machine-system reliability tests only). See the protocol's scope notes; these references do not establish component acceptance or standards conformity.
