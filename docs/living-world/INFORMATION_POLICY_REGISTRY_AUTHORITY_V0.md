# Living World Information Policy Registry Authority V0

Status: normative design contract; documentation only.

## Purpose

The process-information algebra may use easy-to-construct values for tests and analysis, but canonical ecology must not let a caller self-assert the information contract that makes its own operation legal.

A canonical runtime therefore evaluates process sufficiency from **registry-resolved authority**, not arbitrary call-site policy objects.

## Core invariant

> The subject of a sufficiency decision may choose which registered process and representation it is invoking, but it may not invent or weaken the process requirements, representation capabilities, or closure evidence used to authorize that invocation.

## Registry-owned state

A versioned information-policy registry should resolve at least:

- process descriptor -> authoritative process-information profile;
- representation descriptor -> authoritative capability profile;
- closure evidence lineage -> recognized qualification record/status;
- schema/version compatibility;
- evidence revocation/staleness where applicable.

The registry identity/version is future-bearing policy state and belongs in replay/persistence/evidence manifests whenever a canonical decision depends on it.

## Process requirements

A process implementation/version has one registered information profile.

An invocation cannot:

- omit a registered requirement;
- replace Exact with a weaker closure requirement;
- lower the minimum authority level;
- alter accepted closure model/domain/error semantics;
- keep the same process version while materially changing its information needs.

A semantic requirement change requires a new process/profile version and qualification lineage.

## Representation capabilities

A canonical representation/schema has a registered capability profile.

A caller cannot append capabilities such as:

- exact joint covariance;
- exact contact structure;
- Level-A individual state;
- Level-I persistent identity;
- exact conservation-account ownership;

merely by constructing a richer capability map.

Schema adapters may define static exact capabilities, but they become canonical proof only through the selected registry boundary.

## Closure evidence

A closure claim is not trusted because it contains fields named model/version/domain/error/lineage.

Before canonical use, its evidence lineage must resolve through the selected registry and match the recognized qualification record. Unknown, mismatched, revoked, or stale evidence fails closed.

Low-level ecology does not require cryptographic PKI. The registry/evidence identifiers may be opaque there, while higher layers bind them to stronger provenance when needed.

## Sealing / mutation

V0 SHOULD distinguish bootstrap construction from runtime evaluation.

A sealed registry generation is immutable for the lifetime of decisions bound to it. Changing policy/evidence status creates a new registry generation/version rather than mutating the meaning of an old one in place.

Prepared realization, collapse, promotion, and process-activation plans that depend on registry policy bind the registry identity/version they evaluated against.

## Failure atomicity

Unknown or invalid process/representation/evidence resolution fails before ecological mutation.

Registry resolution failure MUST NOT change:

- population count/biomass;
- partition allocation state;
- active-owner state;
- process-set generation;
- ecological ledger state;
- source projection authority.

## Determinism

Equivalent registry contents are canonicalized independently of registration/insertion order.

Runtime evaluation must not depend on:

- thread order;
- hash-map iteration order;
- renderer state;
- camera distance;
- GPU/CPU load;
- wall-clock time.

## Qualification fixtures

- unknown process -> reject;
- unknown representation -> reject;
- caller-added fake Exact capability cannot alter registered truth;
- caller-weakened process profile cannot replace registered requirement;
- unknown closure lineage -> reject;
- same lineage with mismatched evidence record -> reject;
- revoked evidence -> reject;
- registered qualified evidence -> eligible for normal sufficiency evaluation;
- equivalent registration order -> identical sealed registry;
- registry version change invalidates stale prepared policy proof.

## Relationship

This contract authorizes the policy/evidence inputs consumed by the process-information sufficiency algebra. It does not itself prove spatial/temporal validity, transition reachability, Level-A ownership, or scientific closure quality.

Relates to #259, #263, #269, #279, #210, and the other Living World authority contracts.
