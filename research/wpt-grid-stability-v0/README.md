# WPT interconnected grid-stability benchmark v0

**Status:** research specification / draft  
**Parent:** [WPT-GRID-STABILITY-001 #3715](https://github.com/Luminous-Dynamics/symtropy/issues/3715)  
**Claim ceiling:** no hardware validation, stability certification, standards compliance or SOTA superiority is claimed.

## 1. Purpose

Establish a reproducible benchmark for the closed-loop stability of interconnected wireless-power-transfer (WPT) converters and their grid/feeder environment.

The benchmark tests a gap between two distinct statements:

- A passive electromagnetic coupling/impedance model is physically admissible.
- The complete interconnection of active converters, controllers, resonant networks, feeder impedance and loads is dynamically stable.

The first statement does not imply the second. In particular, converter control, sampling delay, current limiting, synchronization loops and negative incremental input impedance can change the stability of an otherwise passive physical network.

This benchmark is intended to connect qualified multiport WPT models to robust dispatch and route-level energy service without allowing an optimizer to assume unverified closed-loop stability.

## 2. Scope and non-goals

### In scope

- Small-signal stability around a declared operating point.
- Multiport impedance/admittance interconnection analysis.
- State-space eigenvalue and damping analysis.
- Generalized Nyquist / return-ratio analysis with explicit sign and pole-count conventions.
- Bounded parameter and grid-impedance uncertainty.
- Independent time-domain cross-checks.
- Topology/controller version binding and fail-closed qualification results.

### Not established by v0

- Large-signal stability for all disturbances.
- Switching-level semiconductor stress or protection adequacy.
- EMC, human exposure or electrical safety compliance.
- Hardware reliability or grid-code certification.
- Stability of models outside their linearization, frequency, topology or uncertainty domain.

A small-signal certificate is one evidence item, not an authorization to energize.

## 3. Evidence identity

Every evaluation episode must bind the same immutable system subject:

| Field | Meaning |
|---|---|
| `schema_version` | Benchmark schema version |
| `case_id` | Unique frozen case identity |
| `system_graph_digest` | Exact interconnection graph and port mapping |
| `operating_point_digest` | Setpoint, load, temperature and steady-state point |
| `model_digest` | Physical plant / multiport model identity |
| `controller_digest` | Firmware/control law and parameter identity |
| `grid_equivalent_digest` | Feeder, transformer and Thevenin-equivalent identity |
| `uncertainty_set_digest` | Parameter ranges, dependencies and sampling method |
| `linearization_digest` | Exact small-signal operating-point linearization |
| `frequency_domain_digest` | Complex impedance/admittance data and frequency grid |
| `reference_digest` | Independent analytical or EMT reference |
| `measurement_plane_ids` | Port definitions, orientation and units |
| `evidence_kind` | ANALYTIC_KNOWN_TRUTH, SIMULATED, PUBLISHED_DATA, or MEASURED |
| `claim_scope` | Declared method, operating envelope and exclusions |

Changing topology, controller, grid impedance, port convention, model, linearization, or uncertainty set invalidates prior qualification unless a separately reviewed equivalence rule proves otherwise.

The benchmark evaluator must not trust a candidate's own `stable=true` field. It recomputes results from the supplied system description and reference evidence.

## 4. Port and sign conventions

Each electrical port must declare:

- positive voltage reference;
- positive current direction;
- passive sign convention or explicit alternative;
- phase reference and synchronization convention;
- RMS or peak amplitude;
- phasor convention (`e^{+j\omega t}` or `e^{-j\omega t}`);
- frequency unit and complex representation;
- power boundary and loss allocation.

Mixed conventions must be rejected rather than silently normalized.

For a declared source/load partition, define the return-ratio matrix explicitly. Do not assume all papers or tools use the same order for source impedance, load impedance, admittance and feedback sign. Store the actual equation and sign convention alongside every generalized Nyquist result.

A generalized Nyquist conclusion must include the open-loop right-half-plane pole count and the encirclement convention. A plot without those details is not an independently reproducible stability result.

## 2026 converter-interaction and dual-frequency comparators

Gebremedhin et al., “Impedance-Based Characterisation of Grid-Forming and Grid-Following Converter Interactions in Multi-VSC Systems,” *IET Renewable Power Generation*, first published 28 July 2026, DOI [10.1049/rpg2.70319](https://doi.org/10.1049/rpg2.70319), is a contemporary method comparator. The open-access study evaluates two- and three-converter weak-grid systems using frequency-dependent impedance models, impedance-ratio (IR), eigenvalue impedance-ratio (EIR), eigenvalue and generalized-Nyquist analyses, with time-domain simulation cross-checks. It studies GFL PQ/PV control, GFM converters and converter placement. The authors also note that GFM does not automatically guarantee stability in every condition.

This is general converter-dominated AC-grid research, not a WPT experiment. Use it to expand matched scenarios, not to import its reported numerical regions as WPT safety thresholds.

Add matched cases for:
- GFL–GFL, GFM–GFL and GFL–GFM–GFL interconnections;
- central versus edge placement of a GFM unit;
- weak/strong grid impedance and remote-converter coupling;
- PQ versus PV GFL outer-loop control;
- PLL bandwidth, filter resonance and virtual-impedance variation;
- IR/EIR diagnostic ranking versus generalized-Nyquist and eigenvalue classification;
- independent time-domain cross-check.

A 2026 experimental study of an electrolytic-capacitorless matrix-converter WPT system also models dual-frequency coupling between a 50 Hz grid-side stage and an 85 kHz resonant stage under non-ideal grid conditions, with harmonic suppression experimentally verified (IEEE Transactions on Power Electronics, DOI [10.1109/TPEL.2025.3616346](https://doi.org/10.1109/TPEL.2025.3616346)).

This motivates a distinct dual-timescale benchmark lane:
- grid voltage harmonics, unbalance and frequency excursions;
- resonant-frequency and modulation-sideband interaction;
- DC-link ripple and capacitor-less energy buffering;
- grid-current THD and individual harmonic/interharmonic magnitudes;
- resonant current/voltage stress and ZVS boundaries;
- phase coupling and harmonic transfer functions.

Use harmonic state-space or another declared linear-periodic model when the operating point is periodically time-varying; a stationary LTI model is not automatically sufficient. Verify frequency-domain predictions against switching/EMT waveforms. Keep grid-current quality, internal resonant stress, stability margin and useful energy as separate scoreboards.

## 5. Reference methods

Implement independent lanes and report disagreement rather than collapsing them:

1. **State-space eigenanalysis:** eigenvalues of the fully interconnected linearized model; report rightmost pole, damping ratio, oscillation frequency and conditioning.
2. **Impedance/admittance analysis:** frequency-dependent multiport data with declared source/load partition and port ordering.
3. **Generalized Nyquist:** determinant/eigenlocus or another mathematically justified MIMO criterion, with pole count, contour and encirclement convention.
4. **Passivity/dissipativity:** supporting sufficient conditions for the exact declared ports; do not treat a subsystem passivity result as a universal interconnection proof.
5. **Independent EMT/time-domain analysis:** disturbances and transient response, with an independent solver/configuration where feasible.
6. **Robust analysis:** evaluate the entire declared uncertainty set or return NOT_EVALUABLE. A finite random Monte Carlo sample is not a proof of robust stability.

A scalar impedance ratio may be reported as a diagnostic, but it must not be promoted to a MIMO stability certificate when significant cross-coupling is present.

## 6. Frozen known-truth corpus

### A. Analytic interconnection controls

Include minimal analytic systems with exact eigenvalues:

- Stable isolated subsystems with a stable interconnection.
- Stable isolated subsystems whose coupling makes the interconnection unstable.
- A near-critical interconnection with a declared stability margin.
- A known unstable open-loop pole case exercising generalized Nyquist pole accounting.
- A non-normal stable matrix with transient amplification, to distinguish asymptotic stability from benign transient behaviour.
- A duplicated or permuted port ordering that must be detected.
- A sign-convention inversion that must be detected.
- A singular/ill-conditioned return-ratio case that must return NOT_EVALUABLE where the method cannot establish a result.

The stable-subsystem / unstable-interconnection case is a logical known-truth control, not a claimed WPT hardware model. Its purpose is to prevent the evaluator from inferring network stability from isolated unit stability.

### B. Converter and resonant-network cases

Progressively add reference cases for:

- LCL-filtered grid interface;
- PLL / grid-following control;
- grid-forming voltage-source control;
- inner current loop and outer DC-link / power loop;
- series and parallel compensation networks;
- receiver-side regulated converter and battery emulator;
- weak-grid and frequency-dependent feeder equivalents;
- two or more converters connected through a common feeder;
- multiple transmitter coils and multiple receivers with full multiport coupling.

Every case must include an operating point and model-validity envelope.

### C. Topology and disturbance cases

- one, two, four, eight and sixteen converter / segment groups;
- shared-feeder parallel operation;
- segment arrival/departure and handover;
- simultaneous and staggered switching;
- receiver load step and departure;
- feeder impedance or grid-strength step;
- source voltage sag and phase/frequency disturbance;
- inverter trip and N-1 isolation;
- current limiting, modulation saturation and anti-windup recovery;
- communication loss for supervisory control, while local fast protection remains independent.

Small-signal and large-disturbance claims remain separate. A small-signal pass does not qualify a large-signal disturbance response.

## 7. Metrics

### Classification correctness

- False-stable cases (critical error; target zero in known-truth corpus).
- False-unstable cases.
- NOT_EVALUABLE / abstention correctness.
- Margin to instability and minimum damping.
- Rightmost eigenvalue and dominant oscillation frequency.
- Generalized Nyquist pole/encirclement accounting.

### Model fidelity

- Magnitude and phase error by port and frequency.
- Dominant mode frequency and damping error against reference.
- Linearization residual and operating-point consistency.
- Time-domain waveform error and peak overshoot.
- Sensitivity to reduced-order truncation.

### Robustness and service impact

- Stable-envelope coverage across declared uncertainty.
- Safe isolation / recovery outcome under each N-1 case.
- Time to declared safe current/voltage region.
- Maximum resonant current, DC-link deviation and thermal-state excursion.
- Energy not delivered and route-service loss during the event.
- Computation time, memory and frequency sweep / EMT solve count.

A weighted score must never permit an unstable, false-stable or hard-limit violation to be compensated by improved efficiency or throughput.

## 8. Proposed acceptance gates

These are programme targets, not standards requirements or achieved results:

| Gate | Initial target |
|---|---|
| Analytic known-truth classification | 100% correct stable / unstable / unresolved classification |
| False-stable known-truth cases | 0 |
| Unsupported model or topology | 100% abstain / NOT_EVALUABLE |
| Robust uncertainty claim | Only when the complete declared set is covered by the chosen method |
| Dominant frequency error | <=5% against the independent reference on admitted cases |
| Damping-ratio absolute error | <=0.02 against the independent reference on admitted cases |
| Hard physical limit excursions in qualified operating envelope | 0 |
| Declared N-1 isolation / fallback cases | 100% correct safe transition in the frozen test suite |

The zero false-stable target applies to the frozen corpus and does not prove zero field risk. Confidence bounds and sample dependence must be reported for empirical campaigns.

## 9. Benchmark competition

Compare candidate methods on identical, frozen cases:

- eigenanalysis of full state-space model;
- scalar impedance-ratio diagnostic;
- MIMO generalized Nyquist;
- passivity/dissipativity sufficient-condition checks;
- structured uncertainty / robust-stability methods;
- structure-preserving reduced-order models;
- switching / EMT reference.

Publish accuracy, conservatism, runtime, memory, supported envelope and abstention rate. The strongest method is not necessarily the one that labels the most cases stable; it is the one that correctly classifies its declared domain and abstains on unsupported cases.

## 10. Integration with Symtropy

- #3530 / #3546: supply physically qualified multiport models.
- #3560 / #3565: supply spatial model and operating-domain qualification.
- #3692 / #3660: supply predictive-calibration and dependence-aware evidence where applicable.
- #3559: consume only stability-qualified operation points and uncertainty envelopes.
- #3590: include feeder/network stability in grid and traffic co-optimization.
- #3587 / #3589: measure the energy-service penalty of conservative operation, isolation and recovery.
- #3641: include failure, maintenance, replacement and downtime in lifecycle economics.

A stability certificate may restrict the dispatch optimizer's feasible set. The optimizer may not edit, broaden or self-approve that certificate.

## 11. Execution sequence

1. Agree and freeze port/sign conventions and the evidence schema.
2. Implement the analytic stable/unstable interconnection corpus and independent eigenvalue oracle.
3. Add MIMO return-ratio and generalized-Nyquist evaluator with explicit pole-count rules.
4. Add two-converter and weak-grid circuit references.
5. Add frequency-dependent impedance model and bounded uncertainty.
6. Cross-check selected cases with EMT simulation.
7. Add topology-change / N-1 service-impact profiles.
8. Connect qualification to dispatch only after the independent evaluator passes its known-truth and negative-control corpus.

## 12. References and standards context

- Yu, Lei, Sun & Dai (2026), “Stability analysis of fractional-order wireless power transfer system with multiple transmitters considering memory characteristics,” *Journal of the Franklin Institute* 363(6), 108555. DOI: [10.1016/j.jfranklin.2026.108555](https://doi.org/10.1016/j.jfranklin.2026.108555).
- Asa, Sujan & Onar (2025), “Stability Analysis of Parallel Connected Bidirectional WPT System,” IEEE/AIAA Transportation Electrification Conference.
- Gebremedhin et al. (2025), “Assessment of interactions of multiple converters and network considering impedance change,” IEEE PES ISGT Europe.
- IEC PAS 61980-5:2024, dynamic off-board MF-WPT equipment, interoperability, safety and EMC: [IEC publication](https://webstore.iec.ch/en/publication/64273).
- IEC PAS 61980-6:2025, dynamic MF-WPT communication and positioning: [IEC publication](https://webstore.iec.ch/en/publication/68093).

These sources motivate the benchmark; they do not themselves validate Symtropy's implementation.

## 13. Safety and claim ceiling

Software, analytical fixtures, published-data reproduction and non-energized/HIL design only. This specification does not authorize high-power experimental builds, live-grid connection, public-road energization, exposure testing or field operation.

No result may be described as hardware-safe, grid-code compliant, standards-certified or superior to industry until independently measured and reviewed evidence supports that exact claim.
