# FED-RES-001 reference kernel

This directory contains a small, dependency-free Rust consumer-side regression guard and the draft composition profile for the Federated Defensive Resilience Benchmark. It is intentionally a separate Cargo workspace so its checks do not pull in the much larger Symtropy/Bevy workspace.

## Compose existing work — do not duplicate it

- Mycelix [CIV-RES-000](https://github.com/Luminous-Dynamics/mycelix/issues/2006) owns civic authority/privacy/evidence composition.
- Mycelix [LEX-NET-Q1](https://github.com/Luminous-Dynamics/mycelix/issues/1057) and its related LEX-NET issues own cross-sovereign identity, recognition, revocation, equivocation, policy-conflict, and interoperability qualification.
- Mycelix [CIV-IMPACT-001](https://github.com/Luminous-Dynamics/mycelix/issues/5046) owns composed-stack incremental-value comparison, baseline/ablation discipline and preregistration.
- Mycelix [BANK-RES-001](https://github.com/Luminous-Dynamics/mycelix/issues/5302) owns the critical-operation pilot and partner-specific go/no-go criteria.
- Mycelix [LOG-COMMONS-S0-001](https://github.com/Luminous-Dynamics/mycelix/issues/4901) owns the deterministic offline logistics twin and independent verifier.
- Symtropy [BANK-RES-SIM-001](https://github.com/Luminous-Dynamics/symtropy/issues/2330) owns critical-service disruption/recovery simulation; Sol-Atlas [BANK-RES-GEO-001](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/204) and Symthaea [BANK-RES-DECISION-001](https://github.com/Luminous-Dynamics/symthaea/issues/8206) own adjacent evidence and analysis seams.

FDRB should measure end-to-end service consequences after an upstream disposition enters the simulation. It must not become a second federation authority engine, trust resolver, logistics twin, or composed-stack value leaderboard. Issue and PR existence is not proof that a workstream is implemented or qualified.

## Run locally

```sh
cargo test --manifest-path benchmarks/fed-res-001/Cargo.toml --offline
```

## Covered consumer-side negative checks

- A valid local recovery effect can proceed during a federation partition.
- A shared federation fact is not published while partitioned.
- A model actor cannot directly produce authority-bearing effects.
- Revoked credentials cannot authorize a protected effect.
- Disputed evidence cannot be published as verified fact.
- Claim promotion requires corroboration and two fixture-declared source roots.
- Disclosure cannot exceed the recipient's declared clearance.
- Replay of an already-applied consequential effect is detected.
- A low-assurance observation can be captured without being elevated to authority.
- Model analysis can be proposed without authorization power.

## Explicit limitations

This kernel is a consumer-side guard over synthetic trace fields, not a production access-control service, consensus protocol, distributed database, incident detector, or proof that upstream source roots are independent. It must not be treated as the canonical owner of Mycelix's federation semantics. As the adapters mature, reuse Mycelix's qualified receipts and frozen negative controls, then replace overlapping local checks with conformance tests against those contracts.

The full 12-family operational scenario corpus, independent service/recovery oracle, paired baseline comparison and machine-readable end-to-end result receipt are not implemented yet.

Passing these unit tests does not establish production readiness, absence of vulnerabilities, regulatory compliance, or performance superiority. Tests were authored but not executed in the current environment because neither `cargo` nor `rustc` is installed.
