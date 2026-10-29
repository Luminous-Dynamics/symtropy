# FED-RES-001 reference kernel

This directory is a **small, dependency-free Rust reference model** for the first safety invariants in the Federated Defensive Resilience Benchmark. It is intentionally a separate Cargo workspace so its checks do not pull in the much larger Symtropy/Bevy workspace.

Run locally:

```sh
cargo test --manifest-path benchmarks/fed-res-001/Cargo.toml --offline
```

## Covered invariants

- Local recovery can proceed under a federation partition when the local action is otherwise valid.
- A federation-wide fact cannot be published while the federation link is partitioned.
- A model actor cannot directly perform authority-bearing effects.
- Revoked credentials cannot authorize recovery.
- Disputed evidence cannot be published as a verified fact.
- Claim promotion requires corroboration and two fixture-declared independent sources.
- Disclosure cannot exceed recipient clearance.
- Replaying an already-applied consequential effect is detected.
- An unverified observation can be recorded without being promoted to authority.
- Model analysis can be proposed without authorization power.

## Explicit limitations

This kernel is not a production access-control service, consensus algorithm, distributed database, incident detector, or proof of source independence. It checks the state recorded in synthetic trace events. The independent scenario oracle still needs to validate the event ground truth, issuer/dependency roots, source independence, state transitions, and expected recovery outcomes.

Passing these unit tests does not establish production readiness, absence of vulnerabilities, compliance, or performance superiority. The 12-family / 360-seed benchmark corpus, paired baselines, dependency/resource simulator, and machine-readable run receipts remain later stages.
