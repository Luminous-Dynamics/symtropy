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


### Atomic exclusive space-time resource reservations v0.1

The reservation API closes the gap between a readiness assessment and an actual exclusive resource claim.

- dispatch_readiness::commit_dispatch_reservation revalidates the short-lived readiness receipt against the current package, entity, ledger sequence/head, occupancy-engine build and clearance policy, then appends one SpaceTimeReservationCommitted event through the same site-state reducer.
- The reservation binds the current package/revision/entity/robot, envelope digest, occupancy-assessment receipt, approved engine/policy, resource-catalog digest, site ledger sequence/head and site clock domain.
- Every reservation contains one or more exclusive resource claims with half-open integer-tick intervals [start_tick, end_tick). A claim must fit within the reservation window.
- The ledger performs overlap rejection as part of the same atomic event preparation/commit. A readiness receipt is thereby consumed: the ledger sequence/head changes and verify_dispatch_readiness_receipt_current rejects reuse of the older receipt.
- The active conflict index is keyed by resource ID and ordered by start tick. Since the reducer invariant forbids overlapping active intervals, checking only the predecessor and successor is sufficient. Commit and release update that index in the same mutation as the durable event, giving O(log active intervals for each claimed resource) conflict checks instead of scanning every active reservation.
- dispatch_readiness::release_space_time_reservation accepts only a fresh referee-authored release event bound to the current ledger snapshot, reservation ID, site-clock domain, clear time, site snapshot and verified release receipt.
- Lease expiry prevents use of an old readiness/lease token but does not release physical occupancy. The resource remains locked until the referee records independently evidenced clearance. Publishing a replacement resource catalogue or work-package revision cannot silently orphan an active reservation.

The ledger is a deterministic single-writer reducer. It is not distributed consensus. A production multi-process deployment must place the append and index mutation behind a serializable transaction or compare-and-append primitive keyed by (ledger_sequence, previous_event_digest); every replica must replay the same canonical, authenticated event stream. Two disconnected writers cannot safely assume they have reserved the same physical resource. Event/signature/canonical-digest verification remains the caller's responsibility. None of these functions is a safety permit, actuator command or protective stop.

The v0.1 resource model is deliberately exclusive capacity 1. It is appropriate for a crane slew sector, protected workface, narrow passage, lift exclusion volume or single-occupancy tool cell. It does not yet support capacity-N resources, cumulative loads, precedence constraints, flexible time windows or probabilistic reservations. Those require typed resource-capacity and physical consequence models rather than silently treating all resources as a mutex.

The differential benchmark now measures successful reservation commits and conflicting-claim rejections with 100, 1,000 and 10,000 existing reservations. It reports p50/p95/p99 nanoseconds separately. The unit corpus compares the BTree interval index's decisions against a deterministic brute-force interval-overlap oracle over 256 generated claims, and verifies that active-index entry counts equal the active event-sourced reservation projection after each step. These are software/synthetic measurements; no field throughput claim is made.


The nine-event fixture demonstrates that a reservation is a separate lifecycle from robot completion: it is committed only after an independently qualified package and a readiness/occupancy snapshot, and it remains active until a later referee release event. Event digests and receipt digests in this fixture are synthetic placeholders for schema and replay shape only; they are not valid cryptographic attestations.

Reservation semantics are v0.1 exclusive-capacity-one and half-open: [start_tick, end_tick). The BTree index stores active, non-overlapping intervals per resource; adjacent intervals are allowed. A claim is retained after its lease-valid-until tick until explicit referee clearance. Active reservations block work-package supersession and resource-catalog version changes, so a later design/site update cannot silently invalidate a resource already physically committed. The index is derived from the event stream and must be rebuilt by replay at startup.

The executable benchmark emits two distinct distributions at 100, 1,000 and 10,000 active reservations: successful atomic append latency and rejected overlapping-claim latency. It does not benchmark a distributed database, authenticated event ingestion, cross-process contention or actuator response. A production implementation must use an atomic database transaction or compare-and-append against the ledger head; in-memory &mut serialization is not a distributed lock service.


