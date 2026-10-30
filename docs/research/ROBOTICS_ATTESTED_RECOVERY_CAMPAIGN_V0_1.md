# Attested Recovery Campaign v0.1

**Status:** typed Rust campaign join and adversarial tests authored on a stacked branch. Exact-head CI and compilation are pending. No campaign run, HIL result, physical safety claim, or SOTA advantage is claimed.

## Paradigm shift: a benchmark result is not scoreable without its evidence

The existing paired recovery scorer checks balanced scenario/arm records and keeps command-boundary hard gates separate from task utility. The command auditor checks lifecycle consistency; the receipt binds a verdict to campaign/scenario/arm/configuration and the raw trace; the Ed25519 layer authenticates a designated receipt attestor.

This module makes those layers mandatory at campaign scoring time. Every preregistered scenario × benchmark arm must have:
- a trial record;
- independently expected command identities;
- the original globally ordered command-evidence events;
- a signed receipt;
- a matching preregistered campaign/scenario/arm/profile context;
- a trusted, unrevoked independent-attestor signature that verifies against the original evidence.

A missing or invalid row makes the entire campaign incomplete. It cannot disappear from denominators and cannot be repaired by high task reward.

## API

- `RecoveryCampaignEvidencePolicy` supplies campaign ID/revision, scenario-manifest digest, trace collector ID, evaluator version, and a policy digest for each of B0–B3.
- `AttestedRecoveryTrialRecord` packages the ordinary `RecoveryTrialRecord`, expected command identities, raw ordered events, and signed receipt.
- `score_attested_recovery_campaign(expected, trials, campaign_policy, trust_policy)` requires exact scenario × arm evidence coverage, reconstructs the expected receipt context from the preregistered manifest and trial record, rejects every context mismatch, verifies the signature and re-runs the command auditor, then calls the existing paired recovery scorer.
- `AttestedRecoveryCampaignSummary` returns the existing recovery metrics, the number of verified trial records, and a campaign evidence-root digest.

## Context binding

For each trial, the verifier independently derives:
- campaign ID/revision, manifest digest, trace-source ID, evaluator version from the campaign policy;
- scenario ID and scenario digest from the preregistered expected scenario;
- arm ID and exact source revision from the trial;
- policy digest from the campaign policy's per-arm map;
- task-profile digest, plant digest, and native-controller configuration digest from the trial.

The signed receipt context must match all of these values before signature verification. The nested receipt verifier then recomputes the deterministic receipt against the original expected identities and event stream.

## Campaign evidence root

A versioned SHA-256 root commits to campaign identity, manifest/evaluator/collector context, per-arm policy digests, the trust-policy revision and signer entries, and every signed receipt in deterministic `(scenario_id, seed, arm)` order. Each leaf binds the receipt digest, trust-policy revision, signer key ID, public key bytes, and signature bytes.

The root is an integrity commitment. It is not a trusted timestamp, append-only transparency log, or proof that a trace was emitted by physical hardware. The caller must retain the original raw inputs and obtain both campaign policy and trust policy from independent trusted channels.

## Fail-closed behavior

The scorer rejects empty campaigns, missing per-arm policy digests, duplicate expected scenarios, missing scenario/arm receipts, duplicate receipt rows, unexpected trial rows, receipt-context drift, revoked/unknown/wrong-role signers, invalid signatures, tampered raw traces, and any command-auditor lifecycle error. It does not score partial evidence.

The receipt auditor currently requires at least one expected command identity per trial. A zero-command trial is not silently certified: it remains unsupported until a distinct no-command/observation-only trial contract is specified.

## Tests authored

The initial conformance tests cover:
- complete 2-scenario × 4-arm campaign with all eight signatures and receipts;
- evidence-root stability under input trial permutation;
- missing, duplicate, and unexpected trial-evidence records;
- scenario digest context mismatch;
- corrupted signature;
- revoked attestor;
- missing policy digest for one benchmark arm.

These are authored tests; CI must compile and run them before validation is claimed.

## Benchmark protocol and targets

Preserve independent scoreboards rather than building a weighted safety/reward composite:

1. **Task capability:** NIST-style repeatable elemental capability tests and RoboDojo-like seeded sim/real tasks across embodiments and capabilities.
2. **Recovery resilience:** paired B0–B3 scenarios, reporting B3–B2 paired effect sizes and confidence intervals.
3. **Command authority integrity:** stale/expired acceptance and application, deadline misses, unresolved lifecycles, and hard-gate violations.
4. **Evidence completeness/authenticity:** coverage, context mismatch, invalid signature, revoked/unknown key, and trace-mutation detection rates.
5. **Real-time systems performance:** control-loop p50/p95/p99/max latency and jitter, deadline misses, CPU, memory, and energy.

Targets (not results):
- 100% detection of each seeded integrity/context/signature mutation class.
- 0 false rejection of valid complete receipts under the matching trust-policy snapshot.
- 100% expected scenario × arm evidence coverage before any campaign score is emitted.
- 0 observed stale-epoch or expired command acceptance/application in a frozen qualification corpus.
- B3 at least +10 percentage points over B2 fault recovery with paired 95% CI excluding zero, while nominal success remains non-inferior within one percentage point.
- Report per-class numerators and denominators; no missing data is converted to success.

## What this does not establish

A valid signature proves use of a trusted attestor key, not that the attestor observed the physical platform honestly. The collector may be compromised; an Applied event may still be simulated or fabricated. Production qualification still needs authenticated native event capture, a hardware application witness independent of the command producer and attestor, HSM/TPM-backed key custody, externally anchored append-only storage, restart/rollback/replay/truncation fault injection, HIL, and supervised physical trials.

This module is not ISO 10218 compliance. ISO 10218-1/-2:2025 safety requirements remain the responsibility of the complete robot/application/cell system and its risk assessment. NIST capability test methods and mission-specific thresholds remain separate from evidence integrity. The evidence path runs after the trial and must not add blocking work to the real-time motor-control loop.