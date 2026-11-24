# Translation Swept AABB v0.1

A dependency-free Rust research crate that constructs conservative swept AABBs for a narrow, explicitly declared motion model: **fixed-orientation linear translation** over the represented Symtropy semi-implicit Euler / CCD step.

## Why this model

The current Symtropy integrator updates linear velocity, applies exponential damping and a speed clamp, then advances position using that **pre-quantization post-integration velocity** times dt. The physics step captures pre-integration positions and documents the CCD path as a straight-line sweep from those positions using the **post-quantization velocity**. Under `deterministic-net`, position and velocity are quantized to Q16.16 after the integrator has already advanced position.

The producer therefore consumes both velocity snapshots instead of re-integrating forces. Native-binary64 mode requires the snapshots to match bit-for-bit. Q16.16 mode permits the expected difference and includes three translated endpoint sets:
- the integrator's pre-quantization path `p_start + v_integrator * dt`;
- the CCD pass's post-quantization path `p_start + v_ccd * dt`; and
- the observed post-position endpoint, which can differ due to position quantization or a CCD time-of-impact clamp.

The swept AABB is the coordinate-wise union of the starting shape AABB translated over these paths. For a fixed-orientation shape and linear translation, the union of translated AABBs is bounded by the minima and maxima of the endpoint translations. The observed post-position subtraction is itself outward-enclosed before being composed with the starting AABB. Non-zero translated endpoints are expanded outward by one representable binary64 value. Arithmetic that cannot produce finite outward bounds forces fallback.

## Supported and unsupported cases

Supported:
- finite, ordered world-space starting AABB;
- finite start position, post-position, pre-quantization integrator velocity and post-quantization CCD velocity;
- finite non-negative dt;
- known numeric profile (`NativeBinary64` or `Q16_16`);
- post-integration angular-velocity norm squared at or below the integrator's (10^{-24}) rotor-update threshold, which means the current integrator does not update orientation during this step.

Unsupported and never certified:
- angular norm squared above the rotor-update threshold;
- unknown quantization profile;
- velocity-times-dt overflow;
- observed endpoint-delta overflow;
- outward-bound overflow.

Invalid state, non-finite endpoints, malformed AABBs and invalid time values are rejected.

The receipt binds body/collider/subshape identity, world revision, step, dt, integrator/profile labels, exact starting AABB and exact observed state bit patterns, including both velocity snapshots. Native-binary64 mode rejects mismatched velocity snapshots. Labels are declarative provenance, not signatures or authenticated producer identity. Replay recomputes the envelope against the supplied state.

## Claim boundary

This crate does not implement continuous collision detection, time-of-impact solving, rotation bounds, shape-local AABB extraction, broadphase integration, or a proof that caller-supplied state snapshots are authentic. It does not authorize production pruning. Sampling in tests is a regression oracle, not the mathematical proof. The conservative argument applies only to the stated fixed-orientation translation model.

The first integration must still:
1. source the starting world AABB from an independently qualified shape-bound provider;
2. bind the state snapshot at the correct point in the real physics step;
3. join the resulting envelope to #7307's AABB separation checker and #7255's pair lifecycle ledger;
4. compare all candidate omissions against an independent time-of-impact oracle.

## Tests

The tests cover positive, negative and multi-axis translation, stationary motion, observed post-position divergence, rotor-threshold fallback, unknown quantization fallback, Q16.16 context binding and distinct velocity snapshots, native-mode velocity mismatch rejection, non-finite and negative time, state errors, multiplication/outward-rounding overflow, replay invalidation, 3D behavior, and 256 deterministic randomized 2D cases with 101 samples along each of the three represented translation segments.

## Run

```sh
cargo fmt --manifest-path research/translation-swept-aabb/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/translation-swept-aabb/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/translation-swept-aabb/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## External benchmark qualification

Use the [NYU Continuous Collision Detection benchmark project](https://continuous-collision-detection.github.io/), which provides handcrafted degeneracies, simulator-generated queries, rounded queries and a full time-of-impact dataset with analytic ground truth. Its recent scalable-CCD work compares 13 algorithms over linearized trajectories and includes broadphase and narrowphase evaluation. This is the right external benchmark for broadphase recall, TOI accuracy, false positives, false negatives and runtime.

Keep the following suites separate:
- supplied-AABB checker replay and corruption tests (#7307);
- fixed-orientation swept-envelope containment and fallback tests (this crate);
- the NYU CCD broad/narrowphase benchmark with its analytic TOI ground truth;
- full physics step outcomes and solver stability.

Promotion targets (not results): 100% recall against analytic TOI ground truth on the supported motion/model subset; zero invalid inputs resulting in a certificate; zero rotation/unknown-profile/overflow cases certified; report fallback and false-positive rates separately; median swept-envelope volume inflation <=2.0x versus the reference swept envelope; p50/p95/p99 producer time; and at least 20% lower narrowphase work on two held-out workloads with <=5% p95 end-to-end regression and no false negatives.
