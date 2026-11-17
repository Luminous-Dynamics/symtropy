# ORS skyhook turn-force reference v0.1

**Evidence class:** historical source-arithmetic reproduction  
**Tracking:** Symtropy [ORS-DYNAMIC-SUPPORT-001 #5387](https://github.com/Luminous-Dynamics/symtropy/issues/5387)  
**Source:** Paul Birch, “Orbital Ring Systems and Jacob's Ladders — I,” *JBIS* 35 (1982), Eqs. 35, 44, 53, 54 and Table 4.

## Purpose

This small, dependency-free Rust reference reproduces the idealized force required when a moving orbital-ring stream changes direction at a skyhook. It is an analytical checkpoint between the fixed circular-stream limit and the broader moving-support transient programme.

Birch treats the orbital ring as free-falling except at skyhook locations. At a skyhook, a stream with line density m and speed V0 is turned through a small angle DeltaTheta. The mass flow is m * V0, the velocity change is approximately V0 * DeltaTheta, and the momentum-rate force is:

~~~text
F_T = (m * V0) * (V0 * DeltaTheta)
    = m * V0^2 * DeltaTheta
~~~

The geometry is then reduced to Eqs. 44, 53 and 54. The frozen CSV reproduces every cell in Table 4, including the eight cells shown as dashes because they lie outside the source's stated geometric/small-angle domain.

## Run

From the repository root:

~~~sh
cargo fmt --manifest-path tools/ors-skyhook-reference/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/ors-skyhook-reference/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path tools/ors-skyhook-reference/Cargo.toml --locked --all-targets
SOURCE_COMMIT="$(git rev-parse HEAD)"
CORPUS_SHA="$(sha256sum tools/ors-skyhook-reference/cases-v0.1.csv | awk '{print $1}')"
cargo run --quiet --manifest-path tools/ors-skyhook-reference/Cargo.toml --locked -- --source-commit "$SOURCE_COMMIT" --corpus-sha256 "$CORPUS_SHA"
~~~

The executable emits deterministic JSON with the exact source commit and corpus digest. It has no wall-clock timestamp and no third-party Rust dependencies.

## Frozen constants and comparison

The reference preserves the rounded constants printed with Table 4:

- Earth radius: 6,400 km
- Surface gravity: 9.81 m/s^2
- Ring line density: 2.5 * pi * 10^3 kg/m

It compares all combinations of:

- Skyhook height H: 300 km and 600 km
- pi/alpha: 2, 4, 8, 16, 32 and 64
- Perigee drop DeltaH: 3 km, 30 km and 300 km

Results:

- 28 numeric table entries reproduce within 2% of the paper's rounded values; the maximum observed relative difference is about 1.38%.
- 8 table dashes are classified as outside the source model's stated domain using its turning-angle bound and speed denominator.
- A second calculation path evaluates Eq. 35 after Eqs. 44 and 53, then cross-checks the direct Eq. 54 expression.

The 2% threshold is only a rounded-literature comparison tolerance. It is not a physical-model uncertainty bound.

## Claim ceiling

This proves only that the equations and rounded table in the 1982 paper are arithmetically reproducible over the frozen grid. It does **not** validate the paper's assumptions or establish:

- a real orbital-ring/skyhook equilibrium;
- magnetic lift, superconducting coil, linear-induction or field-momentum performance;
- distributed ring dynamics, flexible modes, perturbation stability or transient containment;
- cryogenic, thermal, eddy-current or atmospheric-drag predictions;
- a constructible ORS, PORS, spaceport, accelerator support, or safe launch service.

The 8 km/s velocity used elsewhere in Birch's drag discussion is not substituted for the geometry-derived V0 in Table 4. The separate power-budget audit remains separate.

The T0 inertial circular-stream equation q_n = m * V0^2 / R gives a net normal resultant for a fixed circular stream; this skyhook reference instead reproduces the local momentum-rate force at the turn. Neither is a full ORS model. The next physics gate remains an independently reviewed force and momentum balance for the actual chosen ring/skyhook/control-volume geometry.
