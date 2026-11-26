# SAFRB shared-pump known-answer reference v0.1

**Status:** synthetic algorithm reference; not field evidence or deployment qualification.  
**Owner:** Symtropy #7483, composed with the SAFRB global profile PR #7507.  
**Scenario:** `SAFRB-SHARED-PUMP-2ROUTE-001`.

## Why this case matters

Two terminals feed two logical delivery routes. Both routes traverse the same physical pump, `physical:pump-01`. The logical route labels are not independent capacity. The physical-resource alias map is authoritative for this fixture.

The pump has a conservative lower bound of 8 t and a separate optimistic upper bound of 14 t. The beneficiary obligation is 12 t. The single-commodity lower-bound max flow and minimum cut are both 8 t; the upper-bound max flow is 12 t. The lower-bound shortfall is therefore 4 t.

The result is **INDETERMINATE / NOT GUARANTEED**, not a proof of physical impossibility. Do not promote the optimistic bound to a guarantee.

## Independent verification

Run the standard-library verifier:

```sh
python3 verify_shared_pump.py SAFRB_SHARED_PUMP_KNOWN_ANSWER_V0_1.json
```

The verifier checks the scenario digest, physical-resource aliases, lower/upper max flow, flow conservation, cut membership, max-flow/min-cut equality and hostile mutations. It is test-only; no production Python service is introduced.

## Model and claim boundary

This is only an ordinary single-commodity directed capacitated network with one synthetic commodity, one time window and integer tonnes. It does not model multiple fuel grades, storage, transit delays, pump curves, pressure, nonlinear process constraints, integer scheduling, title, custody, legal authority, metrology, generator conversion, grid feasibility or actual beneficiary service.

Classical max-flow/min-cut is valid only for this declared network model. Multi-commodity or process-coupled LP models need a suitable independently checkable dual/Farkas certificate; integer infeasibility needs a separate proof mechanism or an explicit unproven/indeterminate outcome.

## Cross-project composition

- **Symtropy:** owns this bounded physical-flow witness and obstruction.
- **Mycelix #6692 / #5762:** owns source, authority, semantic fidelity and common-cause dependency evidence; this fixture does not assert legal title or grant access.
- **Symthaea #13696:** may recommend which proof obligation to investigate next; it must not recompute flow or authorize an inspection.
- **Sol-Atlas #106 / #895:** may render the exact nodes, aliases, source maturity and certificate as a read-only projection; a map must not upgrade this result.
- **Power Fabric:** may consume the scoped result only while preserving `FUEL-DELIVERABILITY` and the indeterminate disposition.

## Known-answer digest

The certificate binds the SHA-256 of the scenario object, serialized as UTF-8 JSON with recursively sorted object keys and compact separators. This fixture's digest is recorded in the certificate. This is a local fixture canonicalization rule, **not** a claim of RFC 8785 conformance.

No operational, legal, safety, SOTA or global deployment claim follows from this synthetic case.
