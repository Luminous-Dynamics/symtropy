# Evidence-Gated Maritime Resilience v0.1

**Status:** architecture and metadata-contract prototype. This is not a maritime standard, simulation engine, certified digital twin, or operational authorization system.

## Paradigm shift: from dashboard scores to claim-specific evidence graphs

A single resilience score can hide incompatible facts: a quarterly network index, a live port-call event, a bathymetric grid and an untested assumption are not interchangeable observations. Combining them into one weighted number can make a poorly supported conclusion look precise.

The proposed unit of work is a **scenario evidence graph**. It makes the derivation of a scenario claim inspectable from the raw inputs through pinned transformations and explicit assumptions. It does not try to compress provenance, freshness, uncertainty, coverage, identity confidence and authority into one trust score.

The graph has four node types:

- **Source evidence** binds an exact receipt and artifact digest, source class, declared evidence state and any time-coverage bounds carried by the receipt.
- **Transformation** records its parent nodes, pinned algorithm revision and configuration digest. It cannot declare itself to be an observation.
- **Assumption** states a model assumption and a declared sensitivity-review status. Assumptions are not disguised as data.
- **Claim target** specifies the kind of conclusion being investigated and links to every dependency. It is a target, not an executed output.

The DAG is deliberately conservative: duplicate identifiers, unknown parents, self-dependencies, cycles and unused nodes are rejected. Every node must contribute to the claim target. Source receipt and artifact bytes are checked against the exact declared SHA-256 values, and the source paths in each receipt must agree with the manifest. The verifier resolves referenced paths inside the workspace and rejects symlinks that resolve outside it.

## Do not collapse evidence into a scalar

The verifier carries several distinct questions rather than averaging them together:

| Dimension | Question | Example of an invalid substitution |
|---|---|---|
| Byte integrity | Are these exactly the bytes referenced by the receipt? | Treating a valid hash as proof that the publisher is authentic |
| Source provenance | What publisher, dataset, revision and retrieval path were declared? | Treating an official-looking URI as signed publisher identity |
| Semantic validity | Do the rows/fields mean what the model thinks they mean? | Treating parseable JSON as correct port-level data |
| Time applicability | What period did the source cover, and is it relevant to this scenario's decision time? | Treating capture time as observation time or freshness |
| Spatial identity | What location, port, facility or terminal does the source entity actually identify? | Fuzzy matching a city-level record to a terminal |
| Quality and uncertainty | Is the value observed, estimated, synthetic, unknown, disputed or stale? | Zero-filling missing data or promoting a model output to an observation |
| Model validity | Were transformations pinned, assumptions exposed and sensitivities examined? | Treating reproducibility as proof that the model is physically correct |
| Authority | Is this kind of conclusion permissible for this purpose? | Treating a scenario result as navigation or operational authorization |

These dimensions are not fungible. For high-consequence uses, a missing critical proof obligation should block the claim rather than be offset by strong scores elsewhere.

## Temporal semantics: three times, not one

Source capture time, source data time and scenario decision time must remain distinct. The generic receipt binds retrieval metadata and source-declared coverage separately. The scenario manifest records the decision time and analysis window. It does **not** calculate freshness or infer source coverage that the receipt did not declare.

The manifest reports unbounded source time coverage explicitly. A future applicability gate should be purpose-specific: an annual throughput statistic can be suitable for a long-horizon scenario yet unsuitable for determining current berth availability. No universal freshness interval should be applied indiscriminately across evidence classes.

## Claim-specific gates, not a universal “trust score”

The first implementation validates the manifest contract and verifies source receipt/artifact bytes. Its output always states `NotAdmitted` for operations and navigation and explicitly reports that source authenticity, row semantics, freshness, transformation correctness and scenario results remain unproven.

A later decision pipeline should implement separate proof obligations:

