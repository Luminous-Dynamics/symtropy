# In-Situ Torque Sensor Transfer Qualification v0.1

**Status:** M1 installation-grouped interval calibration and held-out transfer evaluator with synthetic fixture. This is not a MEMS compensation model, a physical torque calibration or an HRI safety qualification.

**Research owner:** [IN-SITU-TORQUE-SENSOR-TRANSFER-001 #6362](https://github.com/Luminous-Dynamics/symtropy/issues/6362)  
**Paired actuator observability:** [#6171](https://github.com/Luminous-Dynamics/symtropy/issues/6171)  
**Synchronized metrology:** [#6266](https://github.com/Luminous-Dynamics/symtropy/issues/6266) / [PR #6272](https://github.com/Luminous-Dynamics/symtropy/pull/6272)  
**Grouped prediction calibration:** [#6262](https://github.com/Luminous-Dynamics/symtropy/issues/6262) / [PR #6263](https://github.com/Luminous-Dynamics/symtropy/pull/6263)  
**Lifecycle qualification:** [#6195](https://github.com/Luminous-Dynamics/symtropy/issues/6195)

## 1. Research anchor: flexspline-integrated MEMS torque sensing

Shen et al., *Fabrication and calibration of the in-situ MEMS torque sensor for robotic harmonic reducer*, Measurement 286, article 122361 (15 September 2026), DOI https://doi.org/10.1016/j.measurement.2026.122361.

The article reports:
- resistive MEMS sensing grids integrated on the harmonic-drive flexspline, avoiding an external sensor element that would reduce joint torsional stiffness;
- multiple sensitive grids distributed circumferentially around the flexspline;
- periodic wave-generator squeezing as a torque-independent interference/ripple component;
- joint-angle-dependent calibration to compensate residual ripple;
- a BPNN to model nonuniform circumferential deformation;
- in-situ calibration to reduce installation-induced error;
- temperature influence on the torque-sensor output;
- validation using two prototypes.

This is a strong sensing architecture and calibration contribution. It is not yet a broadly independent transfer benchmark across manufacturing lots, installation variation, all operating quadrants, temperature profiles, speed, aging and service. The full source record should be reproduced before claiming a matched paper result.

Source: https://www.sciencedirect.com/science/article/pii/S0263224126020701

## 2. Paradigm shift: calibration is a condition-dependent system model

Do not treat the installed sensor as a strain bridge plus one gain and offset. The observed signal can combine:
- periodic wave-generator/angle ripple;
- circumferential strain nonuniformity and local grid placement;
- temperature-dependent zero/gain/drift;
- torque-direction hysteresis, friction and backlash;
- speed/acceleration-dependent phase and vibration;
- mounting/preload and assembly-specific offsets;
- aging, adhesive/strain-transfer changes and calibration drift.

A compensation map must declare its validated domain. An unseen angle sector, thermal band, torque quadrant, speed range or installation is not automatically covered by a low training residual. Out-of-domain conditions abstain.

## 3. M1 data contract

The plan binds:
- compensation-model digest and feature-schema digest;
- distinct training/calibration/test split digests;
- exact system boundary, sensor-family, external-reference method and condition-domain digests;
- target miscoverage alpha and independent-reference evidence class;
- explicit training installation, sensor, flexspline and reducer identities;
- angle, temperature, speed, direction and bin-edge domain.

Each installation group preserves:
- installation ID and digest;
- sensor serial and assembly digest;
- flexspline and reducer serials;
- sensor calibration digest;
- independent external reference-transducer calibration digest, source, method and raw-trace digest;
- synchronized metrology receipt digest binding the sensor and reference channels to common clock, calibration and system boundary;
- in-situ sensor raw-trace digest;
- all matched trajectory observations.

Each trajectory observation contains:
- unique sample ID;
- joint angle (rad), temperature (°C), angular speed (rad/s) and torque quadrant;
- uncorrected raw-sensor torque interval (N·m);
- compensated sensor torque interval (N·m);
- independently measured external reference torque interval (N·m).

The independent reference evidence is required to be IndependentPhysical. Vendor/model-derived output cannot validate itself.

## 4. Installation-group split and leakage rules

Time-series rows are not independent specimens. A whole sensor/flexspline/reducer installation is the primary calibration/test unit.

This M1 implementation rejects:
- training/calibration/test split digest collisions;
- installation IDs or sensor serials repeated across splits;
- training flexspline/reducer serials appearing in calibration or test;
- flexspline or reducer serial reuse across calibration and test;
- installation-digest and sensor-assembly-digest reuse;
- duplicate sample IDs;
- mismatched model, feature, system-boundary, sensor-family, reference-method or domain digest.

Calibration and test installations must use the same frozen trajectory sample count. Their sample indices are implicit in ordered observation arrays; the domain/mission digest binds the sampling schedule. Variable-length task definitions should be separate frozen cohorts or use a future explicitly designed event/trajectory scoring contract.

## 5. Domain and stratum handling

The plan freezes:
- joint-angle range and angle-sector edges;
- temperature range and temperature-band edges;
- maximum speed and speed-band edges;
- supported motoring, regeneration and static-hold directions.

Every calibration observation must be in-domain and its declared direction must agree with the sign of its reference torque and speed. An out-of-domain calibration sample invalidates that calibration record.

For test data, any out-of-domain sample causes the **entire installation group** to abstain. The evaluator does not drop only the difficult angle/temperature point and report optimistic installation coverage.

In-domain observations are also stratified by:
- angle sector;
- temperature band;
- speed band;
- torque direction/quadrant.

Stratum metrics are descriptive sample-level views. They do not create conditional statistical coverage guarantees for an individual angle, temperature or direction.

## 6. Grouped interval calibration

For each independently held-out calibration installation g, compute the maximum extra interval expansion required to make its compensated torque intervals contain the independent reference intervals:

s_g = max_i max(0, Lhat_i - Lref_i, Uref_i - Uhat_i).

For n independent calibration installations and target miscoverage alpha, the finite-sample rank is:

k = ceil((n + 1) * (1 - alpha)).

If k exceeds n, status is **InsufficientCalibrationInstallations**. No finite correction or coverage claim is issued.

Otherwise q is the kth sorted installation score. Each compensated interval is expanded symmetrically:

[Lcal, Ucal] = [Lhat - q, Uhat + q].

A held-out test installation is Covered only when **every** independent reference interval across that frozen installation trajectory is fully contained by the expanded compensated intervals. This is a marginal group-level split-conformal construction under exchangeability of physical installation groups, a fixed compensation model and frozen condition domain.

This does not establish:
- conditional coverage for each angle/temperature/direction stratum;
- transfer under arbitrary distribution shift or new sensor architectures;
- independence/exchangeability when multiple installations share a correlated production lot;
- traceability or fitness-for-purpose of the reference transducer itself.

Those assumptions require separate evidence and study design.

## 7. Raw versus compensated performance

The report keeps separate:
- raw sensor centre error against the independent external reference;
- compensated sensor centre error;
- raw interval miss;
- compensated interval miss before calibration expansion;
- calibrated interval miss after q expansion;
- calibration correction q and final interval widths.

raw_improvement_nm is positive when the compensated mean centre error is smaller than the raw mean centre error; it can be negative when compensation makes the estimate worse. There is no forced “compensation always helps” score.

The installation-level report also preserves per-group maximum raw, compensated and calibrated interval misses, reference source/raw-trace IDs, sensor calibration digest and synchronized metrology receipt digest.

## 8. Synthetic known-answer fixture

The file examples/in-situ-sensor-transfer-known-answer.json contains:
- three synthetic calibration installations;
- three synthetic held-out installations;
- two trajectory samples per installation;
- calibration installation scores 0.1, 0.3 and 0.5 N·m;
- alpha 0.25, yielding rank 3 and q = 0.5 N·m;
- one covered held-out installation;
- one uncovered held-out installation;
- one OOD held-out installation that abstains as a whole.

All IDs, models, references, certificates, torque intervals, sensor outputs and traces are synthetic placeholders. No physical MEMS sensor, reducer or external torque transducer has been measured.

## 9. Run

Dependency-free crate, Rust 1.96:

\`\`\`bash
cargo fmt --manifest-path benchmarks/synthetic-joints/in-situ-torque-sensor/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/in-situ-torque-sensor/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/in-situ-torque-sensor/v0.1/Cargo.toml --all-targets --locked -- -D warnings
\`\`\`

Dedicated workflow: .github/workflows/in-situ-torque-sensor-transfer.yml. The authoring environment has no local Cargo/rustc/rustfmt; exact-head CI is the compilation and formatting gate.

Twenty-eight Rust test definitions cover grouped installation ranks, whole-installation held-out coverage, OOD abstention, raw-versus-compensated error, insufficient calibration count, split/specimen/sensor/flexspline/reducer/assembly leakage, training-lineage inventory, synchronized metrology receipt presence/propagation, duplicate sample IDs, calibration-domain rejection, direction/sign consistency, independent-reference evidence, all model/boundary/feature/sensor/reference/domain digests, invalid intervals/edges/alpha, frozen trajectory length, stratified reporting, zero in-domain coverage, Wilson bounds, evidence receipts and static/reverse-quadrant handling.

## 10. M0–M5 promotion

- **M0:** reproduce the paper's sensor geometry, calibration pipeline, data, model features and two-prototype result.
- **M1:** schema, grouped split, interval-calibration arithmetic, condition-domain and leakage controls.
- **M2:** compare constant gain/offset, periodic/harmonic angle map, angle+temperature surface, direction-aware map, multi-grid fusion and learned compensation on identical held-out splits.
- **M3:** independently instrumented physical installations with in-situ MEMS sensor plus traceable external torque transducer and common clock.
- **M4:** dynamic robot missions with held-out installations, payloads, temperature profiles, directions and speeds.
- **M5:** aging, fatigue, adhesive/strain transfer drift, service, remounting and post-service requalification.

## 11. Claim ceiling and safety

This M1 crate evaluates externally supplied torque intervals and calibration metadata. It does not fabricate a MEMS sensor, learn a BPNN, verify certificates, calibrate instruments, prove traceability or validate physical torque accuracy. Synthetic fixtures are software tests only. Physical work needs separate engineering review, hazard analysis, traceable reference measurement, guarded rated equipment, competent operators and predeclared stop limits. A low training residual or successful synthetic test is not a safety-relevant torque measurement.
