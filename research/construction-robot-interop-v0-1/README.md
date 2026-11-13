# Construction Robot Interoperability & Mixed-Fleet Conformance — v0.1

**Status:** research protocol / synthetic conformance contract. Not a safety standard, production fleet manager, or field qualification.

## Purpose

Qualify whether heterogeneous construction robots, fleet managers, site systems and evidence services can coordinate the same construction work without losing task meaning, site context, authority boundaries or evidence lineage.

The design intentionally does **not** create another message transport or replace vendor controllers. VDA 5050, ISO 21423 and existing vendor interfaces are comparators and potential transports. This profile defines the construction-specific contract above them.

## The paradigm shift

A transport-level success is not useful-work interoperability.

The unit of interoperability is a **construction work contract** that survives vendor adapters and system boundaries:

`approved work package + design revision + frame/site epoch + capability envelope + resource constraints + postconditions + evidence requirements`

A task can be accepted by an executor, queued, started, blocked, physically completed, inspected and finally accepted. These are different states. No adapter may turn a robot's `completed` message into independent acceptance.

## Current industry baselines

- **VDA 5050 v3.0.0** (March 2026) is the current open interface for central fleet control and mobile robots. It adds free-navigation support, zones and path sharing. It targets mobile-robot fleet operation / intralogistics rather than construction-specific work-package semantics. Official sources: [VDA current version](https://www.vda.de/en/topics/automotive-industry/vda-5050), [official specification repository](https://github.com/VDA5050/VDA5050).
- **ISO 21423:2026** (published 14 October 2026) specifies communications among industrial mobile robots, fleet managers and other equipment/applications in a shared industrial environment. Its stated scope excludes navigation and safety requirements and excludes public-road use. Assess site applicability; do not claim that it applies to all construction sites or grants safety authority. [ISO catalogue](https://www.iso.org/standard/86749.html).
- **MassRobotics AMR Interoperability Standard** provides an existing mobile-robot data-sharing baseline covering location, speed, direction, health, tasking and availability. [Specification repository](https://github.com/MassRobotics-AMR/AMR_Interop_Standard).
- Construction research remains less mature in field validation than factory mobile-robot integration. A 2026 PRISMA review of 375 studies reported 19.3% field-tested/real-world deployments, 58.9% lab-based prototypes, 13.9% simulation/virtual testing and 7.1% conceptual/theoretical work. A separate 2026 review of autonomous earthwork machinery identifies cross-vendor fleet communication as a deployment gap. These are literature synthesis results, not a population census of commercial robots.

We should **reuse** established interfaces and test their semantic fit. Do not create a proprietary competing transport protocol.

## Scope and ownership

Compose existing Symtropy work:
- #1834 — task-to-capability matching;
- #1843 — accepted construction work under failures and replanning;
- #3566 — end-to-end permission-to-evidence conformance;
- #3542 / #3558 — worksite epochs, requalification and observation coverage;
- #3521 / #3522 — physical trial receipts and claim admissibility;
- #3605 — accepted-output definitions and production accounting;
- #3591 — cross-laboratory reproducibility;
- #3331 — component identity and evidence continuity.

This work owns the normalized interop profile, construction-specific mappings, adapter conformance vectors and mixed-fleet comparison. It does not create a second safety controller, identity/approval ledger, event store, survey engine, task planner, fleet manager or acceptance authority.

## Interoperability layers

Conformance is reported independently at each layer, not as one generic PASS.

1. **Transport and syntax:** versioned, bounded messages; authenticated sender identity is verified by the applicable external identity mechanism.
2. **Semantic data:** units, frame, design revision, site/work-package/zone identity, resource IDs, capability terms and time semantics retain meaning.
3. **Task lifecycle:** submit, accept, queue, start, pause, block, resume, cancel, revoke, physically complete, fail, inspect and independently accept/reject have explicit state semantics.
4. **Coordination:** work zones, corridors, staging, charging/fuelling and other shared resources have explicit reservation, expiry and conflict semantics.
5. **Evidence continuity:** task, input, configuration and event digests link to physical measurement and acceptance receipts; adapters cannot self-award evidence grade or acceptance.
6. **Authority separation:** scheduling and interop messages never become a safety permit or motion authorization. Safety remains under the approved machine/site safety architecture.

## Normalized task contract

Every task binds, as applicable:
- immutable task/work-package/project/site/work-zone IDs;
- task-family and functional-unit class;
- exact design/model/export revision and content digest;
- target coordinate frame, site epoch and transform/control evidence references;
- material/component identity and lot where relevant;
- capability requirements (payload, reach, tool, process, terrain, environment and precision as applicable);
- preconditions, postconditions and task dependencies;
- resource/zone reservations;
- deadline and priority semantics, cancellation/revocation and idempotency policy;
- required physical evidence and independent acceptance profile references;
- operating-domain boundary and unsupported-condition response.

An adapter must not invent missing fields, silently convert units or coordinate frames, or coerce an unsupported task into success.

## Message envelope and ordering

The profile requires message ID, schema version, actor and actor-session identity, task ID, task-scoped sequence, idempotency/correlation keys, source time with clock uncertainty, receiver-observed time, site epoch, design/frame binding where applicable, payload digest and explicit message kind.

The reference core scopes sequence to (actor, session, task), starts a new task stream at sequence 1 and requires contiguous events. Duplicate delivery with the same message ID/digest is idempotent; reuse of an ID with different content is a conflict. A gap, stale sequence or changed actor session is rejected without changing task state. A changed session requires an externally validated state checkpoint before the new session is admitted. A globally synchronized clock is not assumed; the core does not infer event order from wall-clock timestamps.

This is an envelope and state-semantics contract, not a cryptographic identity or event-store implementation.

## Required task states

`submitted -> accepted -> queued -> started -> paused/blocked -> resumed -> executor_completed -> measurement_pending -> independently_accepted | independently_rejected`

Additional terminal/hold states include `failed`, `cancelled`, `revoked_or_stale` and `indeterminate`.

Rules:
- executor acceptance is not physical completion;
- physical completion is not independent acceptance;
- only an exact, authorized external acceptance receipt can support independently accepted state;
- no direct transition from `executor_completed` to `independently_accepted` without that receipt;
- stale site epoch, design revision, frame or machine configuration prevents task start/resume under the prior qualification;
- failure, conflict or unknown state must not be silently coerced to success.

The reference transition oracle may verify receipt reference presence and binding; it cannot authenticate the external authority or decide whether the physical work meets engineering requirements.

## Shared-resource reservation

Resources include work zones, access corridors, crane/telehandler operating envelopes, material staging, charging/fuelling, lifts/hoists and protected survey-control areas.

A reservation binds resource ID/revision, owner, task, scope, time interval, expiry, priority/override policy and source digest. Exclusive conflicts must be rejected. Expiry, owner loss, clock uncertainty, contradictory reservations or site-epoch changes trigger the declared invalidation path. A reservation is a scheduling fact only, never a safety permit.

## Executable reference core

The dependency-free Rust crate in `src/lib.rs` implements a deterministic task-lifecycle oracle:

- immutable binding to site epoch, design revision digest, target frame and robot configuration digest;
- per-actor/session/task sequence continuity, duplicate idempotency and conflicting message-ID rejection;
- explicit actor-session reconciliation after reboot;
- fail-closed start/resume on stale task context;
- distinct executor completion, measurement pending, independently accepted and independently rejected states;
- acceptance-receipt binding to exact task, context and output-evidence reference.

Run it with:

```sh
cargo test --manifest-path research/construction-robot-interop-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-robot-interop-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

The dedicated GitHub Actions workflow checks Rust 1.96 formatting, tests, metadata/lockfile consistency and strict Clippy on Ubuntu, Windows and macOS. Its concurrency policy does not cancel an in-progress matrix when a newer commit arrives.

This core checks structural reference bindings only. It does not authenticate actors, verify receipt signatures or authority, compute hashes, evaluate construction tolerances, coordinate real resources, or grant motion/safety permission. Resource-reservation evaluation and protocol adapters remain later stages.

## Synthetic differential adapters

The `adapters` module contains two deliberately different payload shapes that normalize to one canonical construction work contract. Vendor B uses milligrams and micrometres; the mapping converts only exact integer multiples to canonical grams and millimetres. Missing design/frame/epoch/tool fields, unsupported task families and lossy unit conversions are typed rejections. The differential tests compare full normalized task equality, then mutate frame, epoch and output class as negative controls.

These are synthetic test adapters in one Rust crate, not two independently authored vendor implementations and not VDA 5050 / ISO 21423 implementations. They establish a local semantic test seam only. The next conformance step is to add an adapter implementation independently maintained outside this module and compare against an external finite-state/schema oracle.

## Shared-resource reservation evaluator

The dependency-free `resources` module adds a deterministic calendar for exclusive and capacity-limited resources. It uses half-open time intervals, a declared common clock domain, explicit expiry, owner-bound release and site-epoch invalidation. It uses an event sweep over uncertainty-expanded half-open intervals and checks peak simultaneous occupancy, rather than summing every reservation that intersects a candidate window. This admits a valid long reservation spanning disjoint earlier/later reservations while rejecting a genuine peak overbooking. It preserves released/expired/invalidated IDs so an old reservation cannot be resurrected by reusing its identifier.

The evaluator is scheduling-only: its `Granted` result is not a safety permit, work-zone clearance or motion authorization. The caller supplies a project-specific maximum clock uncertainty in the declared site clock domain; requests above that bound are rejected, interval overlap is conservatively expanded by each request's uncertainty, and reservation expiry uses the earliest possible expiry. This does not synchronize clocks or establish that the chosen uncertainty bound is physically justified. Spatial conflict geometry, external identity authorization and live dispatch remain outside this module.

### Resource-calendar temporal broad phase (research branch / draft PR #4993)

The current implementation under research in [RESOURCE-TEMPORAL-INDEX-001 #4992](https://github.com/Luminous-Dynamics/symtropy/issues/4992) adds two indexes while keeping the exact capacity rule unchanged:

- Per-resource 60-second time buckets index every bucket touched by uncertainty-expanded half-open intervals. Reservations spanning more than 4,096 buckets use a per-resource global fallback; an oversized query falls back to the active-reservation set for that resource.
- An ordered expiry index keyed by earliest possible expiry lets `expire(now)` visit only due reservations rather than scanning the full historical ledger.
- Grant, owner-bound release, expiry and site-epoch invalidation keep active indexes synchronized. Historical records and consumed IDs remain in the reservation map.
- The original exact `overlaps()` predicate and peak-concurrency event sweep remain authoritative. Bucket inclusion may create false-positive candidates; it may not omit a true overlap.
- Public diagnostics expose indexed candidate counts and a deliberately full-scan reference count for differential tests/benchmarks. The reference method is never used for admission.

The temporal-index test corpus checks bucket boundaries, uncertainty-expanded overlaps, per-resource partitioning, long-reservation fallback, duplicate idempotency, release/expiry/epoch cleanup, and agreement with full-history overlap/expiry reference scans. The research benchmark compares sparse, multi-resource, dense same-bucket and expiry-heavy workloads. It reports p50/p95/p99 insertion and query measurements where repeated samples exist, indexed candidate counts, reference overlap counts and expiry scan comparisons.

This is an optimization experiment, not yet a production performance claim. It adds memory/index maintenance and may retain linear behavior for dense same-bucket or oversized-interval cases. The exact capacity oracle and full-scan reference must remain in the test suite.

The Rust source currently contains 44 unit tests on the temporal-index branch: 13 lifecycle, 24 resource-calendar and 7 synthetic-adapter cases. Exact-head cross-platform CI is still required before calling this branch validated.

## Frozen reference corpus

The v0.1 machine-readable corpus is in `fixtures/construction-robot-interop-cases-v0.1.json`. It covers:
- VDA 5050 field mapping and explicit unsupported construction semantics;
- equivalent normalized tasks across two adapters;
- task/capability mismatch and unknown units/frames;
- stale design/site epoch/configuration;
- duplicates, conflicting message IDs, sequence rollback, reorder and session reset;
- reservation conflicts, expiry and site-epoch invalidation;
- executor-completed without inspection; inspection reject; mismatched acceptance receipt;
- broker outage/reconnect, fleet-manager restart and replay;
- external safety hold propagation and worker-data minimization.

The corpus is synthetic, hand-checkable and designed to seed a deterministic finite-state oracle, property-based tests, message fuzzing and blind combination holdouts. It is not field evidence.

## Baselines and comparative method

Compare the same frozen work contracts against:
1. direct vendor-specific point-to-point integrations;
2. a common gateway over vendor-specific APIs;
3. VDA 5050 v3.0 mapping where the target device/task is in scope;
4. ISO 21423:2026 scope mapping where applicable (do not claim conformity without the normative text and appropriate assessment);
5. this construction-specific normalized task and evidence profile.

Measure: required integration effort; custom fields; semantic field loss; unsupported-task detection; task-state divergence; duplicate/replay safety; outage reconciliation time; resource conflict detection; handoff correctness; evidence continuity; independent acceptance binding; and latency distributions with clock uncertainty.

## Prospective targets (not standards or achieved results)

- 100% preservation of required task, site, frame, epoch, unit, capability and authority fields, or explicit typed rejection.
- Zero silent field loss or implicit unit/frame conversion in frozen tests.
- Zero unintended state changes from duplicate, stale, replayed or conflicting messages.
- Zero stale-epoch or expired-reservation resumes in the frozen corpus.
- Zero admitted exclusive-resource conflicts in the reference coordinator.
- 100% preservation of input/configuration/evidence references.
- Zero conversion of executor completion into independent acceptance without the exact authorized receipt.
- 100% agreement with an independent finite-state oracle on the frozen transition corpus.
- Report p50/p95/p99/max latency and recovery time with sample counts and clock uncertainty. Do not impose a universal latency threshold before a task family defines its actual deadline and timing budget.
- Publish unsupported features and version incompatibilities; no composite score may hide a hard failure.

## Staging

- **Stage 0:** schemas, deterministic reference state machine, fixtures and synthetic adapters.
- **Stage 1:** at least two independently implemented adapters; differential semantic tests over identical work contracts.
- **Stage 2:** hardware-in-the-loop using physically isolated, non-actuating or explicitly safety-approved configurations; inject communication and state faults.
- **Stage 3:** controlled, site-authorized trial with at least two vendors and two sites, matched work packages, common independent referee, raw evidence and full workflow accounting.

Software CI does not authorize progression to physical stages.

## Safety, privacy and claim ceiling

This profile does not qualify navigation, functional safety, cybersecurity certification, physical compatibility, productivity superiority or construction-site suitability. ISO 21423:2026 itself is not a safety or navigation standard. Physical operation requires the responsible site's approved risk assessment, safety architecture, permits and competent personnel.

Use task-scoped opaque worker IDs where worker participation must be attributed. Do not collect biometrics, infer affect/intent, or expose personal identity merely to score robot interoperability.

Passing v0.1 shows only that the declared semantics and transitions are enforced for the frozen corpus and adapters actually tested. It does not prove universal interoperability or field reliability.
