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

### Multi-transmitter cross-interference control

Ishihara et al., **Multiple Transmitters with Controlled Amplitude and Phase of Transmitter Currents to Eliminate Cross-Interference Effects for Large-Area Wireless Power Transfer Systems**, IEEJ Journal of Industry Applications 15(3), 385–396 (published 1 May 2026). DOI: https://doi.org/10.1541/ieejjia.20250352

The paper identifies coupling between multiple transmitters and receivers as a cause of transmitter-current amplitude/phase changes, receiver output-power degradation, stronger magnetic fields, poorer power factor and possible loss of inverter ZVS. The authors experimentally evaluate DC–DC input-voltage control to hold transmitter-current amplitude and a switching network to cancel unwanted induced EMF / self-inductance variation.

**Engineering implication:** our multi-coil state needs both static matrix admissibility and dynamic operating-state tests. A passive, reciprocal inductance matrix does not prove that a controller maintains benign current amplitude/phase, ZVS, field or thermal behavior during multi-transmitter activation.

### Three simultaneous receivers in a near-field RF metasurface

**A Multimode Near-Field Radio Frequency Metasurface Resonator Transmitter for Multireceiver Power Transfer at 6.78 MHz**, IEEE Transactions on Microwave Theory and Techniques, online 9 September 2026. DOI: https://doi.org/10.1109/TMTT.2026.3729012

The reported fabricated 335 × 335 mm aperture has a 9-position scan and a three-simultaneous-receiver evaluation. The source reports total efficiency of 77.89–80.30% across the three-receiver layouts and branch imbalance of 14.88–24.71 percentage points. Its RF metasurface topology and 6.78 MHz operating regime are not directly comparable to 85 kHz EV IPT. Its relevance is that aggregate efficiency can obscure receiver-to-receiver imbalance, and a multiport resonator's collective modes need to be analysed rather than treating each receiver as an isolated link.

**Engineering implication:** keep aggregate efficiency, per-receiver efficiency, delivered energy, and fairness separate. A matrix-valid model should be followed by a network-level performance test.

### End-to-end MIMO DC-to-DC optimization is already a 2026 SOTA baseline

Lee et al., **An End-to-End DC-to-DC Efficiency Optimization and Design Analysis for MIMO Wireless Power Transfer Systems**, IEEE Transactions on Microwave Theory and Techniques 74(9), 8545–8562, published 1 September 2026. DOI: https://doi.org/10.1109/TMTT.2026.3714844.

The authors use measurement-based power-amplifier and rectifier models in a semidefinite-programming formulation that maximizes DC-to-DC efficiency under a transmit DC power budget. Successive convex approximation recovers a feasible rank-one excitation vector with a reported recovery ratio above 97% across tested conditions. The 5.64 GHz simulation uses a 12×12 TX array and 8×8 receive rectenna array at 0.5 m; it reports 31% and 48% higher end-to-end efficiency than RF-to-DC and RF-to-RF optimization formulations. A fabricated 16×16 TX array and 5×5 rectenna provide experimental validation over distance and azimuth sweeps.

**Competitive implication:** we must not present a generic genetic algorithm or the inductance passivity gate as beating this optimizer. Our next digital-twin benchmark should compare against (a) unconstrained pairwise-inductance modelling, (b) the published SDP/SCA class of optimizer where its RF architecture applies, and (c) a robust/uncertainty-aware constrained formulation. Evaluate achieved useful DC output, source energy, constraint violations, rank-one feasibility/recovery, runtime and sensitivity to model error. This microwave MIMO result is not directly comparable to 85 kHz EV IPT and must remain a separate transfer lane.

### Modular dynamic transmitter tracks

Zeng et al., **Design and Implementation of Modular Transmitter Tracks for Dynamic Wireless Power Transfer Systems With Stable Coupling and Outputs**, IET Power Electronics 19(1), e70189 (first published 26 February 2026). DOI: https://doi.org/10.1049/pel2.70189.

