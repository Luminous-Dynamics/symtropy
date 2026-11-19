# Industry Baselines and Matched Benchmark Protocol

**Programme:** GEOFUSION-INTEGRATION-001  
**Snapshot date:** 19 November 2026  
**Evidence status:** published tool descriptions, company-reported portfolio figures, program announcements, and research-model results; not an independent market census.

## 1. Current reference landscape

| Reference | Publicly described capability / current result | Evidence class | What Symtropy should compare |
|---|---|---|---|
| Fervo Energy Q3 2026 | Company reports one 33 MW GeoBlock commercially operating at Cape Station (36 MW total portfolio operating when its 3 MW pilot is included); GeoBlock 2 was grid-synchronized and exporting test energy but not yet commercial, GeoBlock 3 was mechanically complete / in commissioning; 466 MW under construction and 550 MW ready-to-build. A 15-year 396 MW take-or-pay Google PPA is expected to begin delivery in mid-2028. Phase I three-GeoBlock cost is reported at about $7,000/kW; Cape Google 400 target is $5,500/kW, with a longer-term $3,000/kW target. | Company-reported operation, project status, contractual commitments and targets | Benchmark actual net output, availability, wellfield decline, drilling/casing design, auxiliary loads and full installed cost separately. Do not treat 550 MW ready-to-build, 54.7 GW early pipeline or the 396 MW future PPA as operating capacity; cost targets are not achieved cost data |
| ARPA-E SUPERHOT | Eight selected projects for up to a combined $30M; target resources above 375 °C and 22 MPa | Program announcement / selection, not completed awards | Superhot well materials, well construction, instrumentation and resource-state cases; record selected-project status separately from demonstrated performance |
| Scott (2026), *Geothermics* | Calibrated deterministic reservoir–wellbore–power-cycle model: reservoir pressure and transmissivity materially affect deliverable power; near-isenthalpic expansion causes modeled 50–150 °C reservoir-to-wellhead cooling; >30 MWe/well requires very high transmissivity in the examined conditions | Published calibrated model | Reproduce selected published tables/curves from full source inputs, then test held-out parameter cases. Do not treat model output as field proof |
| NREL SAM / GETEM | SAM's geothermal-power model is based on GETEM and estimates annual/lifetime electricity output and LCOE for utility-scale power plants; it represents monthly resource/electric-output change over years, with hydrothermal / EGS and flash / binary cases. SAM explicitly does not model ground-source heat pumps or geothermal heating/cooling; DOE describes GETEM as electricity-only. | Established engineering/economic model | Use for matched electricity-generation, resource-decline, recapitalization and economic subproblems. Do not claim it validates direct-use heat, absorption cooling or building geothermal heat pumps. |
| GEOPHIRES-X | Hydrothermal / EGS production, wellbore and surface plant modelling; electricity, direct heat and cogeneration; may couple to external reservoir simulators | Open-source specialist model | Reservoir/well/plant lifecycle cases and direct-use heat cases |
| OpenGeoSys TH2M | Hierarchical coupled thermo-hydro-mechanical verification cases checked against closed-form solutions, physical experiments or other codes | Specialist multiphysics verification suite | Use its valid subproblem benchmarks for reservoir / porous-media model adapters; do not compare a 0D ledger to TH2M as solver peers |
| NETL STEP sCO₂ pilot | Approximately 4 MWe grid-synchronized simple-cycle pilot demonstration in 2024 | Physical pilot demonstration | sCO₂ component / cycle validation boundary and source-specific performance data |
| UKAEA PROCESS | Integrated fusion systems code covering machine physics and plant engineering / power conversion | Specialist fusion systems model | Fusion plant design outputs, gross-to-net power terms and heat-system boundaries; retain exact PROCESS version and input manifest |
| DOE Fusion S&T Roadmap (June 2026) | Calls for whole-plant digital twins spanning plasma, materials, tritium, power conversion and controls, calibrated and stress-tested against measured data with quantified uncertainty over steady operation, transients and faults | Government roadmap | Evidence traceability, uncertainty calibration, multi-fidelity validation, fault and maintenance coverage |
| DOE AI-Fusion Digital Convergence Platform (updated 13 November 2026) | DOE describes a planned national-scale platform integrating HPC codes, foundation models, physics/chemistry-informed neural networks, surrogate models, digital twins and facility data across six fusion challenge areas; it is a programme direction, not a deployed operating product or measured benchmark result | Announced capability / roadmap | Compare interoperability, source-data custody, model admission, uncertainty-calibrated cross-domain exchange and validation workflows. Do not claim a capability or performance lead over a platform that is still being developed |
| Council for Geoscience (South Africa) | Public programme states preliminary low-enthalpy potential and ongoing magnetotelluric work near known hot springs to map structures / potential hot-water reservoirs | Public national geoscience programme | A South African low-enthalpy evidence case; unknown values stay unknown. This is not evidence of a South African superhot resource |
| ASHRAE absorption-equipment chapter | Reports representative single-effect indirect-fired water/LiBr heat-source conditions (115–132 °C steam-heated, some waste-heat machines lower) and 160–188 °C hot-fluid input for typical double-effect equipment | Engineering handbook reference ranges | Temperature-grade and equipment-map checks for a cooling co-product; actual machine, cooling-water conditions and parasitic loads remain required |
| Zhou et al. (2024), geothermal polygeneration | Modelled ORC/flash + absorption refrigeration + Kalina + humidification-dehumidification desalination configuration; reports 342.3 kW electric, 301.7 kW cooling, 0.233 kg/s freshwater and 31.52% exergy efficiency | Published simulation output, not physical demonstration | Reproduce the complete source-pinned model as a co-product reference case; do not treat the output vector as measured plant performance |
| Wang et al. (2026), desalination review | Modern seawater RO approaches its practical thermodynamic minimum; future performance gains are increasingly system-level (loss distribution, high recovery, pretreatment/cleaning/post-treatment, brine handling) | Peer-reviewed review | Avoid a membrane-permeability-only benchmark; compare full water-treatment train and product/residual service |

