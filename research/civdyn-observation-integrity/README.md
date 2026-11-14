# CIVDYN Observation Integrity Sweep

This isolated Rust package exercises the candidate-facing observation-integrity triage in `src/lib.rs`.

## Run

From the repository root:

```bash
cargo test --manifest-path research/civdyn-observation-integrity/Cargo.toml
cargo run --manifest-path research/civdyn-observation-integrity/Cargo.toml --bin observation-integrity-sweep
```

The sweep emits tab-separated case results and a summary. A non-zero exit status means at least one frozen expected disposition or error state did not match.

## Frozen cases

The sweep covers:
- telemetry-only observational equivalence and correct abstention;
- independent physical agreement and disagreement;
- shared source, gateway and trust-root common causes;
- excessive event-to-ingestion lag, calibration mismatch and unauthenticated primary telemetry;
- derived reconstruction, instrument self-test and forensic-integrity evidence;
- duplicate IDs, missing primary telemetry and inverted event/ingestion time;
- evidence-order invariance.

The library's unit tests remain the finer-grained contract suite. The binary is a deterministic runnable smoke/evidence harness, not a substitute for the library tests.

## Important scope limits

This package consumes already-constructed `EvidenceRecord` and `AdmissionPolicy` values. In this version:

- `authenticated` is an upstream boolean assertion; this crate does not verify signatures or source keys.
- `source_digest` is required to be non-empty; this crate does not fetch bytes or verify a content digest.
- The current `fresh` predicate compares `ingestion_tick - event_tick` with `maximum_age_ticks`. It bounds event-to-ingestion lag; it does **not** calculate observation age at a later decision cutoff. The CLI names this quantity accordingly.
- Independence is conservatively screened using source, gateway and trust-root identifiers only. It does not prove organizational, software, calibration, operator, upstream-data or physical-measurement independence.
- Ticks, the integer `service_index`, tolerance and synthetic identity strings are fixture units, not calibrated physical quantities.
- A disposition is a software triage result, not physical truth, cyber attribution, instrument qualification, safety authority or permission to act.

Those limitations are intentional. The next integration work should bind this contract to owner-qualified manifests, richer dependency closure, domain-specific time/space/calibration profiles and an independent oracle without creating a second evidence or authority system.

## Composition

- CIVDYN observation integrity: [Symtropy #4819](https://github.com/Luminous-Dynamics/symtropy/issues/4819)
- Prior-free contingent sensing: [Symtropy #4832](https://github.com/Luminous-Dynamics/symtropy/issues/4832)
- Cross-domain scientific observability: [Mycelix #6141](https://github.com/Luminous-Dynamics/mycelix/issues/6141)
- Symthaea investigation loop: [Symthaea #12075](https://github.com/Luminous-Dynamics/symthaea/issues/12075)

The sweep uses synthetic fixtures only. It makes no claim about field sensor performance or a scientific discovery.
