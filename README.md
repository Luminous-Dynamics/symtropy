# Symtropy

Deterministic, state-coupled N-dimensional rigid-body physics and simulation substrate. The native core provides collision detection, contact resolution, constraints, replay and extensible per-body coupling. The broader engineering programme composes Symtropy with specialist physical models through explicit interfaces; Symtropy is not presented as a universal multiphysics solver.

The PhysicsCallback extension point enables application-defined state coupling, including experimental energy budgets and metric-gated actuator behavior. Such callbacks define model or policy behavior; they do not make arbitrary metrics intrinsic physical laws. Phi-related mechanics remain research experiments, not an established universal law of nature or a claim about subjective experience.

**827 tests | 5 shapes | 5 joints | CCD | raycasting | warm-starting | island sleeping | 11 crates on crates.io**

## The Symtropy Adoption Story

Symtropy is a reusable simulation and physical-systems substrate intended to compose with Symthaea and Mycelix. Broader civilizational applications are a long-term research direction, not a claim of current physical completeness.

Why choose Symtropy over traditional solutions?

| Traditional Need | The Symtropy Stack |
|---|---|
| AI (behavior trees) | **Symthaea** cognitive loop — ethics-aware, Phi-gated, Free Energy Principle |
| Player accounts | **Mycelix** DIDs (W3C standard, ZKP selective disclosure) |
| Cloud save | **Mycelix** personal cluster vault |
| In-game economy | **TEND** time-exchange with demurrage |
| UGC attribution | **Mycelix-craft** talent marketplace |
| Governance/voting | **Mycelix** DHT-backed proposals |

### 1. The Permissive Core (Track B)
For game developers and industrial simulation, Symtropy provides a high-performance, N-dimensional rigid body engine on Bevy. Use it for standard games where you need deterministic replay, 4D physics, or custom per-body metrics (health, trust, wealth) without any copyleft obligations.
- **Crates:** `symtropy-core`, `symtropy-physics`, `symtropy-bevy-core`.
- **License:** Apache-2.0 OR MIT.

### 2. The Research Hero (Track A)
For researchers in AI, A-Life, and Integrated Information Theory (IIT), the research layer provides experimental Phi-aware coupling through declared callbacks and energy-accounting models. These experiments investigate how a modeled metric can affect agent motor authority or resource budgets; they do not establish Phi as a general physical law or demonstrate subjective experience.
- **Crates:** `symtropy-consciousness-physics`, `symtropy-bevy`.
- **License:** GNU AGPL v3.

### 3. The Civilizational Stack (Symthaea + Mycelix)
When combined with **Symthaea** (cognitive loop) and **Mycelix** (decentralized governance), Symtropy becomes a test-bed for autonomous societies.
- **Cognition:** FEP-driven agents (`symthaea-fep`) with multi-theory consciousness.
- **Governance:** Real-time peer-to-peer voting and resource allocation via Holochain DHT.
- **Metabolism:** Digital twins of physical bootstrap hardware (plastic shredders, solar grids).

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

## Reality Simulation and Multiphysics

Symtropy's long-term engineering direction is a **composable, evidence-aware physical-model coordinator**. The strategy is to keep the native N-dimensional rigid-body engine focused, connect specialist solvers where they are better suited, and qualify the interfaces, model applicability and coupled results.

This is deliberately not a promise that the native core already solves CFD, FEM, electromagnetics, electrochemistry, plasma, fusion or geoscience. Current fluid and plasma work has explicit scaffolding/adapter boundaries; specialist solvers remain responsible for their supported equations and regimes.

- [Reality Physics Platform](docs/tech/REALITY_PHYSICS_PLATFORM.md) — architecture, domain gap map, model passports, typed coupling, solver federation and project mapping.
- [Reality Benchmark Portfolio](docs/ops/REALITY_BENCHMARK_PORTFOLIO.md) — matched SOTA comparisons, verification/validation, uncertainty, adversarial cases and qualification targets.

