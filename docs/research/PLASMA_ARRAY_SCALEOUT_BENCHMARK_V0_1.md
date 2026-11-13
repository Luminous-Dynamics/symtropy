# Plasma Array Scale-Out Benchmark V0.1

**Benchmark ID:** `PLASMA-ARRAY-SCALEOUT-001`  
**Status:** research contract + synthetic fixture; not a physical qualification  
**Date:** 2026-11-12  
**Owner:** Symtropy scenario/replay; Symthaea evidence and process benchmark

## 1. The field-level opportunity

The September 30, 2026 *High Voltage* roadmap, *Large-Scale Applications of Plasma Technologies: Progresses and Challenges* (DOI [10.1049/hve2.70241](https://doi.org/10.1049/hve2.70241)), identifies four recurring barriers across application domains:

1. controllability of plasma parameters and discharge modes during scale-up;
2. system energy efficiency from grid to outcome;
3. long-term reliability and maintainability under realistic duty cycles;
4. quantitative, transplantable diagnostics and harmonised benchmarking tied to application outcomes.

This matches the central miniaturisation problem: making the discharge smaller is easy to celebrate and hard to translate. The real test is whether a complete array delivers more *accepted useful service* per installed mass, cost, energy and maintenance burden without losing uniformity or controllability.

The roadmap also warns that increasing plasma area, volume, flow or duty cycle can change discharge mode, electron-energy distribution, gas heating and reaction pathways. A scale-out law must therefore be measured, not assumed.

## 2. Paradigm shift: from device metrics to a service-producing array

The benchmark makes five design shifts explicit.

### 2.1 Number-up is a systems problem

Compare one cell, small arrays, published reference sizes and prospective stress sizes. Cell count alone is not throughput. Common power electronics, gas manifolds, pumps, cooling, control, exhaust, packaging and maintenance can become dominant bottlenecks.

### 2.2 Preserve local state; aggregate only after qualification

Every cell has an immutable identity, configuration digest, declared state, observed state, useful output, energy/resource share, uncertainty and terminal disposition. Do not let array averages hide a dead, unstable or badly performing cell.

### 2.3 Optimize accepted service, not plasma appearance

Each application owns its own quality-gated service unit:
- surface activation: accepted area meeting a declared surface-property threshold;
- water treatment: treated volume meeting declared contaminant and by-product endpoints;
- material synthesis: accepted product mass meeting size, distribution, purity and yield constraints;
- decontamination: accepted treated area or load with declared organism/matrix and validated dose.

These are separate leaderboards. Do not combine them into one plasma score.

### 2.4 Design graceful degradation before scaling

Test ignition/quench cross-talk, failed-cell isolation, shared-supply bottlenecks, manifold gradients, stale telemetry and service remaining after a fault. A 100-cell array that fails catastrophically when one element shorts is not better than a smaller maintainable array.

### 2.5 Use plug-to-product accounting

The numerator is complete grid-side energy for the declared duty cycle, including conversion losses, standby/startup, gas supply or generation, pumps, cooling, control, exhaust and failed attempts. The denominator is quality-accepted service—not plasma-cell watts, electrode energy, plume brightness or nominal throughput.

## 3. Literature and industry anchors

The source register in Symthaea [MICROPLASMA-001 #11780](https://github.com/Luminous-Dynamics/symthaea/issues/11780) currently includes:

| Architecture | Published reference | What it establishes | What it does not establish |
|---|---|---|---|
| Miniature helium APPJ | Mao et al., *Environmental Technology & Innovation* (2026), [10.1016/j.eti.2026.104852](https://doi.org/10.1016/j.eti.2026.104852) | Compact supply and in-vitro outcomes under a specific test | Clinical performance, long electrode life or low lifecycle gas cost |
| Membrane-stabilized microfluidic DBD | Liu et al., *Lab on a Chip* (2025), [10.1039/D4LC00960F](https://doi.org/10.1039/D4LC00960F) | Stable plasma/liquid interface and controlled nanoparticle synthesis | Industrial throughput or whole-system efficiency |
| Five-cell RF array | Martinet et al. (2023), [10.1002/ppap.202200131](https://doi.org/10.1002/ppap.202200131) | Five independently ignitable/quenchable cells | A demonstrated tens- or hundreds-cell production array |
| 43-element LTCC CAP array | Bhattacharya et al. (2026), [10.1109/TPS.2025.3646554](https://doi.org/10.1109/TPS.2025.3646554) | 43-element / 10 × 10 cm array and one specified biofilm result | Universal efficacy, full lifecycle economics or a qualified 100-cell design |

The September 2026 roadmap is important because it argues that plasma scale-up requires quantitative process-window control, diagnostics and grid-to-product metrics. This benchmark is designed to make those requirements executable rather than invent another universal plasma score.

## 4. Metrics and definitions

All calculations are performed separately for each declared application endpoint and exact service-quality gate.

Let:
- (N) = installed cell count;
- (q_i) = accepted useful service delivered by active cell (i);
- (Q_N = \sum_i q_i) = aggregate accepted service;
- (Q_1) = accepted service from the matched one-cell reference;
- (E_{grid}) = complete grid-side energy for the duty cycle.

### Scale-out efficiency

\[
G_N = \frac{Q_N}{N Q_1}
\]

(G_N=1) is ideal linear scale-out; (G_N<1) indicates sublinear delivery. Values above one are possible only if the declared cell design or operating point differs; they must not be interpreted as free scale-up gains.

### Per-cell output uniformity

\[
CV = \frac{s(q_i)}{\bar q}
\]

Use the sample standard deviation over active, valid cells. Report failed/isolated cells separately; never erase them from the fault/reliability denominator. CV is undefined when the mean is zero or fewer than two eligible cells remain. A confidence interval or uncertainty bound is required for any physical acceptance claim.

### Complete-system specific energy

\[
SEC = \frac{E_{grid}}{Q_{accepted}}
\]

Report both total system energy and each sub-meter. Unknown auxiliaries are not zero. The energy boundary must remain identical for baseline and candidate.

### Cross-talk and fault containment

For each declared adjacency/interference edge, report the fractional change in a neighbour's output against its isolated baseline, with intervention identity and uncertainty. Fault containment is the fraction of injected faults isolated without uncommanded state transitions in unaffected cells.

### Graceful degradation

\[
R_{service} = \frac{Q_{faulted}}{Q_{healthy}}
\]

Compare against an application-specific service floor. The floor is not universal; water treatment, a moving web and a batch synthesis process have different acceptable failure envelopes.

### Reliability and lifecycle

Per physical unit, preserve operating hours, start/stop cycles, pulse/discharge count, drift, failed attempts, component replacement, service interval and uncertainty. Do not convert programme-wide exposure across multiple units into a per-unit life claim.

## 5. Proposed targets (research goals, not standards or achieved results)

After the source-specific measurement system and one-cell baseline are independently reproduced:

- (G_N \ge 0.90) at N = 5, 43 and 100, under identical service-quality constraints;
- upper uncertainty bound of cell-output (CV \le 5\%) over a declared process window;
- isolate a failed cell without inducing an uncontrolled neighbour transition;
- preserve an application-specific minimum service floor under at least one injected cell failure;
- retain every attempt, failed run, misfire and cell replacement in the denominator;
- stretch objective: at least 25% lower complete-system energy per accepted service unit or lifecycle cost than the best matched incumbent.

These are falsifiable programme targets. They should be revised if calibration uncertainty, application sensitivity or safety constraints do not support them.

## 6. Synthetic scenario ladder

The accompanying `benchmarks/plasma/plasma-array-scaleout-v0.json` freezes twelve solver-free cases at N = 1, 5, 43 and 100. N=1 is the reference; N=5 corresponds to the published individually controlled RF architecture; N=43 corresponds to the published LTCC array; N=100 is a prospective stress case, not a published demonstration.

The fixture includes:
- ideal linear scale-out;
- a 43-cell manifold gradient;
- an outlier cell hidden by aggregate mean;
- 100 cells with one failed cell isolated;
- cross-talk during neighbour operation;
- stale cell telemetry;
- incomplete grid-to-outcome energy boundary;
- mismatched service units;
- stale geometry/configuration identity;
- zero-output and missing failure-denominator cases.

All fixture values are artificial and marked synthetic. Expected dispositions are explicit: synthetic pass, blocked, or rejected. The current frozen fixture has one SYNTHETIC_PASS, four BLOCKED and seven REJECTED dispositions. In particular, the five-cell linear and 100-cell isolated-failure scenarios retain their useful-service arithmetic but are BLOCKED because cross-talk coverage is not measured. A fixture pass only validates the benchmark contract and arithmetic, not plasma physics.

## 7. Reproducibility and admission contract

A physical benchmark record should carry:
- device, cell, geometry, material, gas, power-supply and firmware identities;
- calibration IDs, traceability and uncertainty;
- measured current/voltage waveforms and complete power boundary;
- per-cell gas/flow/pressure and temperature where applicable;
- application-specific output measurement, quality gate and raw data digest;
- all attempts, outages, failures, fault interventions and maintenance;
- ambient/environment conditions and exact test configuration;
- independent review and source/evidence class.

Do not admit a vendor datasheet as an independent observation. Keep reported, simulated, bench-tested, independently reproduced, qualification-tested and field/flight-demonstrated evidence separate.

## 8. Future implementation sequence

1. Freeze and validate this synthetic fixture.
2. Implement a deterministic evaluator and mutation/negative tests in Symtropy without a plasma solver.
3. Add an independently implemented calculation oracle.
4. Reconstruct the five-cell and 43-cell sources only to the extent raw data and calibration are available.
5. Establish a matched one-cell baseline before interpreting N-fold scale-out.
6. Compare against the best non-plasma or incumbent process at matched accepted service, energy, quality, cost and duty cycle.
7. Only then propose an instrumented physical test, owned by the appropriate facility/operator and safety authority.

## 9. Scope boundary

This benchmark covers low-temperature process-plasma arrays. It is not a pulsed-plasma spacecraft propulsion benchmark, thermal-arc model, MHD solver, fusion model, medical-device qualification or authorization for high-voltage/RF/gas operation.

Related:
- Symtropy [PLASMA-TWIN-001 #1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685)
- Symtropy [PLASMA-ARRAY-SCALEOUT-001 #4926](https://github.com/Luminous-Dynamics/symtropy/issues/4926)
- Symthaea [MICROPLASMA-001 #11780](https://github.com/Luminous-Dynamics/symthaea/issues/11780)
- Symthaea [PLASMA-ENERGY-001 #11718](https://github.com/Luminous-Dynamics/symthaea/issues/11718)

## 10. Cross-talk is a measured coverage contract, not an empty-list default

The evaluator distinguishes three statuses:
- `NOT_APPLICABLE`: only for the one-cell reference, with no pair/edge records;
- `NOT_MEASURED`: there is no admitted cross-talk observation; for N>1 this blocks array qualification;
- `COMPLETE`: an explicit sampling-plan ID, required neighbour-pair IDs, observed pair IDs and an edge record for every declared sampled pair are present.

An empty edge list must never mean measured zero cross-talk. A finite empty sample is not a null effect. For each observed edge, preserve the signed neighbour-output delta but threshold its absolute magnitude, because both an increase and a decrease in neighbour output indicate coupling.

The v0.1 fixture currently has only one explicit synthetic neighbour-pair observation (scale-n5-cross-talk), deliberately above the 5% limit. Other multi-cell cases have NOT_MEASURED coverage and cannot qualify as cross-talk-free. Later physical qualification must define the topology and sampling design from the real hardware architecture, including common-manifold and common-supply coupling, not assume that a one-edge synthetic example covers a real array.

## 11. Expected metrics and energy-score gating

The evaluator now consumes the frozen expected_metrics object:
- numeric scale-out efficiency is checked against the independently derived G_N;
- numeric CV is checked against the derived sample CV;
- computed_from_cells requires a valid cell-derived CV;
- a null CV requires an undefined result (e.g. N=1);
- faulted-service retention is checked against the declared reference.

A result cannot be called internally consistent solely because its terminal disposition matches if its expected numeric metric is wrong.

Grid-energy-per-service is only emitted when the full declared boundary is complete, its utility flags and meter identities are present and gas accounting is valid. A blocked boundary produces None, not a scoreable partial SEC. The one-cell reference scenario is itself run through the same evaluator before it can normalize any array candidate.

The Rust evaluator also rejects unknown fields at the schema boundary. This prevents silent typos in frozen scenario data from disappearing during deserialization.

**Current S0 read-back:** the 12-scenario JSON was independently replayed from the fetched branch: 1 SYNTHETIC_PASS, 4 BLOCKED, 7 REJECTED; 12/12 expected dispositions matched. The independent formula oracle gives G_5=0.96 and G_100=0.9405. These are synthetic scenario calculations, not physical array results.

**Execution status:** 23 Rust tests are authored, including explicit unknown/unmeasured cross-talk, signed-delta, expected-metric, incomplete-energy, reference-validity and independent G_N oracle tests. They have not been compiled or executed in the current environment; no CI pass is claimed.