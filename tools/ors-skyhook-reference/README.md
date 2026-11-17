# ORS skyhook turn-force reference v0.1

**Evidence class:** historical source-arithmetic reproduction  
**Tracking:** Symtropy [ORS-DYNAMIC-SUPPORT-001 #5387](https://github.com/Luminous-Dynamics/symtropy/issues/5387)  
**Source:** Paul Birch, “Orbital Ring Systems and Jacob's Ladders — I,” *JBIS* 35 (1982), Eqs. 35, 44, 53, 54 and Table 4.

## Purpose

This small, dependency-free Rust reference reproduces the idealized force required when a moving orbital-ring stream changes direction at a skyhook. It is a missing analytical checkpoint between the fixed circular-stream limit and the much broader moving-support transient programme.

Birch's construction states that the orbital ring is in free fall except at skyhook locations. At a skyhook, a stream with line density (m) and speed (V_0) is turned through a small angle (Delta	heta). The mass flow is (mV_0), the velocity change is approximately (V_0Delta	heta), and the force follows from the momentum rate:
[
F_T = m V_0^2 Delta	heta.
]

The geometry is then reduced to Eqs. 44, 53 and 54. The frozen CSV reproduces every cell in Table 4, including the eight cells shown as dashes because they lie outside the source's stated geometric/small-angle domain.

## Run

From the repository root:

```sh
cargo fmt --manifest-path tools/ors-skyhook-reference/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/ors-skyhook-reference/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path tools/ors-skyhook-reference/Cargo.toml --locked --all-targets

SOURCE_COMMIT="$(git rev-parse HEAD)"
CORPUS_SHA="$(sha256sum tools/ors-skyhook-reference/cases-v0.1.csv | awk '{print $1}')"
cargo run --quiet --manifest-path tools/ors-skyhook-reference/Cargo.toml --locked -- \
  --source-commit "$SOURCE_COMMIT" --corpus-sha256 "$CORPUS_SHA"
```

The executable emits deterministic JSON with the exact source commit and corpus digest. It has no wall-clock timestamp and no third-party Rust dependencies.

## Frozen constants and comparison

The reference preserves the rounded constants printed with Table 4: (R=6{,}400,mathrm{km}), (g=9.81,mathrm{m/s^2}), and (m=2.5pi	imes10^3,mathrm{kg/m}). It compares 36 combinations: (Hin{300,600},mathrm{km}), (pi/alphain{2,4,8,16,32,64}), and (Delta Hin{3,30,300},mathrm{km}).

- 28 table entries reproduce within 2% of the paper's rounded values; the maximum observed relative difference is about 1.38%.
- 8 table dashes are rejected by the source model's own (Delta	hetaleq2alpha) limit / non-positive speed denominator.
- A second calculation path evaluates Eq. 35 after Eqs. 44 and 53, then cross-checks the direct Eq. 54 expression.

The 2% tolerance is only a rounded-literature comparison tolerance. It is not a physical-model uncertainty bound.

## Claim ceiling

This proves only that the equations and rounded table in the 1982 paper are arithmetically reproducible over the frozen grid. It does **not** validate the paper's assumptions or establish:

- a real orbital-ring/skyhook equilibrium;
- magnetic lift, superconducting coil, linear-induction or field-momentum performance;
- distributed ring dynamics, flexible modes, perturbation stability or transient containment;
- cryogenic, thermal, eddy-current or atmospheric-drag predictions;
- a constructible ORS, PORS, spaceport, accelerator support, or safe launch service.

The (8,mathrm{km/s}) velocity used in Birch's drag discussion is not substituted for the geometry-derived (V_0) in Table 4. The separate power-budget audit remains separate.

The T0 inertial circular-stream equation (q_n=mV_0^2/R) gives a net normal resultant for a fixed circular stream; this skyhook reference instead reproduces the local direction-change impulse. Neither is a full ORS model. The next physics gate remains an independently reviewed force and momentum balance for the actual chosen ring/skyhook/control-volume geometry.