1. **Contract gate:** schema, identifiers, source revisions, dependency structure and mandatory policy.
2. **Byte gate:** receipt and artifact lengths/digests, without mutation or normalization.
3. **Provenance/terms gate:** independently check publisher identity and exact reuse rights; the current byte verifier does not perform these tasks.
4. **Semantic gate:** validate actual source rows, units, datums, identity granularity, coverage flags and permitted missingness behavior.
5. **Applicability gate:** determine whether evidence is relevant to the target decision's time, geography and operating context.
6. **Model gate:** pin code/configuration, preserve assumptions, propagate uncertainty and run sensitivity/negative-control scenarios.
7. **Human/authority gate:** independent review and any required domain-specific certification before real-world action.

Passing a lower gate never implies passing a higher one. Some claims will remain impossible because the required data do not exist, cannot lawfully be used, or lack the necessary granularity.

## Standards interoperability rather than a competing vocabulary

The manifest is a provenance and dependency layer, not a replacement for maritime standards:

- The [IMO Compendium and Maritime Single Window guidance](https://www.imo.org/en/mediacentre/pressbriefings/pages/maritime-single-window-advancing-digitalization-of-shipping.aspx) provide harmonized semantics for information exchange between ship and shore.
- The [DCSA Port Call Standard](https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope) addresses operational information exchange among carriers, terminals, ports and service providers.
- IHO S-100 product profiles remain a separate product-specific lane for hydrographic and other marine information.
- UNCTAD LSCI/PLSCI remain macro-connectivity indicators, not live operational feeds or proof of an individual vessel call.

Adapters should map these standards into typed source nodes while preserving the source-native identifiers and revisions. The adapter should never silently upgrade the strength of a claim.

## Scenario reproducibility contract

Before a scenario runner is built on top of this manifest, an executable run receipt should bind:

- the manifest digest and each exact input receipt/artifact digest;
- the exact model commit, build identity, configuration and relevant dependency lockfile;
- each transformation's inputs and output digest;
- assumptions, parameter ranges and sensitivity-run identities;
- deterministic execution settings and random seeds, where applicable;
- per-output uncertainty and the exact claim ceiling.

A rerun should explain any difference in outputs as either an input change, model/configuration change, or nondeterminism—not silently overwrite prior results. The initial manifest validator does not execute transformations or create this run receipt.

## Measurements that encourage honest progress

Measure evidence coverage and decision quality separately: proportion of source rows covered by the selected window; share of source time coverage that is unknown; unresolved port/entity mappings; stale or disputed evidence by class; sensitivity of results to assumptions; reproducibility of outputs; performance against explicit synthetic fault scenarios; and calibration against independent observed outcomes when available.

Avoid a single composite “resilience” score until each constituent metric, uncertainty and decision consequence is interpretable. If a composite is eventually useful for prioritization, publish its decomposition and show which missing data or assumption would change the ranking.

## Next engineering milestones

1. Capture and preserve one permitted real official dataset and its exact terms/source metadata; no such PLSCI or production UN/LOCODE archive is bundled today.
2. Add a Rust row-level validator for the actual captured PLSCI export format; do not invent column semantics before obtaining the export.
3. Add explicit adapter contracts for DCSA/IMO port-call events and retain source-native event IDs and timestamps.
4. Implement a claim-specific applicability gate using declared data coverage, capture time, target window, spatial granularity and evidence state—without granting authority.
5. Add fault injection for port closure, connectivity loss, unknown/stale evidence and unresolved identity; compare multiple scenarios rather than producing a single unsupported forecast.
6. Create an independent replay check and compare predictions with subsequently observed outcomes before claiming predictive capability.

## Claim ceiling

The command `--validate-scenario-manifest` verifies declared graph structure and exact receipt/artifact byte bindings only. It does not authenticate the source publisher, interpret licence terms, validate actual PLSCI rows, establish data freshness or truth, execute transformations, independently test assumptions, run a simulation, validate a digital twin, certify a vessel/port, or authorize navigation or operations. A structurally valid graph is a reproducible *description of intended reasoning*, not proof that the reasoning is true.
