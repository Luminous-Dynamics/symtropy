# Shipyard digital-twin commissioning benchmark v0.1

Tracking: [SHIPYARD-TWIN-COMMISSIONING-001 #4924](https://github.com/Luminous-Dynamics/symtropy/issues/4924)

## Why this matters

ISO 23247-5:2026 and ISO 23247-6:2026 are now published. They address digital threads and composition across independently developed manufacturing twins. ISO/TR 23247-101:2026 gives a welding-specific digital-twin use case. ISO/AWI 23247-7 (commissioning) and ISO/AWI 23247-8 (VVUQ) are approved work items under development, not published normative standards.

The benchmark turns their publicly stated themes into a reproducible engineering test. It asks whether a component twin and its interfaces meet declared requirements for update frequency, jitter, latency, freshness, semantic fidelity, provenance, clock synchronization and uncertainty, for a specific declared operational purpose.

This is not ISO certification or conformance testing. It is a research profile and synthetic contract.

## Composition modes

- **Integrated:** each integrated interface declares the canonical composition schema it maps into. The verifier rejects a missing target or silent schema divergence.
- **Unified:** separate component schemas are retained and connected by explicit interface mappings. No global common schema is silently imposed; a shared orchestrator does not erase native source meaning.
- **Federated:** component twins coordinate peer-to-peer. The verifier requires peer identity and authority evidence rather than assuming a central canonical source.
- **Mixed:** a composition graph can contain more than one mode; mode is checked per interface and every integrated edge must name its canonical schema.

## Commissioning semantics

Component identity and digital-thread identity are distinct from interface health. The evaluator checks interface-specific:
- update interval and p95 jitter;
- p95 communication/processing latency;
- source data age (freshness);
- semantic fidelity in permille;
- source / mapping digest presence and authority mode;
- clock synchronization receipt and absolute offset;
- uncertainty against the declared purpose limit.

A fast delivery does not prove a source is fresh. A shared schema does not prove semantic equivalence. A component's availability does not prove an interface meets its purpose SLA. Readiness is scoped to the purpose: failed required interfaces yield NOT_READY; failed optional interfaces yield READY_LIMITED; all gates passing yields READY.

## Frozen corpus

The v0.1 corpus contains 12 finite cases:
- 4 positive cases: integrated, unified, federated and mixed topologies;
- 8 single-dominant-fault negatives: update frequency, jitter, p95 latency, stale source, semantic fidelity, provenance/authority, clock synchronization and uncertainty.

The synthetic values are test fixtures only. They are not measured latency, real update rates, actual twin-fidelity scores or qualified uncertainty bounds.

## Benchmark metrics and targets

| Metric | Benchmark target |
|---|---|
| Frozen oracle agreement | 12/12 exact readiness + reason-code matches |
| False-ready on seeded required-interface faults | 0 |
| Missing/unbound provenance accepted | 0 |
| Semantic-fidelity failures promoted | 0 |
| Stale source hidden by low delivery latency | 0 |
| Optional fault scope | only affected purpose becomes READY_LIMITED |
| Composition modes | integrated, unified, federated, mixed all exercised |

Next scale ladder: 2 / 10 / 100 / 1,000 components; event bursts; component outages; schema upgrades; adapter reconfiguration; source revision changes; clock drift; model uncertainty increase; recovery/recommissioning latency.

Do not combine all metrics into one weighted score: a required interface fault must remain visible as a readiness blocker.

## Industry comparison

Commercial PLM/MES/industrial-twin products often demonstrate data connectivity, 3D visualization, dashboards and analytics. Public announcements seldom provide matched, reproducible commissioning tests with identical topology, update load, schema semantics, p95/p99 latency, stale-source injection, authority/provenance checks and degraded-operation scope. We should not claim superiority without a matched vendor baseline.

Our intended differentiation is the evidence-bearing commissioning receipt: purpose-scoped readiness, source/transform digests, exact schema and clock revisions, separate latency/freshness/fidelity budgets, transparent failure reasons, and automatic invalidation when dependencies change.

## Ownership

- #4433 owns process/pass/equipment and workpiece trace identity.
- #4614 owns physical model VVUQ.
- #4633 owns weld sequence/fixture/release optimization.
- #4643 owns BAM source rights/admission.
- #4649 owns inter-pass pass recommendations.
- #4924 owns twin composition/commissioning profile. It consumes component evidence but does not replace any source owner or safety authority.

## Claim ceiling

A PASS proves only that the frozen synthetic corpus matches its commissioning oracle. It does not establish ISO 23247 conformance, safety under ISO 10218, physical model validity, source-authority correctness in production, operational readiness of a real shipyard or permission to command equipment.
