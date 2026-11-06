# Fragmentation–Transport Co-Design Benchmark v0.1

**Parent issue:** #3864  
**Composes with:** #1952, #3812/#3814, #3825, #3844, #3845, #3846/#3863, #3853  
**Claim ceiling:** measured-record consistency, evidence binding and offline capacity-envelope comparison. This benchmark does not qualify a conveying system, machine, mine plan, safety case or control policy.

## 1. Research objective

The machine does not produce useful output when it fractures rock; it produces useful output when the resulting material can be captured, transported, separated and discharged without unacceptable residual accumulation or downtime.

This benchmark connects the generation process to the material-removal process:

\`rock + tool + operating state -> fragment distribution + generation rate -> capture -> conveying -> separation -> discharge -> independently accepted output\`

The benchmark is concerned with sustained operation under a declared geometry, material cohort, installed path, time window and system boundary. It preserves cutting-only metrics but does not let them stand in for full-cycle output.

## 2. New field and research evidence

### 2.1 South African integrated undercutting and vacuum extraction trial

Raju & Esterhuyse, *Mining, Metallurgy & Exploration*, published 1 October 2026, DOI 10.1007/s42461-026-01722-w, report a seven-month underground validation of an integrated hard-rock undercutting and vacuum extraction system in narrow tabular platinum mining.

The paper reports:
- Average advance of 1.20 m per cutting hour against a 1.05 m/h business-case target; peak 1.4 m per cutting hour.
- A recorded excavation rate of 8.9 m³/h at the reported standard setting; this is not interchangeable with elapsed-shift accepted output.
- Predominantly anorthosite averaging 237 MPa UCS, above the 100–130 MPa design envelope.
- Extraction efficiency of 88% for fragments smaller than 20 mm and 95% for fines smaller than 10 mm. These thresholds are nested, not mutually exclusive bins; they must never be added.
- A 50 mm residual loose layer on parts of the floor, affecting gripper reaction and machine position reference.
- Design conveying length of 260 m, but the maximum underground length tested was 205 m.
- Underground suction/airflow did not fully reproduce factory-acceptance performance. The paper reports a 90–100 m/s design inlet velocity at 280 mbar, and identifies hose friction, length and rock density as contributors to reduced removal and fallback.
- A 60 mm depth of cut at 15 mm penetration was demonstrated but could not be sustained because removal became limiting at the corresponding fragment-generation rate.

The authors explicitly say the trial demonstrates technical feasibility and identifies controls needed before extended production evaluation. Cutter life, full reliability, availability-adjusted production and lifecycle economics were not quantified to production-assessment standard. Record these limits whenever using the result.

Source: https://doi.org/10.1007/s42461-026-01722-w

### 2.2 Manufacturer figures are denominator-specific

Komatsu's MC51 product pages illustrate why a benchmark needs a denominator dictionary:
- The Australian page lists 30 m³/h as **in-situ material handling**, and a typical maximum cut rate of 20 m³/h.
- The US page lists **up to 60 m³/h remove cut material**.
- Both pages describe a 320 kW machine power figure and a maximum 5.2 m × 5.2 m tunnel profile; the product page lists rock hardness up to 250 MPa.

These are manufacturer specifications, not independently matched underground production measurements. Do not treat 30, 20 and 60 m³/h as the same measurand or infer a field productivity lead from them. A matched comparison needs the same solid/loose/in-situ volume basis, accepted-output rule, time boundary, geology and included system boundary.

Sources:
- https://www.komatsu.com/en-au/products/equipment/hard-rock-continuous-miners/mc51
- https://www.komatsu.com/en-us/products/equipment/hard-rock-continuous-miners/mc51

### 2.3 Coarse-particle horizontal pneumatic conveying

Ma et al., *Powder Technology* 482 (October 2026), article 122715, DOI 10.1016/j.powtec.2026.122715, use an experimentally validated CFD–DEM model to examine horizontal coarse-particle conveying. They identify transitions among stratified flow, sedimentation and dune formation; particle diameter changes local pressure response and flow regime, and pressure-drop behaviour differs by regime. Their study focuses on 2 and 4 mm particles and does not qualify a mine-scale rock vacuum line.

Source: https://doi.org/10.1016/j.powtec.2026.122715

### 2.4 Vertical plug-flow behaviour

Erken, Ooi & Hanley, *Chemical Engineering Science* 328 (15 June 2026), article 123719, DOI 10.1016/j.ces.2026.123719, use CFD–DEM to investigate vertical dense-phase plug conveying. Plug coalescence, particle shape, friction and inlet velocity influence pressure fluctuations and blockage mechanisms. The simulated/experimental basis is a 3 m laboratory pipe, so its quantitative limits cannot be transferred to long mine installations.

Source: https://doi.org/10.1016/j.ces.2026.123719

### 2.5 Electrical-current condition identification

A 2026 *Measurement* study, DOI 10.1016/j.measurement.2026.121117, reports electrical drive-current signals for condition identification in a very dilute-phase pneumatic conveying test. It reports >96.4% classification accuracy across its tested conditions, including no-load, light-load, blockage and water accumulation. This is a low-sensor-cost research direction, not a hard-rock, long-hose or underground qualification. We should test current + pressure fusion, not assume the result transfers.

Source: https://doi.org/10.1016/j.measurement.2026.121117

### 2.6 Near-blockage pressure signals

Mittal & Mallick, *Powder Technology* 464 (2025), article 121243, DOI 10.1016/j.powtec.2025.121243, examine pressure fluctuations in dense-phase conveying as airflow is reduced toward blockage. Pressure-signal behaviour changes near blockage. Dense-phase fine-powder results are an adjacent sensing precedent; the test protocol must be revalidated for the coarse, abrasive and possibly dilute-phase rock regime.

Source: https://doi.org/10.1016/j.powtec.2025.121243

## 3. Paradigm shifts

### 3.1 Co-design the generated particle distribution and conveying path

Treat fragment size, shape, density, moisture and generation rate as inputs to transport design. Cutter penetration or lowest cutting energy is not the primary objective if the generated stream exceeds the removal envelope.

### 3.2 Installed-path capacity, not nameplate capacity

A factory blower rating or short pilot line cannot stand in for the installed path. Capacity evidence must bind to hose/pipe length, diameter, bends, elevation, material/lining, inlet, cyclone/filter state, air conditions and material cohort. The South African trial's 205 m maximum tested length versus 260 m design length makes this distinction operationally important.

### 3.3 Treat the conveying line as an observable dynamic system

Fuse:
- fragment generation and particle-size distribution;
- airflow and differential pressure at multiple line locations;
- blower/fan electrical current and power;
- filter/cyclone pressure drop and loading;
- recovered mass flow and residual accumulation.

A blockage-risk estimate must be tested against independently labelled blockage, partial restriction, water accumulation and normal transient events. Do not treat a current or pressure signature as blockage truth without independent physical evidence.

### 3.4 Distinguish steady-state capacity from transient recovery

A line can have adequate average throughput and still accumulate material locally during starts, turns, swing reversals, filter loading or temporary flow reduction. Benchmark time-resolved inventory, not only campaign averages.

### 3.5 Optimize accepted output, not movement or recovered mass alone

The primary functional output remains independently accepted excavation. Recovered mass, loose-spoil volume, floor residual, rejects and inaccessible material are separate accounting terms. A high recovery fraction does not establish that geometry, dilution, support or safety requirements were met.

## 4. Measurement contract

### 4.1 Common profile

Each case must bind:
- task, material cohort and acceptance profile;
- site and machine configuration;
- exact installed transport configuration and its revision;
- transport-capacity profile reference;
- native resource ledger and declared energy boundary;
- dry-solid mass basis;
- measurement protocol and clock synchronization reference;
- ordered stages: capture, convey, separate, discharge.

Capacity intervals are measured lower/upper bounds under the referenced installed configuration and declared material/operating condition. Factory-only capacity must not be substituted into the installed-path calculation.

### 4.2 Time windows and mass balance

Every measurement window uses the same start/end interval and synchronized clock reference. The following are measured on the same dry-solid mass basis:

- generated mass;
- recovered/discharged mass;
- residual material;
- measured reject/disposal mass;
- fragment size bins;
- per-stage capacity evidence.

Mass balance residual:

\`r_m = m_generated - m_recovered - m_residual - m_rejected\`

Compare \`|r_m|\` with the sum of stated standard uncertainty magnitudes as a conservative screening envelope. This L1 sum is not a probability interval and does not establish 95% coverage. Covariance and systematic error require a fuller uncertainty model. If mass balance is outside this envelope, return an indeterminate accounting result; never set unaccounted mass to zero.

### 4.3 Non-overlapping particle bins

A fragment-size distribution must use one ordered, contiguous partition of particle size:
- first lower boundary is zero;
- each finite upper boundary equals the next lower boundary;
- each bin is half-open \`[lower, upper)\`;
- only the last bin is open-ended \`[lower, infinity)\`;
- bin masses reconcile to independently measured generated mass within their combined stated uncertainty.

Cumulative thresholds such as “less than 10 mm” and “less than 20 mm” are valid derived summaries, but cannot be added together. The validator accepts bins, not overlapping cumulative thresholds.

### 4.4 Stage capacity and installed path

Every interval records one capacity interval for each stage:
1. CAPTURE
2. CONVEY
3. SEPARATE
4. DISCHARGE

Each capacity is a conservative measured lower/upper bound in kg/s with calibration, source and exact installed transport configuration reference. Only independent measurement or a declared derivation from independent measurements is admissible. Model/simulation or nameplate-only estimates may be retained in separate evidence records but cannot be promoted to measured installed capacity.

## 5. Derived quantities

For interval duration \`T\`:
- generated mass rate: \`m_generated / T\`, retaining mass and time uncertainty;
- recovered output rate: \`m_recovered / T\`;
- extraction efficiency: \`m_recovered / m_generated\`, only with positive independently measured generated mass;
- mass-balance residual and uncertainty screening result;
- conservative generation-rate interval from mass and time bounds;
- stage capacity comparison against the generation-rate interval.

A stage is a **definite capacity bottleneck** only when its measured capacity upper bound is below the generation-rate lower bound. It is a **possible/ambiguous bottleneck** when the intervals overlap. A stage is below the current generation demand only if this comparison uses matched installed-path, material and time evidence. If all measured stage lower bounds exceed the generation-rate upper bound, the evaluator says only that no stage-capacity bottleneck is evidenced in this interval; it does not infer breakage-limited operation.

## 6. Baselines and model tournament

- B0: cutter-only rate model ignoring removal (negative control).
- B1: factory/nameplate or static capacity assumption (reference only; rejected for installed-path claims).
- B2: empirical capacity curves calibrated on the actual installed transport configuration.
- B3: coupled CFD–DEM / airflow and particle model, calibrated against independent pressure and mass-flow measurements.
- B4: signal-fusion model using current, pressure, airflow, particle input and recovery measurements.
- B5: coupled physics + learned residual / OOD model.
- B6: Symtropy/Symthaea model, evaluated only on identical frozen records, labels and splits.

Hold out complete material cohorts, hose/path configurations, filter states, campaigns and sites. Random adjacent windows are diagnostic only. Test and interval-calibration data remain separate.

## 7. Metrics and proposed targets

Report all values by campaign, material and installed-path subgroup:

- bin-wise particle-size mass error and distribution distance;
- mass-balance residual / combined uncertainty and fraction of windows with closed accounting;
- generation and recovered mass flow (kg/s);
- extraction efficiency by non-overlapping bins;
- pressure drop per unit length, airflow, line velocity and blower electrical energy;
- capacity lower/upper bounds and stage bottleneck confusion;
- blockage event precision/recall, false alarms per operating hour, detection lead time and missed-event count;
- residual floor mass/depth and local accumulation rate;
- blockages, cleanout/restart time, filter loading, wear and availability;
- accepted solid volume per full-cycle/elapsed hour and declared-boundary kWh per accepted m³, when the linked resource ledger is complete.

Proposed hypotheses, not achievements:
- 100% rejection of planted overlap, gap, nested-bin, unit, time-window, source-role, calibration and configuration mismatch controls.
- Exact deterministic replay on frozen synthetic cases.
- On held-out installed configurations, reduce sustained-throughput prediction error by at least 20% versus B1/B2 while retaining calibrated 90%/95% intervals.
- At matched severe-blockage recall, reduce false alarms by at least 20% versus pressure-only or current-only baselines.
- No productivity improvement is credited when residual accumulation, accepted geometry, safety constraints, wear burden or full-cycle energy worsens beyond preregistered bounds.

Preregister the primary outcome, the clustered split, the uncertainty procedure, sample-size/power analysis and the operating envelope before inspecting the holdout. If the independent dataset is too small, report estimates and uncertainty without declaring superiority.

## 8. Adversarial corpus

Include:
- cumulative nested size thresholds presented as disjoint bins;
- gaps, overlaps, duplicate bins and unsorted bins;
- bin masses inconsistent with generated mass;
- generated/recovered/residual/reject mass not closing within uncertainty;
- mass flow measured on a different clock window;
- missing or drifting clock synchronization;
- kg vs tonnes or wet mass vs dry-solid mass mismatch;
- nameplate/factory capacity passed as installed-path evidence;
- capacity measured on a shorter hose, different bends/diameter/lining/elevation or material cohort;
- capacity model or simulation passed as a physical measurement;
- pressure tap blocked or drifting;
- airflow drop without blockage (e.g. planned transient or filter change);
- current increase from blower electrical fault without a conveying blockage;
- water accumulation, filter loading, partial restriction and complete blockage;
- adequate average capacity but local deposition/floor accumulation;
- dense/coarse fragments at unchanged nominal mass flow;
- generation surge at unchanged transport configuration;
- missing material falsely recorded as recovered;
- recovered mass counted again as discharge mass;
- elapsed shift compared to cutting-only duration;
- unsupported mine-scale extrapolation from short test pipe.

## 9. Integration

- #3864 owns fragmentation / transport co-design.
- #1952 owns method-level full-cycle comparison.
- #3812/#3814 own trial metrology, accepted output and time boundary.
- #3825 owns typed native resource and energy accounting.
- #3844 owns fracture-path preconditioning.
- #3845 owns composite strata response.
- #3846 / #3863 own cutter-health evidence and maintenance transfer.
- #3853 owns robust strategy decisions and abstention.
- #1933 owns mine-campaign and material-to-saleable-output boundaries.
- #3591 owns metrology reproducibility.

## 10. Implementation claim ceiling

The v0.1 Rust crate validates the record contract and computes descriptive interval arithmetic. It does not simulate fluid/particle dynamics, infer real blockage states, control blowers or cutters, certify a transport line, or demonstrate field performance. Its fixture is fabricated. Cargo tests must be executed before merge; structural replay is not a Rust test pass.
