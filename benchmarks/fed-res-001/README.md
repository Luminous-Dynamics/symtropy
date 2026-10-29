# FED-RES-001 consumer-side reference kernel

This directory contains a small, dependency-free Rust consumer-side regression guard and the draft composition profile for the Federated Defensive Resilience Benchmark. It is intentionally a separate Cargo workspace so its checks do not pull in the much larger Symtropy/Bevy workspace.

## Compose existing work — do not duplicate it

- Mycelix [CIV-RES-000](https://github.com/Luminous-Dynamics/mycelix/issues/2006) owns civic authority/privacy/evidence composition.
- Mycelix [LEX-NET-Q1](https://github.com/Luminous-Dynamics/mycelix/issues/1057) and related LEX-NET issues own cross-sovereign recognition, revocation/currentness, equivocation, policy-conflict and interoperability qualification.
- Mycelix [CIV-IMPACT-001](https://github.com/Luminous-Dynamics/mycelix/issues/5046) owns composed-stack incremental-value comparison, baseline/ablation discipline and preregistration.
- Mycelix [BANK-RES-001](https://github.com/Luminous-Dynamics/mycelix/issues/5302) owns critical-operation pilot and partner-specific go/no-go criteria.
- Mycelix [LOG-COMMONS-S0-001](https://github.com/Luminous-Dynamics/mycelix/issues/4901) owns the deterministic offline logistics twin and independent verifier.
- Symtropy [BANK-RES-SIM-001](https://github.com/Luminous-Dynamics/symtropy/issues/2330) owns critical-service disruption/recovery simulation; Sol-Atlas [BANK-RES-GEO-001](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/204) and Symthaea [BANK-RES-DECISION-001](https://github.com/Luminous-Dynamics/symthaea/issues/8206) own adjacent evidence and analysis seams.

FDRB measures end-to-end service consequences after an upstream disposition enters the simulation. It must not become a second federation authority engine, trust resolver, logistics twin, or composed-stack value leaderboard. The integration does not yet exist; issue and PR status is not proof of implementation or qualification.

## Run locally

```sh
cargo test --manifest-path benchmarks/fed-res-001/Cargo.toml --offline
```

## Included consumer-side checks

The 11 authored unit tests cover:

- local recovery remains possible during a federation partition when an upstream disposition permits it;
- cross-organization publication is rejected during partition;
- model output cannot directly apply consequential effects;
- consequential effects require an upstream disposition;
- denied/indeterminate upstream dispositions are not treated as permission;
- a disposition for one scope cannot be reused for a different effect;
- duplicate applied effects are detected;
- missing upstream receipt references fail the consumer gate;
- recovery plans that overrun resource budgets are rejected;
- plans below a declared minimum service capacity are rejected;
- observations and analysis proposals can be recorded without an authority receipt.

## Explicit limitations

This kernel is a consumer-side guard over synthetic trace fields. It does **not** validate Mycelix signatures, legal authority, evidence truth/currentness, policy negotiation, revocation semantics, or source independence; those remain upstream ownership and qualification responsibilities. The synthetic `UpstreamDisposition` is not a signed production receipt.

The operational scenario corpus, independent service/recovery oracle, paired baseline comparison, exact adapter, and machine-readable end-to-end result receipt are not implemented yet. Passing these unit tests does not establish production readiness, absence of vulnerabilities, regulatory compliance, or performance superiority.

Tests were authored but not executed in the current environment because neither `cargo` nor `rustc` is installed.
