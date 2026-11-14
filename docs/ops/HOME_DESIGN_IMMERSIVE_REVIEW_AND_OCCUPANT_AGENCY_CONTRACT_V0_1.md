# Home Design Immersive Review and Occupant Agency Contract v0.1

**Status:** design contract plus a validated route player, schema-versioned comfort profile, typed review overlays, and synthetic-room floating-camera example in symtropy-bevy-scene. This is not an implemented VR product, a validated building simulation, or a permit/compliance service.

## Purpose

Before committing to expensive or irreversible construction, people should be able to experience candidate homes at realistic scale, compare distinct alternatives, discover discomfort or spatial misunderstandings, and express what makes a place feel like home. This review should work on a normal monitor first; a floating camera and accessible teleport route should not require a headset. XR is an additional adapter and must never become the only way to inspect a design.

The design system's goal is not to normalize architecture into one approved style. It is to help the occupant explore a wide range of lawful, feasible choices while making constraints, uncertainties, and trade-offs legible.

## Review modes

- **Static plan and section:** always available, keyboard navigable and exportable.
- **Floating camera:** inspect room scale, sight lines, ceiling heights, circulation, and relationships between inside and outside without forcing a first-person path.
- **Guided walkthrough:** a model-space route with explicit speed ceiling, named stops, dwell time, and gaze target. A separate geometry-aware adapter must handle collision and inaccessible/blocked routes.
- **Orbit/object review:** examine a feature, assembly, room, or connection from multiple angles.
- **Accessible teleport:** user selects and confirms each stop. A timer must never silently teleport the person.
- **Head-tracked VR:** optional XR runtime applies headset pose to an explicit review origin. The route-data enum in this prototype does not implement OpenXR/WebXR, headset I/O, stereoscopic rendering, motion safety, or device support.

The architectural_review module provides validated route metadata, deterministic pose sampling, a Bevy playback component/system, waypoint pose extraction, a user-controlled floating camera, typed revision-pinned review annotations, a comfort profile, a jurisdiction-rule snapshot reference, and a model representation context pinned into review-session schema v3. Routes start paused by default; autoplay requires an explicit opt-in profile. The default profile slows the route, uses a reduced free-camera speed, disables fast-movement boost and vertical flight, and preserves static-plan review as an option. This is an initial local walkthrough primitive, not a full architectural authoring tool or production model viewer. It does not import IFC or glTF, test collisions, simulate daylight/energy/acoustics, or certify clearance.

## Occupant agency and comfort

Every experience must leave the user in control:

1. Pause, restart, exit, recenter, return to a static view, and lower or disable camera motion at any point. Direct free-camera movement and authored route playback must not move the camera concurrently; enabling free camera pauses the route. A stored comfort profile must be validated and must never accelerate the authored route above its declared timing; automatic start requires an explicit opt-in.
2. Offer adjustable movement speeds and dwell times; a single global motion speed is not suitable for every user.
3. Do not force continuous locomotion. Keep teleport as an explicit user action.
4. Separate authored camera route from a tracked headset pose; never overwrite the user's head tracking with the authored gaze direction.
5. Offer seated, standing, reduced-motion, large-text, high-contrast, captions/transcript, and non-VR alternatives as the product develops.
6. Identify assumptions and unknowns: placeholder finishes, unknown furniture, unverified views/privacy, unsimulated sun and weather, and unvalidated acoustics must not be rendered as facts.
7. Allow an owner to compare culturally, personally, and functionally different alternatives instead of scoring their taste as a defect.

A design walkthrough can reveal a perceived issue; it cannot prove real dimensions, egress safety, structural integrity, thermal comfort, legal permissibility, or the physical performance of an assembly. Those claims require separate validators and qualified review.

## Link to composable rules and homeowner freedom

The viewer should overlay constraints rather than hide them. The new `ReviewAnnotation` type makes the distinction machine-readable and ties each annotation to an exact design revision. The validator forbids labeling an owner preference as a binding failure, requires source/module provenance for binding findings, and keeps guidance and code-reform proposals informational. A stale annotation from a previous revision is rejected. Any session showing a binding requirement, binding conflict, or required-evidence overlay must also pin a `ReviewJurisdictionContext` containing a jurisdiction-profile ID and the exact composed-rule artifact ID/digest. A purely creative review may omit that context, but must not present legal overlays. These checks validate presentation contracts and digest syntax only; trusted evidence and legal-status adapters must establish the underlying determinations and verify the artifact bytes.

