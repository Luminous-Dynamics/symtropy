# Glass and Silicon Process Benchmark Suite v0.1

**Status:** Research contract and synthetic software-control corpus; not a qualified process model.  
**Issue:** [GLASS-SILICON-001](https://github.com/Luminous-Dynamics/symtropy/issues/2715)  
**Stack boundary:** Symthaea scientific reasoning; specialist process/materials solvers; Symtropy process/workcell simulation; Mycelix manufacturing evidence and provenance.  
**Safety boundary:** simulation and evidence modeling only. This document does not authorize furnace control or any physical actuator.

## 1. Decision

Build a benchmark for **verified functional output per unit of resource**, not a leaderboard for predicted properties or nominal production. A candidate is useful only when product grade and quality constraints are met, the process boundary is comparable, and inputs, outputs, energy, emissions, uncertainties, and evidence classes are explicit.

v0.1 freezes terms, comparison rules, claim ceilings, and a negative-control corpus. It does not claim that an optimizer, digital twin, or physical production loop is already implemented.

## 2. Prior art and the gap we should target

The work must compare fairly with current practice rather than claim novelty for combining a database and machine learning:

- A 2024 glass-industry energy study estimates roughly **10% sectoral improvement potential** against best-practice container-furnace performance, using energy measurement and modeling: [Energy Conversion and Management: X, DOI 10.1016/j.ecmx.2024.100720](https://doi.org/10.1016/j.ecmx.2024.100720). This is a literature-level opportunity estimate, not a guaranteed saving at a specific plant.
- A 2025 paper describes an ontology-based infrastructure coupled to robotic melting, glass-property ML, and patent information extraction: [Advanced Engineering Materials, DOI 10.1002/adem.202401560](https://doi.org/10.1002/adem.202401560). An ontology, predictor, or robotic experiment system alone is therefore not our differentiation.
- A 2025 techno-economic study of flat-glass decarbonization compares hybrid melting, full electrification, hydrogen, and CCS. Its model reports distinct emissions and cost trade-offs, including a 5–22% energy-demand increase for its CCS cases: [Computers & Chemical Engineering, DOI 10.1016/j.compchemeng.2025.109329](https://doi.org/10.1016/j.compchemeng.2025.109329). These are scenario-specific model results, not universal plant factors.
- A June 2026 study compares container-glass decarbonization with amine scrubbing, calcium looping, and electrification. It reports more than 76% direct-emissions reductions in its modeled cases but different energy and cost consequences: [Journal of Cleaner Production, DOI 10.1016/j.jclepro.2026.148702](https://doi.org/10.1016/j.jclepro.2026.148702). Treat the numbers as results for the declared 250 t/day case, not a sector-wide guarantee.
- An April 2026 MG-Si techno-economic study compares conventional submerged-arc furnaces with molten-salt electrolysis. It assumes 12 versus 9 kWh/kg in its baseline comparison and explicitly notes that MG-Si electrolysis is not deployed at that scale; its finding of potential cost competitiveness is conditional on scale-up and financing assumptions: [Energies, DOI 10.3390/en19092023](https://doi.org/10.3390/en19092023). The 9 kWh/kg figure is a model assumption, not validated commercial production performance.
- NIST's July 2026 manufacturing digital-twin workshop report identifies interoperability, verification/validation/uncertainty quantification (VVUQ), cybersecurity, and workforce readiness as continuing challenges: [NISTIR 8620](https://doi.org/10.6028/NIST.IR.8620). These are explicit benchmark dimensions, not optional documentation extras.
- A 2024 open-access review reports conventional metallurgical-grade silicon (MG-Si) production at approximately **11–13 kWh/kg Si** and **4.7–5 t direct CO₂/t Si**. It identifies biocarbon/CCU pathways with differing maturity and says several metallothermic/electrochemical alternatives remained laboratory-scale in that review: [Journal of Sustainable Metallurgy, DOI 10.1007/s40831-024-00913-3](https://doi.org/10.1007/s40831-024-00913-3).
- Electrochemical silicon production has an active research literature, but laboratory demonstration is not evidence of commercially competitive throughput, purity, lifetime, or cost: [review indexed at PubMed](https://pubmed.ncbi.nlm.nih.gov/37214200/).
- An August 2026 review surveys 17 silicon production/purification routes and emphasizes grade-specific purity/energy/carbon trade-offs: [Mineral Processing and Extractive Metallurgy Review, DOI 10.1080/08827508.2026.2716681](https://doi.org/10.1080/08827508.2026.2716681). It reports much higher energy ranges for some electronic-grade routes than MG-Si reduction and lower laboratory-scale values for some electrochemical approaches. Treat these as review-reported, pathway-specific ranges—not comparable plant results without matched grade, yield, lifecycle boundary, and scale-up status.
- The SCALE Zenodo record is **version v1**, published January 30, 2026 and labeled “Dataset Open”. The file table lists V14/V18 datasets and models, iterative-learning code, helper packages, and environment files, with **MD5 checksums**; for example, `models_data_V14.zip` has published MD5 `deb614846654bda761a66614dac6b4e0` and `models_data_V18.zip` has published MD5 `b7f7ffa6253050420de1b58b58b271b1`. The record's Rights/License field is blank and no SHA-256 digests are displayed. Therefore the source is **not admitted as benchmark data yet**: verify file-level permissions/terms, obtain the bytes, calculate SHA-256, and pin the exact artifact manifest before reproduction or redistribution. [Zenodo record 10.5281/zenodo.18430657](https://doi.org/10.5281/zenodo.18430657).

- A July 2026 glass-discovery study introduces SCALE, integrating molecular dynamics, ML, and robotic synthesis. It reports more than 42,000 simulations, validation against 55 synthesized sodium alumino-borosilicate glasses, density/model bias up to 5%, and composition-dependent correction over three experimental iterations of six measurements each: [npj Computational Materials, DOI 10.1038/s41524-026-02249-w](https://doi.org/10.1038/s41524-026-02249-w). This is a particularly useful matched-budget baseline; its results are chemistry- and property-specific, not a universal glass-model score.
- Glass Futures announced an AI/physics-informed twin for its multi-fuel pilot furnace in June 2026, reporting 16 sensor points and 11,530 calculations processed at once: [Glass Futures AI-GLASS](https://www.glass-futures.org/news/glass-futures-launches-ai-driven-digital-twin-to-reinvent-glass-manufacturing/). The public announcement does not define a standardized error dataset or an externally reproducible accuracy benchmark. Treat throughput as context, not proof of predictive superiority; compare on held-out campaign predictions, calibration, energy and accepted-product quality.
- A September 2026 PNNL / Journal of Non-Crystalline Solids study experimentally validated ML-guided low-activity nuclear-waste glass formulations and reports that adding experimental results back to the database improved subsequent formulations; more than half of its first-round ML formulations failed at least one property constraint: [DOI 10.1016/j.jnoncrysol.2026.123990](https://doi.org/10.1016/j.jnoncrysol.2026.123990). This is a high-value **multi-property feasibility** baseline. Nuclear waste glass is not container glass, and the study's data statement says data are available on request; do not claim open-data reproduction without obtaining and admitting the exact data. The separate implementation/replication task is [Symthaea MAT-GLASS-MULTI-001 #8481](https://github.com/Luminous-Dynamics/symthaea/issues/8481), which uses existing MAT-UQ/MAT-ACTIVE/MAT-METRO/MAT-REPLAY owners and blocks unsupported open-data claims.
- The Western Australian Minerals Research Institute describes a 2026–2028 project targeting sodium-silicate electrolysis and a 100 kg/day pilot for extracting high-grade silicon from iron-ore waste: [MRIWA Phase 1 project M10625](https://www.mriwa.wa.gov.au/research-projects/project-portfolio/advancing-low-carbon-high-grade-silicon-production-from-iron-ore-waste-phase-1/). Its page describes intended work and target benefits, not a verified completed pilot. This makes a useful prospective scale-up benchmark: distinguish funding/project descriptions, installed capacity, measured sustained throughput, product grade, yield, and independent economic qualification.


**Targeted gap:** join source-grounded process models, uncertainty-aware experiment selection, a deterministic process/workcell twin, evidence-bounded manufacturing routes, and lifecycle comparison under one falsifiable decision protocol. Every subsystem remains accountable to its own physical authority.

## 3. v0.1 scope and functional units

### A. Soda-lime container glass

Functional unit: **1 metric tonne of saleable, specification-conforming container glass at the declared factory gate**.

The reference boundary must declare whether it includes incoming cullet and virgin batch preparation; carbonate decomposition and batch reactions; melting, refining, and conditioning; forming, annealing, inspection, rejected product, and cullet return; electricity, combustion fuels, recovered heat, and export/useful heat; and upstream feedstock, transport, and waste treatment for lifecycle results.

Do not mix container glass, flat glass, optical glass, and fused silica into one baseline. Do not treat all recycled glass as interchangeable: color, chemistry, contamination, forming, and customer specifications can constrain admissible cullet.

### B. Metallurgical-grade silicon (MG-Si)

Functional unit: **1 kg of saleable MG-Si at an explicitly declared composition/purity grade and factory gate**.

The conventional submerged-arc furnace/carbothermic route is the reference route. Compare biocarbon, capture/recirculation, metallothermic, molten-salt/electrochemical, or other alternatives only when product grade, allocation, energy boundary, and lifecycle boundary are compatible. Route maturity, plant scale, feedstock availability, durability, and downstream refining must remain explicit.

**Out of scope for v0.1:** electronic-grade silicon, crystal pulling, wafer slicing/polishing, solar-cell fabrication, and working semiconductor/CPU manufacture. Those are separate transformation stages with separate purity, contamination, and yield requirements. MG-Si results must never be represented as evidence for electronic-grade silicon.

## 4. Evidence contract

Every datum must carry, at minimum:

- stable evidence identifier and exact source/artifact digest where available;
- evidence class: measured, published, simulated, synthetic-test, or unknown;
- value, unit, uncertainty/range, method, date/epoch, location/process/lot, and validity domain where applicable;
- process model, configuration, and code/version identity for derived data;
- declared scope, exclusions, known bias, and permission/license for source data.

Rules:
1. Unknown is not zero, an empty list, a pass, or a free resource.
2. Synthetic fixtures qualify software-contract behavior only.
3. Simulated predictions never count as measured product output.
4. Published literature ranges may seed context but cannot silently become a specific plant baseline.
5. A model outside its validated domain must abstain or require review.
6. A source without enough metadata for the intended comparison remains unusable for that claim.

## 5. Conservation and boundary rules

### Mass

Represent a declared process boundary with complete input/output streams and inventory changes. Check total mass and, wherever chemistry is modeled, element-wise balance (at least Si, O, C, Ca, Na and other elements relevant to that route). Include off-gas, dust, slag, rejects, recycle streams, and retained inventory as appropriate.

For each element e, report:

residual_e = input_mass_e - output_mass_e - inventory_change_e

A balance passes only against a documented tolerance derived from measurement uncertainty / numerical method. Do not pick a generous tolerance merely to make a candidate pass. Incomplete stream census is UNRESOLVED, not a pass.

### Energy

Keep electrical energy, fuel energy, chemical feedstock energy, recovered heat, useful exported heat, stored energy, and losses distinguishable. Report both raw carriers and normalized final-energy metrics. Primary-energy conversion assumptions must be stated before comparing unlike energy mixes.

A process model's energy balance is not automatically a lifecycle-energy assessment. Do not double-count recovered heat or count the same energy stream as both input and useful output.

### Carbon and co-products

Report direct process emissions separately from upstream and lifecycle emissions; identify electricity mix, reductant origin, electrodes, transport, waste treatment, and carbon-capture assumptions. Biogenic carbon is not automatically zero-carbon: include production, land/feedstock, processing, transport, and the chosen accounting method.

No coproduct (e.g. silica fume, slag, or heat) receives avoided-impact credit until its quantity, grade, usable destination, substitution basis, and allocation rule are declared. Publish results both with and without uncertain credits where relevant.

### Product acceptance

Metrics use **accepted product mass**, not furnace throughput or gross output. Quality/purity, defect rates, rework, rejects, yield, durability, and downstream qualification remain explicit constraints. A lower-energy candidate that misses its declared product grade is infeasible, not a Pareto winner.

## 6. Benchmark layers

| Layer | Question | Required comparison / evidence |
|---|---|---|
| B0 — contract integrity | Does invalid evidence get blocked? | Schema, unit, provenance, conservation, boundary, grade, unknown-value and negative-control tests |
| B1 — literature/plant reproduction | Can we reproduce a reference? | Frozen source/configuration; matching product and boundary; reported uncertainty and residuals |
| B2 — predictive validity | Does the model generalize? | Hold out time, plant/site, feedstock lot, or campaign; report MAE/RMSE as appropriate, calibration, interval coverage, and abstention |
| B3 — decision quality | Does search find better feasible processes? | Equal compute/experiment budgets against physics/engineering baseline, grid/DOE, and matched-budget Bayesian or active-learning baselines |
| B4 — physical validation | Does a predicted outcome occur on real equipment? | Independently measured, safe and supervised trials with calibrated instruments, product metrology, preregistered acceptance, and exact artifact identity |

Random row splitting is insufficient when adjacent rows share a furnace campaign, feedstock lot, recipe, or calibration. All split rules must be frozen before tuning.

## 7. Metrics and comparison protocol

Publish a Pareto frontier; do not hide trade-offs behind a single arbitrary green or intelligence score.

Required outputs per accepted product unit, where evidence exists: energy by carrier and declared final-energy boundary; direct process GHG and lifecycle GHG as separate metrics; purchased/raw material, cullet/recycle fraction and quality, water, and consumables; saleable yield, defect/reject rate, rework, grade/chemistry, and uncertainty; variable cost, capex assumptions, throughput, uptime, maintenance/refractory life, and supply risks; balance residuals, source completeness, out-of-domain status, and evidence coverage.

Compare each candidate to: (1) a matched measured plant baseline or source-published baseline; (2) a conventional engineering/physics model; (3) a simple grid-search or design-of-experiments baseline; and (4) a matched-budget Bayesian/active-learning policy where applicable. Report compute, experiment budget, dataset split, stopping rule, and all excluded runs.

### SOTA replication and calibration test

Use SCALE's published protocol as a concrete experimental-efficiency baseline, after verifying dataset licensing, composition overlap, and exact reported metrics. Do not combine chemistry families into a misleading aggregate score.

1. Freeze train/calibration/test splits by composition families or synthesis campaign; exclude near-duplicates and source leakage.
2. Establish a simulation-only baseline and report per-property MAE/RMSE, R², signed bias, uncertainty calibration, interval coverage, and out-of-domain abstention.
3. Compare static simulated-data augmentation, random measurement selection, conventional design-of-experiments, and active learning under the **same physical-measurement budget**. Reproduce the published 3 × 6 measurement structure where the available data supports a fair replication. The paper's own selector comparison is an important anti-assumption: for one reported evaluation metric, random selection reached 0.80, gradient-based selection 0.70, and maximum-deviation selection 0.64. Those values are specific to that paper's protocol, not universal selector rankings; they make random selection an obligatory baseline, not a straw man.
4. Measure the held-out improvement after calibration, not only training or cross-validation fit. Predefine target uncertainty and bias tolerances from measurement repeatability and intended engineering use; do not infer them from the best observed score.
5. A model with high R² but systematic external bias is not qualified for the affected property/domain. Calibration may improve it, but calibration must itself be evaluated on unseen data.

For the industrial process twin, use the same rule: calculation throughput is not predictive accuracy. Request or collect a rights-cleared, time-stamped held-out campaign with sensor calibration, process regime, product grade, operator interventions, and measured outcomes. If such data are unavailable, report the benchmark as blocked rather than fabricate a substitute from the public announcement.

### Robust scenario and resilience evaluation

Freeze a scenario grid before optimization. At minimum vary electricity price and carbon intensity, fuel/reductant price, carbon price, plant utilization/capacity factor, feedstock/cullet quality and availability, and cost-of-capital assumptions. For CCS/electrification/hydrogen cases, include the extra electricity/heat demand and infrastructure boundary. For silicon electrolysis, explicitly sweep current efficiency, cell current/current density, service life, scale-up factor, and financing assumptions. Report feasibility probability, worst-case constraint violation, lifecycle cost/emissions, and regret across scenarios as well as median results. Do not select a pathway from one favorable price forecast.

### Provisional targets (hypotheses only)

- **Integrity:** reject 100% of frozen negative controls, while accepting valid controls. Reject-all is not a passing policy.
- **Reproduction:** fall within source-reported uncertainty/tolerance on the frozen reference cases; if no valid tolerance exists, report the residual without inventing a pass threshold.
- **Optimization:** first seek **at least 5% reduction in energy per accepted tonne** against a matched baseline with no grade, yield, defect, lifetime, or safety regression. A **10% reduction** is a stretch target, motivated by published glass-sector improvement potential, not a promise or an already observed result.
- **Silicon decarbonization:** seek reductions in both energy and CO₂e per accepted kg while keeping grade, yield, electrode/reductant accounting, electricity mix, and lifecycle boundary comparable. A route with lower direct emissions but higher upstream impact must expose the trade-off.
- **Statistics:** publish confidence/credible intervals and repeated-seed variability. Claim improvement only when the predefined decision rule is met; otherwise call the result inconclusive.

The percentage targets apply only after a credible baseline and accepted-product denominator exist. They cannot be assessed with synthetic fixtures or by comparing different product grades or boundaries.

## 8. Frozen negative controls

The corpus and future executable verifier must reject or abstain on:

1. energy or mass expressed in an incompatible unit without a valid conversion;
2. missing stream, impossible balance, non-finite, or negative physical quantity;
3. unknown energy/cost/emissions silently set to zero;
4. coproduct credit without verified amount/grade/use/assessment rule;
5. unlike glass products or unlike silicon grades compared as equivalent;
6. mismatched plant-gate/lifecycle boundaries or inconsistent allocation;
7. model output or synthetic fixture relabeled as measured;
8. training/test leakage from the same lot, campaign, or duplicated source;
9. stale/unlicensed/untraceable evidence used outside its allowed purpose;
10. out-of-domain prediction accepted as feasible without review;
11. quality or yield regression hidden by weighted aggregate score;
12. electronic-grade/wafer capability inferred from MG-Si production.
13. a candidate approved from a favorable single-property score while any required processing, durability, crystallization, chemistry, or grade constraint is failed/unmeasured;
14. a planned or announced silicon pilot target treated as measured throughput, achieved purity, verified energy intensity, or independent qualification;
15. a tolerance that has no stated per-element basis or is silently accepted from missing metadata.

The suite must include valid glass and MG-Si contract controls, otherwise reject-everything could pass.

## 9. Responsibility boundaries

- **Symthaea:** scientific candidate generation, model selection, uncertainty, evidence-aware inference, and experiment selection. It does not invent missing physical data.
- **Specialist solvers:** authoritative task-specific thermodynamics, reaction chemistry, glass-property models, or atomistic calculations within their validated domain.
- **Symtropy:** deterministic process/workcell scenarios, timing, equipment/resource coupling, replay, constraints, and simulation-to-measurement comparison. Its rigid-body engine is not a furnace CFD, thermochemical, or atomistic solver.
- **Mycelix:** identity, immutable manufacturing design/routing references, provenance and attestations, cross-party evidence exchange, and auditable lifecycle assessments. It does not certify a claim merely because a record is signed.

A hardware loop is a separate future phase requiring an explicit, independently reviewed safety case, instrumentation validity, interlocks, operator control, and site-specific authorization.

## 10. v0.1 acceptance and next steps

v0.1 is complete when the protocol, schema, synthetic corpus, source limitations, and claim ceiling are reviewable and internally consistent. The initial corpus is **not** a physics validation dataset.

The standalone Rust reference tool lives at `tools/glass-silicon-contract-audit/`; dependency versions are pinned and a dedicated lockfile is checked in. Its contract verifier and mass-ledger arithmetic tests are written, but must be compiled/executed under the pinned toolchain before any runtime-pass claim. Its tolerance-basis field is provenance metadata, not proof that the tolerance is physically justified.

Next sequence:
1. freeze a source registry and reproduce one container-glass reference plus one MG-Si reference; before a source artifact can enter the benchmark, acquire it under resolved permitted-use terms, run the local `hash-artifacts` receipt to compare published MD5 and calculate SHA-256, independently review the receipt, and update the manifest without relabeling synthetic/simulated data as measured;
2. implement dependency-free Rust contract/conservation checks with mutation tests;
3. connect to Symthaea's existing materials and evidence owners and Symtropy's AtomFab/process-twin contract;
4. define holdout data and baselines before running optimization;
5. only then run candidate search, publish an exact-head receipt, and decide whether a supervised physical pilot is justified.

### Claim ceiling

This benchmark does not establish that any new material recipe works, a model is physically valid, any plant can save 5–10% energy, a lower-carbon route is commercially mature, an industrial facility is safe, or that our stack beats state of the art. Those claims require the corresponding independent evidence.


## Follow-on implementation status — 2026-10-23

The standalone Rust verifier now has an additional `audit-decision-value` command for the synthetic finite-pool contract in [GLASS-SILICON-DECISION-VALUE-BENCHMARK-V0_1.md](GLASS_SILICON_DECISION_VALUE_BENCHMARK_V0_1.md). This complements corpus-integrity and elemental mass-balance checks with exact minimax-regret arithmetic before and after a candidate measurement. It keeps regret reduction, measurement cost, and duration separate; only budget-eligible options enter its one-step oracle and Pareto frontier.

The synthetic known answer has been independently cross-checked outside Rust: baseline worst-case regret 5; informative in-budget measurement regret 0; uninformative measurement gross reduction 0; informative but over-budget measurement excluded. The fixture includes frozen feasibility controls and producer-access contract declarations. These checks validate the example's arithmetic/declared structure only, not a physical scenario or actual process isolation.

The Rust crate now defines **43 unit tests**, but no local Rust toolchain is available in the authoring environment; the tests have not been compiled or executed locally. The current exact-head CI run is the required runtime qualification gate, and a queued state is not a pass. No real source artifacts have been admitted, no SCALE results reproduced, and no decision-policy superiority or process-performance improvement is claimed.


The follow-on adaptive implementation is documented in [GLASS_SILICON_DECISION_VALUE_BENCHMARK_V0_1.md](GLASS_SILICON_DECISION_VALUE_BENCHMARK_V0_1.md) and exposed as `audit-adaptive-campaign`. It adds an exhaustive, small-instance decision tree where later measurements depend on observations, tracks cumulative per-path cost/duration, and selects a robust feasible action at each leaf. Six new tests cover the two-measurement complementarity fixture and bounds. Combined with the six one-step decision-value tests, the standalone Rust crate now defines **43 unit tests**.

The four-scenario adaptive fixture has an independent JavaScript-known-answer calculation: regret 6 before observations, regret 0 after two complementary measurements, total max-path cost 2 synthetic units and duration 20 synthetic seconds. This is static fixture arithmetic, not Rust runtime validation or real experimental evidence. Exact-head CI is still the required compilation/test gate.
