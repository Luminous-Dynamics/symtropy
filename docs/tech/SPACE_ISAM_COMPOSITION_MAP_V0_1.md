# Luminous Space Industrial Composition Map v0.1

**Status:** research and integration architecture  
**Date:** 19 November 2026  
**Evidence class:** mixed source review + synthetic software benchmarks; no new hardware or flight qualification is claimed

## Purpose

This map describes how the public Luminous Dynamics space programme composes into a service-oriented industrial architecture.

The objective is not to claim that Luminous has built a launch vehicle, orbital servicing spacecraft, lunar factory or shipyard. The objective is to make each capability, interface, evidence class, dependency and qualification boundary explicit, then benchmark the composed system against established aerospace practice.

NASA's 2026 *In-Space Servicing, Assembly, and Manufacturing (ISAM) State of Play* is the external capability taxonomy. It contains 621 capability-area entries, 61 facility descriptions and 169 developer profiles across 11 functional areas. It is an inventory and state-of-play survey, not a comparable end-to-end leaderboard. NASA's entry threshold requires physical hardware to exist as a ground prototype or flight demonstration; a concept presentation alone is insufficient.

Sources:
- NASA 2026 ISAM State of Play: https://www.nasa.gov/isam/
- NASA 2026 report (NTRS record): https://ntrs.nasa.gov/citations/20260008638
- NASA 2026 ISAM overview: https://www.nasa.gov/humans-in-space/commercial-space/nasas-isam-state-of-play-the-industry-standard-for-space-servicing-assembly-and-manufacturing/

## 1. Capability taxonomy

NASA's 11 functional capability areas are:

1. Robotic manipulation.
2. Rendezvous and proximity operations (RPO), capture, docking and mating.
3. Relocation.
4. Prepared repair, maintenance, upgrade and installation.
5. Unprepared repair, maintenance, upgrade and installation.
6. Refueling and fluid transfer.
7. Structural manufacturing and assembly.
8. Recycling, reuse and repurposing.
9. Parts and goods manufacturing.
10. Surface infrastructure.
11. Inspection and metrology.

NASA records three distinct cross-cutting capability areas:

- Software and algorithms.
- Logistics and operations.
- Laws, policies and standards.

These cross-cutting areas support the 11 functional areas; they are not three more functional capability areas.

## 2. Composed service architecture

The programme should be evaluated as an end-to-end chain of accepted services:

`Earth manufacturing and launch -> orbital access -> orbital logistics -> RPO -> mechanical capture -> prepared interface connection -> service transfer -> assembly or repair -> independent inspection -> continued operations -> end-of-life disposition`

The chain is conditional, not automatic. Each arrow requires a separately defined interface, authority basis, evidence contract and failure response.

