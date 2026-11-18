# Lunar granular-flow similarity envelope

This small Rust utility compares two declared granular-flow profiles across seventeen caller-defined dimensionless coordinates and hard categorical boundaries. It is a screening tool for experimental similitude—not a physics solver and not a lunar qualification certificate.

## Run

```sh
cargo fmt --manifest-path tools/lunar-granular-flow-similarity/Cargo.toml -- --check
cargo test --manifest-path tools/lunar-granular-flow-similarity/Cargo.toml
cargo clippy --manifest-path tools/lunar-granular-flow-similarity/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/lunar-granular-flow-similarity/Cargo.toml -- examples/synthetic-similarity-input.json
```

Provide the JSON path as the first argument or pipe JSON to stdin.

## Coordinates compared

- Rotational Froude: Fr = ω²R/g.
- Cohesion-to-particle-weight (granular Bond): Bo_m = F_adh/(m_particle g).
- Wall-adhesion-to-particle-weight: Aw = F_wall_adhesion/(m_particle g).
- Inertial number: I = γdot d / sqrt(P/ρ).
- Signed electrostatic projection: El_s = (q/m) E_axis/g, where E_axis is the measured signed field component along the declared electrostatic reference axis. This is the signed projection of the paper's magnitude ratio El = |q|E/(mg); the reference axis is an exact-match categorical boundary. Do not compare signs unless the same axis convention is recorded.
- Particle-to-drum size: d50/R.
- Wall-roughness-to-particle size: Ra/d50.
- Fill fraction, gravity and chamber pressure.
- Vibration acceleration ratio Γ, dimensionless frequency f sqrt(d50/g), and peak displacement/d50.
- Exact-match categorical boundaries: atmosphere, wall material, wall grounding and flow regime.

`Bo_m` and `Aw` are proposed operational similarity coordinates. Their force numerators, representative particle mass, contact sampling and uncertainty must be recorded by the experimenter; the evaluator does not infer them from angle of repose. The signed electrostatic number preserves polarity: equal magnitudes with opposite signs are outside the envelope.

## Comparison semantics

Positive-valued coordinates use symmetric multiplicative deviation: max(candidate/target, target/candidate) <= caller-declared factor tolerance. Fill fraction uses an absolute difference tolerance. The signed electrostatic coordinate requires equal polarity before comparing magnitude. Categorical fields require exact equality.

Overall results are fail-closed:
- `within_declared_similarity_envelope`: every required coordinate is present and within tolerance;
- `outside_declared_similarity_envelope`: at least one known coordinate is outside or categorical boundary differs;
- `indeterminate_missing_coordinate`: no known mismatch proves an out-of-envelope state, but a required coordinate is missing.

No single aggregate similarity score is emitted. Every coordinate and tolerance remains visible. A Froude match cannot compensate for a mismatch or missing `Bo_m`, `Aw`, `I`, signed `El`, or a vibration coordinate.

## Claim ceiling

A within-envelope result means only that supplied coordinates satisfy caller-declared tolerances. It does not prove physical equivalence, constitutive equivalence, lunar gravity, vacuum/electrostatic equivalence, model validity or hardware qualification. The synthetic example contains no physical data and uses analyst-selected tolerances.