### Source register

- Fervo Q3 2026 results (company publication): https://fervoenergy.com/fervo-energy-reports-third-quarter-2026-results/
- Fervo Q3 2026 SEC filing: https://www.sec.gov/Archives/edgar/data/1853868/000162828026071911/exhibit991earningsrelease1.htm
- ARPA-E SUPERHOT programme announcement, 12 November 2026: https://arpa-e.energy.gov/news-and-events/news-and-insights/does-arpa-e-announces-30-million-advance-superhot-geothermal-baseload-power
- Scott, S. W. (November 2026), *Thermo-hydraulic drivers of superhot geothermal well performance*, DOI: https://doi.org/10.1016/j.geothermics.2026.103784
- NREL SAM geothermal power: https://sam.nrel.gov/geothermal/56-geothermal-power.html
- DOE GETEM description and electricity-only scope: https://www.energy.gov/hgeo/geothermal/geothermal-electricity-technology-evaluation-model
- GEOPHIRES-X canonical repository and documentation: https://github.com/NatLabRockies/GEOPHIRES-X
- NREL geothermal data and tools / GEOPHIRES: https://www.nrel.gov/geothermal/data-tools.html
- OpenGeoSys TH2M benchmarks: https://www.opengeosys.org/stable/docs/benchmarks/th2m/
- NETL STEP pilot, 16 December 2024: https://netl.doe.gov/node/14437
- UKAEA PROCESS: https://www.ukaea.org/service/process/
- DOE Fusion Science and Technology Roadmap, 9 June 2026: https://www.energy.gov/articles/energy-department-releases-finalized-fusion-science-and-technology-roadmap-accelerate
- DOE AI-Fusion Digital Convergence Platform (updated 13 November 2026): https://www.energy.gov/undersecretaryforscience/genesis-mission/accelerating-delivery-fusion-energy
- Council for Geoscience geothermal programme: https://www.geoscience.org.za/project/geothermal-energy-potential-of-south-africa-project/
- IAPWS R7-97(2012), IF97 water/steam properties and range of validity: https://iapws.org/technical-guidance/release/IF97-Rev
- ASHRAE Handbook 2018, Chapter 18, Absorption Equipment: https://handbook.ashrae.org/handbooks/R18/SI/R18_Ch18/r18_ch18_si.aspx
- Zhou et al. (2024), geothermal polygeneration, DOI 10.1016/j.energy.2024.131667: https://doi.org/10.1016/j.energy.2024.131667
- Wang et al. (9 October 2026), desalination energy efficiency and sustainability, DOI 10.1038/s44359-026-00221-x: https://doi.org/10.1038/s44359-026-00221-x
- Fervo Q3 2026 SEC 10-Q / portfolio definitions (as of 30 September 2026): https://www.sec.gov/Archives/edgar/data/1853868/000162828026072482/frvo-20260930.htm
- Fervo Cape Station first GeoBlock commercial operation (1 October 2026): https://www.sec.gov/Archives/edgar/data/1853868/000162828026064103/exhibit991pressrelease10126.htm
- ASHRAE Handbook, Chapter 18, absorption equipment (single/double-effect operating conditions): https://handbook.ashrae.org/handbooks/R18/SI/R18_Ch18/r18_ch18_si.aspx

