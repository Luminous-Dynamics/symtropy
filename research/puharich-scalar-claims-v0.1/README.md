# Puharich / scalar-longitudinal-wave claim audit v0.1

**Status:** research protocol and source plan; not an experimental result.  
**Parent:** [Power Fabric transport primitives #5832](https://github.com/Luminous-Dynamics/symtropy/issues/5832)  
**Issue:** [PUHARICH-SCALAR-CLAIMS-001 #5878](https://github.com/Luminous-Dynamics/symtropy/issues/5878)  
**Composes with:** [Tesla/Wardenclyffe #3414](https://github.com/Luminous-Dynamics/symtropy/issues/3414), [Tesla Earth-wave audit #3433](https://github.com/Luminous-Dynamics/symtropy/issues/3433), [SPECTRA simulation #3374](https://github.com/Luminous-Dynamics/symtropy/issues/3374), [SPECTRA solver selection #3396](https://github.com/Luminous-Dynamics/symtropy/issues/3396), [WPT empirical envelope #3448](https://github.com/Luminous-Dynamics/symtropy/issues/3448), and [WPT energy accounting #3472](https://github.com/Luminous-Dynamics/symtropy/issues/3472).

## 1. Executive position

Puharich's 1984 ELF-shield proposal is worth examining as a historical and experimentally falsifiable claim. It is not evidence that a distinct scalar-wave energy carrier exists.

This research distinguishes:

- scalar electromagnetic potentials from independently measurable fields;
- longitudinal field components in near-field, material, guided-wave and plasma systems from source-free vacuum radiation;
- field detection or open-circuit voltage from real energy delivered to a useful load;
- a repeatable residual from evidence for a new physical channel.

**No "scalar wave" transport adapter is admitted to Power Fabric v0.1.** The label remains a hypothesis descriptor until its physical mechanism, observables, energy accounting and independent replication meet the gates below.

## 2. Historical source register

Puharich's contribution is titled *ELF-EMF Pollution Shield (Transformation into Scalar Longitudinal Waves)* and is listed in the proceedings of the 1984 Tesla Centennial Symposium.

Source locators:

- Proceedings catalogue: https://teslauniverse.com/nikola-tesla/books/tesla-84-proceedings-tesla-centennial-symposium-light-modern-physics
- Mirrored paper PDF: https://www.edenguard.fr/Docs%20NRJ%20Et%20Cie/PDFs/Andrija%20Puharich_%20ELF-EMF%20Pollution%20Shield%20%28Transformation%20into%20Scalar%20Longitudinal%20Waves%29.pdf
- Related WorldCat record: https://www.worldcat.org/title/tesla-84-proceedings-of-the-tesla-centennial-symposium-in-light-of-modern-physics/oclc/12723745

The proceedings and mirror are source artefacts, not independent replications. Before numerical transcription, obtain the highest-quality lawful scan available and record:

| Field | Requirement |
|---|---|
| Bibliographic identity | Author, exact title, proceedings edition, publisher, date, ISBN |
| Location | Exact page, figure, table, equation and paragraph |
| Provenance | Source URL/catalogue, retrieval date, file digest, mirror relationship |
| Transcription | Manual verification against page image; OCR retained only as a locator |
| Claim type | Proposal, self-reported observation, measurement, calculation, inference or later interpretation |
| Missingness | Unreported apparatus details and unmeasured quantities remain unknown |

Do not backfill missing operating frequencies, power levels, geometry, calibration or uncertainty from later retellings.

## 3. Physics taxonomy

For potentials and fields, use the conventional relations:

```text
E = -grad(phi) - partial(A)/partial(t)
B = curl(A)
```

The scalar and vector potentials are descriptions with gauge freedom. A claim about a new physical mode must identify gauge-invariant observables rather than treating a chosen potential value as a directly measurable energy carrier.

Longitudinal electric components are possible in electrostatic/electroquasistatic near fields, material and guided modes, and plasma oscillations. Their existence does not establish a new freely propagating vacuum mode. In a homogeneous, source-free vacuum, ordinary plane-wave solutions of Maxwell's equations are transverse.

For energy flow, retain the local conservation relation:

```text
partial(u_em)/partial(t) + div(E x H) = -J dot E
```

For a declared control volume and matched time window, account for source real energy, receiver useful energy, losses, auxiliary power and stored-field change. A receiver's open-circuit voltage, a field-probe reading, reactive power or a shielding observation is not a measurement of useful transferred energy.

The models and formulations must be chosen by physical regime. Reuse SPECTRA #3396 for electrostatic, magnetostatic, magneto-quasistatic, conductive-medium and full-wave model selection and convergence; reuse #3374 for shielding/enclosure boundary work. Do not add a second EM solver.

## 4. Competing hypotheses

| ID | Hypothesis | Required disposition |
|---|---|---|
| H0 | Ordinary Maxwellian coupling, grounding, common-mode paths, material response, shielding geometry, propagation or metrology explains the observation | Supported within the tested model/configuration, or unresolved if model inputs are incomplete |
| H1 | Geometry changes field distribution or shielding repeatably, but conventional Maxwellian models explain it | Report ordinary field/coupling effect; do not call it a new carrier |
| H2 | A reproducible residual remains after independent metrology and all applicable Maxwellian explanations are tested | UNRESOLVED; investigate model and instrument gaps |
| H3 | An additional channel is proposed only after H2 survives blinded preregistered repetition and independent laboratory replication | Requires new gauge-invariant observables, quantitative propagation law and source-to-useful-load energy measurement |

H2 is not proof of H3. An unexplained residual may indicate a missing conventional mechanism, poor calibration, an incorrect boundary, hidden coupling or an inadequate model.

## 5. Discrimination model set

The first research pass should be analytical and synthetic. Use distinct model families rather than one all-purpose simulation.

1. **Lumped circuit:** source impedance, resonant network, coil/electrode parasitics, mutual inductance/capacitance, load and common impedance.
2. **Electrostatic / electroquasistatic:** potential distribution, electrode geometry, dielectric boundaries, displacement current and shield response where the approximation is valid.
3. **Magnetostatic / magnetoquasistatic:** inductive coupling, eddy currents, conductive shields and magnetic loss where wavelength and geometry justify the approximation.
4. **Conductive half-space / ground return:** dispersive soil/material properties, electrode contact impedance, current spreading, ground-potential gradients and Joule losses.
5. **Guided / material / plasma modes:** only when a defined medium, boundary and constitutive model exists; distinguish a mode in matter from a new vacuum carrier.
6. **Full-wave radiation:** only when source size, frequency and propagation geometry require it; include radiation, antenna efficiency, boundary truncation and receiver capture.

At least one canonical case should be compared with an independent analytical or numerical reference. Each numerical case must preserve exact material parameters, geometry, boundary conditions, source waveform, ports/probes, mesh, solver version and convergence evidence.

## 6. Synthetic known-truth corpus

The first machine-readable corpus should include one case for each row below, with frozen expected outcomes and explicit uncertainty.

| Case | Injected condition | Expected disposition |
|---|---|---|
| C01 | Ordinary capacitive coupling | Maxwellian explanation |
| C02 | Inductive/common-mode coupling | Maxwellian explanation |
| C03 | Ground-return current and contact loss | Maxwellian explanation |
| C04 | Shield insertion changes field distribution | Report shielding/coupling, not new carrier |
| C05 | Probe orientation or loading changes detected amplitude | Instrument/configuration sensitivity |
| C06 | Open-circuit receiver voltage but no load energy | No useful power-transfer claim |
| C07 | Reactive stored energy released during shutdown | Include stored-energy change |
| C08 | Cable routing or grounding changes observed signal | Common-mode control required |
| C09 | Thermal, acoustic or vibration interference | Reject or isolate confounder |
| C10 | Missing source meter / unsynchronized clocks / overlapping boundaries | NOT_EVALUABLE |
| C11 | Model outside calibration or material-property domain | NOT_EVALUABLE |
| C12 | Residual exceeds declared tolerance but the mechanism is not identified | UNRESOLVED, not H3 |

Negative controls must detect false promotion of potential, near-field longitudinal components, open-circuit voltage, reactive exchange or synthetic anomalies into a new transport channel.

## 7. Future laboratory protocol — design only

Any physical study requires a separately reviewed, low-power, non-human laboratory protocol, calibrated equipment, competent supervision, an approved hazard assessment and independent stop authority. This document does not authorize apparatus construction, energization or human/animal exposure.

A valid future protocol should:

- preregister hypotheses, outcome definitions, exclusions, uncertainty and stopping rules;
- randomize and blind configuration order where practical;
- use independent calibrated source and load measurements on synchronized clocks;
- measure vector E/H fields, frequency/phase, common-mode and return currents, probe orientation and probe loading;
- include transmitter-off, dummy-load, receiver-disconnected, conventional resonator, reversed geometry, cable-reroute and grounding controls;
- record temperature, vibration, acoustic and environmental channels necessary to reject confounders;
- retain raw time series, instrument settings, calibration records, configuration digests and analysis-code digests;
- report shielding effectiveness, detectable signal, and useful load power as separate outcomes;
- close energy accounting without forcing the residual to zero.

No medical shielding, EEG or health-effect claim is in scope. No high-voltage, high-power, intentional earth-current injection, plasma, or outdoor power-beaming experiment is authorized by this protocol.

## 8. Acceptance gates

- All historical claims are source-located and tagged by claim/evidence type.
- Every synthetic case reproduces its frozen expected disposition.
- No unknown quantity is silently imputed as zero.
- No field amplitude, potential, open-circuit voltage or reactive exchange is reported as useful delivered power.
- Any physical residual exceeds combined measurement and model uncertainty, survives preregistered blinded repetition and is reproduced by an independent laboratory before H3 is considered.
- Every energy claim declares source/load meter planes, integrated time window, stored-energy changes, auxiliaries, uncertainty and residual.
- An ordinary Maxwellian explanation closes the specific extraordinary claim for that tested configuration.
- An unexplained result remains UNRESOLVED; it is not marketed as a new carrier, "free energy", superluminal signalling, nonlocal power or medical protection.

## 9. Relationship to Power Fabric

This track is a hypothesis audit, not a new transport modality or ledger.

- Carrier/mechanism/service taxonomy remains owned by #5832.
- Tesla historical model reconstruction remains #3414 / #3433 / #3431.
- Electromagnetic solver selection and shielding models remain #3396 / #3374.
- Empirical wireless-power normalization remains #3448.
- Source-to-load accounting remains #3472.
- Optical transfer remains #3458.
- Underwater acoustic transfer remains #5836.

If a future claim clears its evidence gates, it can be proposed as a new modality through the existing transport contract. Until then it remains outside the deployable energy graph.

## 10. References

- Puharich, A. (1984), *ELF-EMF Pollution Shield (Transformation into Scalar Longitudinal Waves)*, Tesla Centennial Symposium proceedings (source locators above).
- Maxwell's equations and field-energy conservation: use standard Maxwell/Poynting references; record exact edition and page in the source manifest.
- Kühlke, D., critical discussion of Meyl's claimed scalar/longitudinal free-space solution: retain as a critique to evaluate, not as a substitute for our own derivation.
- NIST / ITU-R / peer-reviewed plasma-mode references should be frozen by exact edition, scope and retrieval date before their data enter benchmark fixtures.

---

**Claim ceiling:** source audit, theory, model verification, synthetic evaluation and protocol design only. No scalar-wave existence, physical superiority, safety, certification or field-deployment claim is established.
