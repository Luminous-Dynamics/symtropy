# Orbital Interface Conformance Benchmark v0.1

**Status:** executable synthetic reference; no physical interface has been tested.  
**Owner:** [ORBITAL-INTERFACE-CONFORMANCE-001 #6329](https://github.com/Luminous-Dynamics/symtropy/issues/6329).  
**Claim ceiling:** synthetic profile-compatibility semantics only.

## Purpose

This tool checks whether a provider's declared interface envelope contains a client's complete requirements for a prepared mechanical capture/retention interface and, optionally, a prepared storable-fluid service interface.

A standard name, interface label or matching revision is not enough. Compatibility is evaluated over two exact, versioned profiles, the project ICD, frame, configuration and evidence envelope.

## Standards context

The benchmark is informed by, but does not replace:

- NASA's [2026 ISAM State of Play](https://www.nasa.gov/isam/), which catalogues capabilities, facilities and developers.
- NASA Civil Space Shortfall 512, which identifies cooperative interfaces, aids and standards as an ISAM interoperability need.
- [International Docking System Standard](https://internationaldeepspacestandards.com/) / IDD for its defined docking interface.
- [ISO 24330:2022](https://www.iso.org/standard/78463.html) for programmatic RPO and on-orbit servicing principles/practices.
- [ANSI/AIAA S-157-2025](https://webstore.ansi.org/standards/aiaa/ansiaiaa1572025) for prepared storable-fluid transfer.

These cover different interface and programme layers. S-157 does not prescribe a single physical connector form factor; project-specific form, fit and function remain in the project ICD. A reference to a standard is not proof of certification, conformity, or physical mating.

## Decision rule

The central condition is:

`client required envelope ⊆ provider guaranteed envelope`

For v0.1 the evaluator requires exact project ICD identity/revision, configuration identities, frame identity, geometry profile and typed canonical SI units. A revision difference is indeterminate unless an explicit, evidence-scoped compatibility map is supplied. A frame difference is indeterminate; no guessed transform is applied.

Full range containment is mandatory. Partial overlap is rejection, not compatibility. Capture and fluid are independent gates: a capture-only profile does not prove a fluid connection.

The accepted units are canonical SI values:
- metres (m), metres/second (m/s), newtons (N);
- kelvin (K), pascals (Pa), kilograms (kg).

Upstream adapters must perform any conversion explicitly and preserve its provenance. The evaluator does not guess units.

## Frozen corpus and output

The fixture contains 30 synthetic cases. It includes full-envelope positive controls, mechanical limit failures, same-standard/different-ICD, explicit revision mapping, frame mismatch, partial fluid-range overlap, fluid class/identity/phase/connector errors, missing/stale evidence, invalid digest/source commit, unit mismatch, unsupported claim escalation, inverted range, receiver-mass shortfall and self-compatibility negatives.

Run:

```bash
cargo run --manifest-path tools/orbital-interface-conformance-bench/Cargo.toml -- \
  tools/orbital-interface-conformance-bench/fixtures/v0.1/cases.json \
  --source-commit <40-hex-commit>
```

The CLI emits a deterministic JSON receipt that binds the exact fixture bytes with SHA-256 and reports case-level expected/actual dispositions. A successful corpus replay means the evaluator matched the frozen synthetic oracle; it does not mean every interface is compatible.

## Status meanings

- `compatible_synthetic_profile`: the declared synthetic requirements are contained in the synthetic provider envelope.
- `rejected`: a declared incompatibility, malformed value, invalid unit or claim escalation was detected.
- `indeterminate`: required identity, mapping, evidence scope or validity is missing/unsupported.

No result grants operational, docking, valve, propulsion, safety, mission or command authority.

## Adversarial shortcut baselines (same frozen corpus)

A separate baseline replay measured how three intentionally weak compatibility rules behave on these exact 30 synthetic pairs. Here, a **false-compatible** result includes an oracle-rejected or oracle-indeterminate pair that the shortcut accepts.

| Shortcut baseline | Baseline accepts | False-compatible | False-incompatible |
|---|---:|---:|---:|
| Shared standard label only | 30 | 27 | 0 |
| Matching project ICD + geometry profile IDs only | 28 | 25 | 0 |
| Fluid temperature/pressure interval overlap only (capture-only skips fluid) | 28 | 25 | 0 |

These are synthetic adversarial-corpus results, not measured industry failure rates and not a comparison against a vendor's production conformance system. They show why no single label, identifier or overlap check can stand in for full-envelope compatibility plus identity, evidence and scope checks.

## Benchmarks and competitive comparison

The first baselines are deliberately weak:
1. label-only / standard-name matching;
2. interval-overlap-only matching;
3. exact-ID-only matching.

The frozen negatives are constructed so these shortcuts cannot establish correct compatibility. Report false compatibility, false incompatibility, indeterminate count, reason distribution and limiting envelope dimensions separately. Never collapse safety, physical qualification, interoperability and economics into a single score.

Prospective software targets:
- zero false compatibility on frozen critical negatives;
- 100% agreement with the independently hand-calculated expected dispositions;
- deterministic outcomes under case/profile ordering;
- no missing, stale or out-of-domain evidence promoted to compatibility;
- no claim promotion above synthetic profile conformance.

A later competitive evaluation should use identical profile pairs, requirement envelopes, evidence availability, compute budgets and expected outcomes for Luminous, a standards-only lookup baseline, a label-only baseline and a range-overlap baseline. Report precision/recall and especially false-compatibility count; abstention is preferable to unsafe acceptance.

## Integration and boundaries

Composes with:
- [#6167 ISAM capability crosswalk](https://github.com/Luminous-Dynamics/symtropy/issues/6167);
- [#6221 orbital service receipt composition](https://github.com/Luminous-Dynamics/symtropy/pull/6221);
- [#6256 ground test cell](https://github.com/Luminous-Dynamics/symtropy/issues/6256) and [draft specification PR #6303](https://github.com/Luminous-Dynamics/symtropy/pull/6303);
- [#12903 capture handoff](https://github.com/Luminous-Dynamics/symthaea/pull/12903);
- [#6182 fluid transfer](https://github.com/Luminous-Dynamics/symtropy/pull/6182).

The future physical pathway is a subset of mechanically and fluidically instrumented coupons in the proposed uncrewed ground test cell. That requires separately approved hardware, competent safety review, calibrated instruments, independent measurement and a physical evidence package.

Not modelled or qualified here: docking dynamics, seals, leakage, structural integrity, cryogenic/hypergolic/high-pressure transfer, materials compatibility, physical metrology, standard certification, producer authentication, ground-hardware operation, flight readiness, operational availability or service economics.