## Rust thermophysical-property provider reconnaissance

This is a candidate shortlist for an independent provider bake-off, **not a production dependency decision**. The machine-readable record is [rust-property-provider-candidates.json](../../../benchmarks/geothermal-fusion/v0.1/fixtures/rust-property-provider-candidates.json). The current known-answer corpus is the 19-point official [IAPWS IF97 fixture](../../../benchmarks/geothermal-fusion/v0.1/fixtures/iapws-if97-official-known-answer.json).

| Candidate | Source-pinned evidence | Why it is worth testing | Main qualification risks |
|---|---|---|---|
| `vle-steam` | MIT upstream repository; signed release tag `v0.20.1` points to commit `74cafc927e27026c5d41ff2411a56f765031f724`. Release workflow CI run `43733986531` has successful separate `cargo test` and `fmt + clippy` jobs. Upstream source has direct R7-97(2012) Table 5/15/33/35/36/42 tests plus internal thermodynamic identities, inverse state and solver comparisons. | Best first adapter candidate: IF97 Regions 1–5, saturation, backward equations and no mandatory dependencies are documented. | API takes pressure in kPa absolute (not Symtropy Pa); water/steam only; region-3 critical behavior; confirm packaged registry source matches the pinned tag; limited adoption / single maintainer increases maintenance risk. |
| `seuif97 2.3.8` | MIT crate, Rust 2021, docs.rs package and public source; exposes 12 state input pairs and 36 properties. | Independent IF97 implementation for differential testing; offers transport/process functions and performance-oriented API. | Verify units/region selection, optional native/FFI features, unsafe surface, and source-level test coverage; treat upstream speed claims as hypotheses until matched benchmarking. |
| `iapws95 0.2.5` | Rust implementation of the IAPWS-95 scientific formulation. | Independent formulation cross-check in the critical neighborhood and where IF97/95 are expected to agree within published formulation tolerances. | Not a drop-in IF97 replacement; verify units, validity envelope, license and numerical domain before use. |

The `vle-steam` v0.20.1 release workflow's successful CI is useful upstream evidence, but it does not qualify our adapter. The provider's own verification tests are not a substitute for our independently pinned IAPWS fixture set, and provider-to-provider agreement is not a substitute for agreement to the IAPWS reference.

**Important unit boundary:** `vle-steam` documents temperature in K and pressure in kPa absolute, returning mass-specific SI/engineering properties (for example density kg/m³, enthalpy kJ/kg). The Symtropy wellhead contract uses pressure in Pa. The adapter must accept SI at its public boundary, perform checked Pa↔kPa conversion internally, and test conversion at low pressure, high pressure, and near-critical points. Never pass Pa directly to a kPa API.

**Recommended sequence:** pin the signed `v0.20.1` source tag for the first conformance study; run the 19 official known-answer points and edge/hostile cases; run the same cases through `seuif97`; then use IAPWS-95 as a distinct-formulation cross-check near criticality. Keep all providers out of production dependencies until licensing, source, units, domains, tests, error semantics and exact package provenance are reviewed. IF97 is for ordinary water/steam; saline-brine properties require a separate, composition-aware model.

Sources:
- `vle-steam` upstream repo: https://github.com/miguelju/vle
- Signed release tag: https://github.com/miguelju/vle/releases/tag/v0.20.1
- Upstream release CI: https://github.com/miguelju/vle/actions/runs/43733986531
- `vle-steam` crate docs: https://docs.rs/vle-steam/
- `seuif97 2.3.8`: https://docs.rs/seuif97/2.3.8/seuif97/
- `iapws95 0.2.5`: https://docs.rs/iapws95/0.2.5/iapws95/
- Official IAPWS-IF97 release: https://iapws.org/technical-guidance/release/IF97-Rev

