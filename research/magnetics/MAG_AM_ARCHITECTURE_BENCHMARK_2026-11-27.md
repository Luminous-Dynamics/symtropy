# Magnetic AM / hybrid architecture benchmark

**Programme:** MAG-AM-ARCHITECTURE-BENCH-001  
**Issue:** [Symtropy #7802](https://github.com/Luminous-Dynamics/symtropy/issues/7802)  
**Revision:** 0.1-draft  
**Date:** 27 November 2026

## Decision question

Does the AM/hybrid architecture demonstrate a useful system-level advantage over an optimized conventional PM architecture for the same machine application, duty cycle, thermal/mechanical envelope and acceptance policy?

The benchmark never assumes that geometry freedom equals engineering benefit. The candidate must first deliver qualified function, reliability and manufacturing evidence, then demonstrate a robust architecture benefit.

## Required comparators

Preregister the required conventional comparator families for the application:
- optimized conventional sintered-NdFeB PM architecture;
- optimized bonded-PM architecture when bonded material is relevant;
- optimized PM-assisted synchronous-reluctance architecture when it is a credible industrial alternative.

At least one required baseline must be a conventional PM architecture. A reluctance-only design may be included as a secondary control but cannot be the sole comparator. Baseline candidates must carry a frozen optimization receipt, search-space identity, objective/constraint digests, optimization budget and independent restarts.

## Frozen system boundary

Every architecture binds the same:
- application, target function and acceptance policy;
- stator, winding, inverter/control and supply configuration;
- cooling architecture and thermal boundary;
- air gap, rotor/shaft/bearing and retention boundary;
- speed/torque operating envelope and duty cycle;
- magnet-temperature envelope and demagnetization conditions;
- field-weakening, armature reaction and fault-current exposure where relevant;
- measurement setup, field-map frame/grid/standoff and calibration/reference;
- objective definitions, units, direction, hard limits and non-inferiority margins.

A scope mismatch is NOT_COMPARABLE. Do not normalize different duty cycles or thermal boundaries into apparently equivalent numbers after measurement.

## Mandatory function and reliability metrics

The benchmark may define additional application metrics, but should include where relevant:
- torque/power over continuous and peak duty;
- torque ripple and air-gap field/harmonics;
- efficiency over the declared operating map;
- maximum winding/magnet temperature and thermal margin;
- (B_r(T)), (H_{cj}(T)), demagnetization margin and irreversible flux loss;
- retention margin, fatigue/vibration and interface/bondline integrity;
- rotor inertia, overspeed and mechanical limits;
- accepted-part yield, scrap/rework and accepted-part cost.

Hard limits are non-compensatory. Low cost, mass or inertia cannot offset a demagnetization, thermal, retention, durability or application function failure.

## Architecture benefit metrics

Predeclare the benefit dimensions:
- PM mass and critical-element demand;
- rotor inertia;
- cost per torque or cost per power at intended volume;
- material utilization / machining scrap;
- assembly complexity, integrated retention or thermal-management value;
- qualified recovered-material reuse where the product supports it.

A candidate demonstrates an architecture advantage only when it:
1. has independently qualified M4 as-built component evidence;
2. passes every hard limit with the complete uncertainty interval inside the admissible region;
3. is robustly non-inferior to every required optimized conventional PM comparator on every mandatory function metric;
4. robustly improves at least one preregistered benefit metric against at least one required comparator, with uncertainty intervals strictly separated in the correct direction.

For a maximizing mandatory metric, candidate lower bound must be no worse than the comparator lower bound minus the declared non-inferiority margin. For a minimizing mandatory metric, candidate upper bound must be no worse than comparator upper bound plus the declared margin.

A maximizing benefit improves only when candidate lower bound exceeds comparator upper bound. A minimizing benefit improves only when candidate upper bound is below comparator lower bound. Overlap is not proof of improvement.

## Claim boundary

The v0.1 evaluator checks exact scope, baseline optimization evidence, M4 physical acceptance, hard constraints, interval non-inferiority and interval benefit separation. It does not run an EM/thermal/mechanical solver, validate a physical motor, qualify a manufacturer, or certify standards conformity.
