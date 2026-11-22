# RRVR formal model v0.1 — scope, assumptions and claim ceiling

**Model:** \`RestartSafeEffectContinuity.tla\`  
**Bounded configuration:** three boot/controller epochs; one original logical effect attempt; at most one recovery attempt; finite persistence, observer and authority states.  
**Status:** model source and TLC configurations authored. TLC results are produced only by the dedicated exact-head workflow; no PASS is claimed before that run completes.

## Why a second fence is modeled

A controller queue epoch change does not automatically prove that an accepted command has disappeared from every downstream queue. A driver, PLC, fieldbus gateway, device firmware, or actuator-side scheduler may still retain work.

The model therefore separates:

1. **Controller queue fence:** invalidate commands/tickets from the previous controller epoch.
2. **Downstream execution quiescence:** obtain evidence that all previously accepted work can no longer execute.

The second transition is deliberately stronger than "the ROS/DDS queue looks empty". It abstracts an owner-qualified cancel/drain/fence operation at the downstream execution boundary. If a platform cannot produce such evidence, the recovery permit remains unavailable. This is a model assumption to be mapped to the real stack, not a claim that any particular driver or actuator already provides it.

## State and transition correspondence

| Model element | Owner / behavior represented |
|---|---|
| \`ReserveAttempt\` | Durable attempt reservation before dispatch |
| \`MarkDispatchStarted\` | Dispatch-start lifecycle boundary |
| \`NativeAcceptOld\` | Original boot/controller epoch ticket reaches the native acceptance seam |
| \`OldDeviceApplies/DoesNotApply/PartiallyApplies\` | Nondeterministic physical outcome, distinct from native acceptance |
| \`BeginEffectRecordCommit\`, \`CommitEffectRecord\`, \`AbortEffectRecord\`, acknowledgements | Durable effect-record transaction and lost-ack ambiguity |
| \`Crash\`, \`Restart\` | Process/host incarnation boundary; physical outcome and durable-store truth are not rewound |
| \`FenceControllerQueue\` | Controller epoch transition and invalidation of old queue work |
| \`FenceDownstreamExecution\` | Downstream cancel/drain/quiescence receipt |
| \`ObserveOldEffect\`, \`ObserveDisputedEffect\` | Independent exact-attempt effect observation |
| \`ResolveMission\` | M1 resolution of the same logical-effect/attempt identity |
| \`ResolvePersistence\` | Mycelix-owned exact reconciliation of the durable commit outcome |
| \`IssueRecoveryPermit\`, \`CreateFreshTicket\` | Owner-issued one-shot recovery authority and fresh M1/M2 ticket |
| \`NativeAcceptRecovery\` | Linearized permit consumption plus fresh native-boundary admission |
| \`RecoveryDevice*\` | Physical outcome after the recovery attempt has been accepted |
| \`NaiveRetry\` | Deliberately unsafe old-command replay, enabled only in the negative-control configuration |

The physical-effect transitions can occur while the process is crashed or after it restarts. This is intentional: an accepted command may still be executing in a downstream component after the process that submitted it has disappeared.

## Checked invariants

- \`NoDuplicatePhysicalEffects\`: at most one Applied physical effect for this logical-effect identity.
- \`NoRecoveryFromUnknownPartialOrDisputed\`: missing, partial or disputed evidence never creates a recovery admission.
- \`NoRecoveryFromApplied\`: an already-applied effect never creates a second attempt.
- \`IndeterminatePersistenceBlocksRecovery\`: ambiguous durable commit state blocks recovery.
- \`RecoveryRequiresExactNoEffect\`: admission requires exact old-attempt NoEffect observation and matching M1 resolution, plus resolved persistence.
- \`RecoveryAdmissionWasCurrentAndFenced\`: admission records current boot/controller binding, controller and downstream fences, current M1/M2, and binding equality at the native boundary.
- \`RecoveryPermitIsOneShot\`: permit consumption is bounded to one admission.
- \`OldCommandWasAcceptedOnlyInItsOriginalEpoch\`: the original ticket cannot be accepted as a new-epoch ticket.
- \`DownstreamFenceClosesPendingCommand\`: a current-epoch downstream fence leaves no old accepted command pending.
- \`DownstreamFenceRequiresControllerFence\`: downstream closure follows the controller queue fence.
- \`ReservationPrecedesNativeAcceptance\`: a durable reservation precedes original native acceptance.
- \`StoreVerdictMatchesDurableRecord\`: resolved local commit classifications agree with the abstract store record.

The negative-control configuration enables \`NaiveRetry\`. TLC is expected to find a counterexample to \`NoDuplicatePhysicalEffects\`; the workflow passes only when the expected invariant violation is observed. This demonstrates that the invariant is sensitive to the precise lost-ack/restart/replay failure being prevented.

## Deliberate limits

This model is a bounded transition-system abstraction, not a proof of the deployed controller or robot. It does not model cryptographic authentication, Byzantine owners, physical sensor completeness, false-negative observation, correlated failure domains, timing/deadlines, analog plant dynamics, multiple concurrent logical effects, an unbounded fleet, or all vendor-specific queues.

The downstream fence transition assumes a trustworthy owner can linearize cancellation against device execution: either the command has already had its physical outcome, or cancellation prevents that pending command from executing. Real hardware may not expose this primitive. A stop command, queue clear, process restart, or empty software queue is not automatically equivalent.

The model also stops after one recovery attempt. It intentionally does not claim recursive retry correctness, durable permit consumption in a real database, exactly-once physical effects, or safety certification. A subsequent tranche should refine the owner-specific queue hierarchy and storage/permit transaction semantics, then check refinement against the Rust reference contract and actual native adapter.

## Reproducible model checker

The workflow uses Java 21 and a SHA-256-pinned TLA+ 1.8.0 tool JAR from the Microsoft TlaLink release mirror (upstream TLA+ tools, build \`2ef2af8\`). The expected JAR digest is:

\`D15655EEAFF4B3F7B3D65C46B838BC0F35D131B2EF4B1E473689B62C98D4726F\`

The URL is versioned and the digest is checked before TLC runs. A changed or substituted JAR fails before model checking.
