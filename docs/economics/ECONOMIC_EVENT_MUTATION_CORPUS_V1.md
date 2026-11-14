# Symtropy EconomicEvent mutation corpus

## Positive controls

- deterministic event with complete replay identity is admissible as simulation evidence;
- repeated replay produces the same source event identity;
- TEND adapter preserves instrument identity;
- Valueflows adapter preserves EconomicEvent semantics;
- Integral adapter preserves COS source provenance;
- accounting adapter remains a derived projection.

## Negative controls

- same event_id with different simulation_build_id;
- same event_id with mutated resource quantity;
- tick changed without replay identity change;
- wall-clock timestamp substituted for simulation tick;
- missing causal parent for derived market event;
- simulated ownership promoted to legal ownership;
- TEND balance promoted to external currency;
- simulation production promoted to physical production;
- replay match promoted to independent verification;
- foreign recognition rewrites local origin;
- adapter correction mutates source event;
- duplicate settlement across adapters;
- market acceptance without authorization;
- indeterminate transport treated as settled.

## Required trace

source_event_id -> adapter_profile -> target_id -> recognition/settlement record

The trace must be reversible to the original simulation envelope.
