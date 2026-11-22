# AER-01 fixed-target NDE hit/miss screen v0.1

**Tracking:** [AERO-INSPECTION-OBSERVABILITY-001 / #4165](https://github.com/Luminous-Dynamics/symtropy/issues/4165)  
**Parent work:** [inspection observability contract / PR #4167](https://github.com/Luminous-Dynamics/symtropy/pull/4167)  
**Claim ceiling:** `fixed_target_binomial_screening_only`  
**Corpus:** synthetic known-answer and hostile fixtures; no physical POD evidence is included.

## What this adds

The existing inspection-observability contract handles exact scope, coverage, calibration currentness, out-of-domain methods and sensor-vs-damage abstention. This sub-crate adds a deliberately small statistical seam: for a **single predeclared flaw size** and one fixed inspection scope, it computes one-sided exact binomial confidence bounds separately for each inspector:

- lower confidence bound on detection probability from hit/miss trials;
- upper confidence bound on false-call probability from blank trials;
- per-inspector screening against predeclared thresholds;
- exact method/procedure/material/defect/orientation/environment/scope binding;
- equipment and calibration identity checks, unique specimen IDs, per-inspector sample minima and deterministic SHA-256 receipt.

The frozen synthetic configuration uses 10 inspectors, at least 29 target-flaw trials and 69 blank trials per inspector, a 0.90 detection lower-bound threshold at 95% one-sided confidence, and a 0.01 false-call upper-bound threshold at 50% one-sided confidence. These are explicit benchmark settings, not universal NDE requirements. With zero misses in 29 trials, the exact one-sided lower bound is approximately 0.901855; with zero false calls in 69 blanks, the exact one-sided upper bound is approximately 0.009995. One miss or one false call breaks the corresponding fixture gate.

## Important statistical and standards boundary

This is **not** a POD curve, an `a90/95` analysis, a hit/miss regression, an ASTM E2862-23 implementation, or a MIL-HDBK-1823A-compliant study. It evaluates one fixed target size only; it does not interpolate or extrapolate across flaw sizes, fit a POD curve, model censoring, estimate inspector random effects, account for correlated trials, or verify random specimen selection and population representativeness. The binomial confidence bounds are conditional on independent Bernoulli trials within the declared scope; the software cannot authenticate that assumption.

ASTM E2862-23 is an active hit/miss POD statistical practice, but it specifies a broader analysis and validation scope; this tool must not be represented as equivalent to it: https://store.astm.org/e2862-23.html. NASA-STD-5009C provides NDE requirements in its defined NASA fracture-control context and calls for a standard POD study with at least 10 inspectors representative of the intended population, individual inspector analyses, and review/approval; its context is not a universal rule for every aircraft or NDE programme: https://standards.nasa.gov/standard/NASA/NASA-STD-5009. MIL-HDBK-1823A is guidance for NDE system reliability assessment, not itself a requirement: https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=206122.

Accordingly, every receipt hard-codes:
- `physical_pod = NOT_ASSESSED`
- `a90_95 = NOT_ESTIMATED`
- `astm_e2862_compliance = NOT_ASSESSED`
- `mil_hdbk_1823a_compliance = NOT_ASSESSED`
- `inspection_acceptance = NOT_AUTHORIZED`
- `authority_granted = false`

The corpus is synthetic and can only verify software behavior. Replace it with properly designed, representative, blinded physical specimens, qualified procedures, calibrated equipment, competent inspectors, independent flaw truth and an appropriate statistical plan before making any physical detectability claim.

## Frozen scenarios

1. All ten synthetic inspectors meet both exact fixed-target bounds.
2. One missed target flaw breaks the detection lower-bound gate.
3. One false call breaks the false-call upper-bound gate.
4. Nine inspectors fail the predeclared inspector-count minimum.
5. A trial uses a different flaw size and is rejected as out of scope.
6. An inspector has invalid calibration and the study is not evaluable.
7. A duplicate specimen identity is rejected.
8. One inspector has too few blank trials and the study is not evaluable.

The independent Python oracle recomputes the exact binomial tails and per-inspector bounds from the same frozen bytes; CI compares numerical outputs and dispositions to the Rust implementation. This is independent implementation agreement, not independent experimental evidence.

## Run

Requires Rust 1.96+ and Python 3.12+:

```sh
cargo fmt --manifest-path tools/civil-aero-benchmark/nde-pod-screen/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civil-aero-benchmark/nde-pod-screen/Cargo.toml
cargo clippy --locked --manifest-path tools/civil-aero-benchmark/nde-pod-screen/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/civil-aero-benchmark/nde-pod-screen/Cargo.toml -- \
  tools/civil-aero-benchmark/nde-pod-screen/fixtures/v0.1/corpus.json \
  <40-lowercase-hex-source-commit> > /tmp/nde-screen-rust.json
python3 tools/civil-aero-benchmark/nde-pod-screen/tests/reference_oracle.py \
  --corpus tools/civil-aero-benchmark/nde-pod-screen/fixtures/v0.1/corpus.json \
  --rust-output /tmp/nde-screen-rust.json
```

No solver, NDE instrument, aircraft, machine or physical test is invoked.
