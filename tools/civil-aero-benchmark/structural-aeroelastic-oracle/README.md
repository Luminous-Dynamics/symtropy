# AER-01 structural / aeroelastic analytical oracle v0.1

**Parent work:** [Symtropy #4093](https://github.com/Luminous-Dynamics/symtropy/issues/4093)  
**Research contract:** [PR #4094](https://github.com/Luminous-Dynamics/symtropy/pull/4094)  
**Status:** executable synthetic known-answer suite; not an aircraft analysis or certification tool.

## Purpose

This small dependency-free Rust executable makes the first structural-bridge verification targets concrete. A separate Python standard-library verifier independently recomputes the same frozen cases and checks the Rust output. The two implementations do not import one another.

It is intentionally not a finite-element method, computational fluid dynamics solver, aircraft sizing model, or substitute for NASTRAN, SU2, OpenMDAO, or qualified engineering review.

## Included known-answer cases

1. **Cantilever beam:** Euler-Bernoulli prismatic beam under a tip force and uniform distributed load. Reports root bending moment, tip slope/deflection and extreme-fibre bending stress. Assumes small deflection, linear elasticity and constant EI.
2. **Idealized wing box:** two concentrated caps for bending inertia plus a single-cell, uniform-thickness, closed thin-wall Bredt-Batho torsion approximation. Skin/stringer bending contribution, warping, shear-centre offsets, joints, local buckling and combined failure interaction are deliberately absent.
3. **Spanwise load transfer:** linear interpolation from point loads onto ordered span stations. It transfers force vectors and free couples and checks resultant force and moment about the declared beam-axis origin. It is a 1-D spanwise fixture, not a general unstructured 3-D surface-to-volume mapper.
4. **Static aeroelastic fixed point:** a scalar synthetic load/deflection feedback law with a closed-form solution. A gain-1.2 hostile case must exhaust its iteration budget and remain NON_CONVERGED.

The fixture identity contains five independent digest slots: configuration, geometry, loading state, mission phase and environment. Any mismatch rejects the composed evidence identity. The digests in this corpus are synthetic sentinels; they do not authenticate a source or aircraft.

## Run locally

From the repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/structural-aeroelastic-oracle/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/structural-aeroelastic-oracle/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/structural-aeroelastic-oracle/Cargo.toml > /tmp/aerostructural-oracle.json
python3 tools/civil-aero-benchmark/structural-aeroelastic-oracle/verify_independent.py \
  --rust-output /tmp/aerostructural-oracle.json
~~~

Running the verifier without --rust-output checks the Python oracle and its hostile controls only. It deliberately reports rust_python_boundary=not_run; this mode must not be represented as a successful Rust/Python cross-check.

A dedicated GitHub Actions workflow runs formatting, Rust unit tests, the Rust executable and the independent Python boundary check. The workflow is evidence for this exact code head only.

## Locked synthetic reference values

| Case | Quantity | Expected value |
|---|---|---:|
| Beam | Root moment | 5,600 N m |
| Beam | Tip deflection | 0.1980952380952381 m |
| Beam | Tip slope | 0.07238095238095238 rad |
| Beam | Root bending stress | 140 MPa |
| Wing box | Bending second moment | 4.6875e-5 m4 |
| Wing box | Bending stress | 32 MPa |
| Wing box | Thin-wall torsion constant | 7.941176470588236e-5 m4 |
| Wing box | Torsional wall stress | 0.555556 MPa |
| Wing box | Tip twist | 4.663923182441701e-4 rad |
| Load transfer | Force residual | 0 N (within floating-point tolerance) |
| Load transfer | Moment residual | 0 N m (within floating-point tolerance) |
| Static coupling | Fixed-point deflection | 0.125 m |
| Static coupling | Feedback gain | 0.2 |
| Hostile coupling | Feedback gain | 1.2; must be NON_CONVERGED |

The verifier uses scale-aware tolerances for recomputed numeric results. The exact decimal strings are not intended as universal engineering tolerances.

## Hostile controls

Rust unit tests and the Python verifier reject or preserve as non-passing:
- zero/non-positive stiffness or wall thickness;
- duplicate, reversed or insufficient target stations;
- source loads outside the structural span;
- stale geometry/configuration identity;
- force or moment imbalance introduced after transfer;
- a divergent aeroelastic fixed point reported as converged;
- any attempt to promote this bounded corpus to aircraft feasibility.

The fixture is not permitted to emit a blanket structurally_safe=true or aircraft-feasibility pass.

## Qualification and claim ceiling

This corpus verifies hand-checkable arithmetic and a few fail-closed interface behaviors. It does **not** establish:
- full-aircraft design-load coverage or limit/ultimate load compliance;
- structural adequacy, margin of safety, material allowables or manufacturing tolerances;
- buckling, local instability, joints, damage tolerance or crashworthiness;
- dynamic aeroelastic stability, flutter, buffet or control reversal;
- fatigue, damage accumulation, inspection intervals or service life;
- low-speed propeller/wing interaction or Wren 2 / Wayfarer 4 performance;
- DPW-8/AePW-4 reproduction, solver validity or experimental correlation;
- airworthiness, certification, permission to manufacture, test or fly.

Passing the synthetic suite is only Tier 0/1 arithmetic and software-contract evidence. Candidate-specific engineering requires controlling geometry, material/joint/load-path data, applicable design loads, appropriate specialist solvers, independent review and physical correlation under the applicable authority-approved process.
