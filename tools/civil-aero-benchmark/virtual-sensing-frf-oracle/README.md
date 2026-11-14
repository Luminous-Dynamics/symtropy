# AER-01 virtual-sensing analytical FRF oracle v0.1

**Tracking:** [AERO-VS-FRF-ORACLE-001 / #4177](https://github.com/Luminous-Dynamics/symtropy/issues/4177)  
**Parent:** [AERO-VIRTUAL-SENSING-ROBUST-001 / #4176](https://github.com/Luminous-Dynamics/symtropy/issues/4176)  
**Status:** synthetic analytical known-answer substrate; not an FEM solver, flight model or aircraft SHM qualification.

## Purpose

This small Rust-first executable creates a frozen, exact-answer substrate for frequency-domain virtual sensing. A separate Python standard-library oracle independently solves the same 2-DOF complex dynamic-stiffness system and sensor observation transforms. Future virtual-sensing methods can be compared against the physical-reference response without treating a reconstructed signal as truth.

The system is intentionally simple:
`D(omega) x(omega) = F(omega)`
`D(omega) = K - omega^2 M + i omega C`
`x(omega) = D(omega)^{-1} F(omega)`

The frozen convention is `exp(+i omega t)`; frequency inputs are Hz and angular frequency is `omega = 2 pi f`. Inputs are SI: kg, N/m, N s/m, N and m/N.

## Nine frozen scenarios

- HH nominal structure / healthy sensors.
- HF gain and phase error.
- HF additive sensor bias.
- HF dropout (missing channel represented as JSON null, never zero).
- HF stuck-at sensor.
- DH stiffness change with healthy sensors.
- DF stiffness change plus bias and gain/phase faults.
- HH sensors on a physical system whose mass differs from the nominal model.
- HF duplicate sensor locations, leaving the 2-DOF response unidentifiable from the observation map.

Each case runs on nine frequencies from 2 to 30 Hz. The corpus stores expected nominal and physical complex responses plus raw sensor observations.

## Run

From the repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/virtual-sensing-frf-oracle/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/virtual-sensing-frf-oracle/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/virtual-sensing-frf-oracle/Cargo.toml > /tmp/frf-rust-result.json
python3 tools/civil-aero-benchmark/virtual-sensing-frf-oracle/verify_independent.py --rust-output /tmp/frf-rust-result.json
~~~

The CI workflow also retains the corpus, Rust result, Python audit and SHA-256 receipts.

## What is verified

- 2x2 complex dynamic-stiffness solution against frozen known answers;
- nominal and physical model responses are distinct outputs;
- sensor gain, phase, bias, dropout and stuck-at transforms;
- explicit observation-map rank;
- positive mass, positive-definite stiffness, positive-semidefinite damping;
- Hz versus rad/s convention and positive frequency;
- complex real/imaginary response and wrap-aware phase agreement;
- missing channels remain missing and cannot be silently replaced by zero;
- singular systems, invalid matrices, bad sensor DOFs, duplicate IDs and changed frequency schedules reject.

## Claim ceiling

The only positive claim is `analytical_2dof_frequency_response_and_sensor_mapping_only`. The nine cases are synthetic and do not establish material behaviour, a real wing or spacecraft response, SHM performance, POD, damage detectability, sensor field reliability, maintenance release, aircraft safety, certification or flight authority.

A low error on this oracle is not a virtual-sensing SOTA result. The next stage must add a higher-order structural benchmark and matched baselines (spatial interpolation, modal interpolation, POD/ROM, observer/Kalman, physics-informed constrained update and reproducible frequency-attention methods), then test held-out configurations, frequency bands and physical specimen data.
