# symtropy-consciousness-physics

An experimental coupling layer for a modeled integration metric (Φ) in a physics simulation. The equations and callback rules below are software models; this crate does not establish subjective consciousness or a new physical force.

```rust
use symtropy_physics::PhysicsWorld;
use symtropy_consciousness_physics::ConsciousnessField;
use symtropy_math::Point;
use nalgebra::SVector;

// Create world and consciousness field
let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
let mut consciousness = ConsciousnessField::<3>::new();

// Add a simulated agent with a modeled integration score
let handle = world.add_sphere(Point::new([0.0, 10.0, 0.0]), 1.0, 1.0);
consciousness.register(handle, 1000.0, 20.0); // 1000J budget, 20m sanctuary radius

// Physics step with configured simulation callbacks
// The model modifies force authority, sanctuary response, friction, and energy
world.step_with_callback(0.016, &mut consciousness);
```

## Five Coupling Channels

1. **Modeled Φ → Motor gain**: The configured integration score selects a force-authority tier
2. **Φ → Energy budget**: Consciousness gates available Joules per tick
3. **Sanctuary → Collision**: Configured sanctuary zones dampen collision impulse
4. **HarmonyField → Friction**: Aligned nonnegative profiles reduce friction; orthogonal or absent profiles leave baseline friction unchanged
5. **Collision → Prediction error**: Unexpected collision spikes error → reduces motor precision → habituates

## Thermodynamic Closure

The `ThermodynamicLedger` tracks energy conservation with scientific rigor:
- **Joules-per-Phi**: novel metric — energy cost of consciousness (no prior publication)
- **Landauer bound**: minimum thermodynamic cost enforced (2.87×10⁻²¹ J/bit at 310K)
- **Conservation error**: tracked and reported each tick

## Harmony Fields

HarmonyField is a **phenomenological spatial influence model**, inspired by field-coupling hypotheses but not an electromagnetic simulation or validation of the CEMI theory. Sources superpose using a Plummer-softened radial kernel with exponent p_D = max(D - 1, 1) and a finite radius cutoff.

- Similarity is measured with cosine resonance over finite, nonnegative activation profiles.
- Aligned profiles can reduce friction; orthogonal profiles leave the baseline unchanged.
- With nonnegative activations, signed anti-alignment or "dissonance amplification" is not represented.
- Collision prediction error also uses profile resonance in the simulation callback.
- Collision-impulse damping currently comes from SanctuaryZone rules. The standalone HarmonyField impulse-multiplier API is not wired into the physics callback.
- Emotional contagion is a separate proximity-weighted agent update, not an electromagnetic-field simulation.

Treat this as a tunable simulation hypothesis. Its usefulness should be established through deterministic ablations, sensitivity analysis, and comparisons with relevant observations—not from plausible-looking simulation output alone.

## Sanctuary Zones

When the configured Sacred Stillness and Φ thresholds are met, a sanctuary zone can dampen collision impulses up to its configured limit (up to 90% with the corresponding settings). This is a simulation mechanic, not evidence of subjective consciousness.

## Benchmarks

Measured on AMD Ryzen, `cargo bench -p symtropy-consciousness-physics`:

| Operation | N | Median |
|-----------|---|--------|
| `update_entity` (single) | 1 | 2.2 µs |
| `update_entity` (batch) | 8 | 26 µs |
| `update_entity` (batch) | 32 | 77 µs |
| `update_entity` (batch) | 128 | 706 µs |
| `tick_prediction_errors` | 8 | 133 ns |
| `tick_prediction_errors` | 64 | 1.1 µs |
| `tick_prediction_errors` | 256 | 7.7 µs |
| `spread_emotional_contagion` | 4 | 2.7 µs |
| `spread_emotional_contagion` | 16 | 32 µs |
| `spread_emotional_contagion` | 64 | 386 µs |
| `apply_macro_modifiers` | 32 | 626 ns |
| `phi_env_coupler_tick` | 1 | 778 ns |
| `biometric_to_consciousness` | 30 | 1.2 µs |
| `full_step_with_consciousness` | 4 | 13 µs |
| `full_step_with_consciousness` | 16 | 60 µs |
| `bifurcation_sweep` (3k×2) | — | 222 µs |

At 60 Hz with 16 agents: ~60 µs/tick → **16,600 ticks/second** (277× real-time budget).

## WASM Compatible

Part of the [Symtropy consciousness-physics engine](https://github.com/luminous-dynamics/symtropy).
