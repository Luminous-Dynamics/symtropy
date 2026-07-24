---
title: Stellar Relay, Signal Provenance, and Contact Latency Runtime
version: 0.1
status: implementation-spec
scope: interstellar signals, relay chains, provenance, translation artifacts, contact timelines, censorship and silence
owner: engineering/networking/xeno/archive
related:
  - ../canon/INTERSTELLAR_CONTACT_NONINTERFERENCE_AND_LONG_VOW_DIPLOMACY_CONTRACT_V0_1.md
  - LIGHT_DELAY_COMMUNICATION_TIMEKEEPING_AND_ASYNC_COORDINATION_RUNTIME_V0_1.md
  - XENO_SIGNAL_TRANSLATION_AND_CONTACT_STATE_RUNTIME_V0_1.md
  - KNOWLEDGE_ARCHIVE_AND_HISTORICAL_EVIDENCE_RUNTIME_V0_1.md
---

# Stellar Relay, Signal Provenance, and Contact Latency Runtime

## Purpose

This runtime carries observations, messages, beacons, warnings, translations, and diplomatic claims across stellar distances while preserving causal order and evidential humility.

It must answer:

```text
what was detected
by which instrument
how it was transformed
who claims to have sent it
what representation they claimed
when it could have been created
when it arrived
what may have changed meanwhile
```

> **A signal is an event in a chain of evidence, not a transparent window into another civilization.**

# 1. Signal Layers

The runtime separates:

```text
physical carrier
raw observation
calibrated observation
feature extraction
artificiality hypothesis
symbol segmentation
translation hypothesis
interpreted claim
diplomatic action
public narrative
```

No later layer may overwrite an earlier one.

# 2. Core Schema

```rust
struct RawSignalObservation {
    observation_id: StableId,
    sensor_id: StableId,
    observer_location: SpatialState,
    receive_interval: TimeInterval,
    frequency_or_medium: SignalMedium,
    raw_payload: ContentAddress,
    calibration_state: CalibrationRef,
    environmental_context: EnvironmentRef,
    custody_chain: Vec<CustodyReceipt>,
}

struct SignalDerivation {
    derivation_id: StableId,
    parent: SignalArtifactRef,
    operation: ProcessingOperation,
    software_hash: ContentHash,
    parameters: ContentAddress,
    author: AgentOrInstitutionRef,
    uncertainty_delta: UncertaintyChange,
    output: SignalArtifactRef,
}

struct ContactClaim {
    claim_id: StableId,
    source_artifact: SignalArtifactRef,
    claimed_sender: Option<AgentOrInstitutionRef>,
    claimed_representation_scope: RepresentationScope,
    proposition: StructuredClaim,
    confidence: DomainConfidence,
    translation_hypothesis: Option<TranslationHypothesisRef>,
    knowledge_cutoff: CausalTimestamp,
}
```

# 3. Physical Carriers

Supported carriers may include:

```text
radio and microwave
laser or optical pulses
neutrino or speculative bounded channels
physical probes and couriers
modulated stellar or orbital phenomena
acoustic or field signals within local environments
horizon-only gate handshakes
```

Every carrier has:

```text
propagation model
energy and aperture requirements
noise model
bandwidth
latency
interception risk
spoofing risk
```

No carrier becomes instantaneous unless an explicitly validated horizon technology provides that capability.

# 4. Stellar Relay Graph

A relay graph contains:

```rust
struct StellarRelayNode {
    relay_id: StableId,
    position_model: PositionModel,
    operational_state: RelayOperationalState,
    ownership_and_stewardship: StewardshipState,
    accepted_protocols: Vec<ProtocolVersion>,
    storage_capacity: u64,
    power_state: PowerStateRef,
    censorship_policy: PolicyRef,
    archive_roots: Vec<ArchiveRoot>,
}
```

Relay edges are not abstract connectivity.

They require:

```text
line of sight or physical courier route
power
pointing or navigation
protocol compatibility
storage
maintenance
trust or inspection policy
```

# 5. Earliest Arrival and Receive Windows

For each signal path, the runtime computes:

```text
earliest physically possible arrival
expected receive window
uncertainty
path assumptions
relay custody
```

Consumers may not receive an artifact before the earliest arrival.

If the path changes, the system emits a new route estimate rather than rewriting history.

# 6. Artificiality Assessment

Artificiality is a hypothesis based on evidence such as:

```text
nonrandom repetition
narrowband structure
compressibility
mathematical pattern
adaptive response
directionality
energy profile
symbol-like segmentation
```

The runtime must preserve alternatives:

```text
natural source
instrument artifact
local interference
intentional signal
unintentional technosignature
unknown
```

# 7. Translation Hypotheses

A translation hypothesis stores:

