# Orbital Structural Interface Conformance Benchmark v0.1

**Owner:** [ORBITAL-STRUCTURAL-INTERFACE-CONFORMANCE-001 #6523](https://github.com/Luminous-Dynamics/symtropy/issues/6523)  
**Protocol:** [PR #6524](https://github.com/Luminous-Dynamics/symtropy/pull/6524)  
**Status:** executable synthetic reference benchmark; physical qualification not performed.  
**Claim ceiling:** `synthetic_structural_profile_compatibility_only`.

## Purpose

Evaluate a provider's declared structural-interface capability profile against a client's requirement profile. The evaluator is a pre-assembly compatibility gate. It is not a structural solver, interface design tool, robot planner, inspection authority, materials database, or manufacturing release system.

The benchmark preserves the distinctions between:
- exact ICD/configuration identity;
- geometry and mating features;
- assembly tooling and independent lock verification;
- named combined load cases and their referenced interaction profiles;
- deformation, slip and retention requirements;
- environment and declared service-cycle scope;
- material specification, condition, lot and executed process lineage;
- inspection method and critical-surface observability;
- evidence class, currentness and declared common-cause dependency.

## Run

Requires Rust 1.96+ and Cargo:

```bash
cargo fmt --manifest-path tools/orbital-structural-interface-bench/Cargo.toml --check
cargo test --locked --manifest-path tools/orbital-structural-interface-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-structural-interface-bench/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/orbital-structural-interface-bench/Cargo.toml -- \
  tools/orbital-structural-interface-bench/fixtures/v0.1/cases.json \
  --source-commit <40-lowercase-hex-commit>
python3 tools/orbital-structural-interface-bench/tests/reference_oracle.py \
  tools/orbital-structural-interface-bench/fixtures/v0.1/cases.json \
  <40-lowercase-hex-commit>
```

The CLI emits a deterministic JSON receipt with the exact fixture-byte SHA-256, source-commit shape, per-case disposition/reasons, expected-answer match counts and `authority_granted=false`. Source commit and evidence digest values are shape-checked only; this tool does not fetch commits, authenticate producers or verify the referenced raw artifacts.

## Dispositions

- `compatible_synthetic_profile`: all v0.1 synthetic checks pass for the declared pair.
- `incompatible`: one or more declared hard constraints fail.
- `indeterminate`: evidence, frame semantics, interaction profile, uncertainty basis or another required decision input is unresolved.
- `malformed_or_stale`: identity/digest/schema/currentness input is malformed or stale.

Reason codes are sorted for deterministic replay. Overall status uses a non-compensatory precedence: malformed/stale, then incompatible, then indeterminate, then compatible. It never produces a flight, production or structural adequacy authorization.

## Combined load semantics

The client and provider must share the exact named load case, frame and interaction-profile reference. The provider's six declared absolute capacity bounds must each meet or exceed the client's demand bounds **within that same named case**. This is not a universal six-axis strength model and must not be used to invent an interaction law. The referenced `interaction_profile_ref` identifies the separately controlled interaction model / conservative envelope; v0.1 compares identity and supplied bounds but does not evaluate the referenced model. Different or missing interaction-profile identity is indeterminate. Extend only through a separately reviewed model adapter with explicit applicability, uncertainty and independent validation.

## Evidence rules

The fixture uses explicit evidence-kind/class pairs:
- structural capability: `scoped_test_or_validated_analysis`;
- material/process: `process_conditioned_material_characterization`;
- inspection: `method_specific_inspection_report`;
- engagement verification: `independent_engagement_verification`;
- future fatigue and assembled-path evidence use separately typed classes.

A supplier assertion or provenance signature cannot stand in for structural capability. A digest-shaped string is not proof that the artifact exists or matches its bytes. A declared dependency-group label is not proof of statistical independence; the evaluator only catches exact shared labels in the synthetic fixture.

## Frozen corpus and baselines

The corpus includes 44 synthetic known-answer and adversarial pairs. The latest four cases additionally reject evidence bound to the wrong article/configuration/ICD revision, duplicate load-case identities, and vacuous empty client load requirements. Each evidence record binds its subject, configuration and ICD revision; material-process evidence binds to the exact material-lineage reference. It covers identity, frame/units, geometry, pose tolerance, tooling, independent engagement verification, combined load capacity, stiffness, retention, material/process lineage, environment, service life, uncertainty, inspection access, fatigue, assembled load path, common-cause labels, stale evidence, digest mutation and ordering invariance.

Weak baselines to report on this same frozen corpus:
1. connector/name label only;
2. profile-ID or standard-name only;
3. independent scalar checks that ignore named combined load cases;
4. nominal interface compatibility without structural/load-path specialization;
5. manual ICD/drawing review with review time recorded;
6. this typed evaluator, with abstentions counted separately from false acceptance.

The v0.1 target is 100% expected disposition/reason agreement and zero false-compatible results on the frozen critical negatives. Those are synthetic software targets, not achieved physical performance or measured industry statistics. A future comparison must use held-out profile pairs and matched evidence/time budgets.

## Known limitations and integration

- The interaction model referenced by a load case is not fetched or evaluated.
- No coordinate transformation is inferred or calculated.
- No uncertainty propagation or probabilistic reliability is performed; unsupported uncertainty basis abstains.
- Material, process, inspection, authority and article references are not resolved against their canonical owners.
- Supplier signatures, calibration, source artifacts and producer authority are not authenticated.
- Declared dependency labels do not establish independence.
- Coupon-to-component, joint-to-assembly and test-to-field transfer are not inferred. See the cross-scale transfer gate in the parent protocol.
- No physical interface, joint, specimen or spacecraft has been tested.

Compose with Symthaea #5686/#5074/#5763/#6224, Symtropy #5818/#4263/#4165, and Mycelix #5149. Do not duplicate those canonical owners. The next implementation step is an adapter that resolves exact references and applicability through those owners while preserving their authority boundaries.


## Cross-toolchain composition

This crate deliberately does not parse STEP/AP242, QIF, MTConnect or native CAD. Consume semantic identity and quality records from the existing [Symthaea MBD-INTEROP #10595](https://github.com/Luminous-Dynamics/symthaea/issues/10595), [Mycelix AP242-to-QIF semantic benchmark #6348](https://github.com/Luminous-Dynamics/mycelix/issues/6348), and [AeroCommons component packet #6345](https://github.com/Luminous-Dynamics/mycelix/issues/6345). Compose them with Symtropy #4165 inspection observability and #4263 as-built structural acceptance. NIST STEP File Analyzer and CAx-IF AP242/QIF cases are external comparison references, not code to fork or replace.

The composition path is: product definition / semantic PMI → exact characteristic and component identity → manufacturing process and serialized article → QIF inspection evidence → structural-interface profile → as-built assembly acceptance. Every transition needs a declared supported semantic subset and explicit loss/Unknown behavior. A parser pass, a structurally complete packet or a signed trace alone does not establish physical conformance.


## Serialized physical-unit binding

The provider profile's `article_subject_ref` denotes the exact serialized physical article (for example, a qualification specimen or as-built joint), not merely a reusable component definition or part-family number. Structural, inspection, engagement-verification, fatigue and assembled-path evidence must bind to that subject, exact configuration and ICD revision. Material/process evidence binds instead to the exact `material_lineage_ref`, while retaining the same configuration/ICD scope. The client profile's article subject is the target/design subject and is not treated as the provider's physical test article.

This follows the component-definition / serialized-unit distinction in Mycelix AeroCommons #6345. It is a strict identity join only; it does not prove that evidence on a qualification specimen transfers to a production unit or assembled structure. That transfer remains a separate, explicit assessment.