| Stage | Primary Luminous owner(s) | External reference / qualification boundary |
|---|---|---|
| Vehicle production, acceptance and reflight | Symtropy [#6149](https://github.com/Luminous-Dynamics/symtropy/issues/6149), draft [PR #6168](https://github.com/Luminous-Dynamics/symtropy/pull/6168) | Production yield, configuration, serial genealogy, reflight and refurbishment are distinct from payload mission success |
| Launch market and learning | Symtropy [#3242](https://github.com/Luminous-Dynamics/symtropy/issues/3242) | Historical standardized price/learning curves are not vehicle-specific production cost or a current launch quote |
| Earth-to-orbit service chain | Symtropy [#3228](https://github.com/Luminous-Dynamics/symtropy/issues/3228), [#3253](https://github.com/Luminous-Dynamics/symtropy/issues/3253) | Count accepted payload delivered to a mission-specific orbit/window, not nominal capacity alone |
| Capability evidence and claim ceiling | Symtropy [#1972](https://github.com/Luminous-Dynamics/symtropy/issues/1972), [#6167](https://github.com/Luminous-Dynamics/symtropy/issues/6167) | NASA 2026 ISAM taxonomy; hardware, software, analytical and operational evidence remain distinct |
| Orbital depot and shipyard lifecycle | Symtropy [#2161](https://github.com/Luminous-Dynamics/symtropy/issues/2161) | Modular assembly, spare burden, maintenance, repair, expansion and end-of-life |
| As-built structural acceptance | Symtropy [#4263](https://github.com/Luminous-Dynamics/symtropy/issues/4263) | Assembly completion does not equal dimensional/joint/load-path acceptance |
| Rendezvous and servicing-arm capture handoff | Symthaea [#12879](https://github.com/Luminous-Dynamics/symthaea/issues/12879), draft [PR #12903](https://github.com/Luminous-Dynamics/symthaea/pull/12903) | The current relative-motion `Docked` result is not latch, retention or servicing evidence |
| External pose/RPOD benchmark profile | Symthaea [#6261](https://github.com/Luminous-Dynamics/symthaea/issues/6261) | SPEED/SPEED+ pose scores do not qualify docking contact, capture or flight autonomy |
| Orbital fault campaign | Symthaea [#7953](https://github.com/Luminous-Dynamics/symthaea/issues/7953) | Synthetic fault injection is not HIL or on-orbit fault tolerance |
| Prepared fluid service | Symtropy [#6178](https://github.com/Luminous-Dynamics/symtropy/issues/6178) | Receiver-accepted mass/composition/temperature/pressure/phase, not gross transferred mass |
| Cryogenic/pressurized thermofluid model qualification | Symtropy [#6187](https://github.com/Luminous-Dynamics/symtropy/issues/6187) | Model admission by fluid, phase, material, pressure/temperature range and context of use |
| Productive off-Earth industrial supply | Symtropy [#5341](https://github.com/Luminous-Dynamics/symtropy/issues/5341) | Occurrence -> reserve -> recovered feedstock -> qualified product -> dependable supply |
| Provenance, custody and capability milestones | Mycelix [#4020](https://github.com/Luminous-Dynamics/mycelix/issues/4020) | Provenance preserves lineage; it does not prove measurement truth or physical acceptance |
| Surface/ground infrastructure geography | Sol-Atlas [#405](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/405), [#393](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/393), [#106](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/106) | Read-only, source-bounded spatial/logistics context; not physical truth or launch authority |

The archived Mycelix Space repository is reference-only and is not a writable implementation target.

## 3. Non-implication rules

The integration layer must encode these as machine-checkable negative controls:

- `launch scheduled != launch attempted != mission success != payload accepted`.
- `booster recovered != booster inspected != booster requalified != ready to re-fly`.
- `relative pose/velocity within approach tolerance != mechanical capture`.
- `mechanical capture != latch retention != prepared interface connection`.
- `interface connected != leak/pressure test passed != fluid transfer complete`.
- `fluid moved != receiver inventory accepted at required composition, phase, temperature and pressure`.
- `robot reports assembled != as-built structure accepted`.
- `resource occurrence != reserve != recovered feedstock != qualified product != dependable supply`.
- `simulation pass != hardware demonstration != in-space demonstration != operational service`.
- `provenance signature != physical truth`.
- `benchmark score != flight qualification or mission authority`.

Unknown, stale, incompatible or unsupported evidence must return `indeterminate` or `blocked`, not be silently defaulted to zero or promoted.

## 4. Paradigm shifts to benchmark

### 4.1 Service-first optimization

Compare accepted services over the lifecycle rather than a single vehicle or component metric.

Examples include accepted payload delivered to a specified orbit and deadline, accepted fluid mass at a required thermodynamic state, accepted structural modules, or maintenance service hours under a declared availability floor.

### 4.2 Co-design manufacturing and operations

A reusable launch fleet is constrained by more than propulsion. Production yield, acceptance testing, inspection, refurbishment stations, spare inventory, qualified suppliers, range capacity and demand form a coupled industrial system.

### 4.3 Inspectability and repairability as design variables

Design interfaces, joint geometry, access paths, fiducials, sensors and replaceable modules together. Benchmark the trade-offs among mass, stiffness, assembly time, detection probability, rework and future repair.

### 4.4 Capability composition before readiness scores

Model required services as dependency graphs. A required subsystem below its evidence threshold blocks the composed service even when other subsystems are mature. Do not average unlike capability classes into a universal readiness number.

### 4.5 Evidence-driven investment

Use value-of-information and staged-capital methods before irreversible infrastructure commitments. A lower nominal cost cannot override safety, service continuity, rights, environmental or physical-feasibility gates.

### 4.6 Interoperable simulation artefacts

Treat model interfaces, configurations, schedules, event identities and evidence receipts as exchangeable artefacts. Keep solver physics and context-of-use admission separate from a model's name or file format.

## 5. Competitive benchmark scorecards

These are programme targets and research questions, not achieved results.

| Scorecard | Required comparators | Proposed stretch target |
|---|---|---|
| Capability evidence coverage | NASA 2026 ISAM taxonomy; source-backed Luminous work items | Every mapped entry has a cited source and explicit evidence state; no software/synthetic item promoted to hardware or flight |
| Composition correctness | Independent interface and dependency-graph oracle | 100% hard dependency violations rejected; 100% missing/unknown required evidence returns blocked or indeterminate |
| Lifecycle economics | Expendable, scheduled-refurbishment, fixed-reuse, transparent adaptive baselines | At least 10% improvement on frozen held-out scenarios only if hard reliability/service constraints are preserved |
| Mission service | Accepted payload/fluid/structure outputs with exact denominator | Report cost per accepted service, P50/P90/P95 delay and lower-tail service; never compare gross capacity alone |
| Resilience | Single-asset, single-supplier, common-cause, range closure and comms faults | Zero hidden service-floor violations on the frozen fault corpus; disclose unmodeled common causes |
| Model credibility | Analytic/manufactured solutions, independent solvers, ground and HIL observations | Error/uncertainty bounds declared per context of use; no self-certification by the candidate model |
| Interoperability | Exact model/configuration identity and exchange profiles | Independent consumers reconstruct identical interfaces, units, schedules and receipt digests |
| Evidence integrity | Missing, stale, duplicated, contradictory and adversarial records | 100% negative-control disposition correct; no unknown-to-zero coercion |

Do not collapse the scorecards into a single weighted "space readiness" or "SOTA" score.

## 6. External benchmarks and standards

### Industry capability references

- NASA's 2026 ISAM State of Play records historical operational ISAM (ISS assembly/maintenance, Hubble servicing and Northrop Grumman's Mission Extension Vehicle) alongside newer demonstrations and development efforts. It reports RRM-3 methane zero-boil-off storage for four months, but the planned fluid-transfer demonstration was inhibited by a system failure. Separately, it records an approximately 10-tonne LOX transfer and gauging demonstration on Starship's third flight test. These are distinct fluid-service claims, not one interchangeable refuelling readiness level.
- DARPA/Caltech's 5 November 2026 NOM4D ASTRA demonstration assembled a truss structure in orbit using a three-axis gantry robot, composite members, custom joints and reflective mesh. DARPA's programme manager explicitly stated that structures hundreds of metres or kilometres long are not presently achievable. Treat this as a bounded assembly demonstration, not a complete orbital shipyard.
- SpaceX's June 2026 EU prospectus reports 40 Falcon launches in Q1 2026, 39 using flight-proven boosters, and 165 Falcon 9 launches in 2025, 157 using flight-proven boosters. This demonstrates operational reuse and cadence; it does not reveal the full refurbishment labour, internal cost or vehicle-specific reliability data needed to reconstruct a full lifecycle model.

### Standards and model credibility

- NASA-STD-7009B (active) defines general practices for model and simulation lifecycle credibility and program/project-approved acceptance criteria: https://standards.nasa.gov/standard/nasa/nasa-std-7009
- NASA-HDBK-7009B (2026) is an implementation guide for NASA-STD-7009B: https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009
- ECSS-E-ST-40-07C Rev.1 (5 August 2025), Simulation Modelling Platform Level 1, addresses simulation-model and environment interfaces to support reuse/exchange: https://ecss.nl/standard/ecss-e-st-40-07c-rev-1-simulation-modelling-platform-level-1-5-august-2025/
- ECSS-E-ST-40-08C (5 August 2025), Simulation Modelling Platform Level 2, addresses simulator artefact instantiation/configuration, event schedules and model links: https://ecss.nl/standard/ecss-e-st-40-08c-simulation-modelling-platform-level-2-5-august-2025/
- ANSI/AIAA S-157-2025 is scoped to prepared-spacecraft **storable, non-cryogenic** fluid transfer; it must not be used as a cryogenic transfer qualification standard: https://webstore.ansi.org/standards/aiaa/ansiaiaa1572025

Standards inform tailored assurance plans. A standard name or compatible schema does not, by itself, validate a model or qualify a physical system.

## 7. South African industrialisation track

The Sol-Atlas regional programme treats Overberg, Houwteq and Matjiesfontein as distinct nodes in a developing South African space-industrial network. Current programme evidence must be stated accurately: Overberg has a suborbital sounding-rocket reference and national orbital launch feasibility/pilot work; Houwteq's AIT capability is being operationalised in phases; Matjiesfontein's deep-space ground-station project is under construction/commissioning. None should be labelled a complete operational orbital-launch chain without source-backed acceptance evidence.

Primary local owner: [Sol-Atlas #405 SOUTH-AFRICAN-SPACE-ECOSYSTEM-001](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/405).

## 8. Immediate execution order

1. Validate the standalone launch-production lifecycle and maintenance-window contract in [Symtropy draft PR #6168](https://github.com/Luminous-Dynamics/symtropy/pull/6168).
2. Validate the capture-handoff negative controls in [Symthaea draft PR #12903](https://github.com/Luminous-Dynamics/symthaea/pull/12903).
3. Complete the NASA ISAM crosswalk in [Symtropy #6167](https://github.com/Luminous-Dynamics/symtropy/issues/6167).
4. Connect capture/interface receipts to fluid-service acceptance in [Symtropy #6178](https://github.com/Luminous-Dynamics/symtropy/issues/6178), while qualifying cryogenic model paths separately in [#6187](https://github.com/Luminous-Dynamics/symtropy/issues/6187).
5. Use Mycelix to preserve immutable identity/custody/correction lineage only after the record contracts are stable; use Sol-Atlas for read-only spatial/facility context; use Symthaea to propose bounded candidates and investigations; use Symtropy for models and independent verification.

## Claim ceiling

This map is a programme architecture and source-bounded research review. It does not claim Luminous has a flight-proven launcher, operational orbital servicer, cryogenic transfer system, orbital shipyard, lunar mine, qualified life-support system, flight-qualified model, or superior commercial launch price. The two linked executable packages currently provide synthetic software reference contracts only; their exact-head hosted validation must be inspected before any software-pass claim is made, and separate physical/operational qualification remains mandatory.
