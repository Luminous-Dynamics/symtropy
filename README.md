# Symtropy

Reusable N-dimensional math, physics, geometry, and simulation substrate for Luminous Dynamics projects.

The public workspace currently focuses on self-contained substrate crates: geometric primitives, rigid-body physics, fluid/mesh/soft-body scaffolding, and deterministic network-core utilities.

**Symtropy is not Symthaea.** Symthaea owns the cognitive architecture, RHN/Broca, active inference, and higher-level agent loops.

**Symtropy is not Mycelix.** Mycelix owns civic/governance/commons infrastructure.

Symtropy provides the lower-level substrate those systems can build on.

**N-dimensional geometry | GJK/EPA physics | CCD | raycasting | warm-starting | replay | 6 checked workspace crates**

## Public Workspace

The initial public workspace is intentionally conservative:

| Crate | Purpose |
|-------|---------|
| `symtropy-math` | Const-generic N-dimensional geometry primitives. |
| `symtropy-physics` | GJK/EPA collision, CCD, joints, raycasting, replay. |
| `symtropy-fluid` | Early fluid simulation substrate. |
| `symtropy-mesh` | Mesh and narrowphase scaffolding. |
| `symtropy-soft` | Early soft-body substrate. |
| `symtropy-net-core` | Deterministic networking and authority primitives. |

Other exported crates and demos remain in-tree but outside the checked public workspace until their dependencies and platform feature sets are split cleanly.

See [EXPORT_NOTES.md](EXPORT_NOTES.md) for the export boundary.

## Architecture

```
symtropy-math                  N-dimensional geometric algebra (const-generic, stack-allocated)
  |-- symtropy-physics         GJK+EPA collision, CCD, joints, raycasting, replay
  |-- symtropy-fluid           Fluid substrate
  |-- symtropy-mesh            Mesh/narrowphase substrate
  |-- symtropy-soft            Soft-body substrate
  |-- symtropy-net-core        Network authority primitives
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full crate guide.

## Quick Start

```rust
use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

let mut world = PhysicsWorld::<2>::new(SVector::from([0.0, -9.81]));
let agent = world.add_sphere(Point::new([0.0, 10.0]), 1.0, 1.0);

world.step(0.016);
assert!(world.body(agent).is_some());
```

## Five Coupling Channels

Any metric (Phi, health, trust, wealth) couples to physics through 5 channels:

| Channel | Direction | Mechanism |
|---------|-----------|-----------|
| 1. Metric -> Force | NRC 4-tier safety system gates motor authority |
| 2. Metric -> Energy | Metric-dependent energy budget (higher metric = higher maintenance cost) |
| 3. Harmony -> Impulse | Sanctuary zones dampen collision impulses |
| 4. Harmony -> Friction | 1/r^(D-1) spatial fields modulate friction coefficients |
| 5. Collision -> Metric | Prediction error from unexpected collisions reduces motor precision |

See [FORMAL_SPECIFICATION.md](FORMAL_SPECIFICATION.md) for the mathematical details (written in terms of Phi).

## Performance

Measured on NixOS, Rust stable, AMD Ryzen (single-threaded):

| Operation | Time |
|-----------|------|
| GJK sphere-sphere 3D | 156 ns |
| GJK capsule-capsule 3D | 173 ns |
| GJK HyperBox 3D | 116 ns |
| GJK HyperBox 4D | 101 ns |
| EPA sphere-sphere 3D | 30 ns |
| Raycast 100 bodies | 14 us |
| Step 10 bodies | 5.8 us |
| Step 100 bodies | 135 us |
| Step 500 bodies | 910 us |

Zero heap allocation in the physics hot path. All types use `const D: usize` generics with stack-allocated `nalgebra::SVector`.

## Collision Shapes

| Shape | Dimensions | Support Complexity |
|-------|------------|-------------------|
| `Sphere<D>` | Any | O(1) |
| `Capsule<D>` | Any | O(1) |
| `HyperBox<D>` | Any | O(D) |
| `HalfSpace<D>` | Any | Analytical contacts |
| `ConvexHull<D>` | Any | O(vertices) |

## Joint Types

| Joint | DOF Removed |
|-------|------------|
| `FixedJoint<D>` | All (rigid attachment) |
| `BallJoint<D>` | D translational |
| `HingeJoint<D>` | D translational + (n-1) rotational |
| `DistanceConstraint<D>` | Maintains fixed distance |

## Feature Flags

Feature flags are crate-specific. The initial public workspace is validated with default features.

## Research Experiments

The exported tree includes historical research material under
`crates/symtropy-consciousness-physics/`, but that crate is outside the initial
checked public workspace because it still depends on higher-level research
crates. Treat it as source material until it is split into a self-contained
public workspace member.

## Key Results

- **81.3% tighter clustering** under thermodynamic enforcement (cooperation emerges as a thermodynamic necessity)
- **J/Phi converges** to a stable substrate-characteristic value (~10K J/Phi)
- **Prediction 1 confirmed**: Solo agents collapse within ~4 minutes; cooperative agents sustain indefinitely

## Documentation

| Document | What it covers |
|----------|---------------|
| [ENGINE.md](ENGINE.md) | Deep technical dive: architecture, unique features, Bevy integration |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Crate dependency graph, key types, extension guide |
| [FORMAL_SPECIFICATION.md](FORMAL_SPECIFICATION.md) | Mathematical specification of all 5 coupling channels |
| [ROADMAP.md](ROADMAP.md) | Completed features, current priorities, future plans |
| [docs/MK0_BOOTSTRAPPER_PROTOCOL.md](docs/MK0_BOOTSTRAPPER_PROTOCOL.md) | One-room bootstrapper protocol for the immediate deployable starting loop |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to contribute (determinism is a hard requirement) |

## Building

```bash
# Check the public workspace
cargo check --workspace --all-targets --locked

# Run public workspace tests
cargo test --workspace --locked

# Run benchmarks
cd crates/symtropy-physics && cargo bench
```

## License

**Dual-track license model** — see [LICENSING.md](LICENSING.md) for the full breakdown.

- **Permissive today** (`symtropy-math`, `symtropy-physics`, `symtropy-render-bridge`): **Apache-2.0 OR MIT** — zero AGPL deps, ship in proprietary products freely.
- **AGPL today, permissive `-core` variants in Phase 0.5** (`symtropy-bevy`, `symtropy-robotics-bridge`, `symtropy-net`): each currently requires an AGPL dep; see [LICENSING.md](LICENSING.md) for the split plan.
- **Research layer** (`symtropy-consciousness-physics`, `symtropy-sim-bridge`, game crates): **AGPL-3.0-or-later** — modifications must be shared back under AGPL, or negotiate a commercial license.

## References

- Tononi, G. (2004). An information integration theory of consciousness. *BMC Neuroscience*.
- Friston, K. (2019). A Free Energy Principle for a Particular Physics. *arXiv*.
- Adams, Shipp & Friston (2013). Predictions not commands. *Brain Structure & Function*.
- McFadden, J. (2020). CEMI field theory. *Neuroscience of Consciousness*.
- Landauer, R. (1961). Irreversibility and Heat Generation. *IBM J. Res. Dev*.
- ten Bosch, M. (2020). N-Dimensional Rigid Body Dynamics. *SIGGRAPH*.
