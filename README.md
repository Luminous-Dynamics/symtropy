# Symtropy

**A Rust-based N-dimensional mathematics and rigid-body physics engine for interactive simulation, research, and embodied systems.**

Symtropy provides const-generic geometry and physics primitives, collision detection, constraints, raycasting, deterministic replay, and state-coupling hooks. A caller-defined metric can modulate selected dynamics such as forces, friction, and energy budgets.

The coupling interface is generic: applications can supply normalized metrics such as health, trust, skill, or other domain-specific state. The repository also contains optional Phi/IIT-inspired research extensions; these are experimental research mechanisms, not established physical laws or validated measurements of consciousness.

**Repository-reported capabilities:** 827 tests, 5 collision-shape families, 5 joint types, continuous collision detection, raycasting, warm-starting, island sleeping, and 11 crates published on crates.io. Treat these as project-maintained figures; consult current CI, release metadata, and benchmark conditions before using them in external comparisons.

## What Symtropy is useful for

- N-dimensional geometry and rigid-body simulation.
- Interactive applications and Bevy-based simulation prototypes.
- Deterministic replay and reproducible physics experiments.
- Research into coupling application-defined state metrics to physical simulation.
- Embodied-agent and robotics experiments, subject to platform-specific validation.

Symtropy is a simulation substrate, not a complete industrial digital-twin, CAD, finite-element, or engineering-assurance suite. Such applications require additional domain models, validation datasets, numerical analysis, uncertainty treatment, and independent engineering review.

## Adoption paths

### 1. Permissively licensed physics core

For developers who want the core math and rigid-body functionality in games, interactive applications, and simulation prototypes.

- **Crates:** `symtropy-core`, `symtropy-physics`, `symtropy-bevy-core`.
- **License:** Apache-2.0 OR MIT.

### 2. Research extensions

For research in artificial life, embodied intelligence, dynamical systems, and consciousness-related computational hypotheses. These extensions allow selected metrics and experimental coupling mechanisms to be investigated in simulation. They do not establish that consciousness is a physical field or that Phi is a validated safety or control signal.

- **Crates:** `symtropy-consciousness-physics`, `symtropy-bevy`.
- **License:** AGPL-3.0-or-later.

### 3. Luminous Dynamics ecosystem experiments

Symtropy can be used alongside other Luminous Dynamics projects in research prototypes:

- **Symthaea:** experimental cognitive architectures and active-inference agents.
- **Mycelix:** decentralized identity and coordination components.
- **Sol Atlas:** spatial visualization and scenario exploration.

These are integration directions and experimental compositions, not a single turnkey product. Check each project's README for the current implementation and maturity boundary.

## Will NOT Do (Scope Boundaries)
To maintain focus on our core differentiators (4D physics, consciousness coupling, decentralized governance), we explicitly scope out the following:
- **No bespoke game editor** — We rely entirely on `bevy_inspector_egui` for in-engine tooling.
- **No custom wgpu renderer** — Bevy owns the render graph.
- **No Jacobian solver** — PBD (Position Based Dynamics) + impulse at <1000 bodies is our chosen constraint.
- **No mobile (iOS/Android)** — Mobile support is out of scope. WebXR (WASM) is the reach target.
- **No GPU broadphase in core** — `symtropy-physics-gpu` is provided as an ecosystem crate, not core engine logic.
- **No soft-body in core** — Expected to be community-owned ecosystem crates.

## Architecture

```
symtropy-math                  N-dimensional geometric algebra (const-generic, stack-allocated)
  |-- symtropy-physics         GJK+EPA collision, CCD, joints, raycasting, replay
  |     |-- symtropy-consciousness-physics   Phi-coupling, thermodynamics, harmony fields
  |     |     |-- symtropy-world             Macro/micro simulation bridge
  |     |-- symtropy-render-bridge           ND-to-Bevy projection, 4D cross-section slicing
  |     |-- symtropy-robotics-bridge         FEP agents, 6 embodied platforms
  |     |-- symtropy-net                     P2P spatial authority, lockstep protocol
```

See [ARCHITECTURE.md](docs/tech/ARCHITECTURE.md) for the full crate guide.

