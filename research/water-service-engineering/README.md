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


## WaterLoop physical bench T0 extension

The companion `waterloop-physical-t0` corpus adds a synthetic-first, 25-case known-answer contract for the initial WaterLoop bench. It is deliberately separate from the existing 23-case pump/treatment/leak T0 evaluator.

Inputs, manifest, independent oracle and corpus schema:
- `waterloop-physical-t0-v0.1.json`
- `waterloop-physical-manifest-v0.1.json`
- `waterloop-physical-oracle-v0.1.json`
- `waterloop-physical-t0.schema.json`

The new cases address water-volume closure with conservative uncertainty bounds, measured usable storage, use-qualified source substitution, hydraulic power, wire-to-water energy, service-floor admission and event-level leak metrics. The workflow runs both evaluators on the same exact checked-out revision and publishes separate receipts.

This is software contract/arithmetic evidence only. It does not qualify a pump, tank, meter, treatment train, source, laboratory result, water quality, service outcome or field savings. Physical T1 work remains gated by the WaterLoop design review and instrument/calibration qualification.


### Continuity / outage extension

The input/manifest/separate-oracle corpus now includes 10 time-resolved continuity scenarios (25 total). The continuity evaluator allocates essential demand before discretionary demand, protects a declared reserve from discretionary draw, records source inflow that is not qualified for the intended use, and abstains when critical service metrology or stored-water use-quality evidence is not admitted.

The South African GN 6292 Regulation 4 context case represents four people over 72 hours at the regulation's 15 L/person/day alternative drinking-water reference for interruptions longer than 48 hours. This is a context-specific regulatory scenario, not a claim that a private household installation discharges a Water Services Authority's duties or that WaterLoop is potable-water qualified.


The full time-resolved service semantics, scenario families, South African regulatory context, benchmark targets and T1 physical gates are documented in [`SERVICE_CONTINUITY_BENCHMARK_V0_1.md`](SERVICE_CONTINUITY_BENCHMARK_V0_1.md).


## Household economics and service affordability

[WATER_ECONOMICS_BENCHMARK_V0_1.md](WATER_ECONOMICS_BENCHMARK_V0_1.md) defines the water-specific whole-life and monthly cost boundary, matched industry comparators, tariff/price evidence, fixed-charge handling and resident-authorised burden semantics. It composes with Sol-Atlas #816 and #700 rather than duplicating their whole-household cost or post-occupancy owners.

The initial WaterLoop implementation adds nine synthetic known-answer cases (026–034) to the shared T0 evaluator. It does not calculate tariffs or infer income. The evaluator reports cost arithmetic separately from actual service equivalence and keeps its qualification status NotRun.

The T0 household-economics suite now has nine synthetic cases (026–034). It explicitly separates cost/service comparison from resident affordability: missing/unauthorised income abstains only the water-spend burden, while cost-per-useful-volume remains reportable. The tariff bill's billing-period days must match the system-cost period or bill-delta comparisons abstain.


## Modular household non-potable WaterLoop architecture

The source-aware, serviceable reference design basis is now documented in [WATERLOOP_MODULAR_ARCHITECTURE_V0_1.md](WATERLOOP_MODULAR_ARCHITECTURE_V0_1.md), tracked by [#5822](https://github.com/Luminous-Dynamics/symtropy/issues/5822). The matching machine-readable interface/control contract is [waterloop-modular-architecture-v0.1.json](waterloop-modular-architecture-v0.1.json).

The v0.1 baseline is bathroom-only greywater, with laundry and roof rainwater as separately gated optional modules. Kitchen/dishwasher, toilet/blackwater and pool/spa sources are excluded. The design defines modular treatment selection, mass/energy/storage sizing equations, safe control states, fail-closed release, maintainable BOM classes, T1 bench and endurance tests, and South African approval gates.

No single treatment train is selected: 2026 evidence shows feed variability and treatment trade-offs. The design remains pre-prototype and does not claim treatment efficacy, water quality, regulatory conformity or field readiness.