```text
symbol mapping
semantic mapping
confidence by segment
training evidence
assumed agency model
known contradictions
cultural and sensory assumptions
```

Multiple hypotheses may coexist.

A renderer may present an accessible paraphrase, but the evidence viewer must retain uncertainty and alternatives.

# 8. Representation Claims

A signal may claim:

```text
I speak for myself
I speak for one habitat
I speak for a council
I carry an archived message
I am an autonomous mission
I speak for a species or world
```

The runtime validates only what evidence supports.

It records representation as:

```text
claimed
partially corroborated
contested
historical
revoked
unknown
```

# 9. Contact Timeline

Contact history uses causal events:

```text
SignalEmittedEstimated
SignalObserved
SignalCalibrated
ArtificialityHypothesisOpened
TranslationHypothesisAdded
ReplyAuthorized
ReplyEmitted
ReplyReceiptExpected
RepresentationContested
NoncontactBoundaryRecorded
RelayLost
SignalSilenceDetected
```

A system may not show a reply as received merely because the player authored it.

# 10. Silence

Silence has many causes:

```text
no sender
no reply
receiver loss
relay failure
changed protocol
political refusal
noncontact practice
extinction or collapse
dormancy
message still in transit
```

The runtime stores silence as an observation interval, not a conclusion.

# 11. Censorship and Relay Capture

Relay stewards may:

```text
delay
prioritize
inspect
redact under policy
refuse
forge metadata
suppress existence
```

The runtime preserves:

```text
custody receipts
missing sequence evidence
alternative routes
public and private archive roots
whistleblower disclosures
```

Censorship state affects knowledge, politics, and trust but does not modify the original raw artifact.

# 12. Precursor Beacons

A precursor beacon may be:

```text
active
dormant
damaged
recursive
adaptive
one-time
misconfigured
controlled by a surviving agent
```

Its age, power, or complexity does not make its message true.

The system tracks:

```text
estimated construction age
last active interval
software or behavior lineage
physical maintenance evidence
current agency evidence
historical authority claim
```

# 13. Privacy and Hazardous Information

Signals may contain:

```text
private persons
biological hazards
machine exploits
coercive cognition patterns
location-sensitive refuge data
self-replicating code
```

Access controls must be scoped and reviewable.

Raw evidence preservation does not mean universal public release.

# 14. Worldline Forks

Forks preserve:

```text
which signals existed before divergence
which relays and archives belong to each branch
which messages are still in transit
which branch emits later replies
which unique physical probe carries which archive
```

A single in-flight courier cannot deliver the same unique payload to multiple branches without explicit reality-branch rules.

# 15. Player Interface

The Field Deck should distinguish:

```text
OBSERVED
CALIBRATED
INFERRED
TRANSLATED
CLAIMED
CONTESTED
PREDICTED ARRIVAL
```

Example:

```text
RAW SIGNAL: preserved
ARTIFICIALITY: probable, 0.71
TRANSLATION: hypothesis B, low confidence
CLAIMED SENDER: "Boundary Steward"
REPRESENTATION: unverified
MESSAGE AGE AT RECEIPT: 412 ± 6 years
CURRENT SENDER STATUS: unknown
```

# 16. LOD and Performance

```text
L0 local capture: raw buffers and sensor cadence
L1 processing: feature and hypothesis updates
L2 relay transit: event-driven propagation
L3 deep background: milestone receive windows
L4 archive: immutable completed chains
```

LOD may compress raw payload storage through content addressing and retained summaries, but must not erase evidence needed to reproduce claims.

# 17. Verification Tests

1. Earliest-arrival bound is never violated.
2. Raw observations remain immutable through translation changes.
3. Alternative translations coexist.
4. Representation claims do not become verified automatically.
5. Relay censorship leaves custody evidence.
6. Silence is not converted into consent, extinction, or hostility.
7. Worldline forks preserve message ancestry and unique couriers.
8. Privacy restrictions survive indexing and summary generation.
9. Save/load preserves in-transit routes and receive windows.
10. UI labels observation, inference, translation, and claim distinctly.

# Representative Fixture

A three-relay chain receives an ancient signal in two damaged segments. Two institutions publish competing translations. A captured middle relay suppresses one segment, while a physical courier later restores it. The new evidence changes the interpretation of a noncontact boundary.

The fixture passes when every conclusion can be traced to preserved evidence and no earlier interpretation is silently rewritten.

# Hard Invariants

```text
no interpreted claim without source artifact
no translation overwriting raw evidence
no early receipt
no first speaker promoted to world representative automatically
no silence interpreted as consent
no censorship without custody consequences
no precursor authority accepted from age alone
no worldline fork duplicating unique physical couriers or messages accidentally
```
