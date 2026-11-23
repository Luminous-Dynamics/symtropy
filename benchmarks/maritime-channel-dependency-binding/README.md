# Maritime channel dependency binding v0.1

**Purpose:** a synthetic-first, read-only maritime profile adapter for evaluating whether a source channel and a candidate witness channel are separated for specific evidence stages and fault-domain dimensions.

This is deliberately not a new dependency graph, graph traversal engine, trust score, cut-set enumerator, terminal connector, or operational authority.

## The paradigm shift

A conventional visibility system may show several records, devices, endpoints, providers, signatures, or dashboards. Their count does not establish the number of independent observations or failure domains.

The unit of assessment here is:

> one claim × one as-of snapshot × one source/witness role pair × one stage × one canonical fault-domain dimension

The adapter retains the exact dependencies and path witnesses for that unit. It does not collapse a five-stage dependency vector into a global `independent=true`, a source count, or a scalar confidence score.

The five stages are:

1. **Detection** — physical sensor/reader, installation/read-point, power, clock/calibration and local acquisition.
2. **Transport** — gateway/runtime, network/backhaul, broker/intermediary and transport trust roots.
3. **Persistence/retrieval** — database/event store, object store/archive/index, retention and retrieval/export paths.
4. **Interpretation** — resolver/translation, schema/code-list, verifier implementation and upstream evidence lineage.
5. **Governance/control** — operator/tenant, repository authority, signing/trust roots and provider control domains.

The same two channels may be separated at detection but share a translator, broker, storage provider, operator, or trust root at another stage. Those results remain distinct.

## Owner boundaries and schema status

