# Symtropy Free-Flow Modular Structure Lab

This is a standalone, standard-library-only Rust geometry/topology scaffold for Free-Flow Modular Fractal Structures (FFMFS).

It generates a deterministic 3D recursive branching/truss graph with:
- stable node/member identifiers;
- a declared reusable hub/interface family;
- bounded depth and node count;
- paired branch members and sibling cross-braces;
- graph connectivity and cycle-rank checks;
- member-length and bounding-box metrics;
- deterministic same-build, non-cryptographic regression digest.

## Run

cargo test --manifest-path research/freeflow-structure-lab/Cargo.toml
cargo run --manifest-path research/freeflow-structure-lab/Cargo.toml --bin freeflow-structure

## Important claim boundary

This crate checks geometry and graph topology only. It does not calculate stress, strain, stiffness, buckling, fatigue, vibration, fracture, thermal behavior, manufacturing tolerances, connector strength, human factors or flight qualification.

The generated graph is a candidate geometric grammar, not a demonstrated fractal material or safe load-bearing structure. Graph connectivity and cycle rank are not structural redundancy proof.

The next step is to add independently verified truss/beam analysis and matched physical coupon experiments. Do not connect the output to safety-critical design decisions.
