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

The v0.1 corpus contains 13 finite cases:
- 4 positive cases: integrated, unified, federated and mixed topologies;
- 9 negative cases: update frequency, sample frequency, jitter, p95 latency, stale source, semantic fidelity, provenance/authority, clock synchronization and uncertainty.

The synthetic values are test fixtures only. They are not measured latency, real update rates, actual twin-fidelity scores or qualified uncertainty bounds.

## Benchmark metrics and targets

| Metric | Benchmark target |
|---|---|
| Frozen oracle agreement | 13/13 exact readiness + reason-code matches |
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


## Sampling cadence is not interface update cadence

Each interface now declares both `sample_interval_us` / `max_sample_interval_us` and `update_interval_ms` / `max_update_interval_ms`. The former describes acquisition resolution at the source; the latter describes how often the interface publishes or updates a value. A gateway can publish 10 ms updates while forwarding a value originally sampled every 50 ms; that does not make the underlying measurement a 10 ms sample.

The BAM companion article provides an explicit example of heterogeneous source cadence: current/voltage at 0.1 ms (100 µs), robot TCP coordinates at 8 ms (8,000 µs), and power-source OPC-UA variables at 100 ms (100,000 µs). These are not interchangeable with interface delivery latency or update frequency. The new DTC-013 negative control keeps the update interval within budget while violating the sampling-resolution limit; the only expected failure is `SAMPLE_FREQUENCY`.


## Sampling cadence is distinct from update cadence

Every interface now declares a source sampling interval in microseconds and a separate interface update interval in milliseconds. They are different properties: an adapter may publish a value frequently even though the underlying sensor acquired it less frequently. A compliant update cadence cannot compensate for insufficient sampling resolution.

The BAM Tandem-GMAW paper describes current/voltage sampled every 0.1 ms (100 µs), robot TCP coordinates every 8 ms (8,000 µs) and power-source OPC-UA variables every 100 ms (100,000 µs). These rates are preserved per modality; they are not collapsed into one update rate. DTC-013 is the negative control: the interface update interval passes, but the source sample interval exceeds its task limit, so the evaluator must report only `SAMPLE_FREQUENCY`.


## Continuous commissioning lifecycle — 2026-11-13

Commissioning is now a revocable runtime state, not a permanent green badge. The frozen corpus includes eight exact lifecycle scenarios and the Rust verifier checks every transition:

- `UNCONFIGURED → CANDIDATE → COMMISSIONED` only after configuration and passed commissioning evidence;
- optional interface budget failure moves `COMMISSIONED → DEGRADED`;
- a critical budget failure moves `COMMISSIONED/DEGRADED → SUSPENDED`;
- a source, clock, calibration, transform, model, procedure or digital-thread revision change moves an operating twin to `REQUALIFICATION_REQUIRED`;
- a degraded twin can enter a fresh candidate commissioning cycle;
- a suspended twin must clear into `REQUALIFICATION_REQUIRED`, then enter `CANDIDATE` and pass fresh commissioning before becoming `COMMISSIONED`;
- `DECOMMISSIONED` is terminal.

There is no direct `SUSPENDED → COMMISSIONED` or `REQUALIFICATION_REQUIRED → COMMISSIONED` transition. Unknown or illegal transitions fail closed. This complements the purpose-scoped READY / READY_LIMITED / NOT_READY decision: commissioning state is the lifecycle authority, while readiness is the current interface/purpose result. The additional eight Rust lifecycle tests bring the suite from 19 to 27 tests. These remain synthetic state-machine tests, not ISO certification or a live physical twin.