Each overlay distinguishes:
- binding requirement verified for the selected jurisdiction and code edition;
- required evidence or unresolved interpretation;
- engineering assumption or simulation result with uncertainty;
- non-binding guidance;
- site-specific unknown;
- homeowner preference.

Aesthetic deviation from a catalog or typical style is not itself a compliance finding. When a concept conflicts with a genuine binding requirement, show the exact rule and a lawful alternative-design, variance/appeal, or evidence-backed reform path when the jurisdiction provides one. Do not imply that the render itself has authority to override the rule.

## Compare designs without optimizing away human values

The owner-led design-intent API now includes an optional explicit 0–100 rating on owner-preference evaluations and computes a deterministic Pareto frontier instead of an aggregate "best home" score. It retains alternatives when their trade-offs are real (for example, more daylight versus greater privacy), records pairs with missing ratings as incomparable, and excludes alternatives with failed/unresolved hard constraints from preference ranking while recording why. Ratings are owner-entered, never synthesized as objective facts by the compliance engine.

Acceptance targets:
- 100% of blocked alternatives have a traceable failed hard constraint and evidence; owner-declined options are never mislabeled as noncompliant.
- 0 aggregate preference scores are created by the core.
- 100% of dominance decisions are reproducible from the explicit owner ratings and comparison snapshot.
- 100% of comparisons with an asymmetric missing rating are reported as incomparable rather than using an imputed value.
- No preference rating can change a binding rule, convert an unknown hard requirement into a pass, or grant permission to build.

## Occupant feedback is a design input, not a compliance finding

The `OccupantReviewFeedback` contract stores the user's own category, priority, note, optional desired change, design revision, route/waypoint or room-component reference, sharing scope, explicit public-sharing consent, disposition, and resulting design-change request ID where applicable.

- Preferences such as an arched window or a more intimate living room are legitimate design inputs, not violations.
- An accessibility or circulation concern prompts investigation; it does not itself prove a code breach. Verified measurements, applicable rules, and qualified review remain separate.
- Feedback from an old design revision remains historical and must not be silently applied to a later version.
- Public case-study use needs explicit consent; local/private review is the safe default.
- A feedback item accepted for implementation must link to a traceable change request. Closing or deferring it needs a reason.
- The data structure does not include sensor feeds. Camera/microphone, gaze, or body-pose recording are optional product features, not prerequisites for collecting feedback.

This creates a learning loop: experience a design → record a location- and revision-specific reaction → create a traceable design change → invalidate dependent calculations/route evidence → review the revised design. None of these steps automatically changes binding rules or declares compliance.

## Version and provenance

Review-session schema v3 records the model's declared source kind and format, fidelity label, source linear unit, coordinate-frame ID, and a non-empty list of known limitations. Fidelity labels are provenance claims, not proof of geometry accuracy: a `SurveyedAsBuiltGeometry` label requires a `SurveyedCapture` source type, but surveyed accuracy still needs independent evidence. A purely synthetic design can be explored without being labelled a measured house. The machine-readable contract is `crates/core/symtropy-bevy-scene/fixtures/review-model-context-v1.schema.json`; the synthetic example fixture is parsed by the crate's Rust regression test. The route API normalizes camera path coordinates to metres; `scale_point_to_review_metres` performs only unit scaling and rejects non-finite/overflow values. It does not transform axes, origins, handedness, or geographic datum; those remain explicit import-adapter responsibilities.

When the session shows binding-rule, binding-conflict, or required-evidence annotations, it also requires a `ReviewJurisdictionContext` that pins the jurisdiction profile and exact composed-rule artifact/digest. Creative exploration can omit legal context, but then cannot present those legal overlays. All digests in this viewer are syntax/manifest metadata until a trusted artifact store verifies actual bytes and the jurisdiction adapter validates legal applicability.

Each review route, session manifest, and capture should reference:
- immutable design revision ID;
- owner-brief revision ID;
- model artifact ID and digest metadata, plus the canonical review-manifest artifact and digest metadata;
- viewer build ID and selected review mode (static plan is supported without a route);
- route ID and version, coordinate frame, units, and selected view mode;
- renderer and asset versions;
- which materials, furnishings, lighting, weather, and site conditions are measured, simulated, assumed, or placeholders;
- optional user annotations tied to the exact revision, not rewritten into the model silently.

