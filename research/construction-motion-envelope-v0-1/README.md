# Construction Motion Envelope — v0.1

**Status:** synthetic, dependency-free Rust reference kernel. Draft research artifact, not a safety controller, robot planner, reservation authority or field-qualified swept-volume engine.

Parent: [CONSTRUCTION-MOTION-ENVELOPE-001 #5345](https://github.com/Luminous-Dynamics/symtropy/issues/5345).

## What it does

It converts already-admitted, time-indexed world-space centers for per-link/tool/payload bounding spheres into conservative per-time-cell axis-aligned outer bounds.

For each primitive and half-open time interval it constructs three distinct boxes:

1. physical_swept_outer: primitive radius + path-deviation + localization + tracking + model + numerical bounds.
2. stopping_outer: the physical outer box expanded by the producer-supplied stopping/recovery reachability bound.
3. coordination_outer: the stopping box expanded by the separately declared project policy clearance.

Each per-axis contribution and its evidence reference is preserved in the output. The output is sorted by slice and primitive identity and has deterministic versioned canonical bytes suitable for external hashing/signing.

## Contract and assumptions

Inputs are already in one admitted frame and one monotonic site clock domain. Each primitive center is piecewise linear over a declared time cell. The input producer supplies a per-axis outer bound on the true path's deviation from that chord over the entire cell, plus separately scoped localization, tracking, model, numerical, stopping and policy margins.

The builder binds an exact expected body-primitive ID set to the collision-model manifest and rejects omissions even when a primitive is missing from every time cell. It also preserves the nominal centerline in each output slice. It rejects missing evidence references, malformed context digests, time gaps/overlaps, per-slice primitive omissions, discontinuous centers, changing primitive radius and integer overflow.

The producer-supplied path-deviation and stopping bounds are **not proven by this crate**. This code checks envelope arithmetic and contract consistency, not physical truth, validity or authenticity of upstream evidence. A digest-shaped string is an identifier, not a verified digest.

## Deliberate non-goals

- No IFC/BIM parser or coordinate transform.
- No robot kinematics, inverse kinematics, trajectory planner or dynamics solver.
- No collision mesh, convex decomposition, exact continuous-collision detector or geometric narrow phase.
- No stopping-distance derivation or proof of brake performance.
- No evidence signature/authentication, calibration verification, safety authorization, site permit or motion command.
- No integration that mutates the existing #5106 occupancy book. The returned boxes are a projection for that existing owner.

The bounding sphere must conservatively enclose its assigned robot link, tool or payload in the declared configuration. A producer must supply one consistent primitive set in every time cell. Configuration changes require a new context and a new report.

## Local validation

\`\`\`sh
cargo fmt --manifest-path research/construction-motion-envelope-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-motion-envelope-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-motion-envelope-v0-1/Cargo.toml --locked --all-targets -- -D warnings
cargo bench --manifest-path research/construction-motion-envelope-v0-1/Cargo.toml --locked --bench sweep_scale
\`\`\`

The library has deterministic known-answer tests for three-layer margin expansion and a quarter-circle chord-deviation case, plus hostile tests for temporal and primitive coverage, stale/invalid identity shapes, missing stopping evidence, configuration-dependent canonical identity and arithmetic overflow.

The synthetic benchmark measures envelope construction latency at increasing primitive-time cell counts. No results are claimed until the exact-head CI runner executes; local compilation is not claimed in environments without Rust.

## Claim ceiling

A passing synthetic suite establishes only deterministic arithmetic and structural-envelope behavior under the supplied bounds. It does not establish physical containment, stopping performance, collision freedom, safe separation, construction productivity, standards conformity or vendor interoperability.
