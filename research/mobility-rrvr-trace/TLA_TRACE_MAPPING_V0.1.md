# TLA+ to RRVR trace event mapping v0.1

The typed trace event vocabulary is mapped to the pinned bounded model:

- Model: `research/mobility-restart-reconciliation/model/RestartSafeEffectContinuity.tla`
- Pinned model file blob SHA: `908ddc1d734a7cae6a5c8be358dfe4354592a78b`
- Source branch at review: `research/mobility-restart-tla-v0-1`
- Action count in `Next`: 30

The canonical machine-readable mapping is [model_action_event_mapping_v0_1.tsv](model_action_event_mapping_v0_1.tsv). The Rust mirror is `TLA_ACTION_EVENT_MAPPING` in `src/lib.rs`. A unit test checks the exact 30 action names; CI checks that the mapping count agrees with the model's `Next` relation and that each action declaration remains present.

| TLA+ action | Trace event kind | Notes |
|---|---|---|
| `ReserveAttempt` | `AttemptReserved` | Reservation must precede dispatch/native acceptance |
| `MarkDispatchStarted` | `DispatchStarted` | Dispatch boundary |
| `NativeAcceptOld` | `NativeAccepted` | Old attempt accepted in original boot/controller epochs |
| `OldDeviceApplies` | `OldCommandApplied` | Original attempt physically applies |
| `OldDeviceDoesNotApply` | `OldCommandNoEffect` | Original attempt has no physical effect |
| `OldDevicePartiallyApplies` | `OldCommandPartial` | Partial old physical effect |
| `BeginEffectRecordCommit` | `PersistenceCommitStarted` | Local persistence call begins |
| `CommitEffectRecord` | `PersistenceCommitObserved` | Durable record mutation is visible; caller acknowledgement is separate |
| `AbortEffectRecord` | `PersistenceCommitAborted` | Store call ends without a record |
| `AcknowledgeStoreCommit` | `PersistenceCommitConfirmed` | Owner acknowledges committed record |
| `AcknowledgeStoreNoCommit` | `PersistenceNoCommitConfirmed` | Owner acknowledges no durable record |
| `Crash` | `ProcessCrashed` | Crash cut must be preserved |
| `Restart` | `ProcessRestarted` | New boot epoch |
| `FenceControllerQueue` | `ControllerQueueFenced` | Controller queue epoch advances |
| `FenceDownstreamExecution` | `DownstreamGraphFenced` | Abstract downstream fence; platform graph evidence remains separate |
| `ResolvePersistence` | `PersistenceResolved` | Durable owner reconciles an indeterminate commit |
| `ObserveOldEffect` | `PhysicalEffectObserved` | Payload identifies Applied / NoEffect / Partial; it is never inferred from persistence |
| `ObserveUnknownEffect` | `PhysicalOutcomeUnknown` | Unknown remains non-retryable |
| `ObserveDisputedEffect` | `PhysicalOutcomeDisputed` | Disputed evidence remains held |
| `ResolveMission` | `MissionResolutionRecorded` | Exact M1 outcome resolution |
| `IssueRecoveryPermit` | `RecoveryPermitIssued` | Owner-issued permit reference |
| `CreateFreshTicket` | `FreshTicketCreated` | Ticket binds the new boot/controller epoch |
| `RevokeM1` | `M1AuthorityRevoked` | M1 freshness negative control |
| `RevokeM2` | `M2ApplicabilityRevoked` | M2 freshness negative control |
| `ChangeLiveBinding` | `LiveBindingInvalidated` | Current whole-binding mismatch |
| `NativeAcceptRecovery` | `NativeRecoveryAccepted` | Captures one-shot permit consumption, fresh ticket and new attempt acceptance in the abstract model |
| `RecoveryDeviceApplies` | `RecoveryCommandApplied` | New attempt physically applies |
| `RecoveryDeviceDoesNotApply` | `RecoveryCommandNoEffect` | New attempt has no physical effect |
| `RecoveryDevicePartiallyApplies` | `RecoveryCommandPartial` | New attempt partially applies |
| `NaiveRetry` | `NaiveRetryAttempted` | Deliberately unsafe control used to produce a counterexample |

## Semantics and limitations

- `Init` is represented by the trace header's initial-state commitment, not by a `Next` event.
- `ObserveOldEffect` is one abstract action with a payload outcome; the trace event's raw evidence and input commitment must preserve which outcome was observed.
- The model's `FenceDownstreamExecution` is an abstract transition. A physical trace must additionally reference the concrete execution graph, per-node receipts and independent graph-quiescence verdict.
- TLA+ action names and typed event names are not signatures or authentication. Event content and raw artifact bytes require their own source commitments and provenance.
- A trace adapter must not infer happens-before from incomparable wall clocks. This format uses an explicit predecessor graph; logical sequence is deterministic serialization order only.
- The pinned source blob can change. When the model changes, update the TSV, Rust mapping, exact expected action list, and this document together. CI checks mapping completeness against the pinned model file, but does not parse TLC's binary/JSON trace output.

This is a manually reviewed action catalogue and conformance seam, not yet a TLC trace importer or a claim of Rust/TLA+ semantic equivalence.
