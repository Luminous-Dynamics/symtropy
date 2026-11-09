# AER-01 structural observability co-design oracle v0.1

**Tracking:** [AERO-STRUCTURAL-OBSERVABILITY-001 / #4242](https://github.com/Luminous-Dynamics/symtropy/issues/4242)  
**Related evidence bridge:** [#4093](https://github.com/Luminous-Dynamics/symtropy/issues/4093)  
**Related measurement contract:** [#4236 / PR #4238](https://github.com/Luminous-Dynamics/symtropy/pull/4238)

## Purpose

This crate is a small synthetic known-answer benchmark for sensor-placement observability. It compares D-optimal, E-optimal and one-sensor-dropout-robust E-optimal placements over a two-parameter linear observation model.

It is not a structural solver, finite-element model, aircraft instrumentation recommendation, SHM product, damage-detection classifier, flight sensor system or certification tool. All candidate coordinates, sensitivity vectors, noise values, costs, mass and power are synthetic dimensionless examples.

## Paradigm: co-design the ability to prove the structure

Aircraft architecture should include the cost and capability of observing, inspecting, diagnosing, testing and independently substantiating the physical design.

The target is not "maximize sensor count". It is to find the lowest-cost, maintainable sensor/inspection architecture that makes the required state parameters observable with declared uncertainty under operating and sensor-failure scenarios.

The first small model uses:

\`y = H(x) \\theta + \\epsilon, \\quad \\epsilon\\sim\\mathcal{N}(0,R)\`

For independent Gaussian sensor noise with diagonal \`R\`:

\`F(x)=H(x)^T R^{-1} H(x)\`

where \`F\` is the Fisher information matrix for the declared target parameters.

The current fixture has two normalized synthetic parameters and six candidate sensors. It computes:
- **D-optimal:** maximize \`log det(F)\` (information volume);
- **E-optimal:** maximize the minimum eigenvalue of \`F\` (worst-direction local identifiability);
- **dropout-robust E-optimal:** choose three sensors to maximize the worst minimum eigenvalue after any one selected sensor is lost.

No diagonal regularization is applied to rank-deficient candidates. A singular design remains rank deficient, has no finite log-determinant, and cannot be promoted to a fully observable design.

The information matrix is local and depends on the supplied sensitivity model and noise assumptions. It is not a universal proof of nonlinear/global identifiability or damage detectability.

## Locked synthetic known answers

For the frozen fixture:
- two-sensor D-optimal choice: **S3 + S4**, determinant = 2;
- two-sensor E-optimal choice (lexicographic tie-break): **S1 + S2**, minimum eigenvalue = 1;
- three-sensor one-dropout-robust E-optimal choice (lexicographic tie-break): **S1 + S2 + S4**, worst-case surviving minimum eigenvalue = \`(3 - sqrt(5))/2 ≈ 0.381966\`.

The result also reports selected sensor count, exact sensor IDs and normalized coordinates, Fisher matrix, eigenvalues/rank, deployment cost units, total synthetic mass and total synthetic power. The independent Python implementation recomputes every result from the fixture, rather than importing the Rust implementation.

## Industry and literature baselines

The new work is grounded in existing SHM practice and is not a claim to have invented optimal sensor placement.

- **SAE ARP6461A (2021)** is recommended-practice guidance for implementing SHM on fixed-wing aircraft, including onboard sensor data used to characterize structural health, damage, load and environmental parameters: https://saemobilus.sae.org/standards/arp6461a-guidelines-implementation-structural-health-monitoring-fixed-wing-aircraft
- **NASA Langley SHM laboratory** correlates mechanical tests, NDE, strain fields and analytical models: https://ddtrb.larc.nasa.gov/structural-health-monitoring-laboratory/
- **NASA UAM SHM study** describes heterogeneous sensor types for detecting/localizing impact, fatigue and over-strain events: https://ntrs.nasa.gov/citations/20210022000
- **2025 SHM sensing review** compares deployment suitability, hardware prerequisites, signal characteristics, sensitivity and digital-twin integration across technologies: https://www.mdpi.com/1424-8220/25/5/1424
- **2025 hierarchical sensor placement work** combines modal sensitivity, D-optimality and MAC-based candidate revival, comparing with FIM, effective independence and QR: https://onlinelibrary.wiley.com/doi/10.1155/dsn/9924539
- **2025 optimal placement review** discusses MAC, modal strain/kinetic energy, effective independence and related techniques (primarily infrastructure cases, not direct aircraft validation): https://www.mdpi.com/2224-2708/14/2/22

The next algorithm comparison should include engineering-judgement/uniform placement, greedy modal energy, effective independence, QR-pivot/rank-revealing selection, D-optimal/FIM, E-optimal, MAC-based placement, mutual information/Bayesian selection, robust placement under parameter/environment uncertainty and Symthaea-guided experiment/decision value over the same physical models.

## Acceptance metrics for later tiers

Report separately:
- observability rank and null-space dimension;
- minimum/maximum eigenvalues, singular values and condition number;
- \`log det(F)\` with sensor covariance and parameter scaling declared;
- MAC/mode distinguishability only for admitted mode-shape and sensor models;
- damage/fault detection probability, false-alarm rate, missed detection and localization error on held-out cases;
- robustness under dropout, calibration drift, environment and parameter uncertainty;
- sensor count, mass, power, wiring, storage, bandwidth and lifecycle replacement cost;
- calibration-chain evidence, uncertainty budget and recalibration burden;
- installation/inspection/replacement access and labour;
- total model, test, instrumentation and independent-review cost.

For the first physical application, use a frozen structural model and a finite declared state/fault set. Do not claim aircraft SHM performance until controlling model, material, sensor calibration, installation and physical data are admitted. Robust placement needs multiple failure/environment scenarios; a one-dropout synthetic case is only a software known answer.

## Local execution

From the repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/structural-observability/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civil-aero-benchmark/structural-observability/Cargo.toml
cargo run --locked --quiet --manifest-path tools/civil-aero-benchmark/structural-observability/Cargo.toml > /tmp/aero-observability.json
python3 tools/civil-aero-benchmark/structural-observability/verify_independent.py \
  --rust-output /tmp/aero-observability.json
~~~

The dedicated workflow runs formatting, Rust tests, the Rust CLI, independent Python replay, SHA-256 receipts and evidence-artifact upload.

## Claim ceiling

Positive output claim: \`synthetic_linear_fisher_observability_only\`.

No result from this crate establishes a real aircraft's structural observability, damage-detection accuracy, safety, airworthiness, fatigue life, damage tolerance, continued-airworthiness compliance or certification. The selected positions and sensor types must not be copied into a real aircraft without a controlling model, calibrated hardware, validated measurement operators, appropriate structural substantiation and specialist/authority review.
