# Construction Evidence Dependency Invalidation Benchmark v0.1

**Status:** software benchmark preregistration. Synthetic graph results only; no site or production evidence is claimed.

## 1. Purpose

A construction project has many independently versioned evidence objects: IFC models and entities, IDS requirements and evaluations, survey frames and transforms, station qualifications, robot/tool/payload configurations, work packages, task plans, motion envelopes, execution receipts, inspection evidence and accepted outputs.

A single changed dependency should not force a site-wide restart. It must invalidate every current derived claim that depended on the changed evidence, and no unrelated current claim. Historical receipts remain immutable.

The benchmark evaluates both correctness and cost of that dependency-scoped invalidation.

## 2. Graph semantics

Every edge is directed from a dependent artifact to a prerequisite:

\`accepted output → inspection → execution → motion envelope → task plan → work package → IFC / IDS / survey / robot / tool inputs\`

The graph must be acyclic. Each node carries a version digest. An evidence snapshot binds:
- the subject node's own version digest and node type;
- every transitive prerequisite ID, type and version digest;
- the exact dependency-edge topology of the transitive subgraph.

A snapshot is current only when all of these still match. A revision plan identifies the transitive dependent closure and sorts requalification prerequisite-before-dependent. A graph change or node-version change leaves historical receipts intact while making affected current snapshots stale.

## 3. Correctness oracles

The benchmark uses two independent reference implementations:

1. **Exhaustive closure oracle:** for each changed node, traverse every node/edge in the frozen graph from scratch and compute its transitive dependent closure.
2. **Snapshot oracle:** for each evidence snapshot, compare the complete current transitive dependency closure and edge topology, not just the node's own digest.

Required invariants:
- 100% recall of dependent nodes in the exhaustive closure oracle;
- zero unrelated invalidations within the declared graph;
- 100% deterministic plans for identical graph/change inputs, regardless of input vector order;
- all requalification outputs satisfy prerequisite-before-dependent order;
- 100% of snapshots become stale when the subject version, any transitive ancestor version, or dependency topology changes;
- an unrelated component's snapshots remain current;
- duplicate/unknown nodes, malformed digests, duplicate edges, cycles, no-op revisions and blank change reasons fail closed;
- replaying the same revision input produces the same plan and does not mutate graph state unless the explicit apply operation is called.

The graph is only as complete as the declared dependencies. A missing edge can cause an under-invalidation. That is a model-quality failure, not an algorithm pass. The corpus must therefore include deliberately omitted dependency edges, common-cause dependencies and cross-domain dependencies.

## 4. Test topologies

| Topology | Stress case | Main failure mode to expose |
|---|---|---|
| Linear chain | Deep provenance: model → package → task → envelope → execution → inspection → acceptance | Stack depth, ordering and transitive propagation |
| Wide star | One shared frame/calibration feeding many packages | High fan-out and common-cause impact |
| Balanced tree | Broad dependencies with moderate depth | Queue/set scaling and ordering |
| Eight-way forest | Independent packages sharing no declared dependencies | Unrelated work must remain current |
| Shared-source DAG | Many entities depend on one IFC/IDS source, then converge on shared work packages | Multiple paths must deduplicate affected nodes |
| Multi-source mutation | Tool and survey frame change together | One combined closure; no duplicate invalidations |
| Rewired DAG | Edge is added/removed while node set remains constant | Snapshot must detect topology-only changes |
| Faulted graph | Missing, duplicate, unknown, cyclic or malformed graph members | Fail-closed graph construction and planning |

## 5. Scale programme

Measure at 10², 10³, 10⁴, 10⁵ and 10⁶ evidence nodes as the implementation supports them. For each size, record:
- node count and directed-edge count;
- maximum dependency depth and average/maximum fan-out;
- number of changed roots;
- affected and unaffected node counts;
- graph construction p50/p95/p99;
- invalidation planning p50/p95/p99;
- snapshot-currentness check p50/p95/p99;
- memory peak and allocation count;
- deterministic replay equality.

The current initial runner benchmark starts with 128, 512, 2,048 and 8,192 nodes split into eight independent package chains. It measures construction and planning and compares the selective closure to a naive whole-graph invalidation count. That is only the first baseline; it does not cover wide or convergent graphs yet.

## 6. Initial performance objectives

These are proposed engineering gates, not achieved results or industry-leading claims. Freeze host, Rust toolchain, optimization flags, allocator and graph generator before comparing.

| Scale | Proposed planning p95 | Proposed peak memory | Functional requirement |
|---:|---:|---:|---|
| 10³ nodes | ≤1 ms | ≤8 MiB | Exact closure, deterministic plan |
| 10⁴ nodes | ≤10 ms | ≤32 MiB | Exact closure, deterministic plan |
| 10⁵ nodes | ≤100 ms | ≤256 MiB | Exact closure, deterministic plan |
| 10⁶ nodes | ≤1 s | ≤2 GiB | Exact closure, deterministic plan |

These targets are deliberately explicit and revisable after a measured baseline. They are not a safety case. The preferred optimization objective is **cost per affected evidence node**, while also reporting total graph size and unaffected work avoided.

## 7. Comparison with existing industry techniques

Build systems such as Bazel/Nix already have mature dependency graphs and cache invalidation for software artifacts. BIM/CDE platforms already track model revisions, linked-model dependencies, change issues and information requirements. Robot fleet managers already coordinate task execution and route/resource conflicts.

The research hypothesis is not that dependency graphs are new. The potential contribution is a common, typed dependency graph whose invalidation semantics cross the physical construction evidence chain:
- model identity and IFC GlobalIds;
- IDS rule evaluations;
- coordinate frame, survey transform and instrument calibration;
- station qualification;
- robot, tool and payload configuration;
- continuous motion envelope;
- execution and intervention receipts;
- independent metrology and inspection;
- accepted constructed output and material traceability.

To make a superiority claim, implement matched adapters to a software build-graph engine and at least one BIM/CDE change workflow. Replay the same frozen revision set and compare dependency coverage, under-invalidation, over-invalidation, change diagnosis time, rebuild/reinspection cost and operator interventions. No such head-to-head benchmark has been run yet.

## 8. Safety and authority boundary

An invalidation plan is a statement about evidence currency. It is not itself a stop command, safety-rated function or actuation permit. The consuming execution system must fail closed when a required current snapshot is stale, unresolved or unavailable. Only the competent system owner and applicable safety controls can determine how that condition affects operation.

The reducer must not silently rewrite historical ACCEPTED records. A current asset may be flagged as requiring reinspection or reassessment after a later revision, while the prior acceptance remains an accurate historical statement about the earlier package and observed state.

## 9. Release gates

1. Synthetic graph and exhaustive closure oracle pass.
2. Property-generated DAG corpus and adversarial missing-edge/mutation corpus pass.
3. Benchmarks pass on a second runner and held-out graph distributions.
4. IFC/IDS, survey, tool and asset integrations emit the exact dependency identities they consume.
5. Site-state ledger exposes current/stale evidence snapshots to downstream task admission.
6. Physical site pilot validates that declared dependencies reflect real change propagation and that stale assumptions do not silently authorize new work.

No release gate can be skipped by a fast synthetic benchmark.
