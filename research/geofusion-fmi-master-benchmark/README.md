# GeoFusion FMI Master Baseline Benchmark v0.1

**Status:** protocol and comparator inventory; no external campaign executed.  
**Parent:** [GEOFUSION-BENCH-001 / #6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156)  
**Execution work item:** [GEOFUSION-FMI-MASTER-BASELINE-001 / #6224](https://github.com/Luminous-Dynamics/symtropy/issues/6224)

## 1. Research question

Can a GeoFusion federation achieve a better *qualified-result frontier* than established FMI co-simulation masters—lower trajectory error and carrier-specific balance residual at equal or lower compute cost—without accepting invalid time, boundary, model-revision or source-allocation evidence?

This is deliberately not a solver replacement claim. The FMI standard defines an interface, not one universal master algorithm. The importer/master advances global time, exchanges data, triggers clocks and handles events. Those decisions must be compared and qualified separately from the component FMUs.

Primary reference: [FMI 3.0.2 specification, Co-Simulation](https://fmi-standard.org/docs/3.0.2/).

## 2. Standards layering

Use the complementary standards at the layer they actually define:

- **FMI 3.0.2** describes the interface and package for an individual dynamic model (FMU). It does not prescribe one co-simulation master algorithm.
- **SSP 2.0.1** describes, parameterizes and packages a complete system of interconnected component models. It provides tool-independent topology, connectors, parameterization, signal flow, unit definitions/conversion or checking, and traceability metadata. Use it as the system-structure baseline around the FMUs, not as a master algorithm or a physical-validation oracle.
- **RO-Crate / PROV-O** are evidence-package and provenance interoperability layers, owned by the existing evidence portability programme; do not make the FMI benchmark invent a provenance graph format.

Pin SSP 2.0.1 (released 5 May 2026) for this campaign. The current SSP development documentation may change independently, so a benchmark must record the exact SSP schema/package version used.

Sources:
- [FMI downloads and complete 3.0.2 package](https://fmi-standard.org/downloads/)
- [SSP 2.0.1 specification](https://ssp-standard.org/docs/2.0.1/)
- [SSP current overview and FMI complementarity](https://ssp-standard.org/docs/main/)
- [RO-Crate 1.3 specification](https://www.researchobject.org/ro-crate/specification/1.3/)
- [W3C PROV-O](https://www.w3.org/TR/prov-o/)

## 3. Comparator roles

| Candidate | Role in this campaign | Matching rule |
|---|---|---|
| OMSimulator `oms_ma` | Fixed-step co-simulation master | Direct master baseline |
| OMSimulator `oms_mav` | Variable-step co-simulation master with tolerance controls | Direct master baseline |
| OMSimulator `oms_mav2` | Variable-step v2 master with enhanced error estimation | Direct master baseline |
| FMPy | FMI 3.0 Co-Simulation / Model Exchange importer and simulation harness | Importer interoperability reference, not a like-for-like master algorithm |
| FMI Reference FMUs / `fmusim` | Standard reference models and basic import validation | Conformance/smoke layer |
| FMU-Check | Static FMU structural validation | Static preflight only; not dynamic qualification |\n| SSP 2.0.1 | System topology / parameterization package | System-structure portability reference; not a master algorithm |
| VDMCheck | Formal-model-based FMI behavior checks | Complementary semantic validation where compatible |

OpenModelica documentation lists the three OMSimulator CS master methods above. The current OpenModelica User Guide reports FMI 3.0.2 support as experimental. Therefore:
- pin the exact OpenModelica/OMSimulator release or commit and binary;
- record the actual FMI version used by each FMU and importer;
- record each FMU capability flag and whether the importer supports it;
- report unsupported capabilities as `NOT_SUPPORTED` / `NOT_EVALUABLE`, not as a passing result.

Sources:
- [OMSimulator solver methods](https://openmodelica.org/doc/OpenModelicaUsersGuide/latest/omsimulator.html)
- [OpenModelica User Guide — FMI version support](https://openmodelica.org/doc/OpenModelicaUsersGuide/OpenModelicaUsersGuide-latest.pdf)
- [FMPy supported FMI interfaces](https://github.com/CATIA-Systems/FMPy)
- [FMI Implementers' Guide — validation recommendations](https://modelica.github.io/fmi-guides/main/fmi-guide/)

## 4. Do not conflate the comparison layers

1. **FMU/package correctness:** modelDescription/schema, binaries, variables, units, capabilities, license and platform.
2. **Importer interoperability:** load, instantiate, configure, run, terminate, and report FMI statuses correctly.
3. **Master algorithm:** global communication schedule, interpolation/hold, variable step, early return, rollback/retry, event ordering and convergence.
4. **Federation correctness:** time alignment, exact boundary identity, causal ordering and carrier-specific conservation.
5. **Physical validity:** independently calibrated model equations, parameter domains, sensor uncertainty and physical evidence.

Passing one layer does not imply the next. In particular, FMI protocol compliance does not prove the component physics, and a zero aggregate energy residual does not prove that terms belong to the same physical boundary or time interval.

## 5. Matched experiment envelope

Every campaign must freeze and publish:
- exact FMU binaries, source revision and cryptographic digest;
- FMU licenses and redistribution status;
- FMI version, interface type and every relevant capability flag;\n- exact SSP 2.0.1 package, schema version, component topology, connections, unit definitions and parameter sets;
- importer/master executable, source commit, build flags and platform;
- component model revisions, parameter sets, initial conditions and units;
- graph topology, port mapping, carrier and physical boundary IDs;
- absolute/relative tolerances, fixed/initial/min/max communication step;
- interpolation, extrapolation and event schedule;
- same simulation horizon and observation/sample times;
- same analytic or independently qualified high-accuracy reference;
- compute budget, CPU model, core allocation, memory limit and wall-clock limit.

A result is not comparable when FMU features, sampling schedules, accuracy targets or compute budgets differ. Such results may be useful as exploratory evidence, but they cannot support a leaderboard win.

## 6. Scenario matrix

| Family | Purpose | Required fault/control |
|---|---|---|
| Smooth steady operation | Establish reference behavior and cost | Analytic steady state |
| Fast wellhead / heat-exchanger transient | Stiffness and multirate behavior | Step-size refinement |
| Thermal storage charge/discharge | State continuity and signed storage delta | Lost or double-applied interval |
| Carrier conversion | Separate thermal and electrical first-law ledgers | Cross-carrier double counting |
| Abrupt facility load change | Event-time and dispatch response | Coincident event and communication point |
| Early return | FMI 3 event/coordination path | Early return disabled or malformed |
| Normal successful-step time deviation | Correct actual-time chaining | Next step incorrectly starts at requested time |
| Reject, retry, rollback, restart | State restoration and exactly-once commit | Changed retry state / duplicate energy |
| Multi-rate components | Heterogeneous clock maps and synchronization | Non-integral conversion, drift, stale clock |
| Evidence corruption | Provenance and fail-closed behavior | Wrong model digest, stale boundary, bad unit |\n| System-structure round trip | SSP cross-tool exchange | Missing component, rewired connector, changed unit or parameter |
| Unsupported feature | Honest capability handling | Silent fallback or unreported emulation |

Initial v0.1 trace tests are synthetic and bounded. Event Mode, Intermediate Update, Scheduled Execution and full FMU rollback remain explicit expansion tracks.

## 7. Scorecard

Publish each metric separately; do not collapse them into an arbitrary weighted score.

**Numerics**
- absolute, relative, RMS, peak and integrated trajectory error against the oracle;
- local and global error under step refinement;
- stiffness/step-size stability envelope;
- event-time deviation and event-order correctness.

**Physical accounting**
- thermal and electrical residuals separately at every declared boundary and interval;
- signed storage delta and conversion loss;
- exact source-allocation edge count;
- duplicate, missing, stale and cross-boundary term detection.

**Temporal/protocol**
- clock-map exactness and synchronization skew;
- missed, duplicated, late or reordered event count;
- accepted/rejected/early-return step counts;
- retry, rollback and restart counts;
- interpolation/extrapolation method and order.

**Performance**
- wall time, CPU time, peak memory and real-time factor;
- P50/P95/P99 step latency;
- FMU calls, communication events, rejected work and time-to-qualified-result.

**Evidence quality**
- false conservation closure and false qualification count;
- invalid-input / unsupported-feature refusal rate;
- replay determinism and receipt reproducibility;
- model, solver, artifact and parameter identity coverage.

## 8. Hard gates and stretch objectives

Hard gates:
- **zero false closures** on the frozen deterministic/adversarial corpus;
- 100% of injected wrong-unit, wrong-boundary, stale-revision, duplicate-allocation, clock and step-order faults are detected or explicitly marked not evaluable;
- every accepted term binds to its exact carrier, physical boundary, source interval and model revision;
- no silent fallback, extrapolation, status downgrade or capability emulation;
- deterministic receipts for deterministic fixed inputs/builds;\n- SSP structural round-trip preserves 100% of declared components, connections, parameter values and unit semantics across the tested tools;
- failures, timeouts and unsupported cases remain in the published denominator.

Stretch objectives (not achieved results):
- move the trajectory-error / conservation-residual / latency Pareto frontier outward against each matched baseline;
- require fewer communication calls for equal-or-better accuracy and conservation;
- keep event-time error within one declared master-clock tick on analytic fixtures;
- increase valid-feature coverage without increasing false acceptance;
- minimize time-to-qualified-result rather than raw simulation speed.

A faster result that is less accurate, less conservative, less repeatable or less auditable is not a win. A win must preserve all hard gates under the same scenario, FMUs, observation schedule, tolerances and compute budget.

## 9. Existing work to compose

- [#6198 — FMI trace verifier](https://github.com/Luminous-Dynamics/symtropy/pull/6198): rational clock mapping, step outcomes, retry lineage and common barriers.
- [#6154 — typed GeoFusion energy contracts](https://github.com/Luminous-Dynamics/symtropy/pull/6154): typed carriers, source allocation and first-law arithmetic.
- [#6143 — GeoFusion architecture and fixtures](https://github.com/Luminous-Dynamics/symtropy/pull/6143): system boundaries and analytic benchmark framing.
- [#5871 — generic multirate co-simulation algorithm tournament](https://github.com/Luminous-Dynamics/symtropy/issues/5871): reuse algorithm taxonomy; do not create a parallel generic framework.\n- [#1467 — federated co-simulation boundary](https://github.com/Luminous-Dynamics/symtropy/issues/1467): existing FMI 3.0.2 + SSP 2.0.1 architecture owner; this benchmark consumes its composition boundary.
- [#3173 — conservative shared-flux coupling reference](https://github.com/Luminous-Dynamics/symtropy/pull/3173): reuse its conservation and stale-port negative-control lessons.

## 10. Claim ceiling

A successful campaign can qualify interoperability, numerical coupling, time semantics, physical-boundary accounting and evidence replay for the declared FMUs and synthetic/reference models. It cannot establish geothermal resource quality, superhot materials, fusion plasma performance, net-electric export, plant economics, grid approval, equipment safety or physical plant performance.
