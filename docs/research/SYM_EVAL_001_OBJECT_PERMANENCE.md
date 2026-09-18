# SYM-EVAL-001 — Hidden-ground-truth object-permanence protocol

Status: **protocol scaffold / unqualified**

This tranche defines a passive evaluation boundary for visual object permanence. It does not claim that Symthaea, the Vision Manifold, or any current tracker passes the protocol.

## Purpose

A useful object-permanence test must distinguish at least three different behaviors:

1. preserving a plausible entity hypothesis while direct evidence is temporarily unavailable;
2. retaining the same hypothesis when the entity reappears;
3. revising or retiring that hypothesis when the hidden world actually changed.

A benchmark that tests only reappearance can be gamed by an `always persist` strategy. A benchmark that tests only disappearance can be gamed by an `always forget` strategy. SYM-EVAL-001 therefore uses a deterministic paired intervention.

## Non-interference boundary

Canonical simulation truth belongs to the evaluator only.

```text
                         ┌──────────────────────────┐
                         │ hidden canonical world   │
                         │ CanonicalEntityId        │
                         └───────────┬──────────────┘
                                     │
                         evaluator-private only
                                     │
                   ┌─────────────────┴─────────────────┐
                   │                                   │
                   ▼                                   ▼
          sensor synthesis                       passive oracle
                   │                                   │
                   ▼                                   │
        SensorFrame / SensorBlob                       │
        (NO canonical entity id)                       │
                   │                                   │
                   ▼                                   │
        system under evaluation                       │
                   │                                   │
                   ▼                                   │
           BeliefTranscript ───────────────────────────┘
                               post-hoc scoring only
```

The evaluator must never feed any of the following into perception:

- canonical entity identifiers;
- hidden branch truth;
- hidden removal events;
- evaluator-side target associations;
- post-hoc hypothesis mappings;
- future reveal state.

The public sensor contract contains frame-local sensor observation IDs only. Those IDs identify sensor records, not world entities.

## Paired intervention

For a fixed seed, both branches share the same world setup and the same sensor-visible prefix.

| Frames | Phase | Persist branch | Remove branch |
|---|---|---|---|
| 0–3 | visible prefix | target visible | target visible |
| 4 | partial occlusion | target partly visible | target partly visible |
| 5–8 | full occlusion | target hidden | target hidden |
| 7 | hidden intervention | remains present | removed from world |
| 9–11 | reveal | target reappears | target remains absent |

The sensor transcript is required to be identical through frame 8. Therefore no evaluated system can infer the branch from legal sensor input before the reveal.

A persistent distractor remains visible throughout the trial. This allows the evaluator to detect target hypotheses that silently latch onto another observation while the true target is hidden.

## Post-hoc association

The system under evaluation owns its `hypothesis_id` values.

The evaluator does not hand it a canonical-to-hypothesis map. Instead, after the trial, the evaluator finds the target hypothesis from the system's own source-observation lineage during the visible prefix. That recovered hypothesis is then followed through the hidden and reveal phases.

This preserves a one-way evidence relationship:

```text
sensor observation -> system hypothesis -> evaluator assessment
```

never:

```text
canonical entity -> system hypothesis
```

## Metrics

SYM-EVAL-001 intentionally preserves a vector of measurements rather than an aggregate score:

- `target_hypothesis` — whether a prefix hypothesis can be recovered from observation lineage;
- `prediction_used_during_occlusion` — whether the hypothesis was explicitly prediction-supported;
- `last_observation_not_refreshed_by_prediction` — whether prediction preserved the last-real-observation boundary;
- `cross_associated_non_target_observation` — whether a distractor observation was attached to the target hypothesis;
- `reacquired_same_hypothesis` — persist branch only;
- `removed_target_resolved_by_deadline` — remove branch only;
- `prediction_overran_lost_deadline` — remove branch only.

These are not yet calibrated psychological or robotics scores. They are protocol-level observables.

## Negative controls

The protocol tests itself against simple pathological strategies:

### Always persist

A hypothesis that remains `OccludedPredicted` through the remove-branch lost deadline must be exposed as stale prediction overrun and must not count as successful revision.

### Always forget

A hypothesis that is dropped during occlusion and replaced by a new hypothesis on persist-branch reappearance must not count as identity continuity.

### Prediction refreshes observation age

A prediction-only belief that advances `last_observed_frame` must fail the freshness measurement.

### Distractor capture

A target hypothesis that cites the persistent distractor's sensor observation must be reported as cross-association rather than silently accepted as continued target evidence.

## Determinism

`ObjectPermanencePair::deterministic(seed)` must reproduce the same sensor transcript for the same seed. Branch differences begin only after the shared hidden interval.

Future rendered variants should bind at minimum:

- Symtropy commit and asset hashes;
- renderer/backend identity;
- camera intrinsics/extrinsics and image-plane definition;
- scenario seed;
- physics timestep;
- render timestep;
- relevant GPU/CPU determinism class;
- Symthaea exact head and model/configuration identity;
- belief-export schema version.

## Relationship to Symthaea VIS-004

This protocol is designed to consume the semantics established by the VIS-004 work without importing its hidden truth into cognition:

- entity hypothesis identity;
- competing beliefs;
- revision history;
- typed spatial evidence;
- last-real-observation freshness;
- predicted occlusion state.

A later bridge may translate a qualified Symthaea belief export into `BeliefTranscript`. That bridge must remain one-way from Symthaea to evaluator.

## Next tranches

1. **SYM-EVAL-001A** — protocol qualification: compile/test/lint the hidden-truth boundary and negative controls.
2. **SYM-EVAL-001B** — rendered sensor adapter: replace synthetic blobs with deterministic RGB/depth capture while keeping canonical truth evaluator-private.
3. **SYM-EVAL-001C** — Symthaea belief-export adapter: map typed VIS-004 evidence into the evaluator transcript without exposing oracle state.
4. **SYM-EVAL-001D** — seeded experiment matrix: occluder width, motion, lighting, distractor similarity, disappearance timing, and re-entry location.
5. **SYM-EVAL-001E** — contradiction/recovery evidence report with exact-head artifacts and scenario-qualified claims.

## Nonclaims

SYM-EVAL-001 does not establish general object permanence, human-level vision, calibrated world position, canonical identity recovery, navigation competence, manipulation competence, or motor authority. Passing future instances demonstrates only the exact qualified scenarios and configurations executed.
