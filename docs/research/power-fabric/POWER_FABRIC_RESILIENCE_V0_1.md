# Power Fabric resilience and common-cause analysis v0.1

**Status:** executable declared-topology analysis contract; not a reliability prediction or safety assessment  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5214  
**Parent:** change-impact PR #5218 → diagnostics PR #5217 → IR PR #5211 → service graph PR #5188

## 1. Purpose

The existing source-path evaluator reports one deterministic shortest-hop witness per source/load pair. This increment adds a separate resilience analysis over declared PowerPath edges. It does not change the existing reachability kernel.

The engine distinguishes:
- number of directed edge-disjoint topological routes;
- structural bridges and articulation nodes in the undirected PowerPath projection;
- explicitly modelled failure-domain memberships that affect the computed disjoint routes;
- completeness of the failure-domain inventory.

## 2. Edge-disjoint paths

The engine uses unit-capacity maximum flow on directed PowerPath edges. The resulting integral flow is decomposed into edge-disjoint source-to-load paths. Parallel PowerPath edges count as separate edges when they have different IDs.

An edge-disjoint count of two means only that two directed routes can be found that do not share PowerPath edge IDs in this graph. The routes may still share a source, switchboard, cable tray, enclosure, protection dependency, communication system, cooling plant, authority boundary, or other physical/common-cause factor.

Node-disjoint paths, equipment capacity, direction-dependent operating modes, voltage/frequency conditions and actual switching states are not established by this metric.

## 3. Structural cut candidates

Bridge edges and articulation nodes are computed over the undirected projection of declared PowerPath topology. Shared non-terminal nodes across edge-disjoint paths are reported separately; parallel distinct edges may allow two edge-disjoint paths to pass through one common intermediate bus. These are graph structural indicators, not validated protection devices, physical isolation points, or probabilistic failure cut sets. Directed edge-disjoint path count remains the separate directed metric.

## 4. Failure-domain records

A failure domain has:
- stable domain ID;
- domain type (upstream supply, enclosure, cable route, protection, communications, cooling, authority, other);
- member node IDs and/or edge IDs;
- evidence state and evidence reference.

Memberships must resolve to actual nodes or PowerPath edges in the exact service graph revision. Unknown evidence is retained as unknown. Multiple computed disjoint paths touching one declared failure-domain record produce a potential shared common-cause exposure.

The inventory itself is declared CompleteWithinDeclaredScope, Incomplete, or Unknown. A non-complete inventory, or any failure-domain record with Unknown evidence, can never produce a no-shared conclusion.

## 5. Independence result semantics

| Result | Meaning |
|---|---|
| FewerThanTwoEdgeDisjointPaths | The declared graph does not contain two directed edge-disjoint routes |
| FailureDomainInventoryIncomplete | Two routes may exist, but failure-domain coverage is incomplete or unknown |
| SharedModeledFailureDomainAcrossPaths | At least two computed disjoint routes intersect one declared failure-domain record |
| NoSharedModeledIntermediateDependencyDetected | No shared modeled intermediate dependency was found in a declared-complete inventory; the source/load terminals remain shared by construction |

NoSharedModeledIntermediateDependencyDetected does not prove physical or probabilistic independence. The same source and load terminals are shared by construction; those endpoints are explicitly reported. Shared non-terminal nodes are separately listed, even where distinct parallel edges keep the paths edge-disjoint. The result means only that the model found no shared non-terminal nodes or declared common-cause domain membership in a supplied, declared-complete inventory. An unknown failure-domain evidence record makes the inventory insufficient for a clean no-shared result.

## 6. Frozen benchmark targets

- Exact path-count agreement with independently enumerated small-graph fixtures.
- Parallel edges treated as distinct only when their edge identities differ.
- Zero use of measurement/control/protection association edges as energy paths.
- Shared upstream source/enclosure/route dependencies exposed when they intersect multiple computed disjoint paths.
- Incomplete or unknown inventory never reported as no shared modeled domain.
- Zero reliability probabilities without defensible failure-rate inputs and a validated probabilistic model.
- Preserve existing one-shortest-hop service-graph path outputs unchanged.

Large graph performance is not yet measured. Tarjan cut-candidate traversal, max-flow complexity, recursion depth, and memory should be benchmarked at 100, 1,000, 10,000 and 100,000 nodes before setting performance claims.

## 7. Claim ceiling

This is declared graph analysis, not load-flow, fault-current, arc-flash, protection coordination, grounding, conductor sizing, equipment reliability, site inspection, HIL, physical independence, absence-of-voltage verification, isolation, certification, or permission to energize. Failure-domain completeness is a scope assertion supplied by the caller; it is not evidence that hidden physical dependencies have been discovered.
