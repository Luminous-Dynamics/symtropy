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
| FMU-Check | Static FMU structural validation | Static preflight only; not dynamic qualification |
| SSP 2.0.1 | System topology / parameterization package | System-structure portability reference; not a master algorithm |
| VDMCheck | Formal-model-based FMI behavior checks | Complementary semantic validation where compatible |

OpenModelica documentation lists the three OMSimulator CS master methods above. The release maturity must also be explicit in every comparison. As of 20 November 2026, the OMSimulator repository identifies `maintenance/v2.1` as the stable 2.1 branch and v2.1.3 as its latest release; `master` is explicitly the active unstable development branch. The OpenModelica 1.27 documentation reports an OMSimulator build version of `v2.1.3.post17-g81bd489`. Use a pinned v2.1.3 release/build as the production-stability comparator. Development-branch builds may be tested in a separately labelled exploratory lane, but must never be silently mixed into the stable leaderboard. I found no authoritative release evidence sufficient to retain the former specific `3.0.0`/`3.1` version-plan claim, so it is removed.

FMPy 0.3.32 is the current PyPI release (9 September 2026). The published universal wheel has SHA-256 `b379deb9417b842028003a288d00c1c5f138d1580e1ff12cc6c42a98719cecf0`, but PyPI reports no provenance attestation for that wheel. Pin and retain the wheel digest, and report the absent publisher provenance rather than implying a verified build origin.

The current OpenModelica User Guide reports FMI 2.0.5 as supported and FMI 3.0.2 as experimental. The current download page lists OpenModelica 1.27.1 as the latest official release and 1.28.0-dev as the nightly development build. Keep two separate FMI compatibility lanes: a mature FMI 2.0.5 production-comparator lane and an exploratory FMI 3.0.2 lane. Do not aggregate them into one leaderboard. For every run:
- pin the exact OpenModelica/OMSimulator release, commit and binary;
- record the actual FMI version used by each FMU and importer;
- record each FMU capability flag and whether the importer supports it;
- report unsupported capabilities as `NOT_SUPPORTED` / `NOT_EVALUABLE`, not as a passing result.

