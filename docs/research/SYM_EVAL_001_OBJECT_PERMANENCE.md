# SYM-EVAL-001 — Hidden-ground-truth object-permanence protocol

Status: **protocol scaffold / unqualified**

This tranche defines a passive evaluation boundary for visual object permanence. It does not claim that Symthaea, the Vision Manifold, or any current tracker passes the protocol.

## Purpose

A useful object-permanence test must distinguish at least three different behaviors:

1. preserving a plausible entity hypothesis while direct evidence is temporarily unavailable;
2. retaining the same hypothesis when the entity reappears;
3. revising or retiring that hypothesis when the hidden world actually changed.

A benchmark that tests only reappearance can be gamed by an `always persist` strategy. A benchmark that tests only disappearance can be gamed by an `always forget` strategy. SYM-EVAL-001 therefore uses a paired hidden intervention.

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
        (sensor quantities only)                       │
                   │                                   │
                   ▼                                   │
        system under evaluation                       │
                   │                                   │
                   ▼                                   │
           BeliefTranscript ───────────────────────────┘
                               post-hoc scoring only
```

The evaluated system must never receive:

- canonical entity identifiers;
- hidden branch truth;
- scenario phase labels;
- hidden removal events;
- evaluator-side target associations;
- branch-blinding nonce;
- post-hoc hypothesis mappings;
- future reveal state.

`SensorFrame` therefore exposes only frame index and sensor observations. `ObjectPermanenceTrial` exposes no branch, oracle, seed, nonce, or `Debug` surface.

## Domain-separated determinism and blinding

The protocol takes two logically different evaluator inputs:

- **scenario seed** — determines legal world/sensor content;
- **blinding nonce** — determines whether Persist or Remove occupies public trial A versus B.

The blinding nonce must remain evaluator-private until both belief transcripts are committed. It is not derived from the sensor seed and is not present in the sensor transcript.

This prevents a model that knows the benchmark source, or even recovers the scenario seed from deterministic sensor records, from predicting A/B assignment before reveal.

Only after both transcripts exist does `assess()` restore semantic `persist` and `remove` labels in the evaluator result.

## Paired intervention

For one scenario seed, the hidden branches share the same world setup and sensor-visible prefix.

| Frames | Evaluator phase | Persist truth | Remove truth |
|---|---|---|---|
| 0–3 | visible prefix | target visible | target visible |
| 4 | partial occlusion | target partly visible | target partly visible |
| 5–8 | full occlusion | target hidden | target hidden |
| 7 | hidden intervention | remains present | removed from world |
| 9–11 | reveal | target reappears | target remains absent |

The phase column is evaluator documentation only; phase is not a `SensorFrame` field.

The sensor transcript is required to be identical through frame 8. A persistent distractor remains visible throughout so the evaluator can detect a target hypothesis that silently latches onto another observation while the target is hidden.

## Post-hoc association

The system under evaluation owns its `hypothesis_id` values. The evaluator never supplies a canonical-to-hypothesis map.

After the trial, the evaluator finds the target hypothesis from the system's own source-observation lineage during the visible prefix and follows that hypothesis through the hidden and reveal periods.

```text
sensor observation -> system hypothesis -> evaluator assessment
```

never:

```text
canonical entity -> system hypothesis
```

## Lineage guards

A belief source ID can itself be wrong or fabricated, so the scorer additionally reports:

- `unknown_source_observation` when a target-hypothesis sample cites an observation that never occurred in the legal sensor transcript;
- `cross_associated_non_target_observation` when it cites the distractor;
- `unsupported_observation_refresh` when `last_observed_frame` advances to a frame for which the target hypothesis has no target-backed source lineage.

Prediction therefore cannot make observation age younger simply by changing a timestamp.

## Metrics

SYM-EVAL-001 preserves a vector of measurements rather than one aggregate score:

- `target_hypothesis`;
- `prediction_used_during_occlusion`;
- `last_observation_not_refreshed_by_prediction`;
- `unsupported_observation_refresh`;
- `unknown_source_observation`;
- `cross_associated_non_target_observation`;
- `reacquired_same_hypothesis` — Persist only;
- `removed_target_resolved_by_deadline` — Remove only;
- `prediction_overran_lost_deadline` — Remove only.

These are protocol observables, not calibrated psychological or robotics scores.

## Negative controls

The protocol tests itself against pathological strategies and invalid evidence:

### Always persist

A hypothesis remaining `OccludedPredicted` through the Remove lost deadline is reported as stale prediction overrun and cannot count as successful revision.

### Always forget

Dropping the hypothesis during occlusion and creating a new one on Persist reappearance cannot count as identity continuity.

### Prediction refreshes observation age

Prediction-only advancement of `last_observed_frame` fails both the prediction freshness check and target-lineage support check.

### Distractor capture

Attaching the distractor's sensor observation to the target hypothesis is explicitly reported.

### Invented observation

A source observation ID absent from the legal sensor transcript is explicitly reported rather than treated as evidence.

## Reproducibility

`ObjectPermanencePair::blinded(scenario_seed, blinding_nonce)` reproduces the same A/B assignment and sensor transcripts for the same pair of evaluator inputs. Holding the scenario seed fixed while changing only the blinding nonce may swap A/B assignment without changing the shared sensor prefix.

Future rendered variants should bind at minimum:

- Symtropy commit and asset hashes;
- renderer/backend identity;
- camera intrinsics/extrinsics and image-plane definition;
- scenario seed;
- a committed blinding-nonce digest before execution and nonce reveal after transcript commitment;
- physics timestep;
- render timestep;
- relevant GPU/CPU determinism class;
- Symthaea exact head and model/configuration identity;
- belief-export schema version.

## Relationship to Symthaea VIS-004

The protocol is designed to consume VIS-004 semantics without importing simulation truth into cognition:

- entity hypothesis identity;
- competing beliefs;
- revision history;
- typed spatial evidence;
- last-real-observation freshness;
- predicted occlusion state.

A later adapter may translate a qualified Symthaea belief export into `BeliefTranscript`. That adapter must remain one-way from Symthaea to evaluator.

## Next tranches

1. **SYM-EVAL-001A** — protocol qualification: compile/test/lint the hidden-truth and blinding boundary.
2. **SYM-EVAL-001B** — rendered sensor adapter: replace synthetic blobs with deterministic RGB/depth capture while keeping canonical truth evaluator-private.
3. **SYM-EVAL-001C** — Symthaea belief-export adapter: map typed VIS-004 evidence into the evaluator transcript without exposing oracle state.
4. **SYM-EVAL-001D** — seeded experiment matrix: occluder width, motion, lighting, distractor similarity, disappearance timing, and re-entry location.
5. **SYM-EVAL-001E** — contradiction/recovery evidence report with exact-head artifacts and scenario-qualified claims.

## Nonclaims

SYM-EVAL-001 does not establish general object permanence, human-level vision, calibrated world position, canonical identity recovery, navigation competence, manipulation competence, or motor authority. Passing future instances demonstrates only the exact qualified scenarios and configurations executed.
