# AGENT-OBS-02 — observable cues, not hidden truth

This tranche creates a migration boundary for Symtropy AI:

`world/sensor presentation -> ObservableCue -> ProjectedKnowledge -> observer KnowledgeBase -> reasoner`

The bridge intentionally does not accept or expose authoritative person conditions, exact active-field charge/heat, private biometrics, or hidden capability values.

## Initial cues

- witnessed field interception at a semantic region and approach sector;
- externally detectable equipment-emission band;
- observed mobility-performance and instability bands.

All magnitudes are deliberately coarse `Trace | Low | Medium | High` bands. A sensor integration may choose whether a cue exists and how confident it is; this bridge cannot manufacture a cue from hidden state.

There is intentionally no `FieldAbsent` cue. Failure to observe a field interception does not prove that a field does not exist, is disabled, is depleted, or failed to cover the contact.

## Provenance

Every projected claim carries:

- stable claim identity;
- explicit observer routing;
- sensor/person/device source identity;
- observation tick;
- optional staleness tick;
- bounded confidence;
- existing resident privacy semantics.

The bridge produces `KnowledgeClaim`, not world truth. Reasoners may be rational and wrong.

## Current migration target

The launcher FEP system currently consumes several authoritative world resources directly, including player biometric arousal. This tranche does not rewrite that system. It creates the safe information surface required to migrate individual inputs incrementally and test that NPC cognition cannot regress to omniscient state access.

## Evidence boundary

Observed movement is not a diagnosis. A visible protection flash is not exact shield charge. Equipment emission is not proof of equipment condition. No player biometric value is part of this interface.

## Qualification

Implemented/static only until exact-head formatting, Cargo check/test/Clippy, serialization review, and lockfile validation run successfully.