## Quick Start: Generic State-Coupling

```rust
use symtropy_physics::PhysicsWorld;
use symtropy_consciousness_physics::SimpleCoupledField;
use symtropy_math::Point;
use nalgebra::SVector;

// 1. Create a 2D physics world
let mut world = PhysicsWorld::<2>::new(SVector::from([0.0, -9.81]));

// 2. Add bodies
let agent = world.add_sphere(Point::new([0.0, 10.0]), 1.0, 1.0);

// 3. Create a state-coupled field (works with ANY metric)
let mut field = SimpleCoupledField::<2>::new();
field.register(agent, 100.0, 10.0);
field.set_metric(agent, 0.8);  // your metric: health, trust, skill, wealth...

// 4. Step — forces and friction are modulated by your metric
world.step_with_callback(0.016, &mut field);
```

For Phi-specific research (IIT, Master Consciousness Equation, 7-theory synthesis), use `ConsciousnessField` instead:

```rust
use symtropy_consciousness_physics::ConsciousnessField;

let mut phi_field = ConsciousnessField::<3>::new();
phi_field.register(handle, 100.0, 10.0);
// Phi computed from ConsciousnessInputs via the Master Equation
world.step_with_callback(0.016, &mut phi_field);
```

## Five Coupling Channels

Any metric (Phi, health, trust, wealth) couples to physics through 5 channels:

| Channel | Direction | Mechanism |
|---------|-----------|-----------|
| 1. Metric -> Force | Metric-gated motor authority (see note below) |
| 2. Metric -> Energy | Metric-dependent energy budget (higher metric = higher maintenance cost) |
| 3. Harmony -> Impulse | Sanctuary zones dampen collision impulses |
| 4. Harmony -> Friction | 1/r^(D-1) spatial fields modulate friction coefficients |
| 5. Collision -> Metric | Prediction error from unexpected collisions reduces motor precision |

See [FORMAL_SPECIFICATION.md](docs/tech/FORMAL_SPECIFICATION.md) for the mathematical details (written in terms of Phi).

**Note on channel 1 — two mappings exist, and the NRC one is not what the demos run.**
`symtropy-consciousness-physics` ships both an NRC-inspired 4-tier ladder
(`SafetyTier::from_phi` + `motor_gain`, cut points at Phi = 0.6 / 0.3 / 0.1) and a two-level
sprint/floor gate (`sprint_floor_gain`) taking platform-specific thresholds. **Every platform
demo in this workspace uses the latter.** Measured Phi on these platforms clusters around 0.1
— AUV and humanoid calibrate at 0.130, flight 0.110, vehicle 0.101, helicopter 0.100 — a band
that falls entirely inside the 4-tier ladder's Red/Orange range, so feeding real platform Phi
into `from_phi` would pin motor gain at 0.0 or 0.3 permanently. Measure your own platform's
band (`symtropy-cli` prints a suggested threshold; see also `MANIP_BENCH_PHI_TRACE=1`) before
choosing a mapping. Full detail in the `safety` module docs.

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

| Flag | What it enables |
|------|----------------|
| `consciousness-curvature` | Conformal geometry: Phi curves space, agents follow geodesics |
| `consciousness-hdc` | 16,384D hyperdimensional computing substrate for Phi |
| `mycelix` | Governance/economy/crypto integration (game-specific) |
| `atlas` | Sol Atlas planetary globe view (game-specific) |
| `live-audio` | Real-time Phi-driven music synthesis (requires ALSA) |
| `net` | P2P networking with spatial authority |

## Research Experiments

68 experiments in `crates/domains/symtropy-consciousness-physics/examples/`:

```bash
# Core cooperation emergence (the definitive experiment)
cargo run --example cooperation_emergence

# J/Phi convergence (novel thermodynamic metric)
cargo run --example jphi_convergence

# All examples
cargo run --example cooperation_1000    # scaling to large populations
cargo run --example inequality_emergence # wealth gap self-organization
cargo run --example tragedy_commons      # tragedy of the commons
cargo run --example dunbar_number        # social network size limits
cargo run --example anesthesia_transition # Phi level transitions
# ... and 62 more
```

