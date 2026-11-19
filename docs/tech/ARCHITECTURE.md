# Symtropy Architecture

## Crate Dependency Graph

```
symtropy-math (no deps)
    Sphere<D>, Capsule<D>, HyperBox<D>, HalfSpace<D>, ConvexHull<D>
    Point<D>, Bivector<D>, Rotor<D>, Transform<D>
    Shape<D> trait (the universal collider interface)
        |
        v
symtropy-physics (depends on: symtropy-math, nalgebra, arrayvec)
    PhysicsWorld<D>          -- owns bodies, steps simulation
    RigidBody<D>             -- position, velocity, mass, collider
    GJK + EPA                -- collision detection (works in any D)
    CCD                      -- continuous collision for fast bodies
    ContactManifold<D>       -- multi-point contacts with warm-starting
    BallJoint, FixedJoint, HingeJoint  -- articulation constraints
    Raycast                  -- analytical ray-sphere intersection
    Replay + NetId           -- deterministic record/replay
    PhysicsCallback<D> trait -- low-level application/research callback
        |
        v
symtropy-consciousness-physics (depends on: symtropy-physics, symthaea-consciousness-equation)
    ConsciousnessField<D>    -- experimental metric-coupled callback
    EntityConsciousness      -- per-entity Phi computation + energy budget
    HarmonyField<D>          -- model-defined spatial research field
    EnergyBudget             -- experimental agent resource-budget model
    ThermodynamicLedger      -- model-specific resource bookkeeping and diagnostics
    SafetyTier               -- experimental motor-authority mapping
    FEP gradient             -- free energy principle agent behavior
    Active Inference         -- Bayesian belief updating
    DimensionalLeakage       -- 4D-to-3D energy transfer
    Convergence tools        -- Mann-Whitney U, Cohen's d, Holm-Bonferroni
```

## Which Crate Do I Need?

| I want to... | Use this crate |
|--------------|----------------|
| Do ND collision detection only | `symtropy-math` + `symtropy-physics` |
| Build a game with Phi-coupled physics | All three core crates + Bevy |
| Run consciousness-physics experiments | `symtropy-consciousness-physics` examples |
| Add a new collision shape | Implement `Shape<D>` in `symtropy-math` |
| Add a new joint type | Implement `Constraint<D>` in `symtropy-physics` |
| Couple custom metrics to physics | Implement `PhysicsCallback<D>` |

## The PhysicsCallback Trait

This is the low-level extension point for experimental, application-defined state coupling in the rigid-body loop. It is not the general multiphysics port/federation API, and a callback-defined metric is not automatically an intrinsic material law. The trait has 5 methods, called during the physics step:

```rust
pub trait PhysicsCallback<const D: usize> {
    // Called when computing force on a body
    fn modulate_force(&self, body: BodyHandle, force: &SVector<f64, D>) -> SVector<f64, D>;

    // Called when resolving collision impulse
    fn modulate_impulse(&self, impulse: f64, contact_point: &SVector<f64, D>) -> f64;

    // Called when computing friction at a contact
    fn friction_multiplier(&self, contact_point: &SVector<f64, D>, body: BodyHandle) -> f64;

    // Called after a collision is resolved (for prediction error feedback)
    fn on_collision(&mut self, event: &CollisionEvent<D>);

    // Called when energy is dissipated (friction, damping)
    fn record_dissipation(&mut self, energy: f64);
}
```

`ConsciousnessField<D>` implements this trait. You can also implement it for custom policy or research metrics. Any physical interpretation of a callback's effect requires its own model definition, validation evidence and context-of-use boundary.

## Physics Step Data Flow

```
1. Clear events
2. Swap warm-starting caches
3. Integrate (semi-implicit Euler: v += a*dt, x += v*dt)
4. Broadphase (LBVH with Morton codes, or brute-force for <50 bodies)
   - Collision group/mask filtering
   - Skip static-static pairs
5. Narrowphase (GJK for intersection, EPA for penetration depth)
   - Sensor detection (emit SensorEvent, skip resolution)
   - Multi-point contact manifold generation
6. Resolve contacts (solver_iterations times):
   - Apply warm-started impulse (80% of previous frame)
   - Baumgarte position correction
   - Normal impulse (restitution)
   - Friction impulse (Coulomb, modulated by harmony fields)
   - Cache impulse for next frame
   - Emit CollisionEvent, call PhysicsCallback::on_collision()
7. Solve constraints (position + velocity level)
8. Body sleeping (velocity threshold, tick counter)
```

