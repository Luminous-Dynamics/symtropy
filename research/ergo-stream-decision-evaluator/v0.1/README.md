# ErgoStream Decision Evaluator v0.1

**Status:** bounded evaluator reference; exact-head qualification pending.  
**Candidate policy:** Symtropy `research/ergo-stream-decision-sufficient-probes-v0.1` / draft PR #7558.  
**Research contract:** Symtropy #7555.  
**Owner composition:** Symthaea #13401 / #13402; the production blind research-policy adapter remains #11171.

## Purpose

This package independently scores a frozen candidate decision-disambiguation problem and result against evaluator-owned truth.

It addresses a failure mode that a candidate-only policy cannot detect: a learner can appear decision-sufficient simply by omitting a valid alternative hypothesis, or can predict that a probe separates decisions when the actual hidden outcome supports do not.

## Trust boundary

The candidate supplies only:
- candidate-local hypothesis IDs and candidate-predicted decision classes;
- declared probe options and costs;
- candidate-predicted set-valued outcome supports;
- its frozen policy result.

The owner/evaluator supplies separately:
- the complete admissible witness set for the current evidence prefix;
- evaluator-owned downstream decision classes;
- an evaluator-only binding from candidate-local IDs to canonical witness identities, issued by an independent structural validator;
- actual set-valued probe-outcome supports for each admissible witness.

The binding, admissible witness set, decision labels and actual outcome supports must never be serialized into the candidate view or returned to the candidate before the freeze boundary. This crate does not itself provide OS isolation or episode custody; those remain owner-service responsibilities.

## Independent scorecard

The evaluator reports separate denominators and counts for:
- correctly bound, falsely excluded and falsely included hypotheses;
- false decision-equivalence merges and false decision splits;
- missed and spurious predicted probe outcomes;
- actual baseline decision disagreement and actual worst-case residual disagreement after a proposed probe;
- false decision-sufficiency claims;
- probes that do not reduce true decision disagreement;
- unnecessary abstention when a legal, affordable, truth-discriminating probe exists.

Decision sufficiency is valid only when the candidate covers the complete evaluator-admissible witness set and all evaluator-admissible witnesses imply the same declared decision. Candidate self-report is not sufficient.

A selected probe is credited only when evaluator truth shows that its possible outcomes reduce worst-case cross-decision disagreement. An observation outside every candidate prediction is still a model-invalidation outcome in the candidate reducer; it must not be coerced to a nearest class.

## Validation protocol

### Differential and resource-bound extensions

The current source extends the original finite fixtures with:

- 729 worlds across every three-witness decision partition and non-empty two-outcome support pattern;
- 9,261 worlds across every three-witness decision partition and non-empty three-outcome support pattern;
- 91,125 worlds across every three-witness decision partition and non-empty four-outcome support pattern;
- an identifier/order metamorphic test that renames candidate, decision, probe, outcome and witness IDs and reverses rows;
- explicit evaluator-truth bounds: at most 65,536 admissible witnesses and 1,048,576 truth outcome-support rows, with an early probe × witness pair bound before the missing-support scan;
- checked accumulation for missed and spurious outcome-support metrics.

The brute-force oracle enumerates pairs independently of the evaluator's grouped-count implementation. These tests cover small finite set-valued worlds; they do not establish probabilistic calibration, continuous sensor likelihoods, runtime acquisition, or production blind custody.

**Exact source-head qualification:** exact commit `e05307002b70d3f391f4799dca32ef7021997386` passed the Rust 1.96 push workflow [#45948576423](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45948576423): formatting, evaluator tests (including the 91,125-world four-outcome oracle and truth-resource bounds) and strict Clippy all passed. The branch subsequently received this README-only update; the PR-triggered workflow for the documentation-only tip is separate. Earlier formatter-only failures were corrected before this passing source-head run.


Tiny deterministic fixtures first:
1. complete witness coverage and a truly discriminating probe;
2. false decision sufficiency caused by omitting an admissible witness;
3. false decision-equivalence and false-split errors;
4. a candidate-predicted useful probe whose actual outcome sets do not separate decisions;
5. unnecessary abstention where an affordable truth-discriminating probe exists;
6. missing/spurious candidate outcome supports.

Run:
```sh
cargo fmt --manifest-path research/ergo-stream-decision-evaluator/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-decision-evaluator/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-decision-evaluator/v0.1/Cargo.toml --all-targets -- -D warnings
```

## Limitations

- The evaluator consumes a canonical witness binding from an independent validator; it does not implement general structural witness validation.
- The finite reference uses exact sets and pair counts, not probabilities or calibrated EVSI.
- Ground-truth outcome supports are set-valued and finite; continuous/noisy measurement likelihoods require a separately qualified uncertainty model.
- The current score checks whether a probe can reduce worst-case decision disagreement, not whether the observation physically executes or arrives successfully.
- Resource accounting for actual attempts, timeouts, failures, energy and custody must be joined from the episode owner ledger.
- This is not yet connected to a real Symthaea child policy or a production blind episode service.

No SOTA, real-world acquisition gain, causal discovery, hardware synchronization or production authority is claimed.
