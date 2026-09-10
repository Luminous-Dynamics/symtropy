# PB-01 acceptance matrix

PB-01 is acceptable only when the exact product head demonstrates all of the following under the dedicated Rust 1.96 qualification lane.

| Case | Expected result |
| --- | --- |
| Same authored elements supplied in different insertion order | Same exact intent |
| One-micrometre free-placement change | Different exact intent |
| Same local pose under changed exact coordinate frame | Different exact intent |
| Same authority/subject/revision with competing digests | Reject |
| Foreign or non-prior intent ancestry | Reject |
| Zero/oversized built-in primitive extent | Reject |
| Same operation graph supplied in different insertion order | Same exact plan |
| Unknown/self/duplicate dependency | Reject |
| Cyclic operation graph | Reject |
| Duplicate realization of one intent element | Reject |
| Join with missing endpoint realization | Reject |
| Join not depending on both endpoint realizations | Reject |
| Same output under changed exact compiler reference | Different exact plan |
| Changed exact planning input | Different exact plan |
| Persisted plan replayed against changed intent | Reject |
| Advisory-only constraint report | Remains advisory; no authority upgrade |
| Caller-authored hard failure | Recorded but not self-authenticating execution authority |
| Build projection | Regenerated from exact intent + plan; no Deserialize authority path |
| Unknown adapter proposal profile | Inert proposal only |
| Proposal exceeds bounded atomic limits | Reject |

A future PB-02 adapter needs additional physical-authority tests. PB-01 passing does not establish conservation, construction execution, structural correctness, commissioning, ownership, or civic authorization.