Sources:
- [OMSimulator solver methods](https://openmodelica.org/doc/OpenModelicaUsersGuide/latest/omsimulator.html)
- [OMSimulator branch/release maturity: stable maintenance/v2.1 at v2.1.3; master is active and unstable](https://github.com/OpenModelica/OMSimulator)
- [OpenModelica official releases: 1.27.1 official; 1.28.0-dev nightly](https://openmodelica.org/download/download-windows/)
- [OpenModelica User Guide — FMI 2.0.5 supported / FMI 3.0.2 experimental](https://openmodelica.org/doc/OpenModelicaUsersGuide/OpenModelicaUsersGuide-latest.pdf)
- [FMPy 0.3.32 release and wheel SHA-256](https://pypi.org/project/FMPy/0.3.32/)
- [FMPy supported FMI interfaces](https://github.com/CATIA-Systems/FMPy)
- [FMI Implementers' Guide — validation recommendations](https://modelica.github.io/fmi-guides/main/fmi-guide/)

## 3A. Adjacent method references (not direct FMI-master competitors)

The direct OMSimulator master comparison is only one layer. Use these established solvers/coupling libraries as *method references* when comparing component integration and interface-coupling choices. Do not add them to the direct FMI-master leaderboard without a common FMU adapter and equivalent scope.

| Method reference | Pinned research snapshot | Role | Correct comparison |
|---|---|---|---|
| SUNDIALS ARKODE MRIStep / ExtSTS | SUNDIALS 7.9.0 (released 3 Sep 2026) | Multirate time-integration reference, especially parabolic components | Compare component trajectory/error/CPU envelopes on the same analytic stiff diffusion subproblem; not an FMI master |
| preCICE implicit coupling | v3.5.0 (released 11 Nov 2026) | Explicit/implicit partitioned multi-physics coupling, convergence measures, IQN-ILS acceleration and conservative PU-RBF mappings | Compare interface residual/convergence, mapping conservation, rollback/checkpoint cost and coupling iterations using an equivalent two-participant adapter; not an FMI importer |
| OpenModelica / OMSimulator | Pin exact build; User Guide snapshot reports FMI 3.0.2 as experimental | Actual FMI importer/master path | Direct master leaderboard, with capability flags and unsupported paths listed |

SUNDIALS 7.9.0 adds extended Super Time Stepping through ARKODE MRIStep for multirate parabolic simulations. It is a component integrator, not an FMI co-simulation master. preCICE v3.5.0 adds conservative PU-RBF mappings via Kokkos and more robust timestamp comparisons for longer-running simulations; preCICE supports serial/parallel and explicit/implicit schemes, with quasi-Newton acceleration. A method reference is only valid when the same equations, coupling window, mapping/conservation rules, tolerance, initial conditions and compute budget are compared.

Sources:
- [SUNDIALS 7.9.0 release information](https://computing.llnl.gov/projects/sundials/sundials-software)
- [preCICE coupling scheme configuration](https://precice.org/docs/configuration/basics/coupling/)
- [OpenModelica current User Guide — FMI support table](https://openmodelica.org/doc/OpenModelicaUsersGuide/OpenModelicaUsersGuide-latest.pdf)

## 3B. Verified release-maturity snapshot (20 November 2026)

| Tool / lane | Verified release or branch | Qualification treatment |
|---|---|---|
| OMSimulator stable | `maintenance/v2.1`, v2.1.3 (OpenModelica docs build: v2.1.3.post17-g81bd489) | Primary stable FMI-master comparator; pin binary and commit |
| OMSimulator development | `master` | Explicitly active/unstable development lane; never mix into stable rankings |
| OpenModelica official | 1.27.1 | Latest official release shown by the project download page |
| OpenModelica nightly | 1.28.0-dev | Exploratory only; not a production release |
| OpenModelica FMI compatibility | FMI 2.0.5 supported; FMI 3.0.2 experimental | Separate results into distinct compatibility lanes |
| FMPy importer | 0.3.32 (9 Sep 2026) | Latest published PyPI importer in this snapshot; wheel digest pinned; PyPI reports no provenance attestation |
| preCICE adjacent coupling | 3.5.0 (11 Nov 2026) | Current coupling-method reference, not an FMI master |
| SUNDIALS / ARKODE | 7.9.0 (Sep 2026) | Current multirate integrator reference, not an FMI master |
| SSP | 2.0.1 (5 May 2026) | Fixed system-description package standard for topology/parameterization round trips |

Do not infer FMI 3.0.2 support from a tool's generic FMI 3 statement. Probe each exact importer build against each FMU's advertised capability flags and exercised function path. A tool that cannot run the declared FMI lane is `NOT_EVALUABLE` for that lane, not a failed numerical algorithm and not an implicit pass.


## 3C. SSP 2.0 inter-tool round-trip candidates

The official SSP tool-support matrix currently lists the following declared import/export coverage:

| Tool | Vendor | Declared SSP import/export | Benchmark lane |
|---|---|---|---|
| easySSP / orchideo | eXXcellent solutions | 1.0 / 2.0 | Primary SSP 2.0.1 round-trip candidate |
| FMI Bench | PMSF | 1.0 / 2.0 | Primary independent SSP 2.0.1 round-trip candidate |
| Dymola | Dassault Systèmes | 1.0 / 2.0 | Independent SSP 2.0.1 import/export candidate |
| Scilab/Xcos | Dassault Systèmes | 2.0 | Independent SSP 2.0.1 import/export candidate |
| OMSimulator | OpenModelica | 1.0 | Legacy SSP 1.0 compatibility only unless an exact later build is independently shown to support SSP 2.0 |
| SYNECT Model Management | dSPACE | 1.0 | Legacy SSP 1.0 compatibility |
| Model.CONNECT | AVL | 1.0 | Legacy SSP 1.0 compatibility |
| Simcenter System Architect | Siemens | 1.0 | Legacy SSP 1.0 compatibility |
| Simcenter Studio | Siemens | export 1.0 only | Export-only; cannot qualify end-to-end round-trip |
| OpenMCx | Eclipse Foundation | import 1.0 only | Import-only; useful for read/semantic-inspection lane |
| Ecos | NTNU Ålesund | 1.0 | Legacy SSP 1.0 import/export candidate |

The official SSP tool page gives standard-version coverage, but does not pin product build numbers or feature-by-feature round-trip behavior. Thus this is a *candidate matrix*, not proof that a specific installed release correctly preserves the GeoFusion package. Each campaign must record exact product/version/build, host OS, import warnings, export warnings, resulting package digests and a canonical diff of components, source URIs, connectors, connector kinds, types, unit exponents, parameter values and connections.

SSP 2.0.1 is a maintenance patch to SSP 2.0. The SSD schema's version attribute encodes only major/minor (here `2.0`), not the patch version; the exact SSP release and XSD blob hashes therefore belong in the benchmark manifest. Do not infer SSP 2.0.1 support from a tool's SSP 1.0 listing, or infer correct round-trip behavior solely from a `2.0` label.

The initial cross-tool target is **FMI Bench ↔ easySSP**, with Dymola and Scilab/Xcos as additional SSP 2.0 candidates. Import the exact same SSP package into each, export it, then cross-import the exported results. Compare canonicalized graph and FMU resource digests. Exact build numbers, licensing/access, host OS and supported subset still must be pinned per run. Export-only and import-only entries are never scored as full round-trip support. Unsupported or inaccessible tools are `NOT_EVALUABLE`, not a fail and never a pass.

Source: [official SSP tool support matrix](https://ssp-standard.org/tools/); [SSP 2.0.1 specification and versioning](https://ssp-standard.org/docs/2.0.1/).

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
- FMI version, interface type and every relevant capability flag;
- exact SSP 2.0.1 package, schema version, component topology, connections, unit definitions and parameter sets;
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
| Evidence corruption | Provenance and fail-closed behavior | Wrong model digest, stale boundary, bad unit |
| System-structure round trip | SSP cross-tool exchange | Missing component, rewired connector, changed unit or parameter |
| Unsupported feature | Honest capability handling | Silent fallback or unreported emulation |
| Causal predictor versus oracle | Measure forecast error without future-sample leakage | Predictor reads a future sample, limiter is unreported, or bound is invented |
| Derivative-assisted coupling | Test FMI directional/adjoint derivative paths where advertised | Derivative function called when unsupported or wrong seed/value reference |
| Clock/event coincidence | Preserve clock tick cause, event time and superdense ordering | Dropped, reordered or causally inverted clock/event |
| Predictor envelope stress | Exercise bounded linear prediction near extrema and discontinuities | Predicted flux exceeds declared envelope or projection is not recorded |

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

**Adaptive coupling and prediction**
- predictor family and data-access contract (zero-order hold, causal bounded predictor, exact interval-average oracle, derivative-assisted, iterative);
- number of historical samples consumed and future-sample reads (must be zero for causal lanes);
- predictor residual against the later observed interval integral and trajectory error against the analytic reference;
- envelope-limiter activation count and maximum correction, with the exact declared source bounds;
- local error estimate calibration, rejected-step decisions and error reduction per communication call;
- derivative API calls, seed vectors and derivative results when the FMU advertises directional/adjoint derivatives.

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
- deterministic receipts for deterministic fixed inputs/builds;
- zero future-sample access in any lane labelled causal;
- 100% of predictor limiter activations, declared bounds and interval corrections recorded;
- SSP structural round-trip preserves 100% of declared components, connections, parameter values and unit semantics across the tested tools;
- failures, timeouts and unsupported cases remain in the published denominator.

Stretch objectives (not achieved results):
- move the trajectory-error / conservation-residual / latency Pareto frontier outward against each matched baseline;
- require fewer communication calls for equal-or-better accuracy and conservation;
- keep event-time error within one declared master-clock tick on analytic fixtures;
- increase valid-feature coverage without increasing false acceptance;
- minimize time-to-qualified-result rather than raw simulation speed.

A faster result that is less accurate, less conservative, less repeatable or less auditable is not a win. A win must preserve all hard gates under the same scenario, FMUs, observation schedule, tolerances and compute budget.


## 8A. Adaptive, event-aware and predictor-qualified coupling

The FMI standard deliberately leaves the co-simulation algorithm to the importer. FMI 3's optional mechanisms create a feature space that a serious master benchmark must exercise separately: variable communication steps; state-based reject/repeat where the FMU advertises state support; intermediate updates; early return with the actual `lastSuccessfulTime`; external event mode; clock activations; and optional directional/adjoint derivatives. The current FMI Implementers' Guide explains that early return can preserve the exact internal event time, that event mode transfers event iteration responsibility to the importer, and that clock ticks have a cause which the importer must schedule. Partial derivatives can support iterative co-simulation and algebraic-loop solution.

This yields four distinct predictor/coupling tracks:

1. **Reference ZOH:** left-endpoint sample-and-hold; simple, causal baseline with first-order global error on smooth forcing.
2. **Oracle interval average:** exact whole-interval source integral; non-causal upper bound / method reference, never reported as a causal master win.
3. **Causal bounded linear prediction:** current and previous committed source samples only, explicit source envelope, limiter receipts and later correction against observed interval flux. The current analytic oracle records about 12.94× lower 15-step endpoint error than ZOH while remaining about 6.13× less accurate than the exact-average oracle.
4. **Derivative-assisted / iterative coupling:** use FMI-reported output derivatives or directional/adjoint derivatives only when modelDescription capabilities permit; compare explicit/implicit convergence, iteration count, checkpoint/rollback cost and residual reduction against preCICE-style coupling references.

The next master-algorithm paradigm is an **event-fenced, energy-aware adaptive scheduler**. It should propose a communication horizon from a combined error budget (state prediction error, interface mismatch, conservation residual and model uncertainty), then shorten that horizon before the earliest known event/clock tick. Every decision should carry the input samples and capability flags it used. It must distinguish an observed bound from a heuristic estimate; it may not call a heuristic a proof. On an unavailable checkpoint or derivative feature it must choose a separately qualified fallback lane or return `NOT_EVALUABLE`—never silently emulate the feature.

Sources: [FMI 3.0 Implementers' Guide — early return, events and clock activations](https://modelica.github.io/fmi-guides/main/fmi-guide/#co-simulation-specific-issues); [FMI 3.0 Implementers' Guide — partial derivatives](https://modelica.github.io/fmi-guides/main/fmi-guide/#advanced-features); [FMI 3.0.2 Co-Simulation specification](https://fmi-standard.org/docs/3.0.2/).

## 9. Existing work to compose

- [#6198 — FMI trace verifier](https://github.com/Luminous-Dynamics/symtropy/pull/6198): rational clock mapping, step outcomes, retry lineage and common barriers.
- [#6154 — typed GeoFusion energy contracts](https://github.com/Luminous-Dynamics/symtropy/pull/6154): typed carriers, source allocation and first-law arithmetic.
- [#6143 — GeoFusion architecture and fixtures](https://github.com/Luminous-Dynamics/symtropy/pull/6143): system boundaries and analytic benchmark framing.
- [#5871 — generic multirate co-simulation algorithm tournament](https://github.com/Luminous-Dynamics/symtropy/issues/5871): reuse algorithm taxonomy; do not create a parallel generic framework.
- [#1467 — federated co-simulation boundary](https://github.com/Luminous-Dynamics/symtropy/issues/1467): existing FMI 3.0.2 + SSP 2.0.1 architecture owner; this benchmark consumes its composition boundary.
- [#3173 — conservative shared-flux coupling reference](https://github.com/Luminous-Dynamics/symtropy/pull/3173): reuse its conservation and stale-port negative-control lessons.

## 10. Claim ceiling

A successful campaign can qualify interoperability, numerical coupling, time semantics, physical-boundary accounting and evidence replay for the declared FMUs and synthetic/reference models. It cannot establish geothermal resource quality, superhot materials, fusion plasma performance, net-electric export, plant economics, grid approval, equipment safety or physical plant performance.
