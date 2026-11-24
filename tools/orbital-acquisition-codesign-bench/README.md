# Orbital Acquisition Co-Design Benchmark v0.1

Synthetic exact finite schedule feasibility and Pareto-frontier reference.

The standalone Rust evaluator enumerates a finite set of supplied path-segment speed choices and per-characteristic acquisition options. It consumes supplied upstream evidence-gate states and receipt digests; it does not compute calibration, image motion, photometry, sampling density, feature conditioning, metrology or robot kinematics.

Semantics:
- Choose one speed for every path segment and one option for every required event.
- An option is bound to its exact segment and speed; mismatched choices are infeasible.
- All required gates must PASS. FAIL is infeasible. UNKNOWN is indeterminate, never treated as PASS.
- Adjacent speed transitions must appear in the explicit allowed-transition list.
- Robot energy must leave the declared return reserve. Illumination and projector energy are separate budgets.
- The finite product is enumerated exactly up to max_schedules. An exceeded cap returns SEARCH_LIMIT_NOT_PROVEN.
- The frontier minimizes elapsed time, robot motion energy, illumination energy, projector energy and retakes separately.
- Any indeterminate schedule makes the frontier INDETERMINATE because it could change nondominance. Known feasible candidates are retained but exactness_proven=false.
- Each schedule receipt carries the exact path, speed, transition and gate evidence digests it consumed.

Run:
- cargo fmt --manifest-path tools/orbital-acquisition-codesign-bench/Cargo.toml -- --check
- cargo test --locked --manifest-path tools/orbital-acquisition-codesign-bench/Cargo.toml
- cargo clippy --locked --manifest-path tools/orbital-acquisition-codesign-bench/Cargo.toml --all-targets -- -D warnings
- cargo run --quiet --locked --manifest-path tools/orbital-acquisition-codesign-bench/Cargo.toml --bin orbital-acquisition-codesign-bench -- docs/research/orbital-truss/fixtures/acquisition-codesign-v0_1.json 0123456789abcdef0123456789abcdef01234567
- python3 tools/orbital-acquisition-codesign-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/acquisition-codesign-v0_1.json 0123456789abcdef0123456789abcdef01234567

See Symtropy #7385 and docs/research/orbital-truss/acquisition-codesign-v0_1.md. This synthetic benchmark does not establish physical inspection quality, metrology, POD, safety or certification.