The work composes existing owners: [multi-fidelity admission #4487](https://github.com/Luminous-Dynamics/symtropy/issues/4487), [engineering twin boundary #1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467), [capability census #1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472), [typed subsystem ports #5779](https://github.com/Luminous-Dynamics/symtropy/issues/5779), [FMI coupling benchmark #6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156), and [solver-neutral plasma integration #1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685).

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

Callback-defined metrics (Phi, health, trust, wealth, etc.) can be connected through 5 software coupling channels. They are not automatically constitutive laws of real materials:

| Channel | Direction | Mechanism |
|---------|-----------|-----------|
| 1. Metric -> Force | Metric-gated motor authority (see note below) |
| 2. Metric -> Energy | Metric-dependent energy budget (higher metric = higher maintenance cost) |
| 3. Harmony -> Impulse | Sanctuary zones dampen collision impulses |
| 4. Harmony -> Friction | 1/r^(D-1) spatial fields modulate friction coefficients |
| 5. Collision -> Metric | Prediction error from unexpected collisions reduces motor precision |

See [FORMAL_SPECIFICATION.md](docs/tech/FORMAL_SPECIFICATION.md) for the mathematical details (written in terms of Phi).

**Note on channel 1 — two experimental mappings exist, and the four-tier mapping is not what the demos run.**
`symtropy-consciousness-physics` ships both a four-tier research ladder
(`SafetyTier::from_phi` + `motor_gain`, cut points at Phi = 0.6 / 0.3 / 0.1; not an NRC requirement or endorsement) and a two-level
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
| `PrismaticJoint<D>` | Constrains all but one translational degree of freedom |
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

## Reported Research Results

The following are reported outcomes from particular experimental configurations in this repository. They are conditional on the model rules, parameter values, initial conditions, implementation and analysis; they are not independently validated physical laws or universal predictions.

- **81.3% tighter clustering** was reported in a particular comparison under the model's resource/coupling rules. It does not establish that cooperation is a universal thermodynamic necessity.
- **J/Phi convergence** was reported near ~10K J/Phi for a particular substrate/configuration. This is a model-specific diagnostic statistic, not an established physical or biological constant.
- **Solo-agent collapse near four minutes** was observed in a particular scenario. It is conditional on the configured source, sink and transfer rules, not a universal survival prediction.

## Documentation

| Document | What it covers |
|----------|---------------|
| [ENGINE.md](docs/tech/ENGINE.md) | Native physics scope, research coupling, limitations and Bevy integration |
| [ARCHITECTURE.md](docs/tech/ARCHITECTURE.md) | Crate dependency graph, key types, extension guide |
| [FORMAL_SPECIFICATION.md](docs/tech/FORMAL_SPECIFICATION.md) | Experimental metric-coupled agent model and its assumptions |
| [ROADMAP.md](docs/ops/ROADMAP.md) | Completed features, current priorities, future plans |
| [REALITY_PHYSICS_PLATFORM.md](docs/tech/REALITY_PHYSICS_PLATFORM.md) | Composable multiphysics architecture, model portfolio and evidence boundaries |
| [REALITY_BENCHMARK_PORTFOLIO.md](docs/ops/REALITY_BENCHMARK_PORTFOLIO.md) | Industry comparisons, benchmark families, VVUQ and qualification gates |
| [MK0_BOOTSTRAPPER_PROTOCOL.md](docs/tech/MK0_BOOTSTRAPPER_PROTOCOL.md) | One-room bootstrapper protocol for the immediate deployable starting loop |
| [CONTRIBUTING.md](docs/CONTRIBUTING.md) | How to contribute (determinism is a hard requirement) |

## Building

```bash
# Run all engine tests
cd symtropy
cargo test -p symtropy-math -p symtropy-physics -p symtropy-consciousness-physics --lib

# Run benchmarks
cd crates/symtropy-physics && cargo bench

# Build the game (requires Bevy 0.19)
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
