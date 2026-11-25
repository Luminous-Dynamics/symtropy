# RRVR Matched Benchmark Protocol v0.2

This protocol keeps native task/safety and policy-recovery results separate from RRVR execution/recovery assurance. It defines matched-arm requirements, separate scorecards for fault validity, physical-effect assurance, temporal closure, command semantics, reproducibility/diagnosis and latency, plus claim ceilings by evidence tier.

See [RRVR_MATCHED_BENCHMARK_PROTOCOL_V0.2.md](RRVR_MATCHED_BENCHMARK_PROTOCOL_V0.2.md).

## Known-answer fixture

`fixtures/matched_benchmark_pairs_v0_2.json` contains two synthetic matched pairs. It is not an executed benchmark and carries no measured score. The strict schema is `schemas/matched_benchmark_pairs_v0_2.schema.json`.

## Claim ceiling

The manifest validates only the structure and pairing commitments supplied. It does not prove that a campaign ran, that the fault was realized, that an oracle is independent, or that a physical robot is safe.
