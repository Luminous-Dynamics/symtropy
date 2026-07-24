---
title: Audio, Acoustics, and Music-State Runtime
version: 0.1
status: implementation-spec
scope: sound events, acoustic zones, propagation, machine audio, dynamic music, motif memory, accessibility, networking, LOD, and performance
owner: audio/engine/music/engineering
related:
  - vision/ACOUSTIC_CIVILIZATION_AND_DYNAMIC_MUSIC_BIBLE_V0_1.md
  - tech/CAUSAL_EXPLANATION_AND_PLAYER_FEEDBACK_RUNTIME_V0_1.md
  - tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md
  - tech/VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
  - ops/CONTENT_AUTHORING_VALIDATION_AND_PROVENANCE_STANDARD_V0_1.md
---

# Audio, Acoustics, and Music-State Runtime

## Owned Question

**What runtime connects actual world state to spatial audio, machine signatures, architecture, dynamic music, accessibility, multiplayer synchronization, and performance budgets?**

## Core Thesis

Audio consumes typed world events and continuous state envelopes. It does not infer game truth from animation or duplicate simulation.

```text
Simulation owns cause.
Audio owns audible consequence.
Music owns remembered emotional structure.
Accessibility owns equivalent information paths.
```

# 1. Sound Event Schema

```rust
struct SoundEvent {
    event_id: EventId,
    sound_semantic: SoundSemanticId,
    source: SoundSourceRef,
    transform: SpatialRef,
    intensity: Fixed,
    material_context: Vec<MaterialId>,
    state_context: Vec<StateRef>,
    persistence: SoundPersistence,
    audience_policy: AudiencePolicy,
    caption_key: Option<LocalizationKey>,
}
```

Semantic events select content variants through authored packages. Game code does not reference raw audio filenames.

# 2. Continuous Emitters

```rust
struct ContinuousSoundState {
    emitter_id: EmitterId,
    semantic: SoundSemanticId,
    operating_state: OperatingStateVector,
    load: Fixed,
    speed: Fixed,
    condition: ConditionVector,
    environment: EnvironmentRef,
    modulation_inputs: SmallVec<ParameterBinding>,
}
```

Machine audio derives from authoritative operating state and condition.

# 3. Acoustic Spaces

Use zone-and-portal acoustics with optional local probes.

```rust
struct AcousticZone {
    zone_id: AcousticZoneId,
    volume_ref: SpatialVolumeRef,
    absorption_profile: FrequencyProfile,
    reverberation_profile: ReverbProfile,
    ambient_bed: Option<SoundSemanticId>,
    pressure_medium: AcousticMedium,
    privacy_class: PrivacyClass,
}
```

Portals store openness, area, seal condition, and transmission.

Dynamic structural damage may change portal and leakage state.

# 4. Propagation

Propagation tiers:

```text
direct spatial attenuation
occlusion ray or portal path
zone transmission
important-event diffraction approximation
medium-specific propagation
```

Do not run expensive full acoustic simulation for ordinary sources.

Underwater, vacuum, dense atmosphere, solid conduction, or alien media use authored domain profiles.

# 5. Listener State

The listener model includes:

```text
body and hearing profile
helmet or enclosure
injury or modification
attention focus
accessibility settings
camera and embodiment state
```

Physical filtering must not hide critical information without an alternative cue.

# 6. Machine Signature Model

```rust
struct MachineAudioSignature {
    machine_class: MachineClassId,
    base_layers: Vec<AudioLayer>,
    load_bindings: Vec<ParameterBinding>,
    fault_bindings: Vec<FaultAudioBinding>,
    transient_events: Vec<EventBinding>,
    diagnostic_features: Vec<DiagnosticFeature>,
}
```

Fault cues remain consistent enough to learn while varying by material, environment, and machine class.

# 7. Ambient Ecology and Society

Ambient systems spawn or mix sources from actual population, weather, time, activities, and architecture.