The canonical dependency/corroboration relations remain owned by Mycelix LEX-NET-030 / [#1243](https://github.com/Luminous-Dynamics/mycelix/issues/1243), with correlation-profile semantics in [#1371](https://github.com/Luminous-Dynamics/mycelix/issues/1371). The transitive graph and closure projection remain owned by [Mycelix #6124](https://github.com/Luminous-Dynamics/mycelix/issues/6124). The multi-valued receiving model is proposed in [Mycelix PR #6121](https://github.com/Luminous-Dynamics/mycelix/pull/6121), currently pinned in this adapter to draft head `92948ecc24392a25cec77dd646130f9d98e8c790`.

Important: #6121 is a **draft PR stacked on #5696**, and #6124 has not yet published a final, mainline closure wire schema. Consequently:

- `CLOSURE_SCHEMA` is explicitly named `mycelix-closure-projection-consumer-contract-v0.1`; it is a **local consumer-contract draft**, not a claimed Mycelix API/schema.
- The crate currently consumes synthetic, hand-authored closure projections. It does not import Mycelix code or assume a released Rust type.
- The adapter's copy of the 15 `FaultDomainDimension` variants is pinned to #6121's reviewed draft revision, not a stable dependency. Any upstream change must trigger explicit compatibility review and fixture updates.
- The maritime-to-generic dimension mapping below is proposed adapter policy. It requires owner review before it can be used in a real integration.

The graph owner must provide stable canonical identities, graph/snapshot/profile digests, role bindings, exact node/edge path witnesses, currentness and per-role/dimension inventory completeness. The adapter only evaluates the supplied projection. It does not infer hidden dependencies from device names, vendor labels, IP ranges or organizational names.

## Proposed dimension mapping

The right-hand column is Mycelix #6121's current draft vocabulary. Maritime-specific classes remain attached to each witness so several classes can share a generic dimension without losing the original subtype.

| Maritime dependency class | Proposed owner dimension |
|---|---|
| Physical sensor/reader, installation/read-point, power supply/UPS | `InputAcquisition` |
| Clock/time root, calibration/metrology authority, measurement qualification | `SourceEvidenceLineage` |
| Firmware/configuration, gateway runtime, execution environment | `Runtime` |
| Network/backhaul, broker/intermediary, transport fabric | `ExecutionProvider` |
| Database/event store, object store/archive/index, retrieval/export path, upstream source registry | `SourceEvidenceLineage` |
| Resolver/transformation, verifier implementation | `ImplementationLineage` |
| Schema/code-list | `SpecificationLineage` |
| Repository authority | `RepositoryControl` |
| Operator/tenant control | `OperatorControl` |
| Signing authority | `TrustRoot` |

This is an intentionally small mapping into an existing owner's general vocabulary, not a claim that the generic dimensions fully model every maritime fault domain. In particular, keep the detailed subtype and exact path in the evidence. Never combine two identities into one composite digest or treat a mapping decision as evidence that the inventory is complete.

## Contract

The projection binds:

- closure consumer-contract version, owner graph schema version and source-registry version;
- graph snapshot ID, claim ID, subject digest, profile digest, graph digest and closure digest;
- an explicit as-of timestamp and projection currentness;
- exact source and witness role IDs and their binding nodes;
- all known dependency identities, maritime classes and stable node/edge path witnesses;
- `Complete`, `Partial`, `Unknown`, or `NotApplicable` inventory state for each role/dimension;
- current, stale, revoked, or superseded witness state.

A path witness must start at its role binding, terminate at the named dependency node, have consistent node/edge/relation counts, contain no repeated nodes, and not use an independence conclusion as a dependency edge. Unknown and potentially-correlated path relations remain valid *uncertainty evidence* and prevent a separation result.

A `NotApplicable` declaration includes both a rationale and an accountable owner. It does not discharge a mandatory v0.1 obligation; dependencies known on the other role remain visible in the same finding. A known common dependency may be annotated as explicitly accepted with a rationale, impact statement, and accountable owner; the finding still remains `Shared` and the profile does not claim separation.

### Per-dimension outcomes

- `SeparatedForProfile` — both role inventories are complete and current, their exact owner-native identity sets are disjoint for this dimension, and no unknown/potential-correlation path relation is present.
- `Shared` — at least one current exact dependency identity overlaps. This takes precedence over a partial remainder: the known common cause is still reported.
- `Incomplete` — a known inventory is partial, or the required dimension is absent.
- `Indeterminate` — a dependency is unknown, stale, superseded, or connected by a potentially-correlated/unknown relation, or the closure is not current.
- `NotApplicable` — an owner-backed exclusion is declared. It blocks a mandatory v0.1 obligation.
- `Invalid` — profile/schema binding, path shape, role binding, or projection is malformed, revoked, or contradictory.

Overall outcomes are `ProfileObligationsMet`, `ProfileObligationsNotMet`, `ProfileIncomplete`, `ProfileIndeterminate`, and `ProfileInvalid`. A met profile means only that the declared synthetic obligations are met by the supplied closure snapshot. It does not establish factual truth or statistical, legal, organizational, or universal independence.

## Tests and frozen cases

The crate contains 23 Rust test declarations. The generated finite panel compares the production assessment with a separately expressed set-intersection/completeness oracle for all:

- 15 non-empty subsets of a four-identity universe for the source role;
- 15 non-empty subsets for the witness role;
- four Complete/Partial combinations.

That is 900 deterministic identity-set cases. Additional tests cover:

- distinct device aliases sharing one canonical reader;
- transitive common ancestor behind different immediate gateways;
- shared identity with partial remainder;
- partial disjoint inventory, unknown inventory and stale witnesses;
- revoked or contradictory projection; malformed/cyclic path;
- `PotentiallyCorrelated` and `DependencyUnknown` relations;
- duplicate paths without duplicate identity counting; explicit accepted-shared residual risks with rationale/impact/owner, without changing the shared disposition;
- profile/as-of mismatch, missing required dimension and owner-backed non-applicability;
- fine-grained maritime component mapping and dimension-qualified identity equality.

Expected scenario labels are in `fixtures/channel-dependency-v0.1.tsv`.

**Execution status:** source and fixture are committed, but this environment does not have `rustc` / `cargo`; no local compilation, formatting, test, or Clippy result is claimed. The dedicated GitHub Actions workflow pins Rust 1.96.0 and runs those gates on the exact PR head. A workflow that is queued, skipped or not yet completed is not a pass.

## Industry and SOTA comparison

These systems operate at different layers, so a fair comparison is capability-by-capability rather than a single leaderboard:

| Industry layer | What it standardizes/provides | What this P0 does not assume |
|---|---|---|
| DCSA Track & Trace | Shared container/shipment event vocabulary and exchange semantics; 3.0.0 extends into IoT and reefer visibility | DCSA semantic conformance alone does not supply a transitive device/power/clock/storage/operator dependency audit |
| GS1 EPCIS 2.0.1 | Event and sensor-data semantics; SensorReport `time` is the actual observation time and must be no later than business `eventTime` | Event-schema conformance does not authenticate an independent acquisition path |
| Kaleris N4 TOS | Operational coordination across vessel, yard, rail, gate and control-room workflows | A TOS record or another screen is not automatically an independent physical witness |
| project44 Ocean Visibility | Carrier/forwarder connectivity, shipment visibility, multimodal event stitching, ETA and exception products | Vendor-stated coverage or a second feed is not independent ground truth; use matched, rights-cleared data and a separate adjudicator |
| World Bank / S&P CPPI | Aggregate port-level vessel-time benchmark, including waiting and berth time | It is not a terminal-level, container-level, custody, or receiver-delivery benchmark |
| NIST SP 800-82 Rev. 4 IPD / IMO maritime cyber guidance | OT risk, safety, reliability, asset/network monitoring and maritime cyber-risk control context | Security-control alignment is not proof that a specific event occurred or that two channels are independent |

Current references:

- [DCSA Track & Trace 3.0.0 announcement (30 Sep 2026)](https://dcsa.org/newsroom/track-and-trace-3-0-0-iot-reefer-visibility). The initial release is for DCSA members and DCSA+ partners; public availability is stated for H2 2027.
- [GS1 EPCIS SensorReport](https://ref.gs1.org/epcis/SensorReport) and [EPCIS 2.0.1](https://ref.gs1.org/standards/epcis/2.0.1/).
- [Kaleris N4 Terminal Operating System](https://kaleris.com/terminal-operating-system/kaleris-n4-tos/).
- [project44 Ocean Visibility](https://www.project44.com/platform/visibility/ocean/). Published connectivity/coverage/container counts are vendor claims, not independently audited ground truth for this benchmark.
- [World Bank CPPI methodology note (10 Jun 2026)](https://thedocs.worldbank.org/en/doc/855fe5e7123d85b05d4e84c95b8fd1e2-0400012026/original/Methodology-Note.pdf) and [CPPI overview](https://www.worldbank.org/en/topic/transport/publication/cppi).
- [NIST SP 800-82 Rev. 4 Initial Public Draft](https://csrc.nist.gov/pubs/sp/800/82/r4/ipd), published 21 Sep 2026 with comments due 30 Nov 2026; it is a draft, not a final standard.
- [IMO maritime cyber-risk guidance](https://www.imo.org/en/ourwork/security/pages/cyber-security.aspx), including MSC-FAL.1/Circ.3/Rev.4.

### How to compare fairly

A future comparison must use the same shipment/lane/facility cohort, exact as-of information cutoff, event-class definitions, source-rights envelope, and independently adjudicated truth. Report each system's declared scope and missingness. Compare event coverage, false/unsupported milestones, false identity merges/splits, unique/ambiguous/unmatched association, stale/correction lag, interval uncertainty, false-complete rate, and end-delivery evidence separately.

Do not compare a container-level claim against CPPI's aggregate port-level vessel-time unit. Do not compare Symtropy to project44 marketing coverage or Kaleris deployment counts as though those were matched accuracy tests. TOS and visibility platforms are potential data sources and complementary operational systems, not straw-man competitors to replace.

## Benchmark ladder and research targets

All targets below are prospective.

| Stage | Evidence required | Target metrics |
|---|---|---|
| P0 synthetic contract | Exact independent finite oracle; adversarial fixtures | 100% exact agreement; zero false `SeparatedForProfile` for shared, unknown, partial, stale, or potentially-correlated inputs; 100% preservation of known overlap and unknown remainder |
| P1 owner contract integration | Exact #6124 closure schema + #6121 receiving schema, versioned and reviewed | No silent dimension loss; exact snapshot/profile/source binding; 100% direct mapping conformance |
| P2 independent inventory audit | Independently reviewed dependency inventories and blind adjudication across distinct facilities | Measure false-separation and missed-dependency rates, with clustered effective sample sizes; aspirational corpus of ≥3,000 role × dimension decisions, not 3,000 duplicated rows |
| P3 read-only shadow | Rights-cleared terminal, rail or inland-depot observation under OT review | Capture completeness, dependency-discovery rate, unknown fraction, correction/staleness lag, p50/p95/p99 latency, memory and deterministic replay |
| P4 controlled outcome evaluation | Matched lanes/sites, preregistered intervention and independent outcome adjudication | Test whether better evidence reduces exception backlog and p90 delay without harming safety, essential-cargo prioritization, fairness, energy or emissions |
| P5 end-to-end delivery | Independent receiver evidence and custody-chain audit | False-delivery rate, unresolved handoffs, delivery completeness and durable end-to-end evidence |

The P2 sample target is not a certification threshold. With site clustering and common upstream infrastructure, report the number of independent sites, graph roots, operators, effective sample size and uncertainty intervals; never treat thousands of correlated events as thousands of independent trials.

### Future performance protocol

This adapter is a bounded set-comparison consumer, not the owner graph engine. Do not publish throughput or speed claims yet. Once a real #6124 projection is available, benchmark synthetic and rights-cleared snapshots at 10², 10³, 10⁴ and larger dependency witnesses per role pair. Record projection size, unique identities, duplicate path multiplicity, per-dimension density, p50/p95/p99 latency, peak memory, and exact output equality. Include dense repeated-identity cases, not only sparse disjoint cases.

For the separate #7009 event association crate, the next algorithmic comparison remains interval-index/sweep-window candidate pruning and Hopcroft–Karp matching versus the current DFS matcher, with the same exhaustive small-graph uniqueness oracle. Do not conflate that event-association optimization with this dependency-profile adapter.

## Claim ceiling

A passing P0 would establish only that this local consumer-contract adapter preserves the declared identities, path witnesses and dispositions on its synthetic corpus. It does not:

- authenticate, discover or validate a real terminal/rail/depot dependency inventory;
- prove statistical, legal, organizational or universal independence;
- establish physical event truth, cargo identity, custody or delivery;
- validate DCSA/EPCIS messages, calibrate clocks or verify source bytes/signatures;
- certify OT cybersecurity, authorize operations, book slots, or mutate inventory/custody;
- demonstrate superiority to DCSA, GS1, Kaleris, project44, CPPI or any commercial platform.

The next genuine field milestone is a rights-cleared, read-only source shadow with an independent OT/instrumentation reviewer and receiver-side evidence—not a more impressive synthetic score.