The proposed partially overlapping rectangular transmitter coils use LCC compensation and support straight, curved, ascending and descending paths. Simulation and experiment report coupling-coefficient deviations below 4.99% and output-current deviations below 6.3% across tested trajectories.

**Competitive implication:** segmented-track design is no longer just a simulation opportunity. Our benchmark should record geometry continuity and actual receiver trajectories, then compare coupling drift, output ripple, handover loss, dead-zone energy, segment switching, construction complexity and material cost. The abstract-level stability figures are not efficiency or availability figures.

### Coil packaging as a Pareto trade-off, not an efficiency-only problem

A 2026 ORNL-associated study, **Stackless Onboard Coil for Dynamic Wireless Power Transfer**, IEEE Open Journal of Power Electronics 7, 45–58. DOI: https://doi.org/10.1109/OJPEL.2025.3634599.

A 1.2 kW prototype at a 220 mm air gap uses a four-segment, single-layer composite magnetic structure. The paper reports a 35% reduction in onboard coil thickness, 13% increase in average output power, 1.8 percentage-point reduction in average efficiency, and 21.5 percentage-point improvement in power fluctuation relative to its DDQ reference.

**Competitive implication:** this is a good example of why a single efficiency leaderboard misleads. The benchmark must compare delivered energy, power stability, thickness, mass, manufacturability, thermal stress and vehicle integration on a Pareto front. An efficiency regression may be rational if it buys substantial packaging and service improvements—but the trade must be explicit and measured.

### Transient multi-transmitter dynamics

A 2026 IEEE Transactions on Power Electronics study, **Parametric Study of Transient Dynamics of Multitransmitter Dynamic Wireless Power Transfer for Linear Transport Systems**, 41(1), 1164–1179. DOI: https://doi.org/10.1109/TPEL.2025.3596911.

The study develops a transient multi-coupling model for segmented transmitters and moving receivers, then reports experimental agreement while analyzing motion speed, filter capacitance, acceleration, input voltage and load resistance effects on inverter current and output voltage fluctuations.

**Competitive implication:** static mutual-inductance admissibility is a precondition, not a transient model. A later benchmark must test time-varying coupling matrices and controller response across segment handover and acceleration profiles.


### Dual-receiver dynamic WPT with observer and MPC

**Output Control Method for Dual-Receiver Dynamic WPT System Based on Optimal Efficiency and Sliding Mode Observer-Enhanced Model Predictive Control**, IEEE Journal of Emerging and Selected Topics in Power Electronics 14(3), 4130–4142 (June 2026; published 13 January 2026). DOI: https://doi.org/10.1109/JESTPE.2026.3653770.

The paper addresses receiver-side cross-coupling using an overlapping decoupled receiver structure, a sliding-mode observer and model-predictive control. Its 360 W experimental platform reports output-voltage fluctuation no greater than 2.5% and 90.4% optimal efficiency.

**Competitive implication:** this is a direct dynamic dual-receiver IPT reference for transient output regulation and observer-based control. It is not a large-scale multi-receiver benchmark and the abstract does not make its efficiency boundary fully comparable to an EV grid-to-battery result. Our reproduction must report converter boundaries, per-channel output, transient overshoot, dropout/reattachment, and the same trajectory/load profile.

### Parallel multi-receiver logistics WPT with soft switching

Zeng et al., **Design and Implementation of a High-Efficiency Multi-Receiver WPT System With Soft-Switching**, International Journal of Circuit Theory and Applications (first published 15 September 2025). DOI: https://doi.org/10.1002/cta.70144.

The logistics-sorting study designs parallel multi-receiver power delivery and derives soft-switching conditions. It reports a 1.2 kW experimental platform with inverter soft switching and 91.9% overall system efficiency.

**Competitive implication:** a relevant kW-scale multi-receiver logistics baseline. To claim improvement, compare at matched delivered power, coil/air-gap geometry, receiver count, converter inclusion and measurement boundary. We should target higher useful delivered energy and lower loss under multi-load variation, not claim superiority from comparing unlike efficiency figures.

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
