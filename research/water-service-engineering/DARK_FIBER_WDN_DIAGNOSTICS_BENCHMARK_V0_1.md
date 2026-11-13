# DARK-FIBER-WDN-DIAGNOSTICS-001 — Distributed Fibre Sensing Benchmark v0.1

**Tracking issue:** [Symtropy #4991](https://github.com/Luminous-Dynamics/symtropy/issues/4991)  
**Parent benchmark:** [#4963 Pressurized water leak diagnostics](https://github.com/Luminous-Dynamics/symtropy/issues/4963)  
**Dataset crosswalk:** [#4979](https://github.com/Luminous-Dynamics/symtropy/issues/4979)  
**Status:** research contract and synthetic F0 route-observability corpus; no detector, route or fibre deployment qualified.

## 1. Research question

Can existing telecommunications fibre be repurposed as a distributed measurement layer for water-network condition monitoring, so that a single interrogator can observe useful lengths of underground corridor and provide actionable leak localization without installing a point sensor at every candidate leak location?

Start with distributed acoustic sensing (DAS) over dark fibre and compare it with point acoustic/hydrophone/accelerometer methods, pressure and flow sensing, DMA and hydraulic residual methods, dedicated pipe-adjacent fibre, and hybrid combinations.

Do not assume existing fibre follows the water main closely enough, has sufficient mechanical coupling, has dark strands available, or can legally be interrogated. Each is an admission condition to measure.

## 2. State of the art and limitations

Sharif et al., *Urban Water Leakage Detection System over Dark Fiber Networks Based on Distributed Acoustic Sensing and Sparse Autoencoders*, Sensors 2026, 26(7), 2152, 31 March 2026, DOI: https://doi.org/10.3390/s26072152.

The Pamplona field trial used a spare dark fibre in a 17 km municipal telecommunications cable. A 400 mm asbestos-cement main ran in a separate trench from the telecom conduit, at about 1 m lateral separation at the demonstrated location. A controlled drainage valve generated leak events from 0.14 to 3.75 L/s. The paper explicitly says this did not reproduce every hydraulic/mechanical feature of natural failures. Its effective spatial averaging/gauge length was 2.6 m and effective acoustic bandwidth about 1 kHz. It used 300 five-second no-leak calibration acquisitions, then eight controlled leak-flow settings.

The authors compared feature-based statistical metrics and a sparse-autoencoder baseline with a commercial Sewerin Aquaphone A200. The paper reports that the data are available on reasonable request from the corresponding author.

The authors also report site-dependent pipe/soil/conduit coupling, Rayleigh fading variability, intermittent traffic/pedestrian disturbances, persistent 50 Hz transformer harmonics, and that an autoencoder can flag persistent non-leak environmental changes.

Primary source: https://www.mdpi.com/1424-8220/26/7/2152

This is a valuable field feasibility precedent, not a general sensitivity guarantee or broad natural-leak field-validation set. We must not claim that the 17 km fibre continuously monitored 17 km of pipe, nor equate 2.6 m gauge length with P95 localization accuracy.

## 3. Research opportunity

The 9 November 2026 D-A-T systematic review of 70 primary studies assigned none to its D3 distributed strain/temperature/vibration category. This does not mean no D3 work exists: the March 2026 dark-fibre field study is an important example. It means this modality remains underrepresented in the review's primary-study corpus.

Review: https://doi.org/10.3390/w18212797

We should pursue four linked questions:
1. Infrastructure reuse: what fraction of pipe kilometres can existing fibre geometrically cover without new trenching?
2. Physical observability: how often does a leak vibration couple into the fibre under actual route, soil, depth, pipe and operating conditions?
3. Diagnostic value: at equal cost and false-alarm burden, does DAS improve detection, localization or inspection effort beyond point sensors and hydraulic methods?
4. Economics/governance: who controls the fibre, dark-strand rights, interrogator time, route data, security, maintenance and service availability?

## 4. Keep measurands distinct

- DAS / coherent Rayleigh backscatter: dynamic strain/acoustic vibration along fibre. This is the initial target.
- DTS: distributed temperature; a different physical measurand and potentially an indirect thermal leak signature.
- DSS / Brillouin or Rayleigh static strain: slower deformation/strain response, with separate drift and environmental constraints.
- OFDR: potentially high-resolution short-range fibre measurement; not interchangeable with long-range DAS.
- Point acoustic/hydrophone/accelerometer: local sensors with explicit mounting/coupling locations.
- Hydraulic pressure/flow: direct network-state measurements and useful confounder/context channels.

A fibre used only for communications is not a D3 sensor. Do not label DAS output as pressure or leak flow unless a separate calibrated experiment establishes that relationship.

## 5. F0 — route observability before leak detection

F0 answers whether a fibre could plausibly observe a water asset, not whether it detects leaks.

Inputs:
- Pipe asset and segment geometry, chainage, material, diameter, depth and source revision.
- Fibre route geometry, cable/duct identity, strand availability, chainage and mapping uncertainty.
- Separation from pipe with uncertainty, trench/duct configuration, soil/road layers and physical coupling observations.
- Fibre continuity, attenuation, connector/splice state, interrogator access, gauge length, sample rate and usable bandwidth.
- Data/route ownership, permission, cybersecurity and operational constraints.

F0 outputs:
- Geometric candidate coverage: pipe length within an explicitly declared route-screening distance and with valid route mapping.
- Physically sensing-qualified coverage: only pipe length with method-valid evidence that relevant vibrations can couple to the fibre within a declared envelope.
- Observed coverage: pipe length that is both sensing-qualified and currently receiving valid, healthy measurements.
- Unobserved length: missing route mapping, offline fibre, uncertain geometry, unsupported sensor condition or failed coupling qualification.
- Asset attribution ambiguity: where one fibre span lies near multiple water assets and cannot uniquely attribute an event.

These quantities must never be collapsed. A route may be geometrically near a main but have no proven vibration coupling. An online fibre may be physically unable to detect a small leak. A healthy sensor may observe a span without any known event.

### Route-screen semantics

A route-screening distance is a policy input for candidate selection, not a physical sensitivity threshold. For measured fibre/pipe separation d with uncertainty u, compute a conservative interval:

d_low = max(0, d - u); d_high = d + u.

- WITHIN_GEOMETRIC_SCREEN only when d_high is inside the declared screen.
- OUTSIDE_GEOMETRIC_SCREEN only when d_low is beyond the screen.
- INDETERMINATE_GEOMETRY when the interval crosses the screen or route mapping is incomplete.
- Fibre offline, cut or invalid interrogation is UNOBSERVED, never NO_LEAK.
- WITHIN_GEOMETRIC_SCREEN means candidate geometry only, not proof of physical sensitivity.

## 6. F1 — controlled bench and testbed

Compare:
A. DAS over telecom-type cable in a separate duct/trench at multiple offsets.
B. DAS on dedicated pipe-adjacent cable.
C. Fibre bonded to pipe, only where a justified and safe reference configuration exists.
D. Point acoustic/hydrophone/accelerometer sensors.
E. Pressure/flow and hydraulic residuals.
F. Hybrid fusion at equal sensor and communications budget.

Freeze and vary pipe material, diameter, wall condition, joint and defect surrogate; pipe pressure, flow, leak flow and leak geometry; soil/backfill, burial depth, road surface, trench separation and duct configuration; fibre type, installation, splice/bend/connector state and cable coupling; DAS interrogator, gauge length, spatial averaging, repetition rate, bandwidth and calibration; and environmental/operational confounders.

Seed events: no leak; multiple controlled leak types and flow rates; pump starts/stops; valve transitions; changing legitimate demand; pressure transients; traffic; pedestrians; electrical transformers; construction; sensor drift; fibre cut/fade; interrogator reset; clock/timestamp errors; and simultaneous events.

Use independent flow, pressure and leak-location ground truth. Split by complete physical event, hardware/cable build and testbed configuration—not adjacent windows from the same event.

## 7. F2 — controlled field test with utility

Only utility-approved test sections or explicitly authorized controlled field events. Ground-truth leak flow, location, pressure and event timing independently. Compare a commercial point electro-acoustic detector/correlator, available pressure/flow/DMA baseline, DAS threshold/statistical method, DAS sparse-autoencoder or other unsupervised baseline, and hybrid method.

Include normal corridor activity, pump/valve events, transformer harmonics, road works, rainfall/soil moisture, route/fibre outages and varying traffic. A controlled valve leak remains a controlled event, not natural-failure validation.

## 8. F3/F4 — operational retrospective and transfer

F3 uses approved, independently adjudicated natural utility events and confirmed no-leak periods; unlabelled times are UNKNOWN, not negative examples.

F4 holds out entire fibre routes, cities/utilities, pipe materials, seasons, interrogators and route environments. Report each target population separately. No pooled synthetic/lab/field accuracy.

## 9. Baselines and comparison protocol

Reproduce the 2026 Pamplona feature-based and sparse-autoencoder baseline only if raw traces, label metadata, calibration and permission can be obtained. The paper says data are available on reasonable request; until received, mark exact reproduction BLOCKED, not complete.

Start with simple statistical baselines before larger models: per-location robust baseline and shift detection; frequency-band energy/spectral features; matched point-sensor correlation; hydraulic residual and DMA flow/pressure baselines; unsupervised sparse autoencoder; and multimodal fusion.

A larger model only counts as an improvement if it performs better at equal false-alarm rate, sensor/communications budget, compute and maintenance burden.

## 10. Required metrics

Detection:
- Event precision, recall, F1 and PR-AUC by leak mechanism and flow band.
- Detection probability versus leak flow, pressure, fibre offset, pipe depth/material and route coupling.
- False alarms per fibre-km-day, route-week and DMA-month.
- Detection delay/useful lead time.
- Fixed false-alarm and fixed-recall comparisons.

Localization:
- Chainage error P50/P95/max and interval coverage.
- Fraction of confirmed events within 1m/3m/5m/10m only when route survey precision supports the thresholds.
- High-confidence wrong-segment rate.
- Asset attribution ambiguity.
- Unobserved length and localization abstention.

Characterization:
- Leak-flow interval bias/coverage only where independent flow truth exists.
- Explicit separation of acoustic event intensity from inferred leak flow.
- Held-out mechanism/site performance.

Reliability/economics/deployment:
- Route availability, fibre-cut/fade/splice/interrogator fault detection.
- Bandwidth, storage and compute per fibre-km-day.
- Interrogator capital/operation, fibre leasing/rights, access, maintenance and route survey costs.
- Cost per monitored pipe-km, cost per confirmed leak, inspection hours and pipe length searched.
- Independently verified post-repair water-loss reduction.

Gauge length, distance between fibre and pipe, and chainage localization error are different quantities and must never be reported as equivalent.

## 11. Proposed targets (not achieved)

Hard gates:
- No leak absence from missing fibre signal, route gaps, sensor outage or unsupported geometry.
- No automatic valve/pump/repair action from DAS or model-only alert.
- Every result binds to route/fibre/interrogator/configuration revision and uncertainty.
- Controlled lab, controlled field and natural utility events remain distinct.
- Water saved/repair claims require independent hydraulic or water-balance measurements.

Stretch goals:
- Reproduce the 17 km dark-fibre field precedent and 2.6 m gauge-length configuration before claiming superiority.
- At matched false-alarm burden, seek at least 25% lower P95 localization error than point-acoustic/hydraulic baselines on a held-out route.
- At matched recall, seek 25% fewer false alarms per route-week than a simple DAS statistical baseline on a held-out route.
- Define detectable leak-flow envelopes as functions of fibre/pipe offset, burial, material, soil and pressure, rather than claim a universal minimum leak.
- Reduce technician search length or hours per independently confirmed leak without increasing misses.
- Publish Pareto fronts for existing dark-fibre reuse, dedicated fibre and point sensors, including cost, availability and rights.

These are stretch targets for experiments, not predictions or guarantees.

## 12. Data rights and reproducibility

The Pamplona article's data availability statement says data are available on reasonable request from the corresponding author. Open access to the paper does not imply that raw DAS traces, labels, route geometry or code are freely redistributable.

The first admission task is to request the exact dataset and method details. Until raw data, event labels, baseline acquisitions, calibration and code are obtained under permission, keep exact reproduction marked BLOCKED.

## 13. Nonclaims

No Luminous DAS detector, fibre route, sensing sensitivity, leak localization, leak-flow estimate, natural leak detection, municipal deployment, cost saving or water-savings result is qualified by this document. The 2026 Pamplona work is a controlled-field feasibility precedent, not a universal performance guarantee.
