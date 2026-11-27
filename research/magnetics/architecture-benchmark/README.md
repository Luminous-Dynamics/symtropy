# Magnetic architecture benchmark evaluator

**Programme:** MAG-AM-ARCHITECTURE-BENCH-001  
**Issue:** #7802  
**Status:** standalone Rust 2024 decision evaluator; synthetic interval controls only

## Purpose

Decide whether an AM or hybrid permanent-magnet architecture has a defensible system-level advantage over optimized conventional PM baselines under one frozen machine application and duty-cycle boundary.

This crate does not calculate electromagnetic, thermal, structural or acoustic fields. It consumes independently qualified M4 component evidence and interval-valued metrics.

## Decision outcomes

- `NotComparable` — invalid/mismatched scope, missing required baseline family, invalid baseline optimization receipt, or mismatched metric definitions.
- `Fail` — an admissible physical component observation definitely violates a non-compensatory hard application limit.
- `Indeterminate` — required evidence is missing, uncertain across a hard limit, non-physical, or insufficient to claim M4.
- `NonInferiorNoArchitectureBenefit` — candidate is physically qualified and robustly non-inferior on mandatory function metrics, but no architecture-benefit interval is robustly improved.
- `ArchitectureAdvantage` — candidate is M4-qualified, passes all hard requirements, is robustly non-inferior to every required optimized conventional PM comparator, and robustly improves at least one preregistered architecture-benefit metric versus at least one such comparator.

No weighted scalar score is emitted. A reluctance-only control cannot stand in for conventional PM baselines.

## Metric classes

Mandatory function metrics are evaluated with hard limits and non-inferiority margins: continuous/peak torque or power, efficiency over duty, temperature, demagnetization margin, retention and other application-specific requirements.

Architecture benefit metrics are separately declared: PM mass, rotor inertia, cost per torque/power, material utilization, assembly/manufacturing cost or recovery performance. A benefit requires interval separation in the correct objective direction.

## Required baseline quality

Every required conventional comparator must:
- match the exact frozen system boundary;
- have a qualified M4 component acceptance receipt;
- carry a baseline-optimization receipt tied to the same target and constraint digests;
- meet the preregistered minimum optimization evaluations and independent restarts;
- declare a valid search-space digest and optimization protocol.

A reluctance-only control is secondary and does not satisfy the conventional-PM baseline requirement.

## Files

- [Evaluator source](evaluator/src/lib.rs)
- [Evidence schema](evaluator/architecture-benchmark-v0.1.schema.json)
- [Synthetic known-answer fixture](evaluator/fixtures/synthetic-architecture-benchmark-v0.1.json)
- [Benchmark plan](benchmark-plan.toml)
- [Protocol](../MAG_AM_ARCHITECTURE_BENCHMARK_2026-11-27.md)

## Claim ceiling

A synthetic architecture advantage validates interval/decision semantics only. It is not evidence that AM is physically superior to conventional PM designs, nor an electromagnetic solver result, production capability claim, certification or product-release authority.