## 2. What is actually novel in our target

We should not claim to beat NREL SAM / GETEM, GEOPHIRES-X or OpenGeoSys on their native geothermal equations; nor claim to beat PROCESS on plasma and fusion systems calculations. Those are domain specialists with substantial validation histories.

Our differentiation hypothesis is a **decision-grade integration layer** that can:

1. Preserve the identity, units, boundary, timebase, source evidence and validity envelope of every imported specialist result.
2. Compose geothermal electricity, thermal streams by temperature grade, fusion facility electricity demand, future fusion heat streams, storage and industrial services without double counting.
3. Reconcile full-boundary energy balances and expose rather than hide unknown parasitic loads.
4. Carry common-cause outage and degradation states across domains.
5. Compare architectures using identical service obligations and chronology.
6. Escalate expensive specialist-model calls only where the result could change the design decision.
7. Replay the decision with the exact input, model, configuration, uncertainty and evidence artifacts.

This is an integration hypothesis, not a proven market advantage. It requires matched public benchmarks and external review.

## 3. Benchmark tiers

### Tier A — arithmetic and contract conformance

Run hand-calculable fixtures before any external model integration.

- Exact power × duration = energy identities.
- Electricity and heat remain different carriers.
- Temperature grade and exchanger approach are hard constraints.
- Storage charge/discharge losses are explicit.
- Missing parasitics, invalid pressure/temperature and duplicate energy paths are rejected.
- Primary geothermal and fusion circuits cannot be declared directly cross-connected through a nominal energy edge.
- Direct geothermal heat cannot be treated as D–T plasma heating; geothermal-generated electricity may feed a declared electrical heating supply.

### Tier B — matched specialist-model reproduction

For each external model, freeze its exact version, model class, input deck, solver settings, property database, units, and output artifacts.

- SAM / GETEM: reproduce supported lifetime plant output and LCOE reference cases.
- GEOPHIRES-X: reproduce reservoir, wellbore, surface-plant, direct-use heat or cogeneration cases.
- OpenGeoSys: reproduce selected TH2M verification cases only where the relevant coupled process and boundary conditions match.
- PROCESS: reproduce selected fusion system-code outputs and power balance cases, keeping plasma Q, gross thermal power, gross electric output, fixed/variable recirculating demand and net export distinct.
- STEP: use source-documented pilot performance for an sCO₂ component/cycle boundary; do not infer a geothermal plant's efficiency from the pilot rating alone.
- Absorption cooling / geothermal polygeneration: reproduce the source-pinned literature case with equipment-specific temperature grades and all cooling-water, pumping, electric parasitics, product water and brine outputs. The reported Zhou et al. (2024) vector is a model reproduction target, not a measured benchmark.
- Desalination train: compare RO / MED / HDH and hybrid high-recovery configurations at identical product quality and feed/residual boundaries; use the 2026 review to focus on whole-train losses and brine management rather than membrane permeability alone.

A Symtropy integration PASS means we preserve and reconcile the specialist result at its declared boundary. It does not mean Symtropy has independently validated the specialist solver's underlying physics.

### Tier C — integrated architecture comparison

Compare at least:
1. Grid-only facility baseline.
2. Geothermal-only energy / heat campus.
3. Geothermal + grid support.
4. Geothermal + electrical buffer.
5. Geothermal + electrical buffer + thermal store + heat cascading.
6. Future fusion heat/electric source + geothermal + buffers + industrial sinks, as a published-design / synthetic case only until physical plant data exist.
7. No-build / grid reinforcement / renewable-plus-storage / conventional supply alternatives where they provide a fair service match.

8. Multi-service / tri-generation variants: RO supplied by geothermal-generated electricity, thermal desalination or absorption cooling supplied only by a source with adequate temperature grade, and hybrid water/thermal architectures. Compare identical product-water quality, water recovery, cooling duty, total source energy, pump/pretreatment/cleaning loads and brine/residual streams. Building/process absorption cooling does not satisfy fusion cryogenic refrigeration demand.

