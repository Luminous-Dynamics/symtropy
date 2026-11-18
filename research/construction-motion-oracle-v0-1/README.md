# Construction Motion Oracle & Differential Benchmark v0.1

**Status:** synthetic analytic reference for two bounded geometry families. Not a general collision checker, planner or safety qualification.

Parent implementation: [motion envelope #5345 / PR #5347](https://github.com/Luminous-Dynamics/symtropy/pull/5347). Programme issue: [#5363](https://github.com/Luminous-Dynamics/symtropy/issues/5363).

## Independent oracle

This crate uses separate mathematical formulas for:
- closest approach between a straight 3D segment and a stationary sphere;
- closest approach between an upper semicircle and a stationary sphere;
- exact coordinate extrema for those path families.

It consumes the envelope producer's output only to test containment and broad-phase candidates. It does not reuse the producer's AABB expansion helpers.

The semicircle case is deliberately adversarial: its endpoint chord is the diameter, which misses an obstacle at the arc apex. Endpoints-only and chord-only baselines therefore return false while the analytical continuous oracle returns true.

## Current synthetic corpus

The machine-readable fixture contains 12 explicit geometry cases with path endpoints/arc parameters, body and obstacle radii, all seven per-axis margin channels, expected analytic/endpoints/chord/physical/stopping/coordination outcomes, primitive-manifest completeness, and six controlled mutations. All current cases are development cases; there is no claim of a statistically sufficient held-out corpus yet. See schema/oracle-corpus-v0.1.schema.json. Adapter results use schema/adapter-result-v0.1.schema.json.

The Rust unit suite includes:
- straight-segment analytic containment;
- explicit detection of an understated chord-deviation claim;
- arc collision missed by endpoints and chord baselines;
- thin barrier crossed between safe endpoints;
- conservative AABB false-positive candidate;
- separation of physical collision from stopping/policy overlap;
- uncertainty monotonicity;
- uniform-scale collision invariance.

All values are synthetic and expressed in millimetres. The upper semicircle is planar, has a fixed z coordinate and covers exactly 180 degrees. The linear reference is piecewise straight. The oracle does not prove general articulated-robot containment or arbitrary mesh swept volumes.

## Industry differential adapters

The frozen inputs/results can be adapted to:
- MoveIt continuous robot-world checking with the exact detector backend and collision matrix recorded;
- Tesseract CONTINUOUS and LVS_CONTINUOUS evaluators;
- additional independently versioned checkers.

The MoveIt continuous robot-world API and self-collision checks are distinct scopes; backend support must also be verified. The currently inspected MoveIt FCL continuous overload is a stub, while the Bullet tutorial documents CCD. Tesseract evaluator interpolation/cast rules must be recorded. No adapter result is accepted as analytical truth, and a missing backend is NOT_RUN, never PASS.

See docs/differential-adapter-contract.md.

## Commands

- cargo fmt --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml -- --check
- cargo test --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --locked
- cargo clippy --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --locked --all-targets -- -D warnings
- cargo run --release --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --bin differential-bench

## Claim ceiling

Passing tests establish only the implemented mathematics for the declared line and planar semicircle families and the adapter's tested structural behavior. They do not establish arbitrary continuous collision detection, physical robot model accuracy, localization/tracking/stopping truth, safe separation, construction productivity, standards conformity or vendor interoperability.


## Field layout acceptance benchmark v0.1

The field campaign is governed by the field layout acceptance benchmark profile at docs/field-layout-acceptance-benchmark-v0.1.md. It separates vendor-condition reproduction from a common independent referee. The primary output is independently accepted design-intent entities per total crew-hour, not print-head speed or robot self-reported completion.

The machine-readable per-run observation contract is schema/field-layout-run-observation-v0.1.schema.json. It binds the design work package, robot build/mode/stationing/sensors, site and metrology identities, U95 reference uncertainty, whole-workflow accounting and per-entity observations. It intentionally stores robot-claimed state separately from independent referee state.

The synthetic example at examples/field-layout-run-observation-synthetic-v0.1.json shows a robot that claims a line was printed while the independent referee rejects the measured output. The hashes and values are synthetic placeholders, not field evidence. The dependency-free Rust scorer in src/field_layout_acceptance.rs now performs typed manifest/observation/referee reconciliation and independently derives entity states and whole-workflow metrics. JSON ingestion and full Draft 2020-12 schema validation are still not wired into the CLI/CI lane.

The profile tracks HP SitePrint, Dusty FieldPrinter 2, Civ Robotics outdoor staking/marking, Rugged Mark1 and Hilti Jaibot as separate task tracks; it includes Dusty's November 2026 features and the 2025 multi-trade field study. It references AMRA-201:2026, ISO 10218-1/-2:2025, ISO 12100 and relevant ASTM F45/NIST test-method work. The CivDot+ published "3/100 ft (8 mm)" value is preserved with a unit-conversion discrepancy flagged for clarification. It does not claim certification or standards conformity.


### Independent acceptance scorer

The typed Rust API is exported as field_layout_acceptance::score_field_layout_run. It accepts an evaluator-owned design manifest, a list of robot observations, referee measurements, explicit human waivers, a frozen tolerance/resolution profile and whole-workflow crew-hour accounting.

The scorer checks SHA-256 digest encoding on package/revision/manifest and referee/waiver receipts, rejects duplicate manifest IDs, duplicate/unknown observations and duplicate/unknown waivers rather than silently deduplicating or ignoring extra output. Every manifest ID receives exactly one reconciled state: ACCEPTED, REJECTED, HELD, WAIVED or NOT_ATTEMPTED. The robot claim is preserved for audit but never overrides referee measurement. Insufficient independent reference U95 produces HELD/ReferenceResolutionInsufficient, not an accuracy pass or a geometry failure. A waiver remains WAIVED and is excluded from accepted throughput. Zero crew-hours yields no productivity rate instead of infinity.

The result reports accepted/rejected/held/waived/not-attempted counts, attempted count, first-pass accepted count, rework entity count, total crew-hours, accepted entities per crew-hour, first-pass yield and rework-hour fraction. Unit tests cover false robot completion, geometric rejection, insufficient reference resolution, human waiver separation, duplicate/unknown IDs and zero-hour denominators.

This first scoring owner is typed Rust logic, not a JSON reader, cryptographic digest recomputation or digital signature verifier, physical metrology system or field report generator. Digest encoding is checked, but the ingestion layer must verify that each digest matches canonical source bytes. The observation schema and synthetic JSON shape fixture remain separate; a future ingestion step must verify the manifest and receipt digests before building typed inputs.


### Append-only site-state ledger v0.1

The typed Rust module site_state_ledger projects a pre-verified event stream into per-entity site state. It binds each execution and referee assessment to a frozen work-package digest, design revision, frame, transform and acceptance profile.

Roles are separated: design authority publishes work packages; surveyors qualify station/frame/calibration context; robots start attempts and report observations; referees record measured/no-mark/unresolved outcomes; site quality authorities issue explicit waivers. Robot reports can only move an entity to AWAITING_INDEPENDENT_MEASUREMENT. For measured outcomes, the reducer calls the independent field-layout scorer; robot self-report cannot set ACCEPTED.

The ledger enforces contiguous sequence numbers, previous-event digest links, unique event identities, exact duplicate idempotency, actor-role permissions, station/work-package binding, attempt identity and legal state transitions. Event application uses a two-phase prepare/commit path: all fallible validation occurs against immutable state, then only the affected package/entity/attempt is changed. The append-only event history is not cloned per event; event IDs map to stable indices in the single event vector. This avoids the former O(history + site projection) deep-copy cost on each append. Publishing a new design revision preserves the previous package and its historical accepted records, marks it superseded, and initializes the new package's entities as NOT_ATTEMPTED.

A robot can emit EXECUTION_INTERRUPTED with a receipt and reason. This moves the attempt to AWAITING_INDEPENDENT_MEASUREMENT with robot disposition UNKNOWN; it never means the task completed or that no physical work is present. Only an independent referee assessment can resolve it. A referee NO_MARK_FOUND result creates PHYSICALLY_ABSENT: this is not accepted output, but independently establishes that no mark remains at the design location. It is distinct from REJECTED, where a physical mark exists outside tolerance.

Package supersession is blocked while any entity is EXECUTION_IN_PROGRESS, AWAITING_INDEPENDENT_MEASUREMENT, HELD or REJECTED. Such physical state must be reconciled by remeasurement/rework, a referee-confirmed absence, or an explicit scoped quality-authority waiver before a new package can supersede it. This prevents a design revision from silently orphaning partial, uncertain or rejected physical work.

Important boundary: the caller must cryptographically verify canonical event bytes, actor signatures and source receipts before calling the reducer. This module checks digest encoding and chain linkage but does not recompute SHA-256 or verify signatures. It is not a distributed consensus protocol, safety-rated controller, motion authorization or physical acceptance system. The sequence is a ledger order, not a wall-clock truth; cross-robot clocks are evidence metadata, not ordering authority.

Unit tests cover robot/referee separation, interruption and independent reconciliation, accepted measurement, geometric rejection, referee-confirmed absence, stale transform and wrong actor denial, atomic rollback, duplicate idempotency, sequence gaps, historical acceptance after revision supersession, unresolved-state supersession blocking, and waiver separation. The public replay method reconstructs the full projection from a persisted preverified event list and requires exact event identity/sequence/digest continuity; tests compare the reconstructed ledger with the original and reject duplicate or broken history. The differential benchmark now reports append scaling at 100, 1,000 and 10,000 station events over 31 rounds, including total and per-event p50/p95/p99. This is a single-host software microbenchmark, not a distributed throughput or field deployment claim.


The wire contract for adapters is schema/site-state-event-v0.1.schema.json. Work packages also preserve the exact IDS evaluator build digest, so the same IDS rule specification evaluated by different engines/builds remains distinguishable in the digital thread. examples/site-state-event-stream-synthetic-v0.1.json contains a six-event synthetic stream: package publication, station qualification, a robot attempt/observation, independent referee measurement, and a superseding package revision. Its repeated hexadecimal digests are placeholders for schema/replay tests, not cryptographically valid event receipts or signed evidence. The schema is not yet connected to a JSON parser; it describes the planned versioned adapter boundary.

Adapter conformance should prove round-trip preservation of stable design entity IDs (preferably the source IFC GlobalId), event identity and order, package/revision/frame/transform hashes, actor role, robot/attempt identity, station calibration and referee measurement receipts. Unknown event kinds or missing required fields must fail closed, not be ignored.


### IFC 4.3 / IDS 1.0 preflight adapter

The typed adapter is exported as ifc_ids_preflight::preflight_ifc_ids_work_package. It accepts a normalized record from an upstream IFC and IDS reader, then checks the supported IFC 4.3 ADD2 schema release and IDS 1.0 version, source/schema/requirements/evaluator-build digests, stable 22-character compressed IFC GlobalIds, duplicate IDs, geometry digests, required-vs-delivered IDS property identifiers and one passing evaluation receipt for each applicable IDS requirement. Each in-scope entity also requires a separate IDS applicability receipt binding the complete rule-selection result, including a valid empty set when no IDS rule applies. It emits a robot-neutral SiteWorkPackage bound to the source model digest, IFC schema release, IDS spec digest/version, design revision, CRS, vertical datum, unit conversion, model-to-site frame/transform, source entity-manifest digest and acceptance profile. Scoped-out source objects remain countable but are not emitted into the robot work package.

This is a typed preflight boundary, not an IFC STEP parser, IDS XML processor, geometry kernel or canonical hashing implementation. The upstream reader must resolve IFC placements, mapped representations, units, project CRS/site transform, linked models and IDS applicability. The adapter checks a supported length-unit conversion as an exact rational mm ratio, a declared CRS and vertical datum, a finite millimetre translation, a right-handed orthonormal model-to-site rotation, a positive map-conversion scale, and that the frame/transform digests match the work-package frame identity. The canonical packager must include all of this context in the transform digest and compute package/entity-manifest digests from sorted, canonical contents. The caller must verify the source model and IDS spec digests against the exact source bytes. A syntactically valid package can still encode incorrect site survey control, so field/site-frame qualification remains a separate surveyor gate.

The SiteWorkPackage and its event JSON Schema require source_model_sha256, source_schema_release, ids_spec_sha256 and ids_version. The synthetic event stream was updated to carry these provenance fields in both package revisions.


The IFC/IDS input wire contract is schema/ifc-ids-preflight-input-v0.1.schema.json; examples/ifc-ids-preflight-input-synthetic-v0.1.json is a synthetic two-entity in-scope plus one excluded-entity example. Each in-scope entity carries both property-presence bindings and the complete set of applicable IDS requirement IDs with one pass/fail evaluation receipt per rule; the canonical source entity manifest digest must include these rule IDs and receipt digests. A property-name match alone is not accepted as proof of IDS conformance: every applicable rule must be evaluated exactly once, pass, and carry a well-formed receipt digest. Duplicate, missing, failed, unexpected or malformed evaluations block compilation.

The IDS engine build digest and per-entity applicability/evaluation receipts are part of the package lineage. This Rust module validates coverage, pass state, identifier uniqueness and digest syntax; the consuming application still has to verify each receipt digest against canonical evaluation bytes, bind it to the source model/IDS spec/GlobalId/engine build and verify the evaluator's signature/identity. A syntactically complete receipt list is not itself cryptographic proof that the IDS engine actually evaluated the stated model. The preflight function does not pretend to implement the full IDS rule language itself.


The separate interruption replay fixture is \`examples/site-state-interruption-replay-synthetic-v0.1.json\`. It contains a frozen package, station qualification, robot attempt, explicit power-loss interruption, and independent referee NO_MARK_FOUND receipt. The correct projection is EXECUTION_IN_PROGRESS → AWAITING_INDEPENDENT_MEASUREMENT(UNKNOWN) → PHYSICALLY_ABSENT. None of the robot events can accept the entity. The repeated digest strings are synthetic placeholders; the fixture is not a signed or cryptographically verified event stream.


### Space-time dispatch readiness gate v0.1

The typed API is dispatch_readiness::check_dispatch_readiness. It joins the current append-only site ledger, frozen work package, robot/tool/payload configuration, primitive manifest, trajectory and motion-envelope digest to an independent occupancy assessment for the exact same envelope and site snapshot.

It rejects:
- inactive/superseded packages and mismatched design revision, frame, transform or entity identity;
- entities already accepted, waived, held, executing or awaiting independent measurement;
- expired/not-yet-valid envelopes or occupancy assessments, cross-clock evidence, and occupancy assessments created before the envelope they claim to assess;
- a changed site ledger sequence/head since occupancy evaluation;
- missing primitive, dynamic actor, broad-phase pair or narrow-phase coverage;
- any conflict, unresolved pair, invalid bound, stale actor or unmodelled occupant.

A successful result is a short-lived **readiness receipt** bound to the exact package, design revision, entity, robot, envelope digest, occupancy snapshot digest, assessment receipt, approved occupancy-engine build, clearance-policy build, ledger head and site clock domain. Its expiry is the earlier of the envelope or occupancy assessment expiry. The request names the owner-approved occupancy engine and clearance policy; the assessment must match both. It also reports `candidate_pair_count` and `narrowphase_evaluated_pair_count`, and readiness is blocked when the latter does not cover every candidate. The JSON contract is schema/dispatch-readiness-v0.1.schema.json; examples/dispatch-readiness-request-synthetic-v0.1.json is a synthetic shape fixture.

This component does not calculate robot stopping distance, verify the occupancy engine's maths, recompute source hashes, authenticate signatures, reserve a zone, issue an actuation permit or control an emergency stop. The caller must verify the canonical assessment and envelope receipts and invoke the relevant independent safety authority. A new site ledger event invalidates the old occupancy snapshot and requires re-evaluation. This is a non-actuating readiness gate only.

The benchmark should separately measure readiness-check p50/p95/p99, stale-snapshot rejection, conflict/missing-coverage negative-control recall, false-block rate, receipt replay resistance and re-evaluation work after localized site changes. The safety target is zero false-ready outcomes in a frozen, independently reviewed adversarial corpus; performance cannot compensate for a false-ready result.


The dispatch-readiness receipt is a typed assessment result, not yet a cryptographically signed authorization token. Before handing it to an authorization authority, the consuming application must bind its canonical serialization to a digest/signature or an authenticated Mycelix attestation, verify that proof, and call verify_dispatch_readiness_receipt_current against the current ledger and approved engine/policy identities. The helper only checks logical currentness and schema/digest syntax; it does not authenticate the receipt itself. A rejected physical mark is intentionally not eligible for ordinary dispatch: the ledger must first record a specific remediation disposition or referee-confirmed physical absence. No actuator API is exposed by this crate.


## Construction change-impact compiler v0.1

The pure Rust module change_impact calculates the smallest declared set of derived construction artifacts that must be recomputed or requalified after a design, survey, robot, tool, payload, obstacle, clock, material, occupancy-engine or authority-policy change.

### Paradigm

A new input version does not make every historical fact false, and it does not leave every previously qualified future task valid. The compiler separates:
- direct invalidations: an artifact's recorded input dependency does not match the new/current digest;
- transitive invalidations: downstream derived artifacts depended on an invalidated artifact;
- preserved history: accepted construction, inspection and handover records remain immutable and visible as historical evidence;
- unaffected artifacts: their declared dependency versions still match;
- unmatched change keys: changes with no declared dependent artifact, which require explicit reconciliation rather than being silently ignored.

The dependency key is a typed domain plus explicit scope. A localized entity-geometry change should affect that entity's trajectory, envelope, occupancy and dispatch chain, not unrelated entities. A shared frame/transform or occupancy-policy change should affect all artifacts that depend on that shared input.

The graph is validated for unique artifact IDs, unique dependency keys, known parent artifacts, acyclic dependencies and well-formed SHA-256 encodings. A deterministic topological sort produces a dependency-first requalification order. The raw graph-only compiler is read-only and reports source_records_mutated=false, but it does not qualify dependency completeness.

For selective requalification, use compile_change_impact_with_contract rather than the graph-only compiler. An owner-approved DependencyContractSet defines the minimum dependency domains for every artifact kind. The coverage audit rejects missing artifact-kind contracts and missing required domain bindings. A successful report binds the contract digest and sets dependency_coverage_complete=true. The selective_requalification_gate_passed helper additionally requires that every change key is matched to at least one declared dependency. A graph-only report remains explicitly unqualified and must not authorize selective requalification.

The dependency contract is a coverage floor, not proof that all upstream dependencies were declared. Owners must version and review their contracts, and adapters must supply correct entity/floor/site scope IDs. The completeness of the upstream graph remains an independent assurance obligation.

### Benchmark

The change-impact benchmark builds 3 artifacts per entity: motion envelope, space-time occupancy assessment and dispatch readiness. Each artifact must satisfy the owner-approved dependency-domain contract before the timed operation completes. The measurement includes graph validation, dependency-coverage audit and invalidation closure—not just the final breadth-first walk. It compares:
- one localized geometry change, expected to invalidate exactly 1 direct motion artifact and its 2 downstream artifacts;
- one shared floor-transform change, expected to invalidate all motion artifacts plus their occupancy and readiness descendants.

Run the separate harness with:
\`cargo run --release --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --bin change-impact-bench\`

The frozen synthetic size points are 1,000, 10,000 and 50,000 entities. The harness checks exact invalidation counts and unmatched-change absence before accepting timing samples, and reports p50/p95/p99. These are algorithmic software microbenchmarks, not production site throughput. A second host and held-out dependency topologies are needed before selecting a production invalidation index.

### Proposed benchmark gates (targets, not achieved results)

| Metric | Research target |
|---|---:|
| Stale derived artifacts missed in the frozen dependency corpus | 0 |
| Historical accepted evidence mutated/deleted | 0 |
| Unmatched change keys silently dropped | 0 |
| Requalification order violations | 0 |
| Localized change over-invalidation | ≤10% of otherwise current derived artifacts |
| Shared transform/site-frame change coverage | 100% of dependent artifacts |
| 50K-entity localized-change compile | p95 ≤100 ms on the reference CI host |
| Change receipt replay determinism | byte-identical canonical report for identical graph/change inputs |

The zero-miss target is a correctness gate, not a statistical claim about the completeness of a live site's dependency graph. The ≤10% over-invalidation target applies only to declared localized changes with a frozen independent oracle. Global changes should not be penalized for correctly invalidating a large fraction of the graph.

### Trust boundary

The caller must verify the change event, graph snapshot, source digests and signatures. This crate validates digest encoding but does not recompute SHA-256, verify signatures, prove that an upstream adapter declared every dependency, or mutate the site ledger. An unmatched change is explicitly surfaced; a caller must not treat a report as dispatch-ready until graph completeness and unmatched-key disposition are independently satisfied.


The machine-readable graph/change contract is schema/construction-change-impact-v0.1.schema.json. The localized replay fixture is examples/construction-change-impact-localized-synthetic-v0.1.json. It demonstrates an entity-A geometry change invalidating motion:A, occupancy:A and readiness:A while preserving accepted:A as immutable history and leaving entity B's chain unaffected. Digests are synthetic placeholders; the fixture is not a signed event or complete site dependency manifest.

The wire schema and typed validator enforce the same persistence rule: accepted construction records, as-built measurements and handover records are historical_immutable; derived artifacts such as trajectories, envelopes, occupancy assessments and dispatch readiness are recomputable. The typed Rust module is the authoritative validation path; the JSON Schema is the interoperable input shape.

The schema and synthetic fixture also carry a dependency_contract_set. Every artifact kind must be covered, and the minimum required domains must be unique and non-empty. The typed contract audit remains authoritative because it also checks artifact IDs, parents, cycles, persistence class and the dependency edges used by the invalidation compiler.