## Determinism Guarantees

The engine is designed for deterministic replay:

- **Integer Morton codes** in broadphase (no floating-point heuristics)
- **BTreeMap** for all entity maps (sorted iteration order)
- **Sorted collision pairs** by (BodyHandle, BodyHandle)
- **NetId** for stable body identity across machines
- **Record/replay** via `ReplayTape<D>` and `WorldSnapshot<D>`

What is NOT guaranteed:
- Cross-platform bitwise equality (different CPUs may give different float results)
- Determinism under multithreading (single-threaded physics step is deterministic)

## Extending the Engine

### Adding a New Shape

Implement `Shape<D>` in `symtropy-math`:

```rust
pub struct MyShape<const D: usize> { /* fields */ }

impl<const D: usize> Shape<D> for MyShape<D> {
    fn support(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        // Return the point on the boundary furthest in `direction`
    }

    fn bounding_sphere(&self) -> (Point<D>, f64) {
        // Return (center, radius) of the smallest enclosing sphere
    }
}
```

GJK and EPA will automatically work with your shape.

### Adding a New Joint

Implement `Constraint<D>` in `symtropy-physics`:

```rust
pub struct MyJoint<const D: usize> { /* fields */ }

impl<const D: usize> Constraint<D> for MyJoint<D> {
    fn bodies(&self) -> (BodyHandle, BodyHandle) { /* ... */ }

    fn solve(&self, body_a: &mut RigidBody<D>, body_b: &mut RigidBody<D>, dt: f64) {
        // Apply positional corrections (called solver_iterations times)
    }

    fn solve_velocity(&self, body_a: &mut RigidBody<D>, body_b: &mut RigidBody<D>, dt: f64) {
        // Apply velocity corrections (optional, called after position solve)
    }
}
```

### Feature Flags

| Flag | Crate | What it enables |
|------|-------|----------------|
| `consciousness-curvature` | consciousness-physics | Conformal geometry: `ConformalMetric<D>`, geodesic corrections, Ricci scalar |
| `consciousness-hdc` | consciousness-physics | 16,384D HDC substrate: `HdcConsciousnessContext`, LTC neural network |
| `mycelix` | symtropy (game) | Governance, economy, factions, DKG, federated learning |
| `atlas` | symtropy (game) | Sol Atlas planetary globe view |
| `live-audio` | symtropy (game) | Real-time Phi-driven music synthesis |
| `net` | symtropy (game) | P2P networking (requires `world` feature) |


## Reality-Physics Composition Boundary

The native physics crates remain the N-dimensional rigid-body/contact substrate. Broader engineering simulation is composed through model-specific adapters and explicit typed coupling contracts, not by extending this callback into a universal PDE interface.

Canonical planning documents:
- [Reality Physics Platform](REALITY_PHYSICS_PLATFORM.md) defines the domain portfolio, model passports, specialist solver boundaries and work ownership.
- [Reality Benchmark Portfolio](../ops/REALITY_BENCHMARK_PORTFOLIO.md) defines matched solver comparison, numerical verification, physical validation, uncertainty and qualification gates.
- [Game Engine Component Gap Analysis](GAME_ENGINE_COMPONENTS.md) remains the game-engine feature checklist; it is not the complete engineering multiphysics census.

### Composition responsibilities

- **Native Symtropy physics:** deterministic scenario execution, supported body/contact/constraint paths, bounded synthetic disturbances, replay and run-health diagnostics.
- **Domain solver:** owns the equations, constitutive laws, discretization, convergence and numerical solution within its supported regime.
- **Typed port adapter:** owns explicit units, frames, sign conventions, event time, availability time, mapping, conversion, balance group and provenance.
- **Fidelity admission:** checks exact capability and applicability, then admits, escalates or abstains under the current model-selection owner [#4487](https://github.com/Luminous-Dynamics/symtropy/issues/4487).
- **Evidence/twin authority:** remains outside Symtropy's synthetic execution path; see [#1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467).

FMI 3.0.2 and SSP 2.0.1 are interoperability references where model semantics fit. Protocol conformance does not validate component physics or the coupling master algorithm. Every composed run must retain exact model, adapter, clock, configuration and raw-artifact identity.

### Required separation

`capability advertised` is not `capability exercised`; `execution completed` is not `numerically healthy`; `numerically healthy` is not `model validated`; and `model validated` is not `engineering authorization`.

The same distinctions apply to synthetic scenario replay, physical observation, canonical twin state and design acceptance. A synthetic descendant of a measured initial state remains synthetic.
