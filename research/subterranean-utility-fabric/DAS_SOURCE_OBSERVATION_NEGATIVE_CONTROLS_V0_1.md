# DAS Source-Observation Interchange — Negative Controls v0.1

The JSON Schema is a structural envelope. The following semantic tests must be implemented by an independent validator; a schema pass alone does not prove all rules.

| ID | Mutation | Expected result | Reason |
|---|---|---|---|
| N01 | Replace strain-rate unit with a strain unit without changing phenomenon | Reject | Phenomenon and unit semantics conflict |
| N02 | Use channel pitch as declared spatial resolution | Reject/unsupported | Channel pitch is sample spacing, not resolution |
| N03 | Emit geographic coordinates from optical range | Reject | Optical range is not geographic position |
| N04 | Set axis kind to physical fibre chainage without calibration reference | Reject | Chainage requires an exact calibration reference |
| N05 | Mark fibre identity identified but omit endpoint identities | Reject | Identified span requires endpoint binding |
| N06 | Mark data raw_vendor_native but include parent processing artifacts | Reject | Raw source cannot claim parent-derived lineage |
| N07 | Mark derivative but omit parent digest or transformation parameter digest | Reject | Derivation is not replayable |
| N08 | Use an invalid byte digest or change bytes without changing digest | Reject after byte verification | Metadata digest format is not byte-integrity verification |
| N09 | Declare time unsynchronized but provide a precise finite offset | Reject/require adjudication | Contradictory time-quality claim |
| N10 | Rights state unknown but permitted purposes include production use | Reject/indeterminate | No inferred use permission |
| N11 | Claim quality valid while sample gaps/outages are unaccounted | Indeterminate | Missing samples are not clean negative observations |
| N12 | Claim a route revision but do not bind the fibre revision | Indeterminate | Route/fibre relationship is not version-complete |
| N13 | Normalize vendor units without preserving original value/unit | Reject | Source semantics were destroyed |
| N14 | Present CAMOS electrical current as DAS strain-rate | Reject | Distinct sensing modality and quantity |
| N15 | Treat source digest as proof of publisher identity or physical truth | Reject claim | Digest binds bytes only |
| N16 | Public audience requests restricted critical route geometry | Deny | Disclosure/authorization remains owner controlled |

## Claim ceiling

These are proposed negative controls. They have not yet been executed by a schema validator, Rust semantic validator, live API or vendor adapter.