Avoid one looping “market ambience.” Use bounded emitters, crowd aggregates, and event density tied to simulated activity.

# 8. Music State

```rust
struct MusicState {
    place_identity: Vec<MotifWeight>,
    activity: ActivityMusicVector,
    tension: Fixed,
    wonder: Fixed,
    relationship: Vec<RelationshipMotifState>,
    settlement_state: SettlementMusicVector,
    historical_memory: Vec<MotifMemoryRef>,
    player_preferences: MusicPreferenceProfile,
    silence_policy: SilencePolicy,
}
```

The music director requests phrases, stems, transitions, or generated symbolic material from bounded grammars.

# 9. Motif Memory

```rust
struct MotifMemory {
    motif_id: MotifId,
    subject: SubjectRef,
    first_context: EventId,
    transformations: Vec<MotifTransformation>,
    familiarity: Fixed,
    emotional_associations: Vec<Association>,
    cultural_owners_or_sources: Vec<ProvenanceRef>,
}
```

Motif ownership and cultural provenance are explicit. Borrowing and transformation follow content policy.

# 10. Dynamic Composition Boundary

Generated or adaptive music may control:

```text
motif selection
harmony and mode
rhythmic density
instrumentation
register
texture
form section
transition timing
```

It must remain bounded by:

```text
style and culture grammar
voice-leading and orchestration constraints
content licensing
repetition and fatigue tracking
mix headroom
narrative non-assertion
```

Music generation cannot create canonical lyrics, facts, or alien translations without authored authority.

# 11. Synchronization

Most ambient and score playback is client-local from shared semantic state.

Network-synchronized events include:

```text
diegetic performance
rhythm-dependent group activity
public ritual
alarms
signals used in gameplay
```

Clients synchronize event time and musical phase, not streamed audio.

# 12. Accessibility Runtime

Every critical semantic event may bind to:

```text
caption
visual direction indicator
haptic pattern
UI warning
screen-reader message
```

Captions distinguish:

```text
literal sound
source identity if known
interpretation if inferred
translation confidence
```

# 13. LOD and Voice Management

Priority uses:

```text
criticality
proximity
visibility or occlusion
player focus
semantic novelty
role relevance
source importance
```

Low-priority sources become aggregate beds or virtualized state rather than simply disappearing unpredictably.

# 14. Persistence

Persist only sound-relevant state that represents world truth:

```text
machine operating and fault state
acoustic portal condition
active public performance or ritual
music motif memory
player audio preferences
long-running signal phase where gameplay-relevant
```

Do not persist transient playback cursor for ordinary one-shots.

# 15. Observability

Developer tools:

```text
active voices and priorities
zone and portal paths
semantic event log
machine parameter bindings
caption and haptic coverage
music-state vector
motif fatigue
network sync drift
CPU and memory budgets
```

# 16. Performance Budgets

Representative targets:

```text
hardware voices:               platform-profiled
virtualized emitters:          high but bounded
expensive propagation paths:   reserved for important sources
music stems:                   bounded by mix profile
runtime synthesis:             asynchronous or prebuffered
caption events:                complete for critical semantics
```

Each content package declares concurrency and worst-case composition.

# 17. Acceptance Tests

1. Machine load and fault changes produce deterministic semantic audio state.
2. Zone/portal changes audibly respond to doors, seals, and structural damage.
3. Critical sound semantics have caption and at least one additional alternative.
4. Dynamic music transitions remain musically coherent and avoid rapid thrashing.
5. Motif fatigue prevents excessive repetition without erasing identity.
6. Public synchronized music remains within phase tolerance across clients.
7. Listener filters preserve physically meaningful differences and accessibility.
8. Save/load preserves persistent machine and motif state.
9. Voice management never culls higher-priority critical cues for cosmetic ambience.
10. Representative scenes stay inside audio CPU, memory, and voice budgets.

## Final Rule

```text
Audio should reveal the world’s state without becoming a second, contradictory simulation.
```