### Interoperability boundary: VDA 5050 v3.0 and ISO 21423:2026

The adapter strategy is to preserve industrial robot/fleet protocol meaning while keeping physical assurance and resource authority outside the transport protocol.

- VDA 5050 v3.0 (March 2026) adds a zone concept for freely navigating mobile robots, planned/intermediate path sharing and revised operating/blocking concepts. Its scope is a communication interface between a fleet control system and mobile robots, not a construction safety case.
- ISO 21423:2026 (published October 2026) specifies communication requirements among industrial mobile robots, fleet managers, other machinery/equipment and applications in shared industrial environments. It explicitly excludes safety requirements and navigation requirements. Its industrial-environment scope excludes public access and is not automatically applicable to every construction site configuration.
- A construction adapter should preserve order/task identity, vehicle identity, zone/contour semantics, path-sharing provenance, action state, blocking state, timestamps/clock identity and all vendor extension data that affects execution. Unknown critical semantics must yield a typed unsupported/unresolved disposition, not silent data loss.
- A zone grant or fleet-controller message does not prove conservative physical occupancy, stopping/recovery reachability, independent site clearance or an owner-authorized construction safety permit. The VDA/ISO message is an input observation in the evidence chain, never the authority that accepts the envelope or authorizes actuator motion.

Proposed adapter conformance corpus:
1. Golden encode/decode/replay tests for VDA 5050 v3.0 orders, node/edge sequencing, zone definitions, planned and intermediate path sharing, action/blocking states and error reports.
2. Semantic mutation cases: unknown zone type, changed contour, stale path version, duplicated order/node IDs, lost intermediate path, clock-domain swap, offline/reconnect edits and out-of-order status.
3. Round-trip preservation: 100% of declared construction-relevant fields are preserved or produce a typed rejection. No critical field may be dropped or guessed.
4. Proof separation: even a protocol-valid path/zone/order must still be paired with the exact robot/tool/payload motion envelope and independent occupancy assessment; stale site state invalidates readiness and reservations.
5. Multi-vendor differential: run the same frozen tasks through a VDA 5050 v3.0 reference fleet controller, vendor robot adapter and our construction bridge; compare task meaning, event lineage, conflict handling, re-planning and whole-workflow accepted output rather than transport throughput alone.

Official references:
- VDA 5050 v3.0 release: https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN
- VDA 5050 v3.0 release and source: https://github.com/VDA5050/VDA5050/releases
- ISO 21423:2026: https://www.iso.org/standard/86749.html

This adds a new explicit layer to the evidence architecture:

Protocol intent/status → normalized construction task semantics → independently qualified body/tool/payload envelope → fresh occupancy and exclusive resource reservation → separate safety authority → physical execution → independent referee acceptance.


### Capacity-N resource research oracle (separate from exclusive reservation v0.1)

The experimental module cumulative_resource_capacity implements a deterministic weighted sweep-line evaluator for shared-capacity resources. Each resource has a finite integer capacity and domain unit (for example kW, kg, pallet positions or parallel bays). Each reservation contributes one or more claims with a resource-definition digest, demand units and a half-open tick interval.

At each tick, ending demand is subtracted before new demand is added. This makes adjacent [a,b) and [b,c) reservations non-overlapping, while concurrent demands accumulate. The oracle reports peak load, over-capacity intervals and over-capacity duration per resource. It uses checked u128 sums so combining several u64 demands cannot silently wrap.

This is deliberately a research oracle rather than a scheduler or ledger mutation. It does not decide priorities, preempt reservations, estimate resource capacity from sensor data or authorize execution. The caller must bind the resource definition, site snapshot, clock domain and reservation receipts to independently authenticated source evidence. The exclusive v0.1 ledger remains capacity-one until this capacity-N model has a mature reservation/index implementation and differential evidence.

