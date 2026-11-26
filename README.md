# Symtropy

**A Rust workspace for N-dimensional geometry and rigid-body physics, with Bevy integration, robotics/simulation bridges, and experimental metric-coupled dynamics.**

Symtropy is both a reusable engine codebase and a collection of applications and research experiments. The reusable physics crates are distinct from the Bevy launcher and from optional research-oriented domains. This README describes the repository as it exists in the standalone public workspace; it does not treat every research direction or optional integration as a shipped capability.

## What is in this repository?

| Layer | Main components | Status and boundary |
|---|---|---|
| Geometry and physics | \`symtropy-math\`, \`symtropy-physics\` | Reusable N-dimensional math and rigid-body physics crates |
| Bevy integration | \`symtropy-bevy-core\`, \`symtropy-bevy\`, render and Rapier bridges | Engine integration and adapters; feature-gated where appropriate |
| Applications | CLI, validation scenes, robotics and simulation demos, web gallery | Executable examples and application surfaces; not all are production products |
| Research domains | Metric/Φ coupling, HDC physics, colony/lifesim, terrain, networking | Experimental modules; validate each domain independently before deployment |
| Launcher | Root \`symtropy-launcher\` package | Bevy application that composes selected experiences and optional features |

The root package is named \`symtropy-launcher\` and is not published as a library. The workspace also contains separately licensed crates. **Read [LICENSING.md](LICENSING.md) before copying or redistributing code**; the core math/physics packages include Apache-2.0 OR MIT licensing, while other components may be AGPL-3.0-or-later.

## Core physics capabilities

The \`symtropy-math\` crate exposes const-generic geometric types such as \`Point<D>\`, \`Bivector<D>\`, \`Rotor<D>\`, \`Transform<D>\`, and shape primitives.

The \`symtropy-physics\` crate provides a dimension-parameterized rigid-body simulation API, including:

- GJK intersection and EPA contact support;
- broadphase, contact manifolds, and collision diagnostics;
- continuous collision detection (CCD);
- raycasting and articulated/jointed bodies;
- a \`PhysicsCallback\` interface for application-defined coupling;
- replay-oriented world commands and snapshots.

The package documentation and tests are the appropriate source of truth for the behavior supported by a particular crate version. A feature listed in a domain crate or roadmap is not automatically part of the default launcher build.

## Metric coupling and consciousness research

Some research modules explore coupling a numerical metric to forces, friction, motor authority, or energy accounting. The current code includes an extensible callback boundary and separate experimental domains for Φ-related coupling.

Here, Φ refers to an implementation of an information-theoretic quantity or a model input. It is **not evidence of subjective experience, sentience, or consciousness**, and its use in a simulation does not validate Integrated Information Theory as a physical law. Treat these modules as research software and inspect their assumptions, units, validation datasets, and enabled features before using them in safety-relevant work.

Similarly, robotics, digital-twin, and physical-system integrations are not safety certification. A successful simulation or unit test does not establish real-world control safety.

## Build and validation

The repository pins Rust in \`rust-toolchain.toml\` and provides a Nix development shell.

\`\`\`sh
nix develop

# Fast compilation check
just check

# Canonical headless/core verification recipe
just verify-core

# Full workspace test suite (may include application and platform dependencies)
just test
\`\`\`

The \`verify-core\` recipe is the recommended starting point for the headless engine surface. It includes the repository's license and formatting checks, Clippy checks for selected core crates, selected build/test targets, asset-foundry tests, and a headless governance scenario. It is not a substitute for hardware-in-the-loop validation or an independent safety audit.

For narrower checks, use Cargo directly:

\`\`\`sh
cargo check -p symtropy-math -p symtropy-physics
cargo test -p symtropy-math -p symtropy-physics --lib
cargo clippy -p symtropy-math -p symtropy-physics -- -D warnings
\`\`\`

The root README does not publish fixed test totals or microbenchmark figures because they vary by revision, toolchain, hardware, feature set, and benchmark harness. Use the checked-out commit's Criterion benches and CI results for reproducible comparisons.

## Repository guides

- [Architecture](ARCHITECTURE.md)
- [Engine notes](ENGINE.md)
- [Formal specification](FORMAL_SPECIFICATION.md)
- [Roadmap](ROADMAP.md)
- [Robotics platform roadmap](Robotics_Platform_ROADMAP.md)
- [License breakdown](LICENSING.md)
- [Contributing](CONTRIBUTING.md)

## Project maturity

Symtropy is actively developed research and engineering software. Some crates are independently reusable; other features are prototypes, adapters, demos, or plans. There is no blanket production-readiness claim for the workspace. Before adopting a component, verify its package-level license, API, feature flags, tests, platform requirements, and latest CI evidence at the exact commit you intend to use.

## License

Licensing is component-specific. See [LICENSING.md](LICENSING.md) and the license files in the relevant crate directories.
