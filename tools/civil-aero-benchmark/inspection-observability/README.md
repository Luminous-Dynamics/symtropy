# AER-01 inspection observability v0.1

**Tracking:** [AERO-INSPECTION-OBSERVABILITY-001 / #4165](https://github.com/Luminous-Dynamics/symtropy/issues/4165)  
**Composes:** #3921 AERO-AIRFRAME, #4056 AERO-SERVICE-LOOP, #4093 structural/aeroelastic bridge, #3495 applicability domains, #2890 decision-value experiment selection, #3237 MDO interoperability.

## Why this work exists

Two research directions are now actionable:

1. NASA Langley has published a Design for Inspection / Inspectability Metric methodology that puts inspectability into structural multi-objective optimization beside mass and manufacturability. Its automation-friendly simulation rubric tests defect findability and sensitivity to variation while trying to cover the parameter space with fewer simulations. We should consume/compare with this methodology rather than pretend to independently reinvent it:
   - https://ntrs.nasa.gov/citations/20240003953
   - https://ntrs.nasa.gov/citations/20240007593

2. A June 2026 *Structural and Multidisciplinary Optimization* paper proposes a sensing digital twin that separates structural damage from sensor degradation, corrects affected channels and estimates channel reliability:
   - https://link.springer.com/article/10.1007/s00158-026-04324-8

   Its demonstration is a numerical composite wing-panel study. Damage is simulated by local stiffness weakening, which the paper explicitly notes has fidelity limitations against real damage mechanisms. Its classification figures are study-specific simulated results (roughly 88–100% confusion-matrix diagonal); they are not physical aircraft SHM qualification or POD evidence.

## v0.1 contents

- `inspection-observability-corpus-v0.1.json`: 12 synthetic known-answer cases.
- `inspection-observability-corpus-v0.1.schema.json`: JSON Schema draft 2020-12 format for the case contract.
- `Cargo.toml` / `src/main.rs`: Rust-first typed policy evaluator, schema/invariant validation and deterministic result JSON.
- `verify_independent.py`: independent standard-library Python oracle with eight hostile mutations and Rust/Python result comparison.
- `.github/workflows/aero-inspection-observability.yml`: exact-head Rust tests, independent replay, SHA-256 receipts and retained result bundle.

Run:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml > /tmp/inspection-observability-rust.json
python3 tools/civil-aero-benchmark/inspection-observability/verify_independent.py \
  --rust-output /tmp/inspection-observability-rust.json
~~~

The Rust evaluator is the implementation under test. The separate Python standard-library evaluator independently recomputes all twelve decisions and checks the Rust corpus hash, disposition map and claim ceiling. Neither implementation estimates NDE detectability, physical POD, sensor reliability probabilities or aircraft structural safety.

The verifier is deliberately a small state/identity/coverage policy oracle. It confirms that:
- stale geometry and wrong coordinate frames are rejected;
- expired calibration / stale procedure do not count as valid inspections;
- 95% aggregate coverage cannot hide an unobserved required critical region;
- a method outside its declared domain remains unresolved;
- unknown sensor health abstains;
- a plausible damage hypothesis requires independent NDE even if a sensor-correction module has made the reconstructed signal look nominal;
- sensor-fault suspicion cannot be promoted to structural clearance.

This does **not** calculate inspectability physics, NDE detectability, POD, defect severity, fatigue life, structural safety or aircraft acceptability. The method label `synthetic-ultrasonic-placeholder` is not a real inspection procedure. Every case is labelled synthetic.

## Evidence and claim layers

Keep these evidence classes separate:

1. **Synthetic policy fixtures:** contract and fail-closed behavior only.
2. **Simulation evidence:** NDE/SHM forward models and sensor-fault cases, within declared simulation assumptions.
3. **Physical specimen evidence:** actual defect truth, calibrated inspection method, operators/equipment and test uncertainty.
4. **Component/subcomponent evidence:** representative joints/panels/wing-box articles with independent acceptance.
5. **Aircraft evidence:** aircraft-specific inspection/SHM demonstrations under an approved and authority-appropriate programme.

No POD curve or a90/95 value can be fabricated from synthetic fixtures. A physical POD study must use a suitable specimen population, procedure, flaw/material domain, operator/equipment sample and current statistical method. ASTM E2862-23 is a relevant hit/miss POD practice where its scope is appropriate: https://store.astm.org/e2862-23.html. This repository has not implemented an ASTM POD analysis.

SAE ARP6461A is fixed-wing SHM guidance rather than a regulatory approval. The published product information states that authority agreement is needed before SHM is used to fulfil required inspection tasks:
https://saemobilus.sae.org/standards/arp6461a-guidelines-implementation-structural-health-monitoring-fixed-wing-aircraft

South Africa is the intended primary jurisdiction. SACAA's airworthiness page publishes Engineering Unit guidance for type certification, experimental certificates, design-organisation approval and maintenance-review-board processes. The applicable product category, certification basis and maintenance pathway must be confirmed with SACAA and qualified specialists; a research benchmark does not grant approval:
https://www.caa.co.za/industry-information/airworthiness/

## Next increments

1. Compare NASA DFI/Inspectability Metric publications with the fields in this contract. Record accessible inputs, outputs, known limitations and unavailable implementation details. Do not claim a metric replication until a proper benchmark interface and exact method are available.
2. Define at least one physical NDE method / defect / material domain and obtain controlling procedure and specimen evidence before POD analysis.
3. Add sensor-fault model families (bias, drift, gain, saturation, dropout, stuck-at, detachment/bond degradation, timing, EMI and common-mode power/data faults).
4. Add environmental and operating-state confounders, simultaneous sensor + structure faults and out-of-domain held-out cases.
5. Connect the region and sensor identities to #4093 geometry/load/mission state. Feed measured inspection and restoration task times to #4056. Use #2890 to select the next experiment by expected decision value per full cost.
6. Only then compare physical inspection / supplemental SHM architectures with matched mass, power, maintenance and lifecycle boundaries.

The design goal is not to maximize sensor count. It is to find the least costly defensible structure + access + NDE + sensing system that detects relevant damage, distinguishes faults and remains independently inspectable.