Every architecture must serve the same:
- chronological electric load profile and power-quality boundary;
- thermal demand by temperature grade and interval;
- startup / ramp / reserve service;
- outage and recovery obligations;
- water, land, emissions and environmental constraints;
- lifecycle horizon and financial assumptions.

### Tier D — decision and resilience benchmark

Compare Symthaea candidate search with:
- rule-based dispatch;
- a transparent mathematical optimizer (LP/MILP or exact enumeration when the model is linear/small enough);
- multi-fidelity baseline (cheap screen plus specialist solver refinement);
- external commercial or open optimization platforms where the same problem can be expressed fairly.

Metrics:
- false-feasible architecture rate;
- constraint violation count and severity;
- regret against exact optimum where available;
- Pareto-front hypervolume / coverage and diversity, with fixed objective definitions;
- quality versus solver-call budget, wall time and compute energy;
- forecast calibration / interval coverage;
- robustness under held-out weather, load, resource-decline and outage scenarios;
- failed / abstained candidate disposition and reason;
- provenance completeness and deterministic replay.

Never compare algorithms with different information, physical model fidelity, constraints, or evaluation budgets and call the result a fair SOTA comparison.

## 4. The superhot geothermal benchmark should be pressure- and state-aware

The November 2026 Scott study shows why temperature-only ranking is inadequate. Its steady-state calculations span reservoir pressure, temperature, transmissivity and wellhead pressure, and show that reservoir density / volumetric enthalpy and wellhead state determine deliverable flow and power. The study reports modeled 50–150 °C reservoir-to-wellhead cooling and notes that >30 MWe/well requires transmissivity not yet deliberately engineered in deep superhot rock.

The materials review by Meng et al. (Stanford Geothermal Workshop 2026) adds that corrosion need not increase monotonically with temperature; subcritical conditions around 350 °C can be especially challenging for some low-alloy steels, depending on chemistry and phase state.

The paper's reported calibration anchor is specific and should be captured faithfully: IDDP-1 data were used to calibrate an effective reservoir pressure of about 16.1 MPa and permeability-thickness of about 5,300 mD·m, with reservoir temperature estimated near 500 °C; observed warmed wellhead temperatures were 380–440 °C and the model range was approximately 390–450 °C. This is an **in-sample calibration anchor**, not an independent predictive validation case.

Its parameter sweeps cover reservoir pressures of 15–45 MPa, reservoir temperatures of 375–600 °C, permeability-thickness of 100–4,000 mD·m and (in key figures) a 10 MPa wellhead pressure. Under these stated assumptions, the paper reports >30 MWe only above approximately 4,000 mD·m at 20 MPa, 1,000 mD·m at 30 MPa and 500 mD·m at 40 MPa. Those are idealized steady-state model outputs and must not be copied into a commercial resource estimate.

The same study uses a 50 m/s erosional-velocity reference for the modelled 9⅝-inch casing (inner diameter about 0.217 m) and compares a 13⅜-inch casing (inner diameter about 0.315 m). Treat 50 m/s as the paper's screening reference under its assumptions—not a universal erosional velocity, design allowable or safety limit. The study notes that IDDP-1 production ended after master-valve failure, illustrating why mechanical completion, maintainability and lifetime reliability sit outside a purely thermodynamic performance score.

For water/steam properties, pin the exact IAPWS formulation release and validity envelope. IAPWS-IF97 is an industrial water/steam formulation valid from 273.15 K to 1073.15 K at pressures up to 100 MPa (with a higher-temperature region to 2273.15 K up to 50 MPa). It is not a general saline-brine equation of state. If the fluid contains dissolved salts or non-condensable gases, use an appropriate mixture model with its own validated composition and domain; never silently apply pure-water IF97 to brine.

The benchmark must record, as separate observations:
- reservoir pressure, temperature, chemistry and transmissivity;
- bottom-hole pressure / enthalpy;
- wellbore pressure, temperature, phase and enthalpy profile;
- wellhead pressure, temperature, mass flow, phase and fluid composition;
- casing diameter / velocity and erosion constraints;
- scaling / corrosion / cement / casing / connection evidence;
- production and injection trajectories over time;
- net power after pumping, cooling, reinjection and surface parasitics.

