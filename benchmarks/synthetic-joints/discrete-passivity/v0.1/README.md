# Discrete-Time Passivity and Energy Budget v0.1

**Status:** M1 scalar analytic reference and synthetic evidence fixture. This package does not implement a complete port-Hamiltonian controller, prove passivity for arbitrary multi-DoF systems, or authorize hardware/human-contact use.

**Research owner:** [DISCRETE-PASSIVITY-ENERGY-BUDGET-001 #6200](https://github.com/Luminous-Dynamics/symtropy/issues/6200)  
**Actuator step semantics:** [#6144](https://github.com/Luminous-Dynamics/symtropy/issues/6144)  
**Regenerative power flow:** [#6185](https://github.com/Luminous-Dynamics/symtropy/issues/6185)  
**J1 impact/SEA:** [#6134](https://github.com/Luminous-Dynamics/symtropy/issues/6134)

## 1. Why this is a separate qualification axis

A continuous-time controller can be passive under stated model assumptions and still lose that property after sample/hold, delay, controller saturation, numerical integration, impedance changes or incorrect work accounting.

A compliant actuator can also improve impact behavior while degrading tracking performance. The engineering target is not “maximum passivity” in isolation; it is **task performance subject to an explicit sampled energy inequality**, with every intervention and tracking cost visible.

## 2. Research anchors

### Series-elastic actuator: performance and passivity together

Mehta and Losey, *Combining Performance and Passivity in Linear Control of Series Elastic Actuators*, arXiv:2509.17210 (21 September 2025; ICRA 2026 contribution): https://arxiv.org/abs/2509.17210

The work compares SEA mechanical/control configurations across rendered compliance, coupled stability/passivity and tracking performance. It reports advantages for actuator-side PD control and damping in the elastic transmission, with theoretical analysis, simulation and real-world experiments. This is a source-specific design finding, not a universal controller guarantee.

### Port-Hamiltonian force-impedance control

Ma et al., *An Energy-Based Approach to the Force-Impedance Control Problem for Robot Manipulators*, IEEE Transactions on Control Systems Technology 34(1), 123–138 (2026), DOI https://doi.org/10.1109/TCST.2025.3606519.

The work uses port-Hamiltonian structure, energy shaping of inertia/stiffness, integral force action and damping injection to address force-impedance control. It motivates a port-based representation of joint/environment interactions rather than a disconnected list of gains and torque limits.

### Digital passivity

Mattioni and Borja, *Digital passivity-based control of underactuated mechanical systems*, Automatica 173 (March 2025), article 112096, DOI https://doi.org/10.1016/j.automatica.2024.112096.

This sampled-data work addresses how digital implementation can degrade continuous-time passivity and potentially lead to instability. It motivates timestep, sample rate, zero-order hold and delay tests as first-class benchmark conditions.

### Energy tank source: dimensional audit

Wadawa et al., *A Port-Hamiltonian Framework for Passive Analysis and Energy-Safe Control of Coupled Motor-Robot-Environment Systems*, Acta Mechanica et Automatica 20(3), 738–754 (25 September 2026), DOI https://doi.org/10.65731/ama/2026-0072.

The paper describes energy-tank sizing using maximum gravitational torque `tau_g,max = m g l`. Torque is measured in N·m; tank energy is measured in joules. A maximum torque is not an energy capacity unless an angular displacement/work integral is also specified. The source may use this quantity as a torque bound; Symtropy must not ambiguously call it stored energy.

A dimensionally coherent path-specific gravitational work bound is:

```text
Delta_V = integral(tau_g(q) dq)
```

The tank's state is a separate quantity in joules. Maximum torque, maximum power and maximum transferable energy are different limits.

## 3. Sampled-data energy contract

For a scalar joint mechanical port, choose effort/flow signs and integrate:

```text
P_port(t) = tau(t) * omega(t)
W_port[k] = integral from t_k to t_(k+1) of P_port(t) dt
```

For a declared stored-energy function `H(x)`, the sampled passivity inequality is:

```text
H[k+1] - H[k] <= W_port[k] + epsilon[k]
```

`epsilon` must be justified by measurement/numerical uncertainty and frozen before evaluation. Do not treat it as an arbitrary balancing term. A positive deficit is:

```text
deficit[k] = max(0, Delta_H[k] - W_port[k] - epsilon[k])
```

The record must state exactly what is inside `H` and which mechanical, electrical, storage, environment and dissipation ports cross the control-volume boundary.

## 4. Energy-tank semantics

This M1 reference uses a joule-valued virtual energy tank:
- initial, minimum and maximum energy are explicit;
- positive mechanical work delivered to the plant is debited from the tank;
- negative mechanical work replenishes the tank only through an explicit capture efficiency/path;
- capacity overflow is reported as spilled energy rather than silently stored;
- an injection that would underflow the tank is rejected;
- actuator torque saturation is separate from tank-energy scaling.

Negative mechanical work is not automatically electrical regeneration. The capture efficiency must represent an actual architecture/path or remain zero/unknown. The DC-bus/battery path is separately characterized under [#6185](https://github.com/Luminous-Dynamics/symtropy/issues/6185).

## 5. Passivity controller correction

The scalar correction primitive adds damping effort opposing the current flow. Its correction power is required to be non-positive:

```text
tau_damp * omega <= 0
```

The routine reports energy dissipated by the correction and any remaining deficit when torque limits prevent the full correction. It does not silently claim a passivity guarantee if the controller lacks torque authority.

The actual performance cost—tracking error, task-time, force margin and intervention frequency—must be reported alongside any reduction in passivity deficit.

## 6. Torque is not tank energy

For a declared maximum gravity torque and angular travel:

```text
W_bound = tau_max * abs(delta_q)
```

with torque in N·m and angle in radians, the result is joules. For a varying torque, integrate the torque-angle path. Reversals contribute signed work.

The Rust reference includes:
- a trapezoidal sampled effort-flow work integral;
- sampled storage/passivity deficit evaluation;
- energy-tank debit/credit, min/max and explicit capture efficiency;
- tank-limited effort scaling;
- a dissipative passivity correction with residual reporting;
- generalized torque-angle work and a max-torque/angular-travel upper bound;
- linear torsional spring stored energy and damper dissipation.

This is a scalar rotational reference only. It does not supply the full multiport, multi-axis, delayed sampled-data proof or controller synthesis.

## 7. Synthetic fixture and known-answer tests

`examples/sampled-passivity-known-answer.json` contains three synthetic intervals:
1. positive mechanical work debits the tank;
2. negative mechanical work is captured only at the declared 0.5 efficiency;
3. the effort command is scaled to the remaining tank budget and the tank reaches (but does not cross) its minimum.

Fourteen Rust test definitions cover signed port work, linear sample subdivision, invalid timestamps, storage/passivity inequality, tank debit/credit/overflow/underflow, tank-limited effort, negative work not being clipped as positive injection, dissipative correction, zero-flow deficit, torque-versus-energy dimensionality, direction reversal, spring/damper known answers and invalid domains.

These are synthetic software fixtures, not hardware or HRI experiments.

## 8. Run the analytic reference

Dependency-free crate; Rust 1.96:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/discrete-passivity/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/discrete-passivity/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/discrete-passivity/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

The dedicated workflow is `.github/workflows/discrete-passivity-energy-budget.yml`. The current authoring environment has no local Cargo/rustc/rustfmt; exact-head CI is required before any test/format/Clippy pass can be claimed.

## 9. Prospective qualification goals

- Zero unit inconsistencies between torque, power and energy.
- Zero false-pass passivity outcomes in deterministic/adversarial reference cases.
- Port work and stored-energy change are checked every step, not only at the end of a mission.
- Energy-balance residuals remain within predeclared instrument/numerical uncertainty.
- Energy-tank state remains within explicit J-valued bounds; every debit/credit source is traceable.
- Timestep/sample-rate changes preserve accounting within convergence tolerance.
- Passivity interventions, unremoved deficits and task-performance cost are reported together.
- Held-out contact/delay/variable-stiffness trajectories validate any claimed controller envelope.

These are prospective goals, not achieved Symtropy results.

## 10. Claim ceiling and safety

The crate is a scalar analytic/accounting reference. It does not prove general passivity, guarantee stability under arbitrary delays, certify SEA safety, qualify human-robot interaction or model a real battery/DC bus.

No fabrication, energization, pressure-system work, human-contact experiment or high-energy impact test is authorized by this software reference. Physical evaluation requires separate requirements, hazard analysis, competent procedures, guarded equipment, rated protections and independently calibrated instrumentation.
