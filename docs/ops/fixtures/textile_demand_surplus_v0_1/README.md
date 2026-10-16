# Textile demand and surplus fixtures v0.1

Files:
- demand_scenarios.schema.json: JSON Schema draft 2020-12 contract.
- demand_scenarios.example.json: three synthetic cases for rare-use couture,
  workwear and performance/costume production.

The schema mirrors the typed scenario contract in the Rust demand module, but
the Rust evaluator does not load JSON directly. Treat this directory as a
fixture specification, not evidence of parser compatibility. A future loader
requires independent schema validation and a round-trip test.

All case values and references are synthetic:
- fixture evidence IDs do not authenticate forecasts, orders, inventory condition
  or process intensities;
- acceptance-rate calculations use conservative integer-floor rounding, which
  can undercount accepted output on small lots; physical pilots must record
  observed integer good/rejected units;
- inventory-first outputs are counterfactual until stock issue/transfer receipts
  establish that stock was actually used;
- no second use, virgin-production displacement, carbon saving or regulatory
  conformity is inferred.

Freeze scenario IDs, units, assumptions and exact manifest bytes before a
benchmark run. Record the manifest SHA-256 and tested commit with the results.
Do not silently normalize or rewrite a measured manifest without revising its
version and digest.
