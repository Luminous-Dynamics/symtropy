# MAG-AM-THERMO-ELECTRICAL-STACK-001

**Date:** 2026-11-27  
**Version:** 0.1  
**Evidence status:** protocol and evaluator only; no physical measurements or route claims

## 1. Why this is the next frontier

A 2026 IEEE Access review identifies shaped/hybrid windings, direct-conductor cooling, slot-level heat exchangers, and compact thermal hardware as the clearest near-term AM opportunities for high-power-density electrical machines. It conditions those opportunities on conductivity, AC losses, insulation reliability, hydraulic power, leakage, repeatability, and manufacturing economics. Magnetic-core AM and fully AM electrical machines remain less mature in the reviewed evidence base. [1]

A September 2026 materials-driven perspective likewise requires thermal performance to include pressure drop, pump input, coolant compatibility, leak tightness, and interface stability; conductor performance includes conductivity and AC loss; insulation requires testing on actual 3D geometries. It reports that reviewed EM-AM evidence remains concentrated at M2–M3, with limited M4 and no M5 qualification-grade demonstrations in that reviewed dataset. [2]

The better question than “Can we print a more complex motor?” is:

**Can an AM-enabled thermo-electrical stack remove a measured machine bottleneck while preserving electrical, dielectric, hydraulic, mechanical, and manufacturing integrity?**

## 2. Frozen system boundary

Every candidate and reference must share the exact same:
- application, torque-speed duty and time-at-load profile;
- inverter topology, switching frequency, PWM/current waveform and control;
- winding electrical connection, phase count and terminal boundary;
- coolant chemistry, concentration, inlet temperature, flow, pressure and contamination envelope;
- ambient temperature, housing and external thermal boundary;
- insulation class, electrical clearances and dielectric acceptance policy;
- component/system mass and volume boundary;
- measurement methods, calibration and uncertainty policy;
- acceptance criteria and preregistered optimization objective/constraint digests.

A candidate with a different waveform, coolant, duty, boundary or acceptance policy is **NotComparable**, not “normalized” after the fact.

## 3. Mandatory function metrics

The protocol requires application-specific hard limits for:
1. **Winding hot-spot temperature** over the full frozen duty, not just steady-state average.
2. **AC winding loss** under inverter-relevant waveform; DC resistance is reported separately.
3. **Dielectric margin** on actual 3D conductor/insulation geometry, including partial-discharge or application-relevant withstand evidence.
4. **Coolant pressure drop** at the frozen flow point.
5. **Hydraulic pump input power**, including auxiliaries in the declared boundary.
6. **Leak-tightness / pressure-hold** and coolant compatibility, with method and exposure conditions.
7. **Thermal-cycle integrity** and relevant vibration/mechanical retention.

A value is not comparable if method, calibration, uncertainty, unit, physical evidence identity, or system scope is missing. Uncertainty touching a hard limit is Indeterminate; only a fully out-of-limit interval is a definite Fail.

## 4. Benefit metrics

Select and preregister benefits based on the actual bottleneck:
- winding-to-coolant effective thermal resistance (K/W);
- continuous current capability at fixed hotspot and loss limits;
- system mass and occupied volume;
- total electrical + hydraulic auxiliary energy over the duty;
- accepted-part cost, first-pass yield and manufacturing lead time;
- coolant inventory and maintenance burden where measurable.

Hotspot reduction is not a benefit if it requires more pump power or higher copper loss. Thermal resistance is not comparable without matching flow, inlet temperature and boundary conditions. Yield must preserve the full denominator of initiated builds, including failures, rework and scrap.

## 5. Candidate and baseline strategy

At minimum compare an optimized conventional winding + cooling baseline against the AM-enabled stack. A hybrid thermal subassembly and a second conventional comparator (e.g. optimized jacket, slot exchanger or direct cooling) are recommended. A reluctance-only or unrelated control may add context but cannot replace the conventional baseline.

Baseline optimization receipts are frozen before candidate comparison and bind search space, objective, constraints, evaluation budget and independent restarts. The v0.1 floor is 100 evaluations and 3 independent restarts; this is a fairness minimum, not proof of global optimality.

## 6. Experimental strategy

