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
