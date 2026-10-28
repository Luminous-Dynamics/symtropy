# Synthetic recoverability frontier fixture

This fixture is a small deterministic contract test, not a simulation result or flight dataset.

It registers two independent placeholder families, two training seeds, three checkpoints per flight, two interventions, and four branch seeds: 2 × 2 × 3 × 2 × 4 = 96 rows. The continue arm fails at the final checkpoint. The fallback arm succeeds at that checkpoint for family-a, while one of four branches fails for family-b. This makes the one-sided Wilson lower-bound gate distinguish a robustly recoverable checkpoint from a fragile point estimate.

The 64-character hexadecimal state/tape fields are synthetic consistency tokens designed to test paired branch alignment. They are not hashes of captured simulator state or disturbance files and must never be presented as provenance for a real run. Real experiment generation must hash the canonical serialized snapshot and exogenous disturbance tape.

Expected synthetic oracle behavior: the fallback arm's mean latest recoverability checkpoint exceeds the continue arm's mean, with no claim about real dynamics or control benefit.
