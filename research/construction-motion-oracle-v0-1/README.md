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

A successful result is a short-lived **readiness receipt** bound to the package, design revision, entity, site frame and transform, registered robot identity, robot/tool/payload configuration digests, primitive manifest, trajectory, exact motion-envelope digest, dynamic occupancy and actor manifests, occupancy snapshot, assessment receipt, approved occupancy-engine build, clearance-policy build, broad-/narrow-phase pair counts, ledger sequence/head and site clock domain. Its expiry is the earlier of the envelope or occupancy assessment expiry. The ledger's actor registry must authorize the robot ID as a Robot; an arbitrary caller-supplied robot string cannot pass. The request names the owner-approved occupancy engine and clearance policy; the assessment must match both. It also reports `candidate_pair_count` and `narrowphase_evaluated_pair_count`, and readiness is blocked unless every candidate received narrow-phase evaluation. The JSON contract is schema/dispatch-readiness-v0.1.schema.json; examples/dispatch-readiness-request-synthetic-v0.1.json and examples/dispatch-readiness-receipt-synthetic-v0.1.json are synthetic shape fixtures.

The site ledger exposes a read-only `work_package(package_sha256)` accessor for this check; it does not allow the gate to mutate the current package. The receipt deliberately carries the full execution/occupancy context so the downstream signer can canonicalize and authenticate all fields as one payload, rather than signing a compact token that omits the tool, payload, frame, trajectory or dynamic actor set.

This component does not calculate robot stopping distance, verify the occupancy engine's maths, recompute source hashes, authenticate signatures, reserve a zone, issue an actuation permit or control an emergency stop. The caller must verify the canonical assessment and envelope receipts and invoke the relevant independent safety authority. A new site ledger event invalidates the old occupancy snapshot and requires re-evaluation. This is a non-actuating readiness gate only.

The benchmark should separately measure readiness-check p50/p95/p99, stale-snapshot rejection, conflict/missing-coverage negative-control recall, false-block rate, receipt replay resistance and re-evaluation work after localized site changes. The safety target is zero false-ready outcomes in a frozen, independently reviewed adversarial corpus; performance cannot compensate for a false-ready result.


The dispatch-readiness receipt is a typed assessment result, not yet a cryptographically signed authorization token. Before handing it to an authorization authority, the consuming application must bind the *entire expanded receipt* to canonical bytes and a digest/signature or authenticated Mycelix attestation, verify that proof, and call verify_dispatch_readiness_receipt_current against the current ledger, registered robot identity, and approved engine/policy identities. The helper checks logical currentness and digest syntax; it does not authenticate the receipt, prove the occupancy engine mathematically correct, or independently derive the site snapshot digest. A rejected physical mark is intentionally not eligible for ordinary dispatch: the ledger must first record a specific remediation disposition or referee-confirmed physical absence. No actuator API is exposed by this crate.


### Vendor protocol observations are not construction verdicts

The site ledger now accepts \`RobotProtocolObservationRecorded\` as an append-only, history-only event. Its typed \`RobotProtocolObservation\` preserves protocol ID/version, message kind, external order/update identity, reported header/route/action fields, source timestamp, manufacturer/serial claims and digests of the topic, exact raw payload and adapter build.

This is the bridge to the existing standalone read-only VDA 5050 v3.0.0 observer at \`research/construction-vda5050-observer-v0-1\`. A caller can map the observer's \`DecodedMessage::payload_sha256\`, \`VdaHeader\`, and \`Observation::RobotStateReported\` fields into the ledger record. If a state message contains multiple action states, preserve each action ID/status as a separate observation event with its source payload digest. The observer crate is intentionally not imported as a runtime dependency here; parsing and canonical-byte verification stay at the adapter boundary.

**No protocol field is promoted into local authority:**
- VDA \`orderId\` / \`orderUpdateId\` remain transport identifiers, not IFC GlobalIds, construction task IDs or design revisions.
- A VDA action status such as \`FINISHED\` is an executor report, not proof that the physical output exists, is in tolerance or passed inspection.
- Route node/edge progress is not a conservative space-time occupancy envelope.
- Factsheet capability data is not independently qualified tool/payload geometry or a safety certificate.
- Zone requests, zone responses or zone sets are fleet-protocol observations, not local permits, site occupancy clearance or safety authorization.
- Connection state is not a validated robot-health assessment.
- Protocol timestamps never order the append-only ledger; the ledger sequence and event hash chain do.

The reducer validates the protocol record's required identities and digest syntax, binds it to the same robot identity and execution attempt, and appends it to immutable history. It deliberately does not change \`ExecutionInProgress\`, \`AwaitingIndependentMeasurement\`, \`Accepted\`, reservation state or dispatch readiness. A late observation for an earlier, superseded work package remains historical and cannot mutate the current package. Adding any event advances the ledger head, so previously issued readiness snapshots must be rechecked.

The wire form is part of \`schema/site-state-event-v0.1.schema.json\`; the synthetic replay stream now includes a late VDA state observation with action status \`FINISHED\` after the original package has been superseded. Its digest strings are synthetic placeholders and are not signed/authenticated evidence. The tests prove that \`FINISHED\` leaves the entity executing and unaccepted, a different robot cannot submit the attempt's protocol observation, malformed source digests are rejected, and the event survives deterministic ledger replay.

**Authentication boundary:** the protocol header's manufacturer/serial values are claims, not publisher authentication. The upstream integration must authenticate the MQTT/session participant, bind that authenticated identity to the ledger's \`actor_id\`, verify the original payload/topic hashes and adapter build receipt, and canonicalize/sign the outer site event. The site reducer checks the typed identity linkage and digest format; it does not establish cryptographic identity from VDA fields.