Unit tests compare capacity-one behavior to a mutex model, demonstrate capacity-N sharing and weighted overload intervals, check half-open boundaries and digest/clock/resource mismatches, verify input-order determinism and compare randomized peak loads against a discrete-tick reference oracle.

The CI microbenchmark evaluates 100, 1,000, 10,000 and 100,000 claims across ten resource definitions, reporting p50/p95/p99 sweep latency, boundary count, violation segment count and feasibility. These are synthetic algorithm measurements only.


### Robust time-varying capacity derating oracle

The research module robust_capacity_profile extends the capacity-N sweep with owner-attested, time-varying capacity windows. Examples include a hoist derated by wind, a shared temporary power feed whose available kW changes, a staging area partly unavailable, or a workcell with fewer active bays. Each window is bound to the resource definition digest, site snapshot, owner identity and an evidence receipt, and carries lower/nominal/upper capacity bounds over a half-open site-clock interval.

The classification is deliberately asymmetric:
- WithinLowerBound: observed demand is no greater than the qualified lower capacity bound. This is the only robustly feasible result.
- IndeterminateWithinCapacityRange: demand exceeds the lower bound but not the upper bound. The available capacity is not established tightly enough to admit the work.
- DefinitelyOverUpperBound: demand exceeds even the upper bound.

Windows must be non-overlapping, bind to the same frozen site snapshot, resource-definition owner and explicit site clock domain, have valid evidence digest syntax and obey lower ≤ nominal ≤ upper. The site-clock domain is repeated on every availability window and must equal the base scenario clock; integer ticks from another site clock are rejected. Each resource definition carries its responsible capacity_owner_id, and the window must name exactly that owner. The canonical resource-definition digest must cover the owner identity and its scope. This is a consistency check, not authentication: callers still verify the catalog digest, owner authorization, signed evidence bytes and clock provenance upstream. This profile models derating or partial unavailability only; it cannot increase above the installed capacity in the owner-approved resource catalog. Increasing installed capacity requires a new resource definition/catalog revision, not a wider transient window.

The fixed capacity-N oracle remains the baseline. The robust evaluator combines reservation interval boundaries with capacity-window boundaries, and returns every indeterminate or definitely-overloaded interval separately. Each non-robust segment includes the exact sorted reservation IDs contributing demand in that interval; impacted_reservation_ids() returns a stable deduplicated list for targeted holds and replanning. Multiple overlapping claims under one reservation are reference-counted so a contributor remains active until its final claim ends. It never converts an uncertain capacity estimate into a safe result. The profile is still an assessment oracle: owner/evidence signatures and canonical bytes must be verified upstream, and it does not perform scheduling or actuator authorization.

Unit tests cover time-local derating, indeterminate capacity bands, exact lower-bound acceptance, zero capacity, overlapping windows, invalid receipts, stale snapshots, resource-definition mismatch, permutation invariance and prevention of transient capacity increases.

The Linux microbenchmark evaluates robust windows across 10 resources and 100/1,000/10,000/100,000 weighted claims. It reports p50/p95/p99 latency and separates indeterminate from definitely-over-capacity intervals. These are synthetic algorithm measurements only.


### Robust capacity derating input fixture and validation

schema/robust-capacity-scenario-v0.1.schema.json defines the versioned input shape for capacity resources, weighted reservations and owner-attested availability windows. examples/robust-capacity-scenario-synthetic-v0.1.json demonstrates two resources: a temporary hoist derated enough to be definitely over its upper bound, and a power feed whose demand falls between lower and upper capacity, therefore indeterminate. The hashes and receipts are synthetic placeholders. Cross-field constraints such as window non-overlap, resource-definition binding, snapshot binding and upper capacity not exceeding installed capacity are checked by the Rust evaluator, not all expressible in JSON Schema.

The robust evaluator is exported by the oracle crate and its Linux benchmark compares four sizes (100, 1,000, 10,000 and 100,000 claims) against ten resources with three time-varying windows per resource. The output separates indeterminate windows from definite overloads; a single boolean is not used to collapse those categories.
