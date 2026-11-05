# Construction Change Impact and Requalification — v0.1

**Status:** typed change/evidence graph contract and deterministic Rust evaluator. Not a safety case, engineering authority, acceptance authority, robot controller or certification.

## Purpose

Given an approved, revision-bound dependency graph and a typed change event, determine which evidence/claims must be rebuilt, remeasured, revalidated, requalified or reviewed. A claim may be Unaffected only when the declared dependency closure is complete, current and authority-backed and every relevant dependency edge explicitly covers the change kind.

Unknown is a valid engineering outcome. Missing dependency information must never be interpreted as no dependency.

## Why this is a field-level opportunity

A 2026 BIM–robot lifecycle review of 58 studies reports 46 with data/semantic integration, five with model feedback and none meeting its qualifying cross-stage reuse criterion. The gap is not simply exchanging model fields: downstream applications must know whether earlier evidence is still applicable after design, site, robot, measurement or acceptance changes.

Other comparators:
- [VDA 5050 v3.0](https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN) supports mixed-fleet mobile robot communication, free-navigation zones and path sharing. It does not define construction acceptance or lifecycle evidence invalidation.
- [ISO 10218-1:2025](https://committee.iso.org/standard/73933.html) and [ISO 10218-2:2025](https://committee.iso.org/standard/73934.html) are published industrial robot / application safety standards; applicability to a specific construction/mobile system must be established by competent authorities.
- [ISO/DIS 19650-1:2026](https://www.iso.org/standard/89703.html) is under development, not a published replacement. Revision metadata alone does not prove an information-management process.
- [IFC 4.3.2.0](https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/) is the latest official IFC 4.3 publication. IFC is not a complete process, capability, safety or acceptance contract.
- [ISARC 2026 product-process graph transformation](https://www.iaarc.org/publications/2026_proceedings_of_the_43rd_isarc_singapore/an_integrated_product-process_representation_for_deriving_robotic_task_structures_using_graph_transformations.html) is a strong comparator for deriving task graphs from BIM and construction-method templates.

## Paradigm shift: version tracking -> proof-carrying change

Represent source artifacts, derived task graphs, robot/site/measurement contexts, physical observations, inspection receipts and qualification claims as typed revision-bound nodes. Connect them with scoped dependency edges that declare exactly which change kinds propagate and what action is required.

A change event binds prior/new revision and digest, kind, scope, authority, source evidence, graph revision and policy digest. An inadmissible or unbounded event (missing authority, unknown semantic delta/kind, stale/expired/incomplete closure, or missing metadata-equivalence proof) marks all non-subject claims in the supplied graph `UnknownImpact`; no disconnected node is declared `Unaffected` merely because a dependency path is absent.

The evaluator emits:
- Rebuild;
- Remeasure;
- Revalidate;
- Requalify;
- ReviewRequired;
- Unaffected with dependency-closure witness;
- UnknownImpact when evidence/rules/scope are incomplete;
- StaleInput when the event does not match the current graph revision.

It optimizes for minimal sufficient requalification with zero false-unaffected decisions, not minimum retesting at any cost.

## Composition / ownership

- #3561 design/export/revision/frame admission;
- #3569 control-domain coverage;
- #3566 permission-to-evidence assurance chain;
- #3542/#3558 site epochs and requalification;
- #3521/#3522 physical receipts/evidence admissibility;
- #3591 cross-lab reproducibility;
- #3605 accepted-output semantics;
- #3628 endurance/maintainability;
- #3631 lifecycle cost;
- #3644 vendor-neutral task interop;
- #3664 design/method-to-task compiler;
- #3672 transfer benchmark.

This package owns only change events, typed dependency edges, scope-aware propagation, impact witnesses and synthetic conformance fixtures. It does not reproduce any owning subsystem.

## V0.1 evaluator

The dependency-free Rust crate:
- validates unique nodes/edges, referenced endpoints, policy consistency and acyclic dependency edges;
- checks exact event graph revision and prior subject revision/digest;
- requires a new revision and restricts same-digest changes to metadata-only events with equivalence evidence;
- evaluates explicit global/project/site/work-package/frame/ODD scopes;
- treats a missing rule as UnknownImpact; an edge may be non-triggering only when that change kind is explicitly covered;
- merges multiple paths without dropping any required action;
- propagates unknown upstream impact conservatively;
- permits Unaffected only with complete graph closure, an authority-backed closure manifest, the exact graph revision and an explicit validity expiry;
- produces stable node/edge/rule path witnesses and deterministic reports, including parallel dependency edges and convergent unknown paths; a 100,000-path exploration ceiling fails closed by marking unresolved nodes unknown.

The graph identity and report are deterministic source contracts; the evaluator does not authenticate external signatures or verify the truth of an authority reference.

## Benchmark baselines

Compare the proposed evaluator against (1) blanket invalidation of all descendants, (2) direct version-only diffs, (3) untyped provenance traversal, and (4) manual expert decisions on the same blind case packets.

Report false-unaffected rate (target zero), false-affected/over-invalidation, affected-set precision/recall, disposition accuracy, UnknownImpact rate/reasons, closure/witness coverage, requalification scope relative to blanket rebuild, reviewer agreement/time, deterministic replay, mutation score, graph size and incremental latency.

The frozen machine-readable fixture corpus contains 55 synthetic cases. These are reference scenarios, not physical trial evidence or achieved benchmark results.

## Build/test

```sh
cargo test --manifest-path research/construction-change-impact-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-change-impact-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

CI pins Rust 1.96 and runs formatting, tests, locked metadata and strict Clippy on Ubuntu, Windows and macOS. The crate has no external dependencies.

## Claim ceiling

Passing software tests proves only the declared graph/event/rule semantics for synthetic fixtures. It does not prove that a real-world change is immaterial, verify physical evidence, certify standards compliance, establish safety or grant motion authority. Physical and safety requalification remain with competent project/site authorities.
