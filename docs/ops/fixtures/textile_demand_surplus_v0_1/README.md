# Textile demand and surplus fixtures v0.1

This directory contains:
- demand_scenarios.schema.json: JSON Schema draft 2020-12 contract.
- demand_scenarios.example.json: three synthetic, paired cases for rare-use
  couture, workwear, and performance/costume production.

The schema mirrors the typed scenario contract in the Rust demand module. The
Rust evaluator does not yet load this JSON directly, so the example is a
machine-readable fixture specification, not evidence of parser compatibility.
A future loader needs independent schema validation and round-trip tests.

Important interpretation boundaries:
- The fixtures are synthetic and unweighted, not a sample of market demand.
- Evidence strings beginning with fixture: are identifiers, not authenticated
  observations.
- Acceptance-rate calculations use conservative integer-floor rounding. For
  small lots this can undercount accepted units; a real pilot must use observed
  integer QA pass/reject counts.
- Inventory-first results are counterfactual allocations until physical issue,
  transfer, resale, or second-use receipts establish that they occurred.
- No carbon, avoided virgin production, environmental benefit, or regulatory
  conformity is inferred from this schema.

Freeze scenario IDs, units, input assumptions and exact manifest bytes before a
benchmark run, and record the SHA-256 plus commit SHA in the result artifact.
Do not silently normalize or rewrite a scenario file after measuring it.
