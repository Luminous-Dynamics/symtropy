# Power Fabric role-specific human projections v0.1

**Status:** executable projection contract; no human-participant study or field usability result claimed  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5158  
**Parent:** Electrical Service Graph PR #5188; resilience PR #5222

## 1. Purpose

Generate resident, operator, qualified-practitioner and emergency-responder views from the same versioned SourceMapAssessment. This increment changes explanation and detail level, not the underlying engineering claims.

It does not create a new graph, reinterpret absence of paths, calculate live electrical state, or certify an installation.

## 2. Shared source of truth

Every projection is bound to:
- graph ID and revision ID;
- graph content digest;
- written scope statement;
- graph maturity and maturity evidence;
- current/stale revision state;
- path-witness policy;
- caller-supplied projection digest.

A stale graph remains stale in every role. Stale module identities remain visible in every role. Role views cannot independently promote maturity or claim a revision is current.

## 3. Role detail policy

| Audience | Included data | Deliberately omitted |
|---|---|---|
| Resident | Load coverage summary and declared source count | Internal source IDs and technical route/evidence details |
| Operator | Load coverage plus declared source IDs | Full engineering route/evidence details |
| Qualified practitioner | Full route, node/edge identity and evidence state/reference | No safety or conformity authority is added |
| Emergency responder | Full route, node/edge identity and evidence state/reference, plus conservative scope warnings | No on-scene physical source discovery or isolation proof |

The purpose of reduced detail is to avoid presenting internal topology vocabulary as a resident instruction. It does not delete or rewrite the authoritative source graph; every role projection is regenerable from the exact same graph revision.

## 4. Required notices

Every role receives explicit caveats:
- A declared path is not proof of live energy flow, source availability, capacity or operational state.
- A missing declared path is not proof of disconnection, de-energization, isolation or safety.
- The projection does not establish absence of voltage or permission to work.
- The projection is not installation approval, inspection or Certificate of Compliance.

Additional notices are emitted for stale graph revisions, design-only maturity, stale module configurations and the one-shortest-hop path witness limitation. The path witness limitation is exposed in professional and emergency views, which show the path details.

## 5. Synthetic fixture and acceptance

- All four role views bind the same graph/revision/content digest/scope.
- Resident and operator views present reduced detail without mutating the underlying assessment.
- Practitioner and emergency views preserve ordered node/edge route witnesses and their evidence.
- No declared source path appears only as NoDeclaredSourcePathListed, never as de-energized or isolated.
- A stale graph is marked stale for all roles.
- Stale module configurations appear for all roles.
- DesignOnly maturity stays visible.
- Repeated compilation with identical input and projection digest produces structurally identical projections.

## 6. Human-factors benchmark targets

This code does not establish that people understand the projections. Test it against conventional paper single-line diagrams, circuit schedules and labels under preregistered matched tasks.

Measure:
- source/circuit identification correctness;
- recognition of alternate feeds and unknown source state;
- interpretation of outage/backup claims;
- distinction between topology and live measurement;
- recognition that software status is not isolation;
- time to correct interpretation;
- confidence calibration and severity-weighted errors;
- accessibility across language, literacy and colour-vision variation.

The initial proposed target remains at least 95% correct for preregistered non-hazardous interpretation tasks and zero critical false-safe interpretations in the frozen adversarial set. This is a research target, not a regulatory threshold or evidence of safety. Conduct screen/paper simulations and de-energized mock-up studies only; do not expose participants to energized conductors.

## 7. Claim ceiling

A projection is a human-readable view of a declared, evidence-qualified graph. It is not a live source map, load-flow result, physical source survey, isolation register, safe-to-touch determination, electrical design approval, inspection report, or Certificate of Compliance.
