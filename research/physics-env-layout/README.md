# Symtropy independent environment state-layout benchmark — V0

This isolated Rust research package compares **AoS**, **SoA** and **AoSoA** memory layouts for a frozen independent-environment kernel.

It exists to answer a narrow question raised by [PHYS-GPU-001 #3866](https://github.com/Luminous-Dynamics/symtropy/issues/3866): how much do packed state layouts change CPU data locality and scheduling cost before a GPU port is justified?

## Run

From this directory:

```sh
cargo test
cargo run -- 10000 100
cargo bench --bench layout_throughput
```

The CLI emits a deterministic receipt after comparing the full canonical per-environment f64 bit-pattern snapshots across all layouts. SHA-256 digests are included as compact receipt identifiers; they are not the correctness oracle. The benchmark separates scalar loops from persistent Rayon pools. Cloning/initialization is in Criterion's setup closure, outside the timed operation.

## Frozen kernel

For each independent environment and each axis:

```
velocity = velocity + acceleration * dt
position = position + velocity * dt
```

Initial values are a deterministic function of stable environment ID. There are no collisions, contacts, constraints, reductions, random draws or interactions between environments.

## Layouts

- **AoS:** `Vec<EnvironmentState>`.
- **SoA:** separate contiguous vectors for position, velocity, acceleration and IDs.
- **AoSoA:** tiles of structure-of-arrays fields with lane widths 4, 8 and 16.

The scalar operation sequence is shared, and no cross-environment floating-point reductions occur. Results are canonicalized by ID and compared using exact f64 bit patterns.

## Benchmark matrix

- 1K, 10K, 100K and 1M independent environments.
- 20 repeated steps per timed batch at 120 Hz.
- Scalar AoS, SoA and AoSoA-4/8/16.
- Persistent Rayon worker pools at 1, 2, 4 and 8 workers for AoS, SoA and AoSoA-8.

The benchmark applies one environment-level parallel dispatch per step in every layout, uses persistent Rayon pools, and passes the mutated state through the same optimization barrier. Initialization/cloning is outside the timed operation. It reports environments-steps/second via Criterion element throughput. It does not yet report hardware cache counters or GPU metrics; those require a supported profiling environment. Record compiler, target CPU features, OS, worker count and power/thermal state alongside any published result.

## Correctness gates

- All layouts produce exact per-ID state equality on the frozen scalar kernel.
- AoS, SoA and AoSoA parallel outputs are exact with their scalar references.
- Exact comparisons use the canonical `u64` f64 bit patterns, not only digest equality.
- Permuted input order canonicalizes to the same digest.
- Duplicate, missing, unexpected and empty campaigns fail closed.
- Non-finite or non-positive timestep values are rejected.
- AoSoA tail blocks are exercised by non-multiple fixture sizes.

## Claim ceiling

This is **synthetic-only memory-layout and scheduling evidence**. It is not a contact solver, full rigid-body benchmark, GPU implementation or physical validation result. No throughput superiority is claimed until the benchmark is run on a named machine with repeated timing windows, exact source/toolchain identity and the same correctness gate for every layout.

The initial target for considering a production layout change is a repeatable >=10% improvement on at least two workload sizes with no exact-state regression. If the result is smaller or within noise, retain the existing production representation.
