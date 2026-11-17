# Construction motion adapter observation contract v0.2

## Purpose

Version 0.2 makes the adapter an instrument, not a judge. A MoveIt or Tesseract adapter returns what it actually executed and observed, bound to exact input and backend fingerprints. It cannot declare that its answer is correct.

The independent Rust scorer owns physical/analytic truth, semantic matching, confusion-matrix classification and final case verdicts. The adapter schema therefore has no PASS or FAIL value. Its execution disposition is only:

- OBSERVED: the exact runtime path was proven to execute and produced a parseable backend observation.
- UNRESOLVED: an attempt occurred, but output, capability, input semantics or execution evidence cannot support a trustworthy observation.
- NOT_RUN: the requested backend/query combination is unsupported or was not executed.

None of these means collision-free. NOT_RUN is never a true negative and UNRESOLVED is never a clear result.

## Contract contents

The schema file schema/adapter-observation-v0.2.schema.json requires:

1. Frozen case identity: case ID and SHA-256 digest from the evaluator-owned corpus.
2. Backend identity: exact backend/version/commit/build digest/runtime/plugin.
3. Scene identity: robot, tool, payload and obstacle geometry digests; frame and transform; trajectory; interpolation; collision matrix; margin profile.
4. Query semantics: DISCRETE/LVS_DISCRETE/LVS_CONTINUOUS/CONTINUOUS/ANALYTIC, robot-world vs self-collision scope, static/moving/mixed obstacles, geometry representation and contact tolerance. LVS modes must record their longest-valid-segment length.
5. Runtime capability probe: exact API path and status. An OBSERVED result is schema-invalid unless the probe says EXECUTED.
6. Execution disposition: backend observation, reason, and per-run metrics such as latency, peak RSS, allocations, broad-phase candidates, narrow-phase calls, manager clone count and evaluator count.
7. Raw receipt digest and environment: raw output/log digest, OS/architecture/CPU/thread count/allocator/build profile.

Schema validity does not prove that the digests actually correspond to the supplied files, that the probe log is authentic, or that the backend result is correct. The runner must verify those links against immutable files and the independent scorer must replay the frozen fixture.

## Matching rules

Two runs are comparable only when the scorer verifies exact equality for the required scene/query fields:

- case digest, robot/tool/payload/obstacle geometry, geometry representation and frame transform;
- trajectory timing and interpolation profile;
- collision matrix, margin profile and contact tolerance;
- query mode, scope and obstacle motion class;
- relevant backend and runtime identities.

If a backend changes a mesh to a convex hull, uses a different interpolation/subdivision length, omits tool/payload geometry, checks robot-world but not self-collision, or only supports static obstacles, that is a different comparison stratum. It must not be pooled as an apples-to-apples result.

MoveIt robot-world continuous checks and robot self-collision are separate query scopes. The current inspected MoveIt FCL continuous overload reports that continuous collision is not implemented, so the adapter must record NOT_RUN/BACKEND_UNSUPPORTED until a pinned implementation and runtime probe demonstrate otherwise. This is not a statement about all FCL implementations. See the [MoveIt FCL implementation](https://moveit.picknik.ai/main/api/html/collision__env__fcl_8cpp_source.html).

Tesseract evaluator modes are also separate. DISCRETE checks only states; LVS_DISCRETE samples interpolated states; LVS_CONTINUOUS casts each interpolated segment; CONTINUOUS casts the complete trajectory segment. Both continuous modes use convex-hull sweeps, so segment length changes conservatism and must be recorded. See the [Tesseract collision guide](https://tesseract-robotics.github.io/tesseract_nanobind/user-guide/collision/).

## Scoring contract

The scorer joins each observation with the frozen case and independently computed truth. It then creates a separate verdict record with one of:

- MATCHED_TRUE_POSITIVE
- MATCHED_FALSE_POSITIVE
- MATCHED_FALSE_NEGATIVE
- MATCHED_TRUE_NEGATIVE
- UNRESOLVED
- NOT_RUN
- NOT_COMPARABLE

A backend cannot write this verdict. A broad-phase candidate is not a confirmed collision; a reported no-contact result is not a true negative until the analytic reference says so. Unsupported cases remain visible in the denominator and are reported by geometry family and query scope.

Report TP/FP/FN/TN, unresolved, not-run, not-comparable, malformed receipt and coverage counts separately. Report p50/p95/p99 latency, throughput, peak RSS, allocations, manager clones and evaluator construction separately from planning and end-to-end task time. Never collapse accuracy, safety, abstention and productivity into one score.

## Synthetic examples

- examples/adapter-observation-tesseract-observed-v0.2.json shows a successfully executed observation. Its collision observation is not a verdict.
- examples/adapter-observation-moveit-fcl-not-run-v0.2.json shows how a known unsupported query is recorded without being misrepresented as clear.

The example digests are fixtures, not production run evidence.

## Separate scorer-owned verdict

The scorer output uses schema/score-verdict-v0.2.schema.json. It binds the same case digest to the adapter's raw receipt digest, the oracle truth receipt digest and the scorer binary digest. A MATCHED record is valid only when the adapter disposition is OBSERVED and the oracle truth is CONTACT or NO_CONTACT. The scorer then writes exactly one confusion-matrix cell:

| Oracle truth | Backend observation | Cell |
|---|---|---|
| CONTACT | collision found | TP |
| CONTACT | no collision found | FN |
| NO_CONTACT | collision found | FP |
| NO_CONTACT | no collision found | TN |

UNRESOLVED, NOT_RUN and NOT_COMPARABLE require a reason and cannot contain a confusion-matrix cell. An unknown or unsupported oracle truth cannot be forced into a negative class. This keeps abstention, missing capability and semantic mismatch visible rather than rewarding them as true negatives.

The example examples/score-verdict-false-negative-v0.2.json demonstrates a backend that executed and reported no collision while the independent analytic oracle established a contact; the scorer, not the backend, emits FN. All hashes in the example files are synthetic placeholders and are not production evidence.
