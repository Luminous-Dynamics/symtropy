# Rock Extraction Trial and Comparison Contract v0.1

**Parent tournament:** https://github.com/Luminous-Dynamics/symtropy/issues/1952  
**Implementation issue:** https://github.com/Luminous-Dynamics/symtropy/issues/3812  
**Status:** standalone contract and synthetic implementation draft  
**Claim ceiling:** validation of record shape, comparison boundaries and descriptive arithmetic only; no physical process, model, protocol, causal effect or method is qualified by this work.

## 1. Why this contract exists

The rock-breakage tournament needs a stricter comparison layer before it can responsibly compare mechanical, hybrid and non-mechanical methods. A performance result is not comparable merely because both papers report "specific energy", "penetration rate" or "productivity".

A meaningful comparison must bind:
- the exact task and accepted functional output;
- a characterized and appropriately paired material cohort;
- geometry and operating envelope;
- machine, tool and configuration identity;
- independent measurement procedures and calibrations;
- the same system-energy boundary and auxiliary inventory;
- the same full-cycle time boundary;
- acceptance and exclusion rules;
- protocol/study design and uncertainty.

The first Rust contract in docs/research/rock-extraction-portfolio and crates/domains/symtropy-rock-extraction-bench validates those boundaries for already measured trial records. It does not make a method physically suitable or a protocol scientifically sufficient.

## 2. Current research results that make fair comparison urgent

These results are useful, but they use different mechanisms, materials, test scales and denominators.

### 2.1 Axial–torsional coupled percussion (ATCP)

Sun et al. (Petroleum Exploration and Development, issue date 4 November 2026) report hot-dry-rock drilling research using axial and torsional impacts. The paper uses high-temperature granite specimens, digital image correlation and a high-temperature/high-pressure percussion platform. It reports a Fushen-1 field test in altered granite above 180°C:

| Interval | Footage | Time | Reported ROP |
|---|---:|---:|---:|
| ATCP test, 4,604–4,637 m | 33 m | 6.20 h | 5.32 m/h |
| Adjacent conventional-rotary interval, 4,653–4,824 m | 164 m | 85 h | 1.93 m/h |

The paper reports 175.65% higher ROP and 63.68% lower MSE for the ATCP test interval. This is high-value field evidence, but the comparison is between different adjacent intervals and is not a randomized matched trial. The intervals differ in footage and potentially in rock/operating conditions. Do not treat the ratio as an unbiased causal effect or a transferable granite benchmark.

Source: https://doi.org/10.1016/S1876-3804(26)60774-4

### 2.2 Laser-assisted PDC in granite

Ma et al. (2025) combine experiment and simulation for laser-assisted PDC cutter configurations in granite. The reported cone-hybrid configuration reduces comprehensive specific fragmentation energy by 70.81% against the paper's minimum mechanical baseline, but by 7.4% against the laser-assisted triple-cylinder configuration. Those are configuration-specific fragmentation-energy comparisons—not a whole-rig electrical-energy result, field production result or lifecycle saving.

Source: https://doi.org/10.1016/j.geoen.2025.214317

### 2.3 Oscillating disc cutters (ODC)

Wang et al. (2026) use a full-scale linear rock-cutting rig to test ODC kinematics on red sandstone (reported UCS 61.33 MPa, Brazilian tensile strength 5.46 MPa). The study reports the effect of oscillation speed ratio and cutting depth on thrust, torque, specific energy and fragment size. Increasing oscillation intensity creates trade-offs: thrust/torque/cutting SE increase in the tested range, while downstream crushing energy falls as larger fragments are produced. The paper also describes reported industrial ODC machine deployment, including a completed Sydney underground passage with a Komatsu MC51 in 2025. The study's controlled test material is sandstone, not granite or basalt.

Source: https://doi.org/10.1177/01445987261471924

### 2.4 Full-scale disc abrasion testing (FDAT)

Nasrabadi et al. (2026) introduce a full-scale disc-abrasion apparatus. Twelve tests used 305 mm monoblock single- and double-edge discs on quartz-rich marble (UCS about 33.1 MPa; CAI 2.2), varying normal force, rolling force and rotation speed. Reported specific cutter weight loss ranged from 0.14 to 2.09 mg/m. The paper identifies a relationship between rolling force and wear and a non-monotonic wear trend with rpm. This is a metrology/test-rig precedent; the test rock is not representative of hard granite.