A new design revision must make stale captures obvious. Historic captures remain associated with the revision the person actually reviewed. The new `DesignReviewSession` API checks that the camera route, annotations, and occupant feedback all belong to the pinned revision and that the manifest records the owner brief, model artifact, viewer build, and snapshot artifact. Digest fields are format-checked only; the actual artifact store must verify the bytes. The session also has a schema version and can emit compact deterministic JSON bytes with the self-digest field blanked via `review_session_manifest_bytes`. A trusted store must hash those bytes and bind the digest to the immutable stored object; the current crate does not compute or authenticate the digest, and its encoder is not advertised as RFC 8785 canonical JSON. A review session has no independent permit or compliance authority. Occupant feedback dispositions are separately pinned to both the source revision and the revision against which the team responded. Implemented feedback needs a change request plus evidence; accessibility/circulation concerns also need independent assessment before being called implemented. A decision not to change a preference is allowed, but must remain a preference decision—not a code failure.

## Privacy and device posture

Default to local rendering and no recording. Camera/microphone feeds, eye tracking, voice, body pose, and room scans are not required to inspect a synthetic architectural model and must not be collected by default. If a later product feature uses sensor data, request specific opt-in consent, state its purpose and retention, keep a non-sensor alternative, and make export/deletion available. Do not use gaze duration as a proxy for preference or infer sensitive traits from embodied behavior.

## Validation plan and proposed benchmarks

These are targets, not measured results.

### Geometry and route correctness
- 100% of route samples are finite and deterministic for a fixed model revision, route, and elapsed time.
- Unit normalization matches reference conversions for metre, millimetre, international foot, and inch inputs within declared floating-point tolerance; non-finite and overflow inputs are rejected. This is a unit-conversion target, not geometry-transform or georeferencing validation.
- 100% rejection of duplicate waypoint IDs, invalid coordinates, degenerate look vectors, timeline overlap, invalid comfort-profile values, and violations of the route's explicit speed limit.
- 0 default automatic starts; every session manifest records the profile, and auto-start is permitted only when the session explicitly opts in. These are acceptance targets, not a substitute for human-factor testing.
- Zero automated teleport transitions: each accessible-teleport transition requires explicit user confirmation.
- 100% of collision tests use the actual selected geometry and report detected penetrations; the current route module does not yet implement collision detection.
- Static camera fallback remains available after route load or sampling failure.

### Human experience
- Test at least 20 diverse participants during a pilot; include seated/standing and users who choose non-VR review. This is a research target, not a representative universal sample.
- Measure detection rate of pre-seeded issues involving room scale, door swing, circulation, sight line, storage reach, visual privacy, and threshold transitions.
- Measure task completion, disorientation/motion discomfort reports, pauses/exits, and perceived agency. Do not maximize immersion at the expense of comfort.
- Compare VR, flat monitor, floating camera, and teleport modes rather than presuming that VR performs best for every task.
- Every experimental result publishes the task definition, participant context, limitations, uncertainty, and negative findings.

### Architecture usefulness
- The design must preserve 100% of the explicit hard constraints when a model revision is applied; otherwise it must identify conflicts rather than silently discard requirements.
- A change to a door, wall, stair, opening, or room outline must invalidate affected route/collision/clearance evidence while preserving unrelated evidence.
- A captured walkthrough must never show a ReadyForProfessionalReview or code-compliance status unless a separate versioned evidence pipeline supplies that status; the route itself can produce no such authority.

## Work sequence

1. Route metadata, deterministic sampling, Bevy playback, and an illustrative synthetic-room floating-camera example (this branch).
2. Load a versioned real design exchange model and expose static / floating / guided modes with visible assumptions and annotations.
3. Geometry collision and clearance adapter pinned to the same design revision.
4. Accessible teleport input with explicit confirmation and safe fallback.
5. One optional XR backend, kept separate from the permissive scene core.
6. Benchmark usability and issue detection against static plans and non-VR modes.
7. Connect the route review to Mycelix's immutable artifact/version evidence and to the global, jurisdiction-specific compliance pipeline; require the session's jurisdiction-context snapshot to match the composition result used to prepare binding overlays.

Until those stages are implemented and tested, this contract describes the target architecture, not delivered XR functionality.