3 experiments require feature flags:
```bash
cargo run --example curvature_lensing --features consciousness-curvature
cargo run --example curvature_selforg --features consciousness-curvature
cargo run --example hdc_cooperation --features consciousness-hdc
```

## Key Results

- **81.3% tighter clustering** under thermodynamic enforcement (cooperation emerges as a thermodynamic necessity)
- **J/Phi converges** to a stable substrate-characteristic value (~10K J/Phi)
- **Prediction 1 confirmed**: Solo agents collapse within ~4 minutes; cooperative agents sustain indefinitely

## Documentation

| Document | What it covers |
|----------|---------------|
| [ENGINE.md](docs/tech/ENGINE.md) | Deep technical dive: architecture, unique features, Bevy integration |
| [ARCHITECTURE.md](docs/tech/ARCHITECTURE.md) | Crate dependency graph, key types, extension guide |
| [FORMAL_SPECIFICATION.md](docs/tech/FORMAL_SPECIFICATION.md) | Mathematical specification of all 5 coupling channels |
| [ROADMAP.md](docs/ops/ROADMAP.md) | Completed features, current priorities, future plans |
| [MK0_BOOTSTRAPPER_PROTOCOL.md](docs/tech/MK0_BOOTSTRAPPER_PROTOCOL.md) | One-room bootstrapper protocol for the immediate deployable starting loop |
| [CONTRIBUTING.md](docs/CONTRIBUTING.md) | How to contribute (determinism is a hard requirement) |

## Building

```bash
# Run all engine tests
cd symtropy
cargo test -p symtropy-math -p symtropy-physics -p symtropy-consciousness-physics --lib

# Run benchmarks
cd crates/symtropy-physics && cargo bench

# Build the game (requires Bevy 0.18)
cargo build --release

# Build with Mycelix governance integration
cargo build --release --features mycelix
```

### Building without Nix

The `justfile` / `run.sh` workflow assumes a Nix environment, but you don't need Nix to build Symtropy — you need a Rust toolchain plus (on Linux, for the full game/launcher build only) the same system libraries CI installs (`.github/workflows/ci.yml`):

```bash
# Debian/Ubuntu
sudo apt-get update
sudo apt-get install -y libasound2-dev libudev-dev pkg-config
```

These are required because Bevy pulls in ALSA (audio) and udev (input device enumeration) on Linux. **The pure-library quickstart — `symtropy-math`, `symtropy-physics`, `symtropy-consciousness-physics`, no Bevy — needs none of this.** Only `cargo build --release` (the Bevy-based game/launcher) requires these system packages.

macOS and Windows builds need no extra system packages beyond a standard Rust toolchain.

## License

**Dual-track license model** — see [LICENSING.md](docs/LICENSING.md) for the full breakdown.

- **Permissive today** (`symtropy-math`, `symtropy-physics`, `symtropy-render-bridge`): **Apache-2.0 OR MIT** — zero AGPL deps, ship in proprietary products freely.
- **AGPL today, permissive `-core` variants in Phase 0.5** (`symtropy-bevy`, `symtropy-robotics-bridge`, `symtropy-net`): each currently requires an AGPL dep; see [LICENSING.md](docs/LICENSING.md) for the split plan.
- **Research layer** (`symtropy-consciousness-physics`, `symtropy-sim-bridge`, game crates): **AGPL-3.0-or-later** — modifications must be shared back under AGPL, or negotiate a [commercial license](docs/COMMERCIAL_LICENSE.md).

## References

- Tononi, G. (2004). An information integration theory of consciousness. *BMC Neuroscience*.
- Friston, K. (2019). A Free Energy Principle for a Particular Physics. *arXiv*.
- Adams, Shipp & Friston (2013). Predictions not commands. *Brain Structure & Function*.
- McFadden, J. (2020). CEMI field theory. *Neuroscience of Consciousness*.
- Landauer, R. (1961). Irreversibility and Heat Generation. *IBM J. Res. Dev*.
- ten Bosch, M. (2020). N-Dimensional Rigid Body Dynamics. *SIGGRAPH*.
