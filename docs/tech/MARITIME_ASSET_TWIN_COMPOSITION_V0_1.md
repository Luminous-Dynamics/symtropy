# Maritime Asset Twin Composition and Commissioning Benchmark v0.1

**Status:** synthetic integration contract seed. Not a PLM/MES/EAM replacement, a physical twin, commissioning certificate, engineering acceptance or authority to operate.

## Ownership / composition

Parent: [MARITIME-ASSET-TWIN-COMPOSITION-001 #4406](https://github.com/Luminous-Dynamics/symtropy/issues/4406).

This benchmark tests the join between owner-native systems; it does not create a canonical Luminous copy of every source record.

- Symthaea owns engineering subject/configuration identity, as-designed/as-built/as-operated distinctions, model applicability and engineering requalification.
- Symtropy owns deterministic synthetic execution, replay and run-health evidence.
- Sol-Atlas owns read-only spatial/temporal projections and transformation provenance.
- Mycelix owns participant authority, custody, attestation, approval/revocation and accountable action receipts.
- External PLM, MES, QMS/NDT, EAM/CMMS, hydrographic and asset systems remain owners of their respective records.

The benchmark uses four synthetic federates: product-definition/PLM, manufacturing/MES, inspection/QMS, and maintenance/EAM. A later scenario can add Sol-Atlas projection and Mycelix authority/custody as consumer federates.

## Standards and industry comparison

The published standards landscape now provides a useful decomposition:

- **ISO 23247-5:2026** specifies the manufacturing digital thread across design, planning, production and testing: https://www.iso.org/standard/87425.html
- **ISO 23247-6:2026** addresses digital-twin composition and its integrated, unified and federated approaches: https://www.iso.org/standard/87426.html
- **ISO/AWI 23247-7** is an approved work item under development for commissioning, including configuration/initialization, connection to digital threads and frequency/latency/fidelity checks. It is not a published standard: https://www.iso.org/standard/94859.html
- **ISO 10303-239:2024 (PLCS, Edition 3)** is a published product-lifecycle-support exchange model for complex products, configuration change/effectivity, maintenance and activity history, resource/task planning and approvals: https://www.iso.org/standard/78832.html
- **IFC 4.3 / ISO 16739-1:2024** is useful for ports/waterways/infrastructure exchange but does not replace manufacturing, NDT or maintenance qualification: https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/

These standards are comparators and mapping targets, not a claim that this v0 seed conforms to them. In particular, ISO/AWI 23247-7 remains under development. A partial schema mapping is not standards compliance.

## Paradigm shift

Move from a mutable, centralized "single twin" to a **federated, configuration-controlled product/asset thread**:

`requirement -> design baseline -> manufactured item -> inspection/acceptance -> assembly -> installation -> commissioned configuration -> operation -> maintenance/repair -> requalification`

Every claim is scoped to:
- an exact asset/component and parent assembly;
- a configuration baseline, revision and effectivity interval;
- a source-owned record and its issuer/authority scope;
- state class (as-designed, as-manufactured, as-inspected, as-built, as-operated, as-maintained, inferred or synthetic);
- source time, valid/effective time, event/transaction time, ingestion/retrieval time and decision cutoff as separate temporal roles;
- a declared field-level projection/mapping and its loss/unsupported status.

The federated approach should win where it preserves source autonomy, lowers duplicate mutable state, and handles ownership/revocation better than a centralized mirror. It should not be assumed to win every latency or availability case; the benchmark compares composition types on matched requirements.

## Contract v0.1

Each federate binds:
- owner/system identity and declared system-of-record role;
- stable asset/component subject and configuration revision;
- state class and schema/adapter revision;
- source record identity and source-owned revision;
- source time, effective/valid interval, transaction time, retrieval/ingestion time;
- field-level ownership and read/write mode;
- source authority, approval state/scope, expiry and revocation reference where applicable;
- exact artifact digest if bytes are available, without confusing digest with issuer authenticity;
- projection field mappings: source field, target field, mapping state (exact/normalized/lossy/unsupported), transform revision and parent digest;
- commissioning requirements for frequency, latency, fidelity and recovery appropriate to the declared use case.

No human label, tag, name or location alone is a claim-bearing identity key. No fuzzy matching may silently join two different serials or configuration revisions. Missing, conflicting or revoked evidence remains explicit; it is not defaulted to current or approved.

## Commissioning dimensions

1. Identity and asset hierarchy: asset, component, serial, lot, parent assembly and installed-at links must be repeated on each source federate and each lineage edge.
2. Configuration/effectivity: design revision, part revision, drawing/specification revision and effective period.
3. State semantics: as-designed vs as-manufactured vs as-inspected vs as-built vs as-operated vs inferred vs synthetic.
4. Temporal semantics: source/observation, valid/effective, event/transaction, ingestion/retrieval and decision-cutoff times.
5. Authority/provenance: issuer, role, scope, revocation and custody remain owner-native.
6. Mapping fidelity: all required properties round-trip or loss is explicitly declared; unsupported properties cannot silently disappear.
7. Update behaviour: duplicates, reordering, missed cadence, clock skew, partition, recovery and revocation replay.
8. Write boundary: projections and synthetic/counterfactual state are read-only and cannot mutate system-of-record state.
9. Commissioning readiness: measured update cadence, p50/p95/p99 latency, field-level fidelity and recovery against the use case's declared requirements—not a universal "real-time" badge.


### Temporal budget semantics v0.1

Keep source age, source cadence and ingestion latency separate:

- Require source time <= transaction time <= retrieval time <= evaluation cutoff, allowing only the declared clock-skew tolerance for a transaction clock that is slightly ahead of retrieval time.
- Source age = evaluation cutoff - source time; compare it with max_source_age_s.
- Ingestion latency = retrieval time - transaction time; compare it with max_update_latency_s.
- update_interval_s is the expected source cadence. It does not extend either hard budget.
- Report cadence, source age and ingestion latency independently; do not infer a fresh source from a recent retrieval.

The synthetic positive fixtures must satisfy both hard budgets. Negative cases should isolate stale source data from delayed ingestion so one defect cannot accidentally stand in for the other.

## Frozen corpus v0.1

The corpus contains 32 synthetic positive/negative cases:
- positive federated read-only composition;
- wrong asset, component, serial or parent-assembly identity across source federates and lineage edges;
- wrong design/configuration revision;
- duplicate serial and lot confusion;
- effectivity mismatch;
- source authority/write-owner violation;
- stale source despite recent ingestion;
- retrieval-time/source-time conflation;
- out-of-order and duplicate event application;
- missing/revoked/scoped approval;
- source outage and recovery;
- lossy mapping hidden as exact;
- unsupported field silently dropped;
- PLM/MES/QMS/EAM disagreement;
- repair/replacement without lineage;
- as-built vs as-operated promotion;
- simulation/counterfactual writeback;
- digest match incorrectly treated as publisher authenticity;
- snapshot freshness without fidelity;
- frequency pass but latency/fidelity fail.

Each federate and relationship carries `asset_id`, `subject_id` (component), `serial_id`, `parent_assembly_id` and `configuration_revision`; a top-level identity value is not sufficient unless every participating source record and lineage edge agrees. The corpus is a frozen specification/fixture seed. It is not yet an executed evaluator. The follow-on Rust verifier must independently recompute dispositions; it must not trust a planner-provided status.

## Benchmarks and industry baselines

Compare matched synthetic cases against:
1. document-pack/email/manual register handover;
2. IFC / ISO 19650 exchange-only process;
3. PLCS / ISO 10303-239-style product lifecycle configuration;
4. ISO 23247-5 digital-thread connections;
5. ISO 23247-6 integrated/unified/federated twin composition;
6. proposed Luminous federated owner-native composition with independent verifier.

Report:
- identity and configuration/effectivity accuracy;
- revision/supersession/revocation detection;
- requirement-to-evidence completeness;
- round-trip field preservation and explicitly reported loss;
- p50/p95/p99 update latency and staleness duration;
- outage recovery and post-recovery convergence;
- duplicate/reordered event idempotency;
- change-impact closure time and downstream requalification count;
- reviewer time to reconstruct what was installed, when and under which accepted baseline;
- federation integration cost and ongoing source-maintenance burden where measured.

Initial gates, not achieved results:
- zero false acceptance of wrong asset/part/configuration on frozen negatives;
- 100% detection of revision/effectivity mismatch, revoked approval and wrong-scope authority;
- zero silent loss of required fields;
- zero projection or counterfactual writeback into source-owned physical state;
- deterministic replay for the frozen corpus;
- all latency/fidelity results stratified by declared use case and reported as distributions.

No superiority claim against commercial PLM/MES/EAM is justified by schema coverage alone. Demonstrate measurable advantages in cross-vendor identity continuity, revision/effectivity correctness, field-loss transparency, revocation propagation, commissioning effort and independent historical reconstruction under matched integration budgets.

## Claim ceiling

A passing synthetic evaluator would establish only the declared identity/configuration/temporal/federation/commissioning contract for these frozen cases. It would not establish physical truth, product quality, structural acceptance, NDT validity, certification, seaworthiness, maintenance authorization or permission to operate.
