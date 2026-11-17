# Factory Process Model Assurance v0.1

**Owner:** Symtropy FACTORY-PROCESS-MODEL-ASSURANCE-001 [#5729](https://github.com/Luminous-Dynamics/symtropy/issues/5729)  
**Canonical process semantics:** Symthaea MFG-PROC-000 [#5686](https://github.com/Luminous-Dynamics/symthaea/issues/5686)  
**Fixture generation:** `pump-valve-model-change-v0.1`  
**Status:** frozen synthetic task/mutation corpus plus a dependency-free Rust obligation-trace contract checker; exact-head CI qualification is pending. No solver-backed semantic oracle is claimed.

## Research question

Can a manufacturing intelligence system translate an industrial change request into the *right executable model*, and can an independent verifier detect omitted or weakened requirements even when the optimizer returns a plausible or unchanged objective?

This benchmark separates two proof obligations:

1. **Model semantic correctness:** each authoritative obligation has a correct, typed, revision-bound disposition and is represented by the expected executable constraint(s), or explicitly classified as unsupported, unknown, or not applicable.
2. **Plan correctness:** the emitted schedule satisfies the independently verified model and the declared manufacturing constraints.

A schedule that is feasible for the wrong model is not a correct schedule.

## Paradigm shift

The process-model compiler is an assurance boundary, not a natural-language-to-solver convenience wrapper.

`change request + typed process / resource / qualification evidence
  -> obligation extraction
  -> versioned process-model delta
  -> obligation-to-constraint trace
  -> independent semantic verification
  -> solver
  -> independent schedule certificate verification
  -> first-article / quality disposition`

The generating agent does not author its own ground truth, approve its own constraints, or convert missing context into permission.

## Ownership and reuse

This benchmark does not create a second manufacturing ontology, scheduler or process model. It consumes:
- Symthaea #5686 MFG-PROC-000 for process identity, process definitions, transformation/effect, capability requirements, recipe and qualification semantics.
- Symthaea #12355 for factory capability-network context.
- Symthaea #10702 for design-for-knowability and lifecycle requalification.
- Symthaea #12403 for decision-valued inspections and loss semantics.
- Symtropy #5446 for tiny exact scheduling oracles.
- Symtropy #5459 / #5528 for FJSPLib adapters and solver tournament.
- Symtropy #5520 / #5622 for selective repair and industrial context grounding.

The local pilot uses a deliberately small **obligation identity / disposition / constraint-reference view**. It is an adapter/test projection, not a replacement for #5686's canonical types. Units and quantities remain with SE-SEM; material identity and state remain with materials; physical measurement and inspection authority remain with FIELD/QIF owners; machine execution authority remains with the fabrication/HAL layers.

## v0.1 frozen task families

The corpus contains 18 synthetic change tasks: five explicit edits, eight structural extensions, and five compositional changes. The task descriptions are source packages for a future compiler benchmark, not actual factory instructions.

Task strata:
- **E01–E05 explicit edits:** machine availability, operation timing, route eligibility, product revision, and capacity.
- **S01–S08 structural extensions:** qualified workers, tooling/fixtures, AGV transport, maintenance calendars, sequence-dependent setup/segregation, material genealogy, inspection hold points, and calibration scope.
- **C01–C05 compositional changes:** worker/resource interaction, transport plus sequencing, material/process/inspection coupling, version/time/source conflict, and non-anticipative future changes.

Files include `tasks-v0.1.tsv` (18 tasks), `obligations-v0.1.tsv` (29 typed obligation projections), `mutations-v0.1.tsv` (16 semantic mutations), and `reference-trace-v0.1.tsv` (40 synthetic task/obligation trace rows). The Cargo crate and focused workflow are included alongside these data files. Every obligation and mutation target is cross-referenced against the obligation manifest. Task identifiers use `synthetic-ref:` handles, not fabricated SHA-256 digests. Each task binds a base model revision, a source/change-request revision, effective and observed times, an exact product/process scope, expected obligation IDs, expected model-delta IDs, and forbidden side effects. A production-grade harness must reject source revision, identity, unit, timestamp and applicability mismatch. The obligation manifest is a local verifier projection; canonical quantity, material, process, inspection and authority semantics remain with their existing owners. The current checker validates trace completeness and declared constraint-family identity only; it does not inspect an actual CP-SAT model or prove that a referenced constraint's operands/scope are semantically correct. Synthetic constraint/evidence references are placeholders for contract tests, not executable-model receipts.

## Semantic obligation contract

For every obligation in the independently authored source manifest, the candidate model must emit exactly one of:
- `SATISFIED`: one or more exact constraint references cover the obligation;
- `NOT_APPLICABLE`: explicit reason plus admissible supporting evidence;
- `UNSUPPORTED`: known obligation outside the model profile; blocks acceptance;
- `UNKNOWN`: source evidence is missing, stale or conflicting; blocks acceptance.

An absent trace is not a `NOT_APPLICABLE` disposition. Duplicate or contradictory mappings are verifier failures. A constraint ID alone is not semantic proof: later verifier adapters must inspect the executable model structure and validate the constraint's scope, operands, units, revision, and temporal applicability.

## Objective-preserving mutants

The mutation manifest distinguishes:
- `MUTATION_REQUIRED`: a deliberately introduced semantic omission or incorrect weakening;
- `OBJECTIVE_PRESERVING_CANDIDATE`: a mutant selected because the omitted obligation is designed not to change the base-case objective while still making the model semantically invalid;
- `OBJECTIVE_CHANGING_CONTROL`: an omission expected to change the base-case optimum/feasibility.

Do not report an objective-preserving mutation as confirmed until a paired exact solve establishes equal baseline and mutant objective values **and** the independent semantic oracle establishes that the mutant violates the obligation. This is the central negative control: objective-only checking must be shown to miss an invalid model.

## Baseline arms

- M0: independently authored canonical model.
- M1: document/change-request generator with objective/schedule check only.
- M2: precedence + capability/compliance + availability mask comparator.
- M3: typed obligation extraction and trace-to-constraint verifier.
- M4: M3 with independent structural mutation tests and objective-preserving controls.
- M5: exact tiny model/plan oracle.
- M6: human process planner under matched task/tool/time budgets.
- M7: agent + human review with review time and overrides recorded.
- M8: evaluator-only hindsight reference.

All candidate arms must receive the same observable source package, model profile, evidence prefix and compute/tool budget. The evaluator-held split must be disjoint by underlying base instance and mutation family, not merely by paraphrased task text.

## Metrics and non-compensatory hard gates

Report:
- obligation coverage / traceability;
- exact model-delta match;
- missing, duplicate, contradictory and forbidden constraints;
- source identity, revision, temporal, unit and applicability grounding;
- unsupported / unknown / not-applicable disposition correctness;
- critical mutation detection and objective-preserving mutation detection;
- false acceptance and false rejection;
- schedule certificate correctness / exact-oracle agreement;
- model-generation latency, verifier latency, solver runtime, CPU/memory and external cost;
- human review time, override rate and unresolved escalation.

Hard gates:
- zero accepted models with missing critical obligations in supported v0.1 tasks;
- zero objective-preserving critical mutants accepted after confirmation by the exact paired solver and semantic oracle;
- zero unknown/unsupported requirements converted into positive permission;
- 100% disposition coverage for source obligations;
- no future event leakage under paired observable-prefix controls;
- no schedule certificate accepted against a model revision/digest other than the one independently verified.

Do not average away a critical semantic failure using makespan gains.

## Prospective targets (not achieved results)

After M0–M2 baseline reproduction and held-out split freeze:
- at least 95% exact model-delta match on supported held-out changes;
- at least 99% critical-obligation trace coverage, with 100% for any accepted model;
- at least 100% detection of the declared objective-preserving critical mutants;
- at least 30% lower end-to-end authoring + verification effort than matched manual baseline without any critical false acceptance;
- 100% non-anticipativity on paired future-prefix controls.

These are experimental goals, not standards, safety thresholds, or current measurements.

## Claim boundary

v0.1 is synthetic process-model assurance research. It does not establish universal semantic completeness, industrial process accuracy, scheduling optimality beyond supported exact cases, physical qualification, product conformity, production savings, factory safety, or autonomous execution permission. An obligation receipt proves only that a particular validator accepted a particular model revision against a declared source-obligation manifest; it does not prove that the source documents were true or complete.

## Run state

Run the trace contract checker with:

```sh
cargo fmt --manifest-path tools/factory-process-model-assurance/Cargo.toml -- --check
cargo clippy --manifest-path tools/factory-process-model-assurance/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/factory-process-model-assurance/Cargo.toml --all-targets
cargo run --quiet --manifest-path tools/factory-process-model-assurance/Cargo.toml --bin factory-process-model-assurance
```

The current executable checks obligation coverage, disposition admissibility, evidence currentness and declared constraint-family matching. It does not inspect CP-SAT internals, confirm objective-preserving mutants, or establish full semantic model correctness. Those remain next-stage work, alongside physical data replay and any production evidence.
