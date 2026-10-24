# GLASS-RELATIONSHIP-001 — relationship-aware evidence and split benchmark v0.1

**Status:** proposed evaluation contract; no relationship-graph implementation or empirical result is claimed yet.  
**Parent:** [Glass–Silicon process benchmark #2715](https://github.com/Luminous-Dynamics/symtropy/issues/2715) and draft [PR #2716](https://github.com/Luminous-Dynamics/symtropy/pull/2716).  
**Work item:** [#2769](https://github.com/Luminous-Dynamics/symtropy/issues/2769).  
**Scope:** offline evidence/benchmark tooling only. No furnace control, actuator authority, dataset ingestion, or rights certification.
**Machine-readable contract:** [glass-relationship-graph-v0.1.schema.json](./glass-relationship-graph-v0.1.schema.json). The Rust auditor adds cross-record and cross-split invariants that JSON Schema alone cannot establish.

**Implementation boundary:** the current Rust command validates supplied node/edge declarations, checks split reachability, and emits raw/canonical graph digests. It does not yet discover links automatically, verify source truth or rights, or validate the full accepted-output dependency chain. Those are explicit follow-on milestones below; protocol requirements must not be mistaken for implemented behavior.

## 1. Why a relationship layer is necessary

A manifest can be valid while the evidence it describes is wrong, incomplete, dependent, or leaked across evaluation partitions. Today’s grouped-split audit checks declared composition-family and synthesis-campaign keys. Those are useful controls, but relationships may be transitive or hidden behind aliases and derived artifacts:

`measurement → cleaned table → derived feature → model row → prediction → decision → accepted-output claim`

A near-duplicate experiment can therefore cross a split without sharing the immediate group fields in both rows. Conversely, a broad relation such as "mentioned in the same paper" is not automatically a leakage edge for every task. Treating every relation identically can either miss leakage or collapse most of the corpus into one giant connected component.

The paradigm shift is to represent relationships explicitly, preserve uncertainty about links, and evaluate downstream dependency behavior. This adapts a general lesson from the 2026 T² theorem-testing benchmark: isolated compilation is weaker evidence than checking dependent successor theorems. T² reports 38.9% testing accuracy on its own theorem task; that figure is **not** a glass/silicon baseline. [ACL Anthology paper](https://aclanthology.org/2026.acl-industry.150/).

Leakage-resistant materials evaluation provides a complementary reason to preserve grouping by the highest relevant independence unit. A 2026 review of literature-compiled biochar datasets highlights study, experiment, and feedstock-level leakage risks; that is a methodological warning, not direct evidence that the current glass corpus has such leakage. [DOI 10.1111/gcbb.70173](https://doi.org/10.1111/gcbb.70173).

The 2026 GFRI-ML paper includes non-glass-forming examples across 44,753 oxide-glass samples from 2,771 ternary systems, reporting task-specific results with held-out systems. The benchmark should reproduce its published protocol only after rights, exact artifact, split, and preprocessing review; no result is imported or claimed here. [DOI 10.1111/jace.70980](https://doi.org/10.1111/jace.70980). See the separate [data-admission issue #2762](https://github.com/Luminous-Dynamics/symtropy/issues/2762).

## 2. Core design: one evidence graph, multiple relation profiles

Do not infer one universal equivalence relation. Keep a typed graph and select an explicit edge profile for each evaluation question.

### 2.1 Node kinds

The initial contract should allow, at minimum:

- `source_artifact`, `publication`, `dataset_record`
- `material`, `lot`, `composition`, `recipe`, `process_campaign`, `run`, `sample`
- `instrument`, `calibration`, `measurement`, `derived_feature`, `adjudicated_label`
- `model_version`, `prediction`, `candidate_decision`, `intervention`, `accepted_output`

Every node has a stable identifier, node kind, source references, evidence class, admission/rights state, and any relevant valid-time or measurement-time interval. Missing metadata remains missing; it must not be synthesized as a default value.

### 2.2 Relationship records

A relationship record includes:

- stable source and target node IDs;
- a versioned edge type;
- state: `verified`, `candidate`, `contradicted`, or `unknown`;
- evidence references and the method that proposed or adjudicated the link;
- optional confidence or score with its semantics declared (not a universal probability);
- valid-time interval and creation/version metadata where applicable.

Useful initial edge types include `derived_from`, `replicate_of`, `same_material_family`, `same_lot`, `measured_in_campaign`, `measured_with_calibration`, `feature_computed_from`, `label_adjudicated_from`, `model_trained_on`, `prediction_for`, `decision_used`, and `output_accepted_under`.

**Unknown is not false.** The absence of an edge proves only that the supplied graph does not contain that edge. A candidate relationship must not silently become a verified identity match; a contradicted link remains visible in the receipt.

### 2.3 Relation profiles are task-specific

Each benchmark protocol declares a versioned profile describing which confirmed relationship types create a leakage boundary for the task. For example:

- **Record/derivation leakage:** exact duplicate, alias of the same observation, replicate, or derived-from relationships.
- **Campaign transfer:** all runs/samples from the same campaign held out together.
- **Material-family transfer:** all samples from the declared composition-family or parent family held out together.
- **Source-study transfer:** records from the same source study are held out together when estimating transfer to unseen studies/laboratories.
- **Calibration transfer:** related measurements sharing a calibration event or instrument regime are grouped when the question requires generalization beyond it.

Publication co-occurrence, shared equipment, chemical similarity, and process similarity must not automatically be treated as strict identity edges. They can define alternative sensitivity profiles or candidate links. Report which profile was used; never describe a profile-specific split as universally leakage-free.

## 3. Relationship discovery without silent entity merging

Relationship discovery is a proposal process followed by review/adjudication, not a license to merge records from similarity alone.

Candidate-link generators may use normalized composition plus declared tolerances, source identifiers/DOIs, record hashes, instrument/campaign metadata, recipe and process lineage, or near-duplicate checks. For every proposed relation, preserve the raw identifiers, normalization/version, rule or model version, and supporting evidence. Chemical similarity is not proof of same composition, same material lot, or same experiment.

The initial implementation should be deterministic and offline-capable:

1. Validate node/edge schema, referenced IDs, cardinality, status enums, and relation-profile definitions.
2. Canonicalize records using explicitly versioned rules and deterministic ordering; retain the raw-input digest separately from the normalized graph digest.
3. In the current v0.1 executable slice, do not auto-propose links: audit the supplied edge set only. A future link-proposal generator should start with conservative exact-identifier/hash links and mark heuristic similarity as `candidate`.
4. Permit adjudicated synthetic fixtures now; defer empirical linking until relevant data rights and source artifacts are admitted.
5. Emit a receipt that distinguishes declared, proposed, and independently adjudicated relationships.

A graph audit can validate consistency of supplied declarations. It cannot prove the underlying sample identity, measurement truth, author permission, dataset rights, or scientific validity.

## 4. Graph-safe split evaluation

The current declared-key audit remains a required baseline, not a replacement target. Report at least these split protocols side by side when the data and task permit:

1. Row-wise random split (diagnostic only; never the sole generalization result).
2. Current declared-group split (composition family + campaign).
3. Confirmed relationship-component split under an explicit leakage profile.
4. Leave-source-study/campaign/material-family-out protocols matched to the intended deployment claim.
5. Independent external-site, external-campaign, or prospective holdout when available.

For a relationship-component split, compute connected components using only the profile’s confirmed leakage edges, then ensure each component is assigned wholly to one partition. Emit a concrete cross-partition path for every leak finding (for example, a train record → derived table → alias → test record).

Do not blindly include every edge type in one connected-components calculation. That can over-group records and invalidate the intended transfer question. Instead, define separate profiles and report their results. Candidate or unknown links that could connect train and test must produce a separate uncertainty finding; where the intended claim depends on whether such links exist, disposition is `UNRESOLVED`, not a silent pass.

### 4.1 Leakage and graph integrity metrics

Report independently:

- known confirmed leakage edges/components crossing partitions (target: zero);
- number of cross-partition paths detected and one representative witness path per finding;
- candidate/unknown relationship count and fraction that could affect split isolation;
- audited relation precision/recall, but only against an adjudicated reference set;
- component count/size distribution and records lost to over-grouping;
- usable train/validation/test coverage under each relation profile;
- exact graph/profile/manifest digests and deterministic-replay status.

Never optimize the leakage metric by rejecting all rows, nor call missing relationships "zero leakage."

## 5. Downstream dependency testing

The graph should also supply integration tests for the **meaning** of an industrial claim. **This downstream contract is specified here but is not yet implemented by the v0.1 Rust command.** An accepted-output claim should be traceable to the material/lot and composition, process/run, measurements and calibration status, joint product qualification, mass/energy ledger, and applicable acceptance decision. MG-Si and glass use separate product/task profiles and units.

A successful local prediction must not count as a successful candidate if a required downstream dependency fails or remains unresolved. A syntactically valid edge or schema-valid receipt is insufficient when its referenced evidence is missing, stale, out of domain, contradictory, or not admitted.

The first downstream fixtures should demonstrate:

- valid lineage from measured sample to qualified output;
- a prediction with a missing material identity edge;
- a stale/inapplicable calibration edge;
- a feature or label derived from held-out target information;
- a qualified-looking output whose joint property constraint fails;
- a mass/energy ledger that is unbalanced or uses unknown-as-zero;
- an unresolved rights/admission state that blocks empirical claims;
- an unknown link that changes whether the split is isolated.

The resulting status is `PASS`, `FAIL`, or `UNRESOLVED` under explicit rules. `UNRESOLVED` is not a failure score to be optimized away; it means current evidence cannot support the claim.

## 6. Adversarial synthetic fixture plan

Start with a small, hand-auditable fixture corpus with exact expected outputs:

- **valid-connected:** records sharing a confirmed replicate/derived-from path are kept in one partition; accepted.
- **transitive-leak:** declared split columns are disjoint, but train and test records connect through a derived table; leakage finding with witness path.
- **alias-leak:** alternate record IDs point to the same exact raw observation hash; leakage finding.
- **uncertain-link:** a candidate relationship could cross partitions; report uncertainty and follow the profile’s declared fail-closed disposition.
- **contradicted-edge:** edge has incompatible evidence or invalid direction/cardinality; reject or return unresolved as declared.
- **missing-edge:** required lineage for an output or decision is absent; cannot certify downstream dependency.
- **stale-calibration:** measurement references a calibration that is expired or outside its declared applicability interval; cannot accept the measurement as qualified evidence.
- **target-proxy:** feature provenance descends from the target/held-out outcome; reject the feature as leakage.
- **not-reject-all:** a valid fixture must pass.
- **over-grouping-control:** a broad non-leakage relation must not merge components unless the selected profile declares that boundary.

Mutations must be tested against both a known-positive and known-valid corpus. A parser that rejects every graph, or a graph grouper that merges every record, cannot pass.

## 7. Success criteria and limits

The initial software milestone is a small Rust subcommand plus versioned JSON schema/fixtures. It should be deterministic, bounded, offline, fail closed on malformed input, and return machine-readable findings with stable reason codes. Preserve `cargo test --locked`; do not regenerate lockfiles implicitly or weaken CI after a lock failure.

Acceptance requires:

- a valid, invalid, contradictory, and unknown-link fixture;
- detection of a transitive cross-split relationship despite disjoint declared keys;
- a passing valid control (not reject-all);
- witness paths, profile/version digests, raw and canonical graph hashes;
- explicit handling of candidate/unknown links and an adversarial over-grouping control;
- downstream dependency mutations that prevent invalid evidence from becoming a feasible-output claim;
- current verifier tests and exact-head focused checks; queued/cancelled status is not a pass;
- no empirical benchmark result until data rights, exact artifacts, and preprocessing are admitted and independently reproduced.

Do not fold all outputs into a single weighted leaderboard score. Report graph integrity, generalization, prediction calibration, feasible discovery, false-feasible risk, decision value per fully charged budget, causal impact, and evidence status as separate axes.

## 8. Research basis and bounded interpretation

- Kim, Han & Hwang (ACL Industry Track 2026), *Benchmarking Testing in Automated Theorem Proving* (T²), https://aclanthology.org/2026.acl-industry.150/ — supports downstream dependency tests as a semantic/integration signal, not manufacturing result equivalence.
- Ji et al. (2026), *Glass-Forming-Region-Informed Machine Learning Reveals Quantitative Criteria for Network Glass Formation*, https://doi.org/10.1111/jace.70980 — relevant negative-instance and held-out-system baseline; data/model terms and exact reproduction remain separate gates.
- Jang et al. (2026), *Illusions of Accuracy: Widespread Data Leakage Distorts Machine Learning Results in Biochar Research*, https://doi.org/10.1111/gcbb.70173 — cross-domain methodological warning about hierarchical leakage; not direct glass evidence.
- NISTIR 8620 (2026), *Digital Twins Workshops Summary Report*, https://doi.org/10.6028/NIST.IR.8620 — supports explicit verification/validation, uncertainty, interoperability, and trust boundaries.

No SOTA superiority is claimed by this protocol. The first scientifically meaningful result is to show, on a rights-cleared and independently reproducible task, how much naive random/declared-key evaluation differs from relation-aware transfer evaluation and whether the proposed evidence/decision loop remains valid through downstream constraints.