Source: https://doi.org/10.1016/j.tust.2026.107672

### 2.5 Plasma and other hybrid benchmarks

- SPARC: low-energy electrical pulses (about 80 J/pulse) pre-crack granite before a PDC cutter; the authors report up to 56% lower specific cutting energy within their laboratory cutting-test boundary. https://doi.org/10.1016/j.renene.2025.122351
- Pulsed thermal plasma jet: a distinct direct thermal-removal mechanism; the study reports different specific-energy, penetration and electrode-wear comparisons against a constant plasma jet. https://doi.org/10.1016/j.geoen.2023.212466
- Waterjet + disc cutter, microwave, thermal spallation, ultrasonic and cryogenic assists each require their own mechanism and system-energy boundaries.

None of these headline percentages should be placed on a single leaderboard. They are different measurands from different experiments.

## 3. The comparison paradigm

The primary comparison is not "which device has the lowest reported number?" It is:

**For a preregistered task on a declared material/geometry cohort, what independently accepted useful output was delivered over the full cycle, using how much energy and other resources inside a common declared boundary?**

### 3.1 Required common profile

A ComparisonProfile binds:
- protocol revision;
- task profile;
- material cohort and pairing/block ID;
- target geometry;
- acceptance rule;
- site/test context;
- metrology protocol;
- energy-boundary identity and energy basis;
- exact inventory of included energy-consuming subsystems;
- full-cycle time basis;
- study-design classification;
- minimum replicates per arm.

Every trial must match that profile exactly. The profile is a declared comparison boundary, not an authenticated or certified authority.

### 3.2 Required trial observations

Every trial binds:
- unique trial, method, machine, tool and physical specimen IDs;
- a matched-pair identifier when the declared study design requires paired trials; each pair must contain exactly one baseline and one candidate;
- exact profile/protocol/task/material pairing/geometry/acceptance/site/metrology references;
- energy quantity in kWh, standard uncertainty, energy basis, boundary reference, energy-accounting reference, subsystem inventory, evidence role, calibration and source; derived observations also bind a lineage reference to their upstream measured artifacts;
- independently accepted solid volume in m³, standard uncertainty, evidence role, calibration and source;
- full-cycle elapsed hours, standard uncertainty, evidence role, calibration and source.

Only IndependentMeasurement or DerivedFromIndependentMeasurements may support the primary energy, accepted-output and time metrics. A derived result must preserve the upstream measurement lineage. Process telemetry, operator reports, model estimates and simulations may be retained in other datasets, but cannot silently substitute for independent primary metrics.

### 3.3 Separate measurands

Keep these separate:
- tool-interface mechanical work;
- machine electrical input;
- facility electrical input;
- declared total-system energy with an explicit accounting rule;
- solid volume removed;
- accepted solid volume;
- loose spoil volume;
- active cutting time;
- full-cycle elapsed time;
- scheduled shift time.

A comparison may only use one consistent energy basis, boundary reference and subsystem inventory across both arms. If fuel, cryogen, fluid, abrasive or other inputs are material to the process, record them in the declared energy/resource accounting; do not silently omit them or double-count conversion output and input. The current v0.1 calculator normalizes only the declared energy observation and intentionally does not invent conversion factors.

Loose-spoil volume is not solid-rock volume: swell, fragmentation and bulk density differ. The primary output denominator is independently accepted solid volume, not volume of loose spoil in a bucket or conveyor.

### 3.4 MSE remains a separate metric

Symtropy #1593 owns measured mechanical specific energy and its uncertainty propagation. This comparator must not fork or redefine that calculation.

The v0.1 primary system metrics are:
1. accepted_output_rate_m3_per_hour = Σ accepted_solid_volume_m3 / Σ full_cycle_elapsed_hours
2. full_system_energy_kwh_per_accepted_m3 = Σ declared_boundary_energy_kwh / Σ accepted_solid_volume_m3

These are pooled descriptive arm metrics. They are not a statistical treatment-effect estimate, do not propagate uncertainty and do not rank methods. Uncertainty values are retained for downstream metrology/statistical analysis.