Use two stages:
- **Screening:** DOE / space-filling / expert-prior-informed experiments on coupons and subassemblies. Simulation may prioritize experiments but cannot be promoted as physical truth.
- **Qualification-oriented comparison:** matched full subassembly tests with physical acceptance, independent build replication, and holdout lots/builds. At least 3 independent builds and 2 material lots are an exploratory minimum, not M5 production qualification.

Compare experiment policies by cost to first qualified stack, total cost, false-feasible selections, unresolved outcomes, interval calibration, holdout coverage, and feasible Pareto discoveries. Hold out by lot/build/process revision, never by random rows from one build.

## 7. Decision outcomes

- **NotComparable:** boundary, family, metric or trial identities do not match.
- **Fail:** candidate definitively violates an absolute hard requirement.
- **Indeterminate:** physical evidence, M4 acceptance, uncertainty, calibration, optimization receipt or required comparator is missing/unresolved.
- **InferiorToBaseline:** at least one mandatory function metric misses the frozen non-inferiority margin against a required comparator.
- **NonInferiorNoStackBenefit:** candidate is admissible and non-inferior, but no predeclared benefit robustly beats every required comparator.
- **StackAdvantage:** M4 acceptance, all hard requirements, non-inferiority to every mandatory comparator, and a strict uncertainty-separated improvement in at least one same benefit metric against every mandatory comparator.

No result is standards certification, production release, M5 qualification, or a universal claim that AM is superior.

## 8. Standards/data crosswalk

- **ISO/ASTM 52951:2026:** AM part data packages from design to acceptance; technical basis is PBF-LB/M but generalized workflow requirements can relate to other AM processes. A data-package reference, not a complete test/qualification rule. [3]
- **ISO/ASTM 52953:2025:** registration of process-monitoring and QC data. Its stated scope is metallic PBF-LB, with procedures potentially applicable elsewhere; it does not cover data cleansing, sensor calibration or image processing. [4]
- **ISO/ASTM 52948:2026:** imperfection classification for metallic PBF-LB/PBF-EB; it does not define acceptance criteria or dimensional scales. [5]
- **ISO/ASTM 52920:2023:** industrial AM processes and production-site qualification principles.
- **ISO/ASTM TS 52930:2021:** PBF-LB equipment IQ/OQ/PQ only; not a substitute for winding, dielectric, cooling or component qualification.
- Electrical insulation, dielectric, cooling-loop, leak, vibration and thermal-cycle methods must be selected for the application and recorded with scope. This protocol claims no conformity.

## 9. Benchmark targets and competitive strategy

The first research win is a *matched physical result*, not a large percentage target. A valid advantage should demonstrate one of:
1. lower winding hotspot or thermal resistance at matched AC loss and pump input;
2. lower pump power / total auxiliary energy or system mass at matched thermal and electrical performance; or
3. a Pareto-improving feasible stack across thermal, electrical, hydraulic and cost dimensions while every hard requirement remains satisfied.

Aim beyond the current literature by publishing a reproducible test pack: frozen scope, raw waveform-linked observations, thermal maps, pressure-flow curves, dielectric records, build genealogy, calibrated uncertainty intervals, comparator optimization receipts, full cost/yield denominator and independently replayable decision output.

## Sources

[1] IEEE Access, “Recent Advances in Additive Manufacturing for High-Power-Density Electrical Machines: A Topical Review of Winding Architectures, Integrated Cooling, and System-Level Technology Readiness,” 24 Aug 2026, DOI: 10.1109/ACCESS.2026.3726145, https://doi.org/10.1109/ACCESS.2026.3726145.

[2] Fodor et al., “Materials-Driven Additive Manufacturing for Electrical Machines: Qualification-Oriented Design Beyond Geometric Freedom,” first published 9 Sep 2026, DOI: 10.1002/adem.71259, https://doi.org/10.1002/adem.71259.

[3] ISO/ASTM 52951:2026, https://www.iso.org/standard/76828.html.

[4] ISO/ASTM 52953:2025, https://www.iso.org/standard/84117.html.

[5] ISO/ASTM 52948:2026, https://www.iso.org/standard/84919.html.
