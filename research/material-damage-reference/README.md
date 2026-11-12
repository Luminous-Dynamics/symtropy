# Material Damage Reference v0.1

**Tracking:** PHYS-DAMAGE-REFERENCE-001  
**Status:** dependency-free analytical reference; synthetic verification only.

## Included

- Strictly ordered, material-specific S-N points.
- Piecewise log-log S-N interpolation inside the supplied stress range only.
- Linear Miner damage accumulation over caller-supplied, already-counted cycle blocks.
- Constant-Delta-K Paris-law crack extension inside a caller-declared Delta-K interval.
- Typed failure for invalid, non-finite, non-positive, out-of-range, empty, zero-cycle and overflowing inputs.
- Independent known-answer tests and metamorphic tests for block splitting/permutation, stress-life monotonicity, and constant-Delta-K cycle/coefficient scaling.
- Mutation-sensitivity audit on disposable copies; selected S-N, Miner and Paris formula defects must be killed by the named analytical/metamorphic tests.

## Run

    cargo fmt --manifest-path research/material-damage-reference/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/material-damage-reference/Cargo.toml
    cargo clippy --locked --manifest-path research/material-damage-reference/Cargo.toml --all-targets -- -D warnings
    bash research/material-damage-reference/scripts/verify_material_damage_mutations.sh

The focused GitHub Actions workflow archives the mutation-sensitivity receipt separately from source. A successful mutation gate demonstrates that the selected tests detect those seeded defects; it does not establish model correctness beyond the tested cases.

## Exact claim boundary

This crate is not a general fatigue-life or fracture-mechanics solver.

The Miner model assumes a linear sum of cycle fractions. It does not account for sequence interaction, mean stress, multiaxiality, overload retardation, temperature, corrosion or manufacturing history. Miner damage reaching unity is only a model threshold.

The Paris model evaluates a caller-supplied constant Delta-K. It does not calculate a stress-intensity factor from geometry and loading, determine a fracture-toughness limit, or model threshold/instability regions. The caller-declared Paris interval is not evidence of material calibration.

No specimen data or real-material S-N/Paris constants are embedded. Synthetic values in tests are formula controls, not material properties.

## Industry comparison

Commercial systems are much broader. Ansys nCode DesignLife documents stress-life, strain-life, vibration, thermo-mechanical, weld and crack-growth analyses, integrated with FEA. SIMULIA fe-safe documents multiaxial fatigue, duty-cycle damage, manufacturing effects, probabilistic warranty curves and FE integration.

This reference does not compete with those products on physics breadth. Its contribution is a small, inspectable, dependency-free contract with strict no-extrapolation behavior, metamorphic checks and mutation-sensitive tests that can later be composed with Symtropy's physical-state and material-provenance kernel.

ASTM E647-24 is a relevant experimental-method anchor for fatigue crack-growth rate measurement as a function of Delta-K. Any later experimental comparison must preserve load ratio, test environment, specimen geometry/thickness, residual stress, crack measurement method and uncertainty; matching Delta-K alone is not enough to establish transferability.

## Next qualification steps

1. Acquire rights-cleared fatigue and crack-growth data; do not assume publicly downloadable means redistributable.
2. Separate calibration and blind evaluation by specimen and material lot.
3. Add the load-history reconstruction/cycle-counting layer as a separately qualified component.
4. Add geometry-specific stress-intensity adapters only with independent references.
5. Bind admitted damage estimates to component identity, source state, calibration, uncertainty and model applicability before composing them with construction/repair events.

A passing synthetic formula test does not validate a material, structure, remaining-useful-life forecast, inspection interval, or repair/release decision.
