# LOCO-ENTROPY-ACCOUNT-001 — analytical reference kernel

**Status:** isolated analytical Rust reference; no physical model or robot qualification.

This crate implements one deliberately narrow process:

\[
S_{\mathrm{gen}} = Q\left(\frac{1}{T_c}-\frac{1}{T_h}\right), \qquad T_h \geq T_c > 0
\]

for an idealized heat magnitude \(Q\) transferred from a fixed-temperature hot reservoir to a fixed-temperature cold reservoir.

When a fixed environment reference temperature \(T_0\) is explicitly supplied:

\[
X_{\mathrm{destroyed}} = T_0 S_{\mathrm{gen}}
\]

The API uses newtypes for joules, kelvin, J/K and exergy joules. It rejects non-finite values, negative heat magnitudes, non-positive absolute temperatures, reversed/unproven transfer direction, interval overlap and incomplete heat-port identifier coverage. Interval propagation requires the entire hot-temperature interval to be at least as high as the entire cold-temperature interval.

## Run

\`\`\`bash
cargo fmt --manifest-path research/loco-entropy-account-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/loco-entropy-account-001/Cargo.toml
cargo clippy --locked --manifest-path research/loco-entropy-account-001/Cargo.toml --all-targets -- -D warnings
\`\`\`

## What this does not establish

- It is not an open-system entropy-balance engine for a complete robot.
- It does not model finite-capacity thermal bodies, time-varying temperatures, heat-port networks, mass flow, battery chemistry, friction, electronics, convection or radiation.
- It does not estimate entropy production from motor temperature rise or battery consumption.
- It does not authenticate instruments, calibrate sensors, validate timestamps or establish boundary completeness beyond a structural identifier-set check.
- It does not infer physical entropy from Shannon entropy, FEP, policy uncertainty or a Markov path-ratio score.
- It does not establish that entropy/exergy optimization improves mission performance.
- It does not authorize actuation or replace Symtropy #4610 thermal dynamics, #4044 energy evidence, #4179 actuator loss maps or #4588 future-option evaluation.

The analytical kernel is a reference control. Its exact tests qualify only this equation under these assumptions. Whole-robot entropy accounting requires a complete control-volume boundary, source-qualified heat/work/flow data, calibrated absolute temperatures, uncertainty propagation and independent validation.