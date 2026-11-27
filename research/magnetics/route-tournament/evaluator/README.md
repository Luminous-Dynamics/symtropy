# Magnetic route tournament evaluator

**Programme:** MAG-AM-ROUTE-EVALUATOR-001  
**Issue:** #7770  
**Parent:** [MAG-AM-ROUTE-TOURNAMENT-001 #7757](https://github.com/Luminous-Dynamics/symtropy/issues/7757)  
**Status:** standalone Rust 2024 reference evaluator; synthetic semantic tests only

## Purpose

Compare manufactured magnetic components against the same declared application target and measurement setup, validate their specimen evidence and hard requirements, and return a robust Pareto set across independently stated objectives.

The unit of comparison is the accepted as-built component—not nominal CAD, literature remanence, or an unqualified optimizer score.

## What is implemented

- Route strata for MEX, LPBF/SLM, SLS, binder jet, LCM, PEM and conventional controls.
- Exact application / target-function / measurement-setup / policy scope matching.
- Specimen, material-lot, build, process-genealogy, post-magnetization and as-built-geometry evidence joins.
- Per-metric unit, evidence-role, method, calibration and digest checks.
- Physical, material-characterization and process-record evidence roles; published-summary and synthetic evidence cannot qualify a specimen.
- Interval-valued hard-limit decisions: definite FAIL, safe, or unresolved.
- Robust Pareto dominance using non-overlapping uncertainty intervals; overlapping intervals remain incomparable.
- No weighted composite or forced single winner.

## Read first

- [Route tournament protocol](../MAG_AM_ROUTE_TOURNAMENT_2026-11-27.md)
- [Route tournament plan](../route-tournament-plan.toml)
- [Synthetic source register](../fixtures/route-source-register-v0.1.json)
- [Evaluator manifest and implementation](evaluator/Cargo.toml)

## Limitations

The evaluator validates supplied evidence identity, roles, declared scope and interval arithmetic. It does not calculate magnetic fields, propagate uncertainty, establish calibration competence, authenticate evidence producers, assess a manufacturing process, or issue product-release authority. A digest establishes content identity only.

The initial fixtures are synthetic and do not establish physical superiority of any manufacturing route.


## Exact joins and decision semantics

Every observation binds the candidate's specimen, material lot, build/run, as-built geometry, geometry-inspection record, process genealogy, material state, post-build magnetization and cohort scope. The as-built geometry inspection record also carries a method, calibration digest and reference-chain identity.

Hard-limit decisions use the complete supplied interval `[value - uncertainty_radius, value + uncertainty_radius]`. A limit definitely violated by the whole interval is FAIL; an interval crossing a hard limit is INDETERMINATE. A pass requires every hard limit to be safe.

A robust minimization dominance requires the candidate's upper interval endpoint to be no greater than the other candidate's lower endpoint in every minimization dimension; maximization uses the inverse relation. At least one dimension must be strictly separated. Overlap means incomparable. This deliberately conservative definition may return larger Pareto sets rather than make unsupported rankings.
