# Multi-coil WPT related work and industry gap (2026-11-04)

This note is a source-grounded research map for the passivity/reciprocity validator. The citations are research leads and standards-scope references; they are not measurements made by Symtropy.

## Recent experimental reference points

### Range-adaptive three-coil IPT

**General Design Methodology of Range-Adaptive Three-Coil WPT Systems Based on Orthogonal-Stratified Latin Hypercube Sampling**, IEEE Transactions on Transportation Electrification, vol. 12, no. 5, October 2026, pp. 7693–7707.  
Publisher record: https://ieeexplore.ieee.org/abstract/document/11534348

The abstract describes 2-primary/1-secondary and 1-primary/2-secondary three-coil configurations. It reports a hybrid orthogonal-experiment / stratified balanced Latin-hypercube design method that reduced multi-objective optimization time by 86.4% relative to global scanning. A 1 kW prototype reported output-current variation below 10.6% over a 7.5× load range and below 4.1% as coupling varied from 0.26 to 0.06; zero-phase-angle operation was obtained over the reported coupling/load range.

**What it establishes:** three-coil topology and experiment-design optimization have already moved beyond isolated two-coil pads, with physical prototype validation.

**What it does not establish in the indexed abstract:** complete end-to-end efficiency, a multi-receiver energy ledger, raw inductance matrices, matrix uncertainty, or global reciprocity/passivity certification. Do not rank its output-current-stability results against an EV charging efficiency result.

### Three-receiver mutual-inductance fault detection

Kim et al., **Fault Detection of Coil Misalignment in Multi-Receiver Wireless Power Transfer Systems With the Estimated Mutual Inductance**, IEEE Transactions on Industrial Electronics 73(9), 12967–12979 (published 1 September 2026). DOI: https://doi.org/10.1109/TIE.2026.3675172

The paper estimates TX–RX mutual inductance and equivalent load resistance by sequentially activating receivers, then estimates RX–RX mutual inductance by activating receiver pairs. A three-receiver prototype with 10 mm air gap is reported as validation for mutual-inductance estimation and misalignment-fault detection.

**What it establishes:** RX–RX coupling is operationally important, and pairwise receiver interaction can support fault detection.

**What it does not establish in the accessible abstract:** EV-scale power, efficiency, uncertainty distributions, full inductance-matrix PSD, or large-N scaling.

### Wireless energy internet

Li et al., **Wireless energy conversion in wireless energy internet**, Nature Reviews Electrical Engineering 3, 274–287 (published 2 April 2026). DOI: https://doi.org/10.1038/s44287-026-00279-6

The review discusses energy delivery to non-electrical loads, primary-controlled architectures, multi-frequency compensation, simultaneous wireless power/data transfer, and the need for standardized energy and information interfaces.

**Engineering implication:** our long-term architecture should represent the service endpoint, not assume every receiver ultimately charges a battery. Direct mechanical, thermal or other energy conversion requires its own endpoint and energy accounting.

## Standards and interoperability boundaries

### SAE J2954_202408

SAE's current J2954_202408 scope is stationary, light-duty WPT, forward grid-to-vehicle transfer, with three charging levels up to 11 kVA. The edition covers surface-mounted installations; flush-mounted installations are deferred. Its interoperability approach uses a standardized test station and reference devices. Its page says bidirectional requirements are intended for a future revision, not that they are already normative in this edition.

Source: https://saemobilus.sae.org/standards/j2954_202408-wireless-power-transfer-light-duty-plug-electric-vehicles-alignment-methodology

### ISO/PAS 5474-6:2025

The heavy-duty vehicle-side PAS covers static and dynamic MF-WPT vehicle equipment and interoperability, but explicitly excludes multiple secondary devices in its current edition.

Source: https://www.iso.org/standard/84305.html  
UK catalogue scope: https://knowledge.bsigroup.com/products/electrically-propelled-road-vehicles-functional-and-safety-requirements-for-power-transfer-between-vehicle-and-external-electric-circuit-safety-and-interoperability-requirements-for-heavy-duty-vehicles-in-magnetic-field-wireless-power-1

**Standards gap to investigate:** multiple secondary devices / modular arrangements at higher power classes, particularly where different receiver types, frequency bands, active channel combinations and directional power flows share infrastructure. This is a research gap inferred from the stated scope exclusions, not a claim that no other standard or future work exists.

## What Symtropy should add beyond these references

1. **Global physical realizability:** validate the complete real inductance matrix, not just each TX–RX or RX–RX pair. Pairwise `|k_ij| <= 1` is necessary but insufficient for three or more coils.
2. **Uncertainty-aware qualification:** preserve entrywise calibration/model bounds and return UNRESOLVED when the smallest-eigenvalue interval overlaps the PSD boundary.
3. **Geometry-conditioned matrix atlas:** attach each matrix to coil identities, polarity convention, geometry, gap, offset, frequency, temperature, load and bias. Do not interpolate across unqualified operating regions.
4. **Online-vs-reference reconciliation:** compare the self-sensing estimator against calibrated LCR/impedance-analyser or validated field-model references, parameter by parameter.
5. **System-level consequences:** feed only qualified matrices into the circuit/digital twin, then evaluate cross-regulation, inactive-channel currents, ZVS, thermal stress, energy closure and service fairness.
6. **Independent replication:** repeat selected matrix extraction and full-system tests across labs with identical fixtures and frozen reference artifacts.

## Claim ceiling

The new validator is not a replacement for FEM, PEEC, circuit simulation, a Maxwell solver, laboratory measurement, or standards certification. Passing its synthetic fixture corpus establishes only the evaluator's agreement with those known-truth fixtures once the software tests have actually run. A physical qualification still requires traceable source data, a validated measurement boundary, independent field/circuit comparisons, uncertainty, and a complete test-run receipt.
