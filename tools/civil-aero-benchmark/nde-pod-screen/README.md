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

The frozen synthetic configuration uses 10 inspectors, at least 60 target-flaw trials and 200 blank trials per inspector, a 0.90 detection lower-bound threshold at 95% one-sided confidence, and a 0.01 false-call upper-bound threshold at 50% one-sided confidence. These are explicit benchmark settings, not universal NDE requirements. In the fixture, each inspector has 59 detections in 60 target-flaw trials and one false call in 200 blank trials. The exact one-sided lower detection bound is approximately 0.923360; the exact one-sided upper false-call bound is approximately 0.008378. Adding one more miss or one more false call causes the corresponding gate to fail.

## Important statistical and standards boundary

This is **not** a POD curve, an `a90/95` analysis, a hit/miss regression, an ASTM E2862-23 implementation, or a MIL-HDBK-1823A-compliant study. It evaluates one fixed target size only; it does not interpolate or extrapolate across flaw sizes, fit a POD curve, model censoring, estimate inspector random effects, account for correlated trials, or verify random specimen selection and population representativeness. The binomial confidence bounds are conditional on independent Bernoulli trials within the declared scope; the software cannot authenticate that assumption.

ASTM E2862-23 is an active hit/miss POD statistical practice. Its scope is broader: the known discontinuity sizes are predictor variables, and an appropriate regression/model relationship is fitted, validated and interpreted for the defined examination parameters. The practice explicitly excludes data analyzed using the Point Estimate Method (PEM), also called the "29 out of 29" method. This tool is not E2862 and must not be presented as equivalent: https://store.astm.org/e2862-23.html.

NASA-STD-5009C establishes NDE requirements in its defined NASA fracture-control context. Its standard-NDE POD study provisions refer to at least 10 inspectors representative of a specific population, individual inspector analysis, false-call reporting and review/approval. This benchmark's 10-inspector minimum is a research guardrail inspired by that context; ten synthetic records are not a representative inspector population, and this software does not demonstrate NASA-STD-5009C compliance: https://standards.nasa.gov/standard/NASA/NASA-STD-5009.

MIL-HDBK-1823A is guidance for NDE system reliability assessment, not itself a requirement: https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=206122.

Accordingly, every receipt hard-codes:
- `physical_pod = NOT_ASSESSED`
- `a90_95 = NOT_ESTIMATED`
- `astm_e2862_compliance = NOT_ASSESSED`
- `mil_hdbk_1823a_compliance = NOT_ASSESSED`
- `inspection_acceptance = NOT_AUTHORIZED`
- `authority_granted = false`

The corpus is synthetic and can only verify software behavior. Replace it with properly designed, representative, blinded physical specimens, qualified procedures, calibrated equipment, competent inspectors, independent flaw truth and an appropriate statistical plan before making any physical detectability claim.

## Frozen scenarios

1. Ten synthetic inspectors meet both exact fixed-target bounds with one miss and one false call each.
2. A second missed target flaw for one inspector breaks the detection lower-bound gate.
3. A second false call for one inspector breaks the false-call upper-bound gate.
4. Nine inspectors fail the predeclared inspector-count minimum.
5. A trial uses a different flaw size and is rejected as out of scope.
6. An inspector has invalid calibration and the study is not evaluable.
7. A duplicate specimen identity is rejected.
8. One inspector has too few blank trials and the study is not evaluable.

The independent Python oracle recomputes exact binomial tails and per-inspector bounds from the same frozen bytes; CI compares numerical outputs and dispositions to the Rust implementation. This is independent implementation agreement, not independent experimental evidence.

## Run

Requires Rust 1.96+ and Python 3.12+:

```sh
cargo fmt --manifest-path tools/civil-aero-benchmark/nde-pod-screen/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civil-aero-benchmark/nde-pod-screen/Cargo.toml
cargo clippy --locked --manifest-path tools/civil-aero-benchmark/nde-pod-screen/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/civil-aero-benchmark/nde-pod-screen/Cargo.toml -- \
  tools/civil-aero-benchmark/nde-pod-screen/fixtures/v0.1/corpus.json \
  1111111111111111111111111111111111111111 > /tmp/nde-screen-rust.json
python3 tools/civil-aero-benchmark/nde-pod-screen/tests/reference_oracle.py \
  --corpus tools/civil-aero-benchmark/nde-pod-screen/fixtures/v0.1/corpus.json \
  --rust-output /tmp/nde-screen-rust.json
```

No solver, NDE instrument, aircraft, machine or physical test is invoked.
