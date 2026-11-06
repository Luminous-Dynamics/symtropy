# Mechanism-Resolved Rock/Tool Response Experiment v0.1

**Parent:** ROCK-TOOL-RESPONSE-001 (#3823)  
**Composes with:** #3812 and draft PR #3814; #1588/#1603 material/tool response; #1594 acoustic observations; #1593 measured MSE; #3305 TBM ring data; #1933 mine campaigns  
**Status:** experimental-design and data-contract draft  
**Claim ceiling:** schema/measurement metadata conformance and descriptive physical observations only. No method, constitutive law, response model, causal effect, machine setting or field transfer is qualified by this document.

## 1. Research objective

Build a reproducible, mechanism-resolved experiment programme for:

1. Axial–torsional coupled percussion (ATCP).
2. Laser-assisted polycrystalline diamond compact (PDC) cutting.
3. Oscillating disc cutters (ODC).

The shared question is:

> For a characterized rock cohort and a declared task, what measured loading path and tool/contact geometry create useful fracture and accepted solid removal, and what are the resulting cutting, thermal, wear, material-flow and full-cycle trade-offs?

The common platform is not a universal physics model. It is a common evidence envelope around three different physical mechanisms. Method-specific fields must not be coerced into a common abstraction that erases mechanism identity.

## 2. Verified evidence and limits

### 2.1 ATCP: coupled axial and torsional percussion

Sun et al., *Petroleum Exploration and Development*, DOI 10.1016/S1876-3804(26)60774-4, report high-temperature/high-pressure tests and a Fushen-1 field comparison in altered granite above 180 °C.

The reported field intervals are:

| Interval | Footage | Drilling time | Reported ROP |
|---|---:|---:|---:|
| ATCP, 4,604–4,637 m | 33 m | 6.20 h | 5.32 m/h |
| Adjacent conventional rotary, 4,653–4,824 m | 164 m | 85 h | 1.93 m/h |

The paper reports 175.65% higher ROP and 63.68% lower MSE for the ATCP interval. The article appeared online on 15 October 2026 and has an issue date of 4 November 2026. The field comparator is an adjacent interval, not a randomized matched control. Lithology, depth, discontinuities, operating parameters and interval length may confound the comparison. Preserve it as observational field evidence, not as a causal treatment effect or a transferable performance guarantee.

**Research hypothesis:** coordinated axial and torsional impact can shift failure from repeated near-contact crushing/grinding toward larger-scale tensile/shear crack propagation and spalling. The causal chain must be measured: commanded waveform → actual tool/bit loading → local strain/fracture response → independently measured removed solid volume.

Do not conflate ATCP with sonic drill-string resonance, generic axial vibration or ultrasonic assistance.

### 2.2 Laser-assisted PDC cutting

Ma et al. (2025), DOI 10.1016/j.geoen.2025.214317, combine experiment and simulation for laser-assisted PDC cutting in granite. Their optimized cone-hybrid configuration reports 70.81% lower comprehensive specific fragmentation energy than the study's minimum mechanical baseline and 7.4% lower than its laser-assisted triple-cylinder configuration. The reported configuration includes a 20° back-rake angle, 6 mm cutter spacing, a 9 mm difference in depth of cut and 0° side rake.

These are configuration- and study-boundary-specific fragmentation-energy results. They are not a 70.81% whole-rig electrical-energy, field production, lifecycle-cost or mine productivity improvement. Reproduce the actual study's exact geometry, energy accounting and measurement definitions before using the reported numbers as an experimental target.

**Research hypothesis:** the spatial and temporal relation between the preheated/thermally weakened volume and the PDC contact/fracture field matters as much as laser power. Co-optimize beam placement, footprint, exposure/dwell, preheat-to-cut delay, cutter geometry and depth of cut.

Keep source electrical input, optical output, optical energy incident at the workpiece, absorbed-energy estimate, mechanical cutter work and auxiliary loads distinct.

### 2.3 ODC: eccentric oscillatory rolling-disc contact

Wang et al. (2026), DOI 10.1177/01445987261471924, used a full-scale linear cutting tester with a 100 mm ODC on red sandstone (UCS 61.33 MPa; Brazilian tensile strength 5.46 MPa). The tested speed ratio is:

    Rv = v / (omega * e)

where v is traction velocity, omega is spindle angular velocity in rad/s, and e is eccentricity. Larger Rv means more intense oscillation under the paper's definition.

At a 3 mm cutting depth, the paper reports:

| Rv | Mean thrust (kN) | Mean torque (Nm) | Cutting SE (MJ/m³) | P80 (mm) | Bond-derived crushing SE (kWh/t) |
|---:|---:|---:|---:|---:|---:|
| 0.256 | 2.548 | 51.592 | 7.41 | 15.79 | 0.796 |
| 0.318 | 3.872 | 53.678 | 11.03 | 19.26 | 0.721 |
| 0.593 | 3.986 | 57.135 | 12.36 | 21.43 | 0.683 |
| 0.800 | 6.021 | 64.363 | 17.23 | 21.53 | 0.682 |

The downstream crushing SE was estimated from particle size and Bond Work Index, not measured by running the produced fragments through a real downstream crusher. Across the tested Rv range, cutting SE rose substantially, P80 increased, the Bond-derived crushing estimate fell, and the paper's effective crushing-energy ratio declined. This is a real trade-off, not proof that greater oscillation reduces total process energy.

The experiment used red sandstone, not granite or basalt. Its force and energy response curves must not be transferred to hard granite without new tests.

**Research hypothesis:** eccentric kinematics can promote discontinuous tensile fracture and influence chip size, but the optimum oscillation regime depends on rock texture, strength, abrasivity, cutter geometry, contact duty, traction speed and the actual downstream comminution task.

### 2.4 Industrial operating-envelope comparators

Two industry references should shape our system-level scorecard without being treated as matched technology comparisons.

- Raju and Esterhuyse (2026), DOI 10.1007/s42461-026-01722-w, report a seven-month South African underground validation of an integrated undercutting and vacuum extraction system. It averaged 1.20 m advance per cutting hour (peak 1.4 m/cutting hour) in predominantly anorthosite averaging 237 MPa UCS. Extraction efficiency was 88% for fragments below 20 mm and 95% for fines below 10 mm. The authors report that material removal, residual fines, hose friction and floor/gripper stability constrained sustainable cutting settings. The paper distinguishes instantaneous cutting, full-cycle excavation and advance-per-cutting-hour denominators. Its operational dataset is not publicly available.
- Komatsu's published MC51 specification lists up to 60 m³/h cut-material removal, 320 kW, a maximum 5.2 m × 5.2 m tunnel profile and rock hardness up to 250 MPa. These are manufacturer-published specifications/claims, not an independent matched field dataset and not an accepted-solid-volume-per-full-cycle result.

These references demonstrate why breakage, extraction, navigation, support, machine availability and accepted excavation geometry must be evaluated as one system. Do not compare 60 m³/h material removal, 1.20 m per cutting hour, ROP and cutter-only specific energy on a shared leaderboard.

## 3. Paradigm shifts

### 3.1 From energy input to fracture-path control

Treat load direction, waveform, phase, contact geometry, rate, confinement and temperature as experiment variables. Measure whether they change crack initiation, crack coalescence, spalling, fragment size and damage—not merely actuator power.

### 3.2 From single-point optimization to response surfaces

For each method, estimate a bounded response surface over controllable variables and characterized material properties. Preserve interactions and uncertainty. A high-dimensional model must not interpolate outside its tested support and then present the prediction as qualified.

### 3.3 From cutting energy to the material-flow chain

Measure fragment distribution and actual downstream crushing/sorting/transport energy. Bond Work Index or P80 models can inform hypotheses, but their estimates remain separately labelled from measured plant energy. Include fines, oversize, residual spoil, dust, cooling, flushing, ventilation, tool changes and downtime in the full-cycle ledger.

### 3.4 From fixed machine modes to evidence-bounded adaptation

Symthaea may infer a geological/material state and uncertainty from observations. Symtropy may compare physical response predictions against independently measured trials. A deployment policy may only expose operating regions that have passed a separate engineering/safety qualification process. This research contract provides no machine-control authority.

### 3.5 From benchmark averages to transfer evidence

Hold out entire rock cohorts, sites, machines and tool geometries. Report where the response model is unsupported. A method that wins in altered high-temperature granite, sandstone or one anorthosite panel does not thereby win in all granite/basalt.

## 4. Experiment architecture

### 4.1 Shared specimen characterization

Granite and basalt must be stratified, not treated as single material labels. At minimum record:

- Lithology, source location, petrography/mineralogy and grain-size distribution.
- Density, porosity, saturation/moisture and relevant pore-fluid identity.
- UCS, Brazilian tensile strength, elastic modulus, Poisson ratio and fracture toughness when available.
- Cerchar abrasivity or another declared abrasivity measure.
- Discontinuity orientation, spacing, persistence and specimen/block orientation.
- Temperature and confining/pore pressure for conditions where they are relevant.
- Sampling, test-method, calibration, uncertainty and provenance references for each property.

Use sister specimens or blocked samples from the same characterized parent block where practical. Destructive tests must not reuse a specimen as unchanged. Preserve anisotropy and joint orientation. High-temperature granite is a separate temperature/environment cohort from ambient granite.

### 4.2 Controls and treatment arms

Every mechanism family needs its own relevant control:

| Family | Minimum control | Candidate arms | Important confounders |
|---|---|---|---|
| ATCP | Same bit/tool and rotary conditions with percussion disabled, where equipment supports a valid sham | axial only; torsional only; coupled ATCP; controlled phase/frequency variants | WOB, RPM, fluid flow/pressure, bit wear, depth, temperature, confining pressure, lithology |
| Laser-PDC | Same mechanical cutter/geometry with laser off (plus a sham exposure/control where needed) | laser on; placement/dwell/offset variants; cutter geometry variants | incident/delivered optical energy, absorptivity, surface condition, preheat delay, geometry, cutter wear |
| ODC | Conventional non-oscillating disc/cutter matched as closely as possible for diameter, edge geometry, penetration and rock cohort | ODC eccentricity/speed-ratio/depth variants | cutter geometry, disc wear, traction speed, spindle speed, depth, block anisotropy and clamping |

Randomize run order inside blocks where practical. Record all changes and exclusions before inspecting outcomes. If randomization is infeasible, classify the design as observational and constrain the claim accordingly.

Do not sweep many variables at once and then attribute the outcome to one factor. Begin with one-factor mechanism-isolation and small factorial/response-surface designs only after sensor validity and pilot variance are known.

### 4.3 Mechanism-specific measurements

**ATCP**
- Record commanded and measured axial/torsional waveform separately.
- Capture actual axial force and torque at the closest validated measurement location to the bit/tool; record sensor transfer function and dynamic calibration.
- Record phase offset, excitation frequencies, pulse shape, axial/torsional amplitude or energy, WOB, rotary speed, penetration, flow/pressure and temperature/confinement context.
- Add DIC, strain, high-speed imaging, acoustic emission and post-test CT/fractography where feasible. Treat AE events as observations, not automatic fracture-mode labels.
- Quantify tool/bit damage, connection fatigue, vibration transmission and downtime as well as rock removal.

**Laser-assisted PDC**
- Record laser wavelength, source input, optical output, workpiece-plane energy, spot dimensions, incidence, standoff, offset ahead of cutter, scan path and exposure duration.
- Record preheat-to-cut delay and surface condition. Capture thermal history using calibrated pyrometry/thermal imaging and post-test thermal damage/HAZ measurements.
- Record PDC type, cutter profile, back/side rake, spacing, depth of cut, speed, force, torque and wear.
- Run a closed energy ledger for laser source, optical conversion/delivery, mechanical drive, cooling, extraction and ventilation. Do not infer absorbed energy directly from source electrical input.

**ODC**
- Record e, D, omega, v, depth of cut, cutter orientation, blade/edge geometry, material and measured force/torque histories.
- Compute Rv = v/(omega*e) from measured kinematics; retain the raw variables and calculation revision so the ratio is independently reproducible.
- Measure thrust, lateral and normal forces; torque; displacement; dynamic contact; chip mass and size distribution.
- Use physical sieving/imaging with mass closure. For the downstream process, instrument the actual crusher or comminution stage and record feed/product size distributions, throughput, energy and losses. Report Bond-derived values as estimates in a different field.

## 5. Signal, metrology and provenance contract

The machine's commanded setpoint is not the physical input. The instrument channel is not automatically an independent referee. Every time-series channel must declare:

- Channel ID, physical quantity, SI unit, sensor/transducer identity and measurement location.
- Calibration reference, calibration validity interval, uncertainty and dynamic response/bandwidth where relevant.
- Sampling rate, analogue bandwidth, anti-alias filter, clock domain and synchronization uncertainty.
- Saturation, missing-sample, dropout, clipping and range-exceeded flags.
- Raw artifact reference and SHA-256 digest.
- For every processed trace: algorithm and version, parameters/digest, parent artifact references and transform lineage.

Choose sample rate/bandwidth to resolve the fastest relevant physical event and sensor dynamics. Nyquist alone is not enough if the transducer, mounting or analogue filtering cannot preserve the signal. Do not prescribe one fixed rate for ATCP, laser-PDC, ODC, acoustic emission and thermal channels.

Keep raw samples immutable. Filtering, resampling, event detection, feature extraction and model outputs create new artifacts; never overwrite the raw trace. If clock synchronization is insufficient to establish phase between axial and torsional channels, the phase-dependent ATCP experiment is indeterminate.

A measurement role must be explicit: independent measurement, process sensor, operator reported, derived from independent measurement, model estimate or simulation. A derived signal requires parent evidence lineage. Model/simulation channels may be useful for hypothesis generation but cannot silently become measured physical response.

## 6. B0–B3 benchmark programme

| Level | Work | Exit gate |
|---|---|---|
| B0 | JSON/schema conformance, unit/dimension checks, synthetic time-series, clock and lineage fault injection | 100% frozen conformance; zero false acceptance for missing calibration, units, digest, clock or lineage |
| B1 | Matched/sister specimen tests, single cutter/tool or bench-scale rig, instrument calibration and repeatability | Pre-registered primary outcomes, measured uncertainty, material characterization, independent accepted-volume/fragment evidence |
| B2 | Full-scale linear cutting / representative hardware rigs; ATCP high-temperature/high-pressure rig where available; laser/PDC and ODC full-scale fixtures | Complete subsystem energy boundary, force/torque/thermal/wear response, measured accepted output and full-cycle record |
| B3 | Independent site trial with operational baseline and transfer holdouts | Independent metrology, machine/site/rock transfer evidence, maintainability/reliability and full-cycle economic record |

B0 is a software contract, not physical validation. A B1 or B2 result does not imply B3 qualification.

## 7. Outcome hierarchy and benchmark metrics

**Mechanism response (diagnostic):**
- Axial and torsional impulse/work at declared measurement locations; phase stability; force/torque peak, RMS and distributions.
- Cutting mechanical work; source and delivered auxiliary energy kept separate.
- Fracture observations: independently measured crack/spall volume, fracture-surface morphology, fragment mass/size distribution and damage zone.
- Tool wear, edge/chip damage, temperature and maintenance events.
- Fragment-size distribution, fines/oversize and material-flow stability.

**System outcome (primary):**
- Independently accepted solid volume / accepted excavation length.
- Accepted output per full-cycle hour.
- Declared-boundary energy per accepted solid m³ or accepted advance unit.
- Full-cycle cost per accepted unit only when cost inputs have a separately qualified source.
- Consumable/wear rate, availability, rework, overbreak, extraction losses and residual spoil.
- For downstream processing, measured crusher energy per accepted product tonne/m³ and product-quality constraints.

Never merge cutting-only mechanical work, electrical input, thermal delivery and estimated downstream crushing into an unlabeled single number. The full-cycle comparator in #3814 remains the authority for pooled descriptive accepted-output rate and declared-boundary energy intensity.

**No arbitrary 10–20% performance target is accepted as a scientific result.** First measure pilot variance, instrument uncertainty and the baseline distribution. Then preregister a minimum practically important difference and sample-size/power rationale. Report effect intervals, uncertainty budgets, replicate counts, exclusions and all preregistered outcomes.

## 8. Multi-objective optimization, not a weighted winner

Treat performance as a constrained Pareto problem over:

- Accepted full-cycle output.
- Total declared-boundary energy and cost.
- Cutter/tool wear and replacement time.
- Product fragment distribution and downstream energy.
- Extraction efficiency, dust/fines and material-handling burden.
- Overbreak, heat-affected zone and ground/structure damage.
- Availability, maintainability, safety and environmental constraints.

Do not create an arbitrary weighted composite score in v0.1. Publish Pareto fronts and the constraints used. A candidate only dominates another when the comparison profile, acceptance, evidence quality and uncertainty support that statement.

## 9. Transfer and validation rules

Perform leave-one-rock-cohort-family-out, leave-one-site-out, leave-one-machine-configuration-out and leave-one-tool-geometry-out tests. Report in-domain and out-of-domain results separately.

A model's support domain must be explicit in:
- lithology/mineralogy/texture and abrasivity;
- strength, fracture toughness and discontinuity regime;
- temperature, confinement and fluid conditions;
- cutter/bit geometry, wear state and loading waveform;
- machine stiffness, power envelope and auxiliary system configuration.

Never use interpolation/extrapolation output outside this domain as an operating recommendation. New geological or tool regimes require new evidence.

## 10. Integration boundaries

- This response bundle links specimen/cohort, protocol, machine, tool, method configuration, response channels and immutable artifacts.
- The full-cycle trial record remains in the rock extraction bench contract (#3812 / PR #3814). It owns accepted solid volume, declared system energy and full-cycle time.
- #1593 remains the owner of measured mechanical specific energy and uncertainty propagation.
- #1588/#1603 remains the owner of material/tool response and spatial deltas.
- #1594 owns acoustic propagation/AE model interfaces; AE observations must remain distinct from interpreted fracture.
- #3305 and #1933 remain the owners for TBM ring-level and mine-level records.
- Symthaea may consume characterized geology and qualified evidence for bounded inference; Mycelix may preserve provenance/custody; Sol-Atlas may contribute site/asset context. This response schema creates no new inference, attestation or control authority.

## 11. Open questions before physical protocol freeze

1. Which laboratories can supply representative granite and basalt blocks with petrography, strength, fracture-toughness and abrasivity measurements?
2. What dynamic load/torque transducer bandwidth and mounting are available for the ATCP bit/fixture?
3. Can laser workpiece-plane energy and thermal field be independently measured without changing the optical path or cutter conditions?
4. Can ODC test fragments be passed through a separately metered representative crusher with feed/product mass closure?
5. Which specimen dimensions and clamping conditions limit boundary effects without changing the fracture regime?
6. What sample count is justified after pilot repeatability and between-block variance are measured?

Do not freeze parameter ranges or sample-size claims until these are answered with equipment and pilot evidence.

## 12. Sources

- Sun et al. (2026), ATCP hot-dry-rock drilling: https://doi.org/10.1016/S1876-3804(26)60774-4
- Ma et al. (2025), laser-assisted PDC in granite: https://doi.org/10.1016/j.geoen.2025.214317
- Wang et al. (2026), oscillating disc cutter kinematics: https://doi.org/10.1177/01445987261471924
- Nasrabadi et al. (2026), full-scale disc abrasion test: https://doi.org/10.1016/j.tust.2026.107672
- Raju & Esterhuyse (2026), South African in-stope cutting/vacuum extraction validation: https://doi.org/10.1007/s42461-026-01722-w
- Komatsu MC51 published machine specifications: https://www.komatsu.com/en-us/products/equipment/hard-rock-continuous-miners/mc51
- Colorado School of Mines Earth Mechanics Institute, linear cutting test facilities: https://emi.mines.edu/laboratories-and-test-facilities/


## 13. Executable semantic validation tranche

The standalone crate `crates/domains/symtropy-rock-tool-response-bench/` implements cross-field and cross-reference checks for the v0.1 response bundle. It complements, but does not replace, JSON Schema 2020-12 validation.

Current authored tests cover:
- frozen three-mechanism fixture acceptance;
- schema-version and empty-bundle rejection;
- unique record/specimen/channel/artifact identity and artifact evidence-role consistency;
- mechanism/configuration mismatch and unknown method fields;
- artifact digest format and channel/artifact/source/parent references;
- ATCP waveform artifact resolution;
- raw artifact clock-domain agreement and required coupled-channel clock consistency;
- required synchronization uncertainty, anti-alias reference and sample-rate/bandwidth relationship;
- calibration and non-negative uncertainty;
- rejection of model/simulation evidence as a required independent diagnostic channel;
- mechanism-specific diagnostic channel presence and unit semantics;
- derived observation lineage;
- artifact self-cycle detection;
- ODC speed-ratio recomputation from independently measured encoder observations, raw-channel bindings and matching calibration references;
- laser ON, OFF_SHAM and OFF_BASELINE energy semantics;
- deterministic issue ordering and JSON parse errors.

The laser control distinction is explicit:
- ON requires positive source electrical input and workpiece-incident optical energy.
- OFF_SHAM keeps source electrical input energized while declaring zero optical exposure at the workpiece and zero absorbed-energy estimate.
- OFF_BASELINE requires zero source electrical input, zero workpiece exposure and zero optional output/absorption quantities.

This allows the physical mechanical control to be represented without claiming laser delivery.

The current source contains **44 unit tests authored**. The exact command is:

`cargo test --manifest-path crates/domains/symtropy-rock-tool-response-bench/Cargo.toml`

Execution remains unverified because this environment has no Cargo, rustc or rustfmt. Do not interpret authored tests or structural checks as a Rust compile/test pass. The validator checks SHA-256 digest syntax; only the artifact-ingestion boundary can verify digest-to-byte correspondence. It does not validate physical calibration or infer a fracture mechanism.


The ODC kinematic evidence path was tightened after review: the configuration now references independent angular-speed and traction-speed observations; each observation must use rad/s or m/s respectively, match the corresponding encoder channel's calibration, and cite a raw artifact that actually contains that channel. The stored speed ratio is recomputed from those observed values and eccentricity, not just from configuration scalars. The ODC fixture now contains four synchronized channels (thrust, torque, angular speed, traction speed). This remains schema/semantic validation of synthetic records; no physical encoder calibration or ODC machine performance is established.

A further follow-on issue, #3839, tracks the broader need to distinguish setpoints from actual measured values for ATCP and laser-PDC energy stages, including protocol-specific synchronization tolerances.