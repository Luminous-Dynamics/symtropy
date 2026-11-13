# Water Service Engineering — Research and Benchmark Programme v0.1

**Status:** research contract / synthetic-first; no physical product or system qualified  
**Tracking:** Symtropy [#4958 potable-water trains](https://github.com/Luminous-Dynamics/symtropy/issues/4958) · [#4959 pump-system service](https://github.com/Luminous-Dynamics/symtropy/issues/4959)

## Mission

Develop water infrastructure that delivers the required quantity and quality of water, at the point of use, through disturbances and across its full operating life — while minimizing energy, consumables, residuals, cost, maintenance burden and environmental harm.

The engineering unit is **delivered service**, not a filter cartridge, pump nameplate, treatment-stage removal percentage or AI score.

## Programme boundaries

This directory starts two deliberately bounded benchmark families:

1. **Potable-water treatment:** household / point-of-use, community / small supply and municipal / utility treatment trains. It evaluates source-to-tap service and contaminant-specific claims.
2. **Pump-system service:** household/building booster, community transfer, municipal distribution, wastewater lift, storm/flood drainage and remote/solar duty. It evaluates the complete pump + drive + piping + storage + controls system.

They share typed service profiles, uncertainty discipline, comparable lifecycle metrics and fail-closed evidence rules. They do not collapse drinking water, wastewater, stormwater, floodwater, process water or recovered materials into one untyped quantity.

## Compose with existing work — do not duplicate it

| Existing owner | What this programme consumes |
|---|---|
| Symtropy #2152 | Process water/material/utility/residual closure and its known-answer accounting |
| Symtropy #4902 / #4917 / #4919 | Whole-chain sanitation, wastewater reference and toilet bench |
| Symtropy #4386 / #4397 / #4398 / #4401 / #4420 | PAT component physics, network pressure/leakage/PAT coordination, EPANET differential qualification, uncertainty and PAT product family |
| Symtropy #4905 | Sewer inspection robot mission and condition evidence |
| Symtropy #3372 | Water-asset maintenance, renewal and service feedback |
| Mycelix #6100 / #3233 | Raw water observation versus profile-bound derived assessment |
| Mycelix #6034 / #6057 / #6025 / #6029 | Instrument capability, calibration, condition applicability and measurand profiles |
| Mycelix #5431 | Provenance-first Johannesburg water-service evidence |
| Sol-Atlas water/catchment owners | Site, source, network, geography, exposure and rights-cleared regional evidence |

**Important separation:** the existing PAT work studies recovering electricity from pressure/flow. It does not qualify pump-mode performance of a water-supply or drainage pump. Likewise, the existing water-purity evidence work defines how to represent observations and assessments; this programme tests treatment performance and does not mint safety/certification authority.

## Paradigm shifts

- **From product rating to service delivered:** litres reaching a named endpoint that meet a declared use-specific profile.
- **From one-shot removal to lifetime breakthrough:** performance is tracked from run-in through fouling, cleaning, breakthrough, replacement and restart.
- **From pump efficiency to system efficiency:** pump, motor, drive, piping, control, storage and delivered head are measured together.
- **From average water quality to barrier integrity:** each critical contaminant/pathogen has its own claim, method, uncertainty and release rule; no scalar score can compensate for a failed barrier.
- **From centralization vs decentralization to composable service topology:** select household, distributed, central or hybrid treatment based on raw-water risk, operator capability, lifecycle and failure containment.
- **From forecast-driven actuation to evidence-gated intervention:** prediction may recommend; independent measurements and bounded local protections govern physical operation.
- **From maintenance schedules to condition evidence:** infer degradation only within validated observability envelopes; unknown state triggers inspection, not confident diagnosis.
- **From nominal resilience to graceful degradation:** preserve minimum safe service, isolate failed stages, retain local operation and recover without assuming cloud, grid or communications.

## Benchmark philosophy

Every comparison freezes the service boundary, raw-water / duty profile, measurement methods, hardware configuration, software revision, lifecycle boundary and failure cases. Use paired/matched baselines, blind laboratory samples where applicable, held-out sites/seasons/batches, and publish non-converged cases and failures.

Report a **Pareto surface**, not a weighted composite that can hide safety or service regression.

### Evidence tiers

| Tier | Evidence permitted | Claim ceiling |
|---|---|---|
| T0 | Schema, analytic identities, synthetic fixtures, unit and accounting checks | Contract/arithmetic correctness only |
| T1 | Controlled clean-water / component or system bench | Bench performance within the tested envelope |
| T2 | Qualified independent laboratory / endurance testing | Method- and specimen-specific results |
| T3 | Approved supervised pilot with independent sampling and site/operator approvals | Site-, season- and profile-specific service results |
| T4 | Replicated independent deployments and lifecycle evidence | Only this tier supports broad transfer claims |

Synthetic, modelled, sensor-proxy, operator-reported, laboratory, regulatory and field evidence remain separate. An assessment is not a certification, and neither is an actuation authorization.

## Current external comparator snapshot

- WHO published the fourth edition of its *Guidelines for Drinking-water Quality* incorporating the first, second and third addenda on **17 June 2026**. The current framework emphasizes health-based targets, water safety plans and independent surveillance from catchment to consumer: <https://www.who.int/publications/i/item/9789240121225>.
- WHO's HWT Scheme published Rounds III–IV results on **18 March 2026**; 51 household water-treatment products have now been evaluated across all rounds: <https://www.who.int/publications/b/80163>.
- WHO HWT Scheme microbial targets provide an external reference: 3-star = at least 4-log bacteria, 5-log viruses and 4-log protozoa; 2-star = at least 2-, 3- and 2-log respectively. This is a household-treatment microbial-performance scheme, not a complete chemical-water-quality certification: <https://www.who.int/tools/international-scheme-to-evaluate-household-water-treatment-technologies>.
- ANSI/HI 14.6-2022 defines repeatable rotodynamic pump hydraulic acceptance testing, including performance, NPSH and hydrostatic pressure testing: <https://webstore.ansi.org/standards/hi/ansihi142022>.
- DOE's Pump Systems resources emphasize system-level assessment and matching pump, controls and piping to actual requirements: <https://www.energy.gov/cmei/ito/pump-systems>.
- 2026 leak-detection benchmarking shows that high detection accuracy on simulated networks does not solve field localization/generalization by itself. A May 2026 benchmark reported >95% leak detection accuracy on 47,000+ EPANET/WNTR scenarios but identified localization as the harder task: <https://doi.org/10.1016/j.jwpe.2026.110114>. A separate 2026 review highlights real-world validation, heterogeneous networks and comparability gaps: <https://doi.org/10.2166/hydro.2026.069>.

External results are comparators to reproduce, not Luminous results. Standards must be pinned to the exact current edition, jurisdiction and claim before numerical limits are used.

## Initial research increments

1. Freeze the two benchmark protocols and typed service boundaries.
2. Create a machine-readable test manifest and T0 known-answer / negative-control cases.
3. Reconcile every proposed measurement channel to existing Mycelix metrology owners.
4. Freeze specific commercial product and pump baselines, revisions and test conditions before comparison.
5. Define T1 bench designs and independent reference instrumentation.
6. Move to T2/T3 only with laboratory, electrical, mechanical, pressure-system, site and regulatory review.

## Nonclaims

This programme does not yet establish a safe-water product, treatment efficacy, pump efficiency, hydraulic model accuracy, regulatory conformity, laboratory accreditation, physical reliability, commercial readiness or SOTA superiority.


## Third benchmark family added — pressurized leak diagnostics

The programme now also includes household/building leak detection, community / district-metered-area diagnosis and municipal pressurized-network localization. This is separate from gravity-sewer robot inspection and from PAT energy recovery. See [Symtropy #4963](https://github.com/Luminous-Dynamics/symtropy/issues/4963) and [PRESSURIZED_WATER_DIAGNOSTICS_BENCHMARK_V0_1.md](./PRESSURIZED_WATER_DIAGNOSTICS_BENCHMARK_V0_1.md).

The comparison set includes the 2026 47k+ synthetic EPANET/WNTR Net3 benchmark, the 2026 one-year hourly real-utility SCADA dataset, the physical LeakTestbed (280 controlled signals), and the 2026 systematic review. They remain separate populations; simulated detection accuracy is not field accuracy.


## Physical test-cell concept

The staged, non-construction design basis is in [T1_WATER_TEST_CELL_DESIGN_BASIS_V0_1.md](./T1_WATER_TEST_CELL_DESIGN_BASIS_V0_1.md). It defines isolated PUMP-T1, FILTER-T1 and LEAK-T1 configurations, reuses Mycelix H1 instrumentation concepts without misrepresenting H1 as an industrial acceptance rig, and requires competent pressure/electrical/mechanical safety review before build or energization.


## Specialized membrane-fouling benchmark

A dedicated child workstream now benchmarks mechanism-aware membrane fouling estimation, prediction and safe intervention: [Symtropy #4977](https://github.com/Luminous-Dynamics/symtropy/issues/4977). A 2026 comparative review of 78 studies across MF/UF/NF/RO identifies a gap between prediction and deployable supervisory / actuator-linked control. We will evaluate cleaning decisions, membrane integrity, actual post-cleaning recovery and water-quality preservation—not just TMP prediction. Source: https://doi.org/10.3389/frwa.2026.1863523.

## Executable T0 evaluator

The 23-case input corpus now has a **separate expected-output oracle**. The Rust evaluator is in `tools/water-service-t0`, tracked by [#4978](https://github.com/Luminous-Dynamics/symtropy/issues/4978) and included in [PR #4962](https://github.com/Luminous-Dynamics/symtropy/pull/4962). It computes pump duty, VFD/throttling, water/marker closure and fail-closed treatment/network diagnosis outputs, and emits a digest-bound receipt. Source-level independent replay matched 23/23 oracle outputs; native Rust build/tests/Clippy/CI remain unverified until an exact-head run.


## Membrane fouling dataset and benchmark track

The rights-aware crosswalk is in [MEMBRANE_FOULING_DATASET_CROSSWALK_V0_1.md](./MEMBRANE_FOULING_DATASET_CROSSWALK_V0_1.md), with a [four-population admission register](./membrane-dataset-admission-v0.1.json), Draft 2020-12 [admission schema](./membrane-dataset-admission.schema.json), and [benchmark manifest](./membrane-fouling-benchmark-manifest-v0.1.json).

The first records are: 34-month Bristol POU UF telemetry under OGL v3; a separate 12-week Bristol trial with nine laboratory sampling timepoints at three locations under OGL v3; a six-year Moroccan industrial RO SCADA result whose raw data are available on request; and Armstrong et al.'s controlled RO cross-flow transport dataset, whose repository file-level licence still needs confirmation. None of their raw files is acquired in this branch.

### RO transport M0

The first executable membrane increment is [membrane-ro-m0-manifest-v0.1.json](./membrane-ro-m0-manifest-v0.1.json), with five synthetic [inputs](./membrane-ro-m0-cases-v0.1.json), a separate [oracle](./membrane-ro-m0-oracle-v0.1.json), and a [Draft 2020-12 schema](./membrane-ro-m0.schema.json). The Rust evaluator and `membrane-ro-m0` CLI live in `tools/water-service-t0`. They test net driving pressure, concentration polarization, reference permeance and the error from pressure-only/no-CP simplifications. This is formula arithmetic, not an empirical fouling or membrane-performance claim. The present M0 verifies pointwise J/NDP; Armstrong et al.'s published reference water permeance is a through-origin regression slope across the full pressure series. Exact dataset reproduction is therefore still blocked pending authorized OSF workbook acquisition; grouped multi-pressure regression is the next M0B increment.


### M2 sample-time quality label contract

The new [M2 sample-time contract](./MEMBRANE_WQ_SAMPLE_TIME_CONTRACT_V0_1.md), [schema](./membrane-wq-sample.schema.json), five [synthetic fixtures](./membrane-wq-sample-fixtures-v0.1.json) and [manifest](./membrane-wq-sample-manifest-v0.1.json) ensure lab values remain labels only at exact sample time/location/analyte. The workflow rejects post-sample telemetry features, zero-imputation of below-LOD/LOQ results and rejected lab QC labels. A paired same-time/different-location synthetic fixture verifies that treatment-chain locations remain distinct. No actual Bristol sample records are present in these synthetic fixtures.


### RO transport M0B: grouped multi-pressure permeance fit

The new M0B increment adds synthetic pressure series grouped by replicate, with a through-origin least-squares fit of flux against net driving pressure for each replicate. It reports replicate slopes, residual RMSE, mean slope and sample standard deviation. The Rust evaluator/CLI, three grouped cases, separate oracle, schema and manifest are integrated into the exact-head workflow. It tests the regression procedure only; it does not reproduce Armstrong's OSF workbook or measured reference-A values.


### M1 measurement integrity — distinguish process movement from instrumentation defects

The [M1 protocol](./MEMBRANE_MEASUREMENT_INTEGRITY_PROTOCOL_V0_1.md) and eight-case synthetic corpus, oracle, schema and manifest test stable operation, coherent process movement, stuck-at flux, channel dropout, common historian outage, cross-channel bias, out-of-domain values and an ambiguous case that must abstain. A separate Rust evaluator/CLI and independent Python replay are required workflow gates. These synthetic cases do not diagnose real sensors or identify fouling.


## Industry and SOTA benchmark matrix

See [MEMBRANE_INDUSTRY_BENCHMARK_MATRIX_V0_1.md](./MEMBRANE_INDUSTRY_BENCHMARK_MATRIX_V0_1.md) for a source-linked comparison with current DuPont RO/NF normalized-performance and cleaning guidance, EPA low-pressure MF/UF integrity optimization goals, and the 2026 mechanism-aligned, drinking-water and RO sensing reviews. It defines an evidence maturity ladder and a non-collapsed M0–M6 benchmark stack. These industry thresholds are context-specific references, not universal acceptance limits.


### Observation semantics contract

The [Membrane Observation Semantics Contract](./MEMBRANE_OBSERVATION_SEMANTICS_PROTOCOL_V0_1.md) has a versioned JSON Schema and nine positive/negative fixtures. It distinguishes instantaneous sensor values, rolling aggregates, laboratory samples, derived metrics and normalized metrics; it requires explicit units, time-window semantics, calibration/source metadata, lineage and normalization reference inputs. The workflow rejects missing-as-zero, derived-without-lineage, normalized-without-reference and rolling aggregates without a declared window.


## Evidence-twin reference architecture

The [Membrane Evidence Twin blueprint](./MEMBRANE_EVIDENCE_TWIN_REFERENCE_ARCHITECTURE_V0_1.md) connects data admission, sensor integrity, transport normalization, process estimation, mechanism evidence, independent quality/integrity, counterfactual decisions and human supervisory authority. It deliberately emits separate evidence-qualified state vectors rather than one "fouling probability" or an autonomous cleaning command.
