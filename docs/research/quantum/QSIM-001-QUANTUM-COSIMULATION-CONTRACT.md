# QSIM-001: Symtropy physical-context co-simulation contract

**Status:** research proposal and analytic geometry fixture  
**Related Symtropy issue:** [QSIM-001 #2866](https://github.com/Luminous-Dynamics/symtropy/issues/2866)  
**Related optical-model qualification:** [VIS-OPTICS-001 #1807](https://github.com/Luminous-Dynamics/symtropy/issues/1807)  
**Related Symthaea work:** [QARCH-001 #8878 / PR #8881](https://github.com/Luminous-Dynamics/symthaea/pull/8881), [QARCH-002 #8903 / PR #8905](https://github.com/Luminous-Dynamics/symthaea/pull/8905), [QARCH-003 #8925 / PR #8926](https://github.com/Luminous-Dynamics/symthaea/pull/8926)

## Thesis

Use Symtropy as the deterministic physical-scenario and perturbation layer around quantum workloads, not as an unvalidated quantum-state simulator.

Symtropy's N-dimensional geometry, classical rigid-body dynamics, and replay substrate are a natural place to study how physical layout, mount movement, vibration envelopes, source/detector placement, and mechanical tolerances affect a hardware design. Symthaea's quantum-computing models then consume explicit, typed, provenance-bearing outputs. A separately qualified optical model must map geometry to optical path length, loss, coupling, interference, or detector effects.

**A physical geometry simulation does not, by itself, predict photon loss, phase error, coherence, gate fidelity, or quantum advantage.** Those links need equations and calibration specific to the hardware and regime.

## Three-layer architecture

1. **Symtropy — physical context.** Own scene/layout state, coordinate frames, SI-valued geometry, deterministic perturbation scenarios, mechanical dynamics where the chosen model applies, and replay. Output geometry and motion envelopes with a model/version and validity domain.
2. **Qualified physical adapters — cross-domain mapping.** Convert geometry or environmental state into optical or hardware inputs only through a named, versioned mapping. Record units, assumptions, calibration domain, uncertainty, and missing dimensions. Initially prefer analytic optical fixtures from VIS-OPTICS-001; do not add an opaque learned correction.
3. **Symthaea and the quantum backend — quantum protocol and utility.** QARCH-001 models first-order photonic primitive/path envelopes; QARCH-002 models bounded adaptive fusion; QARCH-003 compares end-to-end expected resource against a matched classical baseline. These should consume adapter outputs only when semantics and assumptions match. Actual backend execution and measurement remain separate evidence classes.

The interface is a data contract between layers, not a dependency from a generic physics engine into the quantum-computing crate. Keep default builds lean and avoid coupling Symtropy to a specific quantum SDK.

## Scenario manifest requirements

Every scenario must declare:

- Stable scenario ID and schema/version.
- Exact hardware-layout and workload fingerprints, plus source/configuration digests.
- Coordinate-frame convention, SI units, component IDs, and the frame transformation assumptions.
- Boundary conditions, initial conditions, controlled perturbations, deterministic seed, and selected physics integrator/model version.
- Explicit validity envelope and numerical tolerance; physical measurement uncertainty must not be confused with floating-point tolerance.
- Reference provenance: analytic oracle, independent simulator, calibrated simulation, measured bench data, or external backend observation.
- Which downstream quantities are emitted, which are not modeled, and which adapter transformed each emitted quantity.
- Toolchain/build identity and raw output digest for qualified replays.

Missing or mismatched required inputs are incomplete/invalid, not successful scenarios. Unknown units must not silently default to metres or seconds.

## First fixture

The fixture file docs/research/quantum/fixtures/QSIM-001-straight-line-layout.json describes a tiny analytic point-to-point geometry case:

- A source at (0, 0, 0) m.
- A nominal detector at (0.2, 0, 0) m.
- A controlled detector displacement of +0.001 m along y.
- Expected nominal separation of 0.2 m and displaced Euclidean separation of sqrt(0.2^2 + 0.001^2) m, approximately 0.2000024999843752 m.

This is a geometry-known-answer fixture only. It does not simulate a lens, mirror, wavefront, coupling efficiency, optical transmission, or a quantum protocol. Its expected value can be reproduced independently from the Euclidean-distance formula.

## Benchmark plan

### Tier 0 — contract integrity

- Reject NaN/Inf, non-positive or missing required dimensions, unknown unit labels, duplicate component IDs, malformed frame transforms, missing model versions, and invalid perturbation bounds.
- Reject output-to-adapter mappings with a unit or semantic mismatch.
- Mutate source/configuration digests, expected values, units, and validity metadata; the qualification checker must reject the mutation.
- Verify missing raw outputs or scenario receipts cannot be promoted to pass.

### Tier 1 — analytic fixture corpus

Start with 6–12 small geometry and kinematic known-answer cases. Each reference value is derived independently of the scenario evaluator. Freeze input JSON, expected outputs, tolerance, and fixture digest. Target 100% of analytic fixtures within their predeclared numerical tolerance and zero false accepts across invalid-input/metadata negative controls.

### Tier 2 — controlled perturbation sweeps

Vary one parameter at a time first (placement, angle, displacement, vibration amplitude or frequency where supported); then use a frozen factorial design for interactions. Report response curves and uncertainty. Only assert monotonicity if it is mathematically guaranteed by the declared model; do not generalize a monotonic result from a simple case to a complete optical system.

### Tier 3 — qualified hardware mapping

For each proposed geometry-to-optics mapping, compare predicted optical quantities with independent analytic controls and then bench measurements. Hold out components, operating regimes, or devices when enough data exists. Report per-quantity residuals, uncertainty coverage, invalid-domain behavior, and calibration range. Do not claim physical accuracy before such data exists.

### Tier 4 — workload utility

Feed the qualified parameter envelope into QARCH-001/002, then into QARCH-003. Compare full end-to-end latency, accepted-answer probability, output quality, energy/cost where measurable, setup, retry, readout, and verification against the strongest credible classical baseline on the same workload. A better simulated path or lower modeled photon count is not an application-level quantum advantage.

## Failure modes and negative controls

Include at least these controls:

- Same coordinates with incompatible declared frames must fail until a frame transform is supplied.
- Millimetres interpreted as metres must be rejected, not silently normalized without provenance.
- A geometry model with a missing component must not silently emit a complete route.
- A requested parameter outside the model's validity envelope must return an explicit out-of-domain result.
- An optical adapter with no calibration must not emit a measured transmission probability.
- Changing only a display/rendering parameter must not alter physical outputs unless that parameter is explicitly part of the physical model.
- A simulation-only success must retain its simulated evidence class after being passed into the end-to-end utility evaluator.

## What is deliberately out of scope

- A Hilbert-space state-vector or density-matrix simulator.
- Full wave optics, vector diffraction, polarization, coherent interference, photon statistics, or detector physics absent a dedicated validated solver.
- Device certification, quantum advantage claims, or substitution for hardware measurements.
- A second generic experiment/replay/receipt framework: reuse Symtropy's existing scenario and evidence machinery wherever it already expresses the semantics.
- Heavy quantum SDK dependencies in Symtropy's core packages.

## Why this could matter

The paradigm is a reproducible **digital twin of the physical conditions surrounding a quantum experiment**, coupled to a quantum-protocol model through an explicit calibration boundary. It can help identify which alignment tolerances, path choices, mechanical conditions, and uncertainty sources deserve the next lab measurement. Its first success criterion is predictive validity and useful experiment selection—not an impressive visualization or a simulated claim of advantage.

Industry comparison should be made at the application layer and should interoperate with existing suites such as Open QBench and domain-specific Hamiltonian/optimization benchmarks. Symtropy's intended differentiator is the physical-context and perturbation evidence that connects an abstract workload to a real hardware regime.
