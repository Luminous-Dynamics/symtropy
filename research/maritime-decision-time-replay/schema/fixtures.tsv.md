# MDTR fixture column contract v0.1

The dependency-free executable reads `fixtures/episodes.tsv`. The first line is a comment/header; each subsequent record is exactly 23 tab-separated fields. Tabs and newlines are prohibited inside scalar values.

| Index | Field | Type | Meaning |
|---:|---|---|---|
| 0 | case_id | string | Immutable case identifier |
| 1 | cutoff | signed integer seconds | Decision-time cutoff |
| 2 | case_subject | string | Canonical subject expected by the episode |
| 3 | source_id | string | Source identity, not proof of independence |
| 4 | event_id | string | Source-native event identity |
| 5 | event_subject | string | Subject bound to this event |
| 6 | event_time | signed integer seconds | Claimed/physical event time; may be in the future |
| 7 | published_at | signed integer seconds | Publisher visibility time |
| 8 | retrieved_at | signed integer seconds | Receiving-system retrieval time |
| 9 | use_from | signed integer seconds | Start of permitted decision-use interval |
| 10 | use_until | signed integer seconds | End of permitted decision-use interval |
| 11 | rights_clear | boolean | Whether this intended use is rights-cleared |
| 12 | physical | enum | Evaluator-only physical result: PASS, FAIL, UNKNOWN |
| 13 | authority | enum | Evaluator-only authority result |
| 14 | participant_floor | enum | Evaluator-only protected service-floor result |
| 15 | expected_units | unsigned integer | Total expected container count |
| 16 | accepted | unsigned integer | Receiver-accepted container count |
| 17 | in_transit | unsigned integer | In-transit container count |
| 18 | outstanding | unsigned integer | Known outstanding container count |
| 19 | unresolved | unsigned integer | Explicitly unresolved container count |
| 20 | duplicates | unsigned integer | Duplicate container count detected by evaluator |
| 21 | expected_disposition | enum | Hand-authored expected result |
| 22 | expected_reason | string | Stable ordered reason code(s), joined by `|` |

All cargo columns are in **container units**, not TEU, tonnes, moves or vessel counts. This is a deliberately narrow synthetic fixture. Real ledgers must additionally model splits, merges, aliases, transfer handoffs, loss events and typed units.

## Temporal rule

Visibility requires rights clearance, exact subject binding, publication and retrieval at or before cutoff, and cutoff within the decision-use interval. Event time is deliberately not required to precede cutoff: a future scheduled event can already be known.

## Claim boundary

The TSV contains both policy-visible source metadata and evaluator-only fields so that one deterministic executable can test the separation. Production benchmark runners MUST materialize policy-view and evaluator-truth artifacts separately using the two JSON schemas. The example policy receives only the Rust `PolicyInput` type; it does not receive `ReplayCase` or `EvaluatorTruth`.