## 4. Design and analysis requirements beyond the validator

- Preregister primary/secondary outcomes, sample size rationale, randomization/blocking, exclusions and statistical analysis.
- Use matched or blocked specimens when physically possible; do not claim the same destructive-test specimen was reused unchanged.
- Use observational labels for field comparisons that cannot randomize or match operating intervals (such as ATCP adjacent-interval results).
- Record geology, machine, tool, and calibration changes; treat out-of-profile data as a separate cohort, not silent pooling.
- Separate cutting-only analysis from full-cycle economics. Include setup, tool changes, cooling, flushing, cleaning, spoil extraction, recovery, maintenance and stoppages in the full cycle.
- Report sample counts, distributions, uncertainty budgets and confidence intervals in the preregistered analysis. The v0.1 Rust crate does not compute these.
- Hold out complete sites, rock cohorts, drives and equipment configurations for transfer claims.
- A model or process sensor must not be its own independent referee.

## 5. Implemented v0.1 validator boundary

crates/domains/symtropy-rock-extraction-bench is dependency-free and has an isolated Cargo workspace.

It checks:
- empty and duplicate identities;
- non-finite or non-positive energy, accepted volume and time;
- negative/non-finite standard uncertainties;
- independent evidence role for all primary observations;
- accepted-solid-volume basis;
- full-cycle time basis;
- matching task/material pairing/geometry/acceptance/site/metrology profiles;
- matching energy basis, boundary reference and subsystem inventory;
- required baseline and candidate arms;
- configured minimum replicate count;
- duplicate trial IDs;
- explicit study-design retention.

It computes only pooled descriptive metrics and emits causal_effect_claim_allowed = false for every comparison. The crate does not authenticate sources, validate a physical model, infer a mechanism, perform uncertainty propagation, or select an optimal method.

## 6. Conformance and adversarial test plan

The current Rust module contains 33 unit tests covering valid pooled arithmetic; modelled output; loose spoil; active-only time; mismatched energy boundaries; omitted/duplicate auxiliary subsystems; process-sensor energy; duplicate trials; changed material cohorts and acceptance profiles; adjacent-interval descriptive status; insufficient replication; missing machine/tool, calibration or energy-accounting identity; missing derived-measurement lineage; duplicate physical specimen IDs; unmatched or multiply assigned specimen pairs; mixed machine/tool configurations within an arm; non-finite/zero values; negative uncertainty; energy-basis mismatch; single-arm and missing-arm comparisons; order invariance; and aggregate overflow.

Next tests to add:
- duplicate profile subsystem IDs;
- missing calibration or energy-accounting references;
- energy uncertainty and accepted-volume uncertainty edge cases;
- changed acceptance profile, geometry or site context;
- wrong protocol/profile revision;
- meter calibration mismatch;
- energy-accounting rule mismatch;
- output acceptance failure/rework and rejected material;
- missing or omitted cooling, ventilation, extraction or standby load;
- a treatment with lower cutting energy but higher full-cycle energy;
- randomization/refusal-to-infer cases;
- insufficient independent measurement provenance;
- profile-level duplicate/invalid IDs.

## 7. Integration

- #1952 owns the rock-breakage method family benchmark and tournament.
- #3812 owns the measured-trial comparison contract and standalone validator.
- #1593 owns mechanical MSE and its uncertainty propagation.
- #1588 / PR #1603 own material/tool interaction and spatial-delta contracts.
- #1594 owns acoustic wave/AE observables.
- #3305 owns ring-scale TBM geology, cutter and energy records.
- #1933 owns mine-campaign fleet, energy, ventilation and saleable output.
- #3605 owns independently accepted output.
- #3631 owns lifecycle cost per accepted service unit.
- #3591 owns metrology reproducibility and cross-laboratory comparison.

No domain ownership is duplicated. This crate consumes references; it does not replace their authorities.

## 8. Claim boundary

This is a measured-data validator and descriptive comparator only. It does not design, control or authorize plasma, laser, high-voltage, cryogenic, high-pressure, percussion, drilling or excavation machinery. Physical trials require separately qualified equipment, competent engineering and safety approvals, and independent supervision.