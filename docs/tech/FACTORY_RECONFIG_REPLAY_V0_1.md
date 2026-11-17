# Factory Reconfiguration Replay v0.1

**Owner:** Symtropy deterministic reconfiguration chronology and replay.  
**Consumer:** Symthaea [MULTIPURPOSE-FACTORY-001 #12355](https://github.com/Luminous-Dynamics/symthaea/issues/12355).  
**Tracking:** [FACTORY-RECONFIG-ACCEPTANCE-001 #5411](https://github.com/Luminous-Dynamics/symtropy/issues/5411).  
**Status:** pure Rust replay validator and synthetic case corpus.  
**Claim ceiling:** synthetic transition conformance only; no physical production authority.

## 1. Acceptance definition

The relevant interval is:

**last accepted output under source configuration A → first independently accepted output under target configuration B**

This is not equal to the time required to move a machine or change a fixture. The full sequence covers WIP and material-lot disposition, safe stop, electrical/pressure/fluid/thermal isolation, module and utility changes, environmental segregation, resource reservation, recipe/product/controller revisions, tooling, calibration, qualified workforce, commissioning, safety interlocks, first-article inspection and stable restart.

A digital-twin first-article prediction is not a physical first-article acceptance. The validator returns only a synthetic disposition and always leaves physical acceptance unresolved and production authorization false.

## 2. Existing owner composition

Compose rather than replace:
- Symtropy #2354 — factory architecture/capacity economics.
- #2412 — sequence and changeover disruption.
- #2439 — multi-family robustness.
- #2444 — asset lifecycle/reconfiguration options.
- #4875/#5311/#4865 — textile process/factory execution and observability examples.
- Symthaea #7677 — capability-constrained schedule proposal.
- Symthaea #12355/#12358 — multipurpose capability contract and synthetic architecture/product/module corpus.

This crate is not a scheduler, MES/ERP, machine controller, digital twin or product-quality authority.

## 3. State sequence

The frozen ordered sequence is:
1. DrainWip
2. SafeStop
3. Isolate
4. ChangeModules
5. LoadConfiguration
6. CalibrateAndStaff
7. Commission
8. FirstArticle
9. StableRestart

Each step carries an exact event ID, phase, outcome, synthetic elapsed-time ticks and evidence flags. Every step retains material/product genealogy. Out-of-order or duplicate steps are rejected.

Terminal outcomes:
- Unknown blocks further mutation until reconciliation. Any subsequent step after an unknown outcome is rejected and remains mutation-blocked.
- A failed step requires verified rollback or returns RollbackRequired.
- Failed commissioning and rejected first article cannot progress to target acceptance.
- An incomplete nonterminal trace is blocked; it is not accepted as a partial configuration.

## 4. Independent gates

The replay checks:
- exact source/target configuration, product revision, process-route revision and time-basis references;
- WIP/lot disposition and genealogy continuity;
- safe stop and separate electrical, pressure/fluid and thermal/chemical isolation;
- module/utility compatibility, environment segregation and no double allocation;
- current recipe, product revision and controller configuration;
- valid tooling/fixtures, current calibration and qualified staffing;
- commissioning, safety interlocks and process-limit checks;
- qualified inspector and completed first-article acceptance;
- stable service after the first accepted target product.

The input digests are opaque identities. The validator checks syntax and supplied equality; it does not resolve/authenticate canonical external artifacts.

## 5. Rust implementation

The tools/factory-reconfig-replay crate is dependency-free and pure Rust. It has no process, network, filesystem, machine-control or solver-execution interface.

A passing result is AcceptedSynthetic, not physical acceptance. Every receipt preserves:
- source/target configuration identities;
- completed-step count and synthetic elapsed ticks;
- first-article disposition;
- physical acceptance = Unresolved;
- production authorization = false.

UnknownOutcome sets further mutation authorization to false. A later event after an unknown outcome is also rejected without reopening the mutation gate.

## 6. Synthetic corpus

The factory-reconfig-replay-cases-v0.1.json corpus covers:
- a complete accepted synthetic transition;
- stale source/target/product/process references;
- missing WIP disposition and broken genealogy;
- safe-stop and hazard-isolation omissions;
- incompatible modules/utility ports and segregation;
- stale recipe, calibration and controller configuration;
- missing qualified staff;
- commissioning failure and first-article reject/hold;
- stable-service failure;
- unknown outcome and illegal mutation afterwards;
- failed-step rollback and overflow.

The corpus is synthetic. It does not claim real changeover durations, factory capacity, product quality, safety compliance or economic performance.

## 7. CI and validation status

The crate is included in the existing cross-platform Symtropy CI and has a focused Rust 1.96.0 workflow for formatting, Clippy with warnings denied and unit tests. Status is determined only on the exact head SHA. A queued or skipped job is not a pass.

## 8. Claim ceiling

A passing synthetic replay establishes only deterministic transition-order and evidence-gate behavior for the supplied trace. It does not establish actual site/equipment compatibility, workplace safety, product quality, first-article physical acceptance, permitted operation, ROI, supplier reliability or off-Earth production readiness.

## References

- Symthaea capability contract: https://github.com/Luminous-Dynamics/symthaea/issues/12355
- Symthaea draft capability corpus: https://github.com/Luminous-Dynamics/symthaea/pull/12358
- NIST IR 8620: https://doi.org/10.6028/NIST.IR.8620
- ISO 23247-5:2026: https://www.iso.org/standard/87425.html
- ISO 23247-6:2026: https://www.iso.org/standard/87426.html
