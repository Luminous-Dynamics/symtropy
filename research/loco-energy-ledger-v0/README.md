# LOCO Energy Ledger V0

Standalone dependency-free Rust reference oracle for LOCO-ENERGY-001 / #4044.

## Scope

V0 performs auditable energy arithmetic for independently metered power boundaries. It does not estimate unmeasured battery, actuator, motor or mechanical energy.

The sign convention is explicit: signed battery current is positive when discharging (power leaves the battery) and negative when energy is returned to the battery.

For sampled voltage and current:

    p_batt(t) = V_batt(t) * I_batt(t)

V0 linearly interpolates signed power between valid samples and integrates each segment exactly, splitting positive and negative triangular areas at any zero crossing:

    E_discharge = integral max(p_batt(t), 0) dt
    E_returned  = integral max(-p_batt(t), 0) dt
    E_net       = E_discharge - E_returned

Power is in watts, time in seconds, energy in joules. Wh = J / 3600.

## Boundaries are separate

Each EnergyChannelReceipt identifies exactly one boundary:
- battery terminal;
- DC bus;
- actuator electrical;
- auxiliary electrical.

Each stream produces its own EnergyLedger, which retains its run, channel, boundary, calibration, clock and timebase-receipt identity. CoT rejects an energy ledger whose run ID differs from the mass/distance receipt. The package intentionally has no function that adds nested battery, bus, motor and auxiliary streams into an invented total: those may be sub-boundaries of the battery meter and double counting is prohibited.

Mechanical work is computed independently from calibrated joint torque and angular velocity. The joint-work ledger preserves the run, joint, torque/velocity calibration, clock and timebase identities:

    p_joint(t) = tau(t) * omega(t)
    W_positive = integral max(p_joint(t), 0) dt
    W_negative = integral max(-p_joint(t), 0) dt

Negative mechanical work is not treated as electrical regeneration.

## Cost of transport

For a declared battery-terminal energy ledger:

    CoT_gross = E_discharge / (m * g * d)
    CoT_net   = (E_discharge - E_returned) / (m * g * d)

Both values are dimensionless. V0 requires positive measured path distance, positive configured robot mass and positive gravity. It refuses to create a finite CoT when net energy is non-positive or when the ledger is not battery-terminal. Horizontal displacement, route elevation and commanded distance are not substitutes for measured path distance.

## Provenance and quality

A run receipt binds run ID, total mass/configuration digest, gravity, measured path distance, route digest and distance-measurement receipt. A channel receipt binds channel ID, one energy boundary, run ID, calibration ID, clock ID and timebase-mapping receipt. Samples bind channel ID, sample ID, time and raw SI voltage/current.

V0 rejects:
- non-finite or non-positive voltage;
- missing, stale, clipped, saturated or otherwise unqualified samples;
- duplicate sample IDs or timestamps;
- wrong channel identity;
- mismatched run, calibration, clock or timebase receipt;
- duplicate timestamps or overly large sample gaps;
- invalid route/mass/distance receipts.

Out-of-order rows are sorted by timestamp before integration, making the calculation deterministic under input permutation. All rows must still share the same qualified channel clock and timebase receipt.

The presence of a receipt ID does not cryptographically verify the source artifact. Upstream instrument and calibration qualification remain part of the trust boundary.

## Validation commands

    cargo fmt --manifest-path research/loco-energy-ledger-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-energy-ledger-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-energy-ledger-v0/Cargo.toml --locked --all-targets -- -D warnings

## Evidence ceiling

Synthetic tests validate arithmetic only. Simulated energy remains conditional on simulator motor, driver, battery and contact models. Hardware CoT requires calibrated source-terminal voltage/current, complete system mass and measured route distance. No biological or hardware efficiency claim follows from this oracle alone.