Scott's November 2026 model identifies wellhead pressure as an important control: >15 MPa can preserve single-phase production in some modeled conditions and reduce condensation/scaling exposure. Its 9 5/8-inch casing cases cross a 50 m/s erosional-velocity reference above roughly 30 MPa reservoir pressure at 450–550 °C. Those values are tied to the paper's steady-state assumptions and case geometry; they are not general safe limits. The validation plan must therefore include pressure/phase/casing tradeoffs and transient start-up, shut-in, circulation loss, restart and injection cases. Do not promote steady-state model fit into well-lifetime or well-integrity validation.

For pure water/steam property calculations, pin the IAPWS-IF97 release and its validity envelope. Brines with dissolved salts require an appropriate mixture-property model; pure-water IF97 is not a brine equation of state.

Material sources:
- Meng et al. (2026) review: https://pangea.stanford.edu/ERE/db/IGAstandard/record_detail.php?id=38347
- IAPWS IF97 industrial formulation: https://iapws.org/technical-guidance/release/IF97-Rev

## 5. Metrics and acceptance discipline

| Category | Primary metrics | Minimum evidence |
|---|---|---|
| Conservation | First-law residual by component and whole system | Independent hand oracle; declared boundary and units |
| Temperature grade | Useful heat delivered at required sink condition; approach margin | Source/sink temperature and exchanger map |
| Exergy | Exergy input/output/destruction with reference environment | Qualified property model and reference state |
| Net power | Gross generation, fixed/variable auxiliaries, parasitics, net PCC import/export | Exact meter plane / solver outputs, full covariance where needed |
| Storage | State trajectory, round-trip efficiency, power limits, standing losses, degradation | Time-resolved charge/discharge evidence |
| Reliability | LOLE/EUE/CVaR, critical-load service, restart and recovery time | Chronological outage/resource scenarios |
| Geothermal sustainability | Temperature/pressure/flow decline, injectivity/productivity, chemistry and water balance | Site-specific measured or calibrated data |
| Economics | LCOE, levelized cost of heat, CAPEX/OPEX/replacements, cost per accepted service | Matched financial assumptions and lifecycle |
| Optimization | Regret, false-feasible rate, Pareto quality, call budget and compute energy | Held-out scenarios and transparent baseline |
| Evidence | Source class, calibration, uncertainty, model validity, replay and provenance | Independent audit of exact artifacts |

Targets remain staged:
- <=0.1% normalized first-law residual on analytic fixtures.
- <=0.5% exergy residual only when reference environment and property models are fixed and qualified; otherwise EXERGY_NOT_EVALUATED.
- 100% of graph streams retain units, boundary, timebase, evidence class and identities.
- Zero hidden parasitic loads, silent missing-as-zero assumptions or heat double-counting in negative-control fixtures.
- Zero false direct geothermal-heat → D–T plasma paths.
- Matched external model reproduction before any accuracy superiority claim.
- At least 10% improvement in a preregistered useful-service/cost objective is a stretch hypothesis, not a product promise; paired uncertainty must exclude no improvement and service/reliability/environment constraints must not regress.

## 6. South African deployment posture

The Council for Geoscience currently describes preliminary low-enthalpy potential and ongoing magnetotelluric investigations around known hot springs. The first South African case should therefore be an evidence-bounded low-enthalpy / direct-use / heat-pump / thermal-storage comparison with unknown superhot-resource fields left unknown.

A future superhot case must wait for measured site evidence, not be inferred from the U.S./Icelandic research literature.

## 7. What would constitute a genuine SOTA claim?

A claim such as "10% less purchased electricity" requires:
- preregistered objective and service floors;
- identical chronological input traces and uncertainty assumptions;
- best fair non-hybrid baseline;
- exact model/code versions and input hashes;
- complete system boundary including parasitic loads, storage losses and heat quality;
- blind or held-out scenarios;
- paired uncertainty intervals and sensitivity analysis;
- independent replay and assessor review;
- no safety, water, environmental or reliability regression.

A claim such as "better superhot well prediction" additionally requires blind field observations, not only fit to IDDP-1 calibration data. A claim such as "fusion-ready" requires source-qualified fusion load/heat streams and a clear design-data versus measured-operation distinction.

## Claim ceiling

This document is a public comparison protocol, not an industry certification, full market census, site assessment, validated model result, or evidence that the Luminous stack currently outperforms any named commercial or open-source tool.
