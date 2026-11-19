# Geothermal–Fusion Evidence Receipts v0.1

**Tracking:** [GEOFUSION-EVIDENCE-RECEIPTS-001 #6158](https://github.com/Luminous-Dynamics/symtropy/issues/6158)  
**Parent:** [GEOFUSION-INTEGRATION-001 #6142](https://github.com/Luminous-Dynamics/symtropy/issues/6142)  
**Core arithmetic contract:** [symtropy-geofusion-contracts PR #6154](https://github.com/Luminous-Dynamics/symtropy/pull/6154)

## Purpose

Bind every geothermal / fusion / storage energy stream to exact raw source bytes, measurement or solver lineage, timebase, uncertainty and claim ceiling.

**A digest is evidence of byte identity, not physical truth.** A passing schema check cannot certify calibration, resource quality, solver validity, safety or operation.

## Files

- `schemas/raw-source-interval-v0.1.schema.json`: heat source-interval payload with thermal power and temperature grade.
- `schemas/raw-electrical-source-interval-v0.1.schema.json`: electrical source-interval payload with electrical power.
- `schemas/evidence-envelope-v0.1.schema.json`: common receipt for source intervals, solver results, derived streams and allocations. Source-interval envelopes require explicit `subject.asset_id` and `subject.interval_id`.
- `schemas/source-allocation-receipt-v0.1.schema.json`: evidence-bound, single-source/single-carrier/single-interval allocation report.
- `schemas/source-interval-batch-manifest-v0.1.schema.json`: bounded manifest for up to 256 source-interval bundles.
- `fixtures/`: two adjacent synthetic heat intervals plus one independent same-clock electricity interval, each with raw payload, evidence envelope and source-allocation receipt; also a batch manifest and null-asset negative schema control.
- `STANDARDS_INTEROPERABILITY.md` and `STANDARDS_INTEROPERABILITY_MATRIX.v0.1.json`: external-standard layer crosswalk and pinned machine-readable owner/claim boundaries.

## Integrity and truth are separate axes

- `artifact.integrity_state = verified` means the runtime verifier was supplied the raw payload bytes, recomputed SHA-256 and byte length, and matched the declaration. The CLI reads local files; it does not resolve or authenticate remote artifact locators.
- `evidence_class = measured` means the source claims a physical measurement; it still requires meter/instrument identity, calibration validity, aggregation method and timebase.
- `model.applicability = applicable` records a solver identity/version, exact input/configuration manifests, property model, convergence state and raw output digest.
- `uncertainty.status = unknown` is valid and carries a reason. Unknown covariance must not silently become zero.
- `review.independent_review_status` and optional Mycelix approval reference are review metadata, not proof that the physical observation is true.

## Ownership

- Symtropy's Rust contracts own arithmetic and carrier/service/source-allocation validation.
- This evidence envelope owns the interchange shape and source artifact references; the runtime verifier in `research/geofusion-evidence-verifier/` recomputes SHA-256 over exact raw payload bytes and checks source/allocation joins.
- Symtropy #1703 owns plasma solver-run capability/state receipts.
- Symtropy #5400/#5333 own covariance and dependence; link their exact receipts instead of implementing another covariance engine.
- Sol-Atlas owns resource and spatial evidence. Mycelix owns provenance and approvals. Symthaea consumes receipts read-only and must preserve stale / UNKNOWN / OOD states.

## Schema limitations

JSON Schema validates field shape, primitive types and some conditional requirements. It cannot prove:
- digest equality against an external payload;
- source artifact availability or freshness;
- equality between a source envelope's digest and the allocation receipt's digest;
- sums of allocation edges or carrier/service compatibility across records;
- measurement accuracy, calibration truth or model physical validity.

Those are runtime verifier / independent arithmetic duties. Do not treat `format: date-time` as clock synchronization evidence.

## Validation performed

- All five Draft 2020-12 schema documents, the batch manifest, nine source/evidence/allocation fixture files, the null-asset negative fixture and the standards matrix were fetched back from the exact branch and parsed as JSON.
- The raw payload fixture is 523 bytes. SHA-256 was recomputed over the exact fetched payload bytes and matched the evidence envelope and allocation receipt: `0cdcab2b9625b3669918761b7a4da378e9d344321301f2dd0bb48feb67b788c8`.
- Manually checked required time-window, artifact, uncertainty, lineage and claim-ceiling fields, plus source digest linkage and the 10 MWh allocation total.
- The Rust runtime verifier implements exact-byte hashing before parsing, source asset/boundary/interval/carrier/time joins, heat/electric power-duration-energy consistency, allocation identity checks, first-law arithmetic replay and batch-scoped duplicate/overlap prevention. It now has 43 authored unit tests and single-record plus batch CLI modes (maximum 256 source intervals per batch). The batch key canonicalizes strict UTC timestamps to integer nanoseconds; it rejects both duplicate and overlapping intervals for the same source asset, boundary and carrier, while allowing adjacent intervals. CI checks all five Draft 2020-12 schemas, positive heat/electric source/evidence/allocation fixtures, a three-item mixed-carrier batch manifest and a null-asset negative control using pinned `check-jsonschema` 0.39.0, then runs Rust 1.96 formatting, Clippy, tests, single CLI and batch CLI. These checks are queued and have not passed yet; no local Cargo run is claimed. The raw-payload schemas now cover separate heat and electricity profiles with carrier-specific power fields; other carriers remain future work. The schemas remain v0.1 drafts and need validator-backed tests before production use.

A PASS in this fixture means only that the synthetic artifact can be bound to the declared bytes and arithmetic. No source values represent a real geothermal site or fusion plant.


Runtime verifier: [research/geofusion-evidence-verifier/README.md](../../../../research/geofusion-evidence-verifier/README.md). It is a bounded source-interval verifier and batch-local uniqueness check, not a full schema engine or persistent archive registry. Standards composition is documented in [STANDARDS_INTEROPERABILITY.md](STANDARDS_INTEROPERABILITY.md), with a pinned [machine-readable interoperability matrix](STANDARDS_INTEROPERABILITY_MATRIX.v0.1.json).
