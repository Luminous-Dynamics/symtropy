# Historical Maritime Validation Corpus v0.1

**Status:** source-discovery and corpus-design increment. The pinned interaction CSV was fetched transiently for exact Git-blob verification and profiling but is not persisted in this repository; no scored historical corpus or predictive model has been validated.  
**Purpose:** move beyond invented examples toward an auditable historical benchmark without inventing observations, flattening temporal semantics, leaking future information or ignoring source-use constraints.

## 1. Research result: three sources with different roles

### Primary candidate: CyPort

The [CyPort repository](https://github.com/ChenchenMobility/Maritime-Data-CyPort), pinned for metadata inspection at commit `5cb1d8778d3968c5e50d0b8cb8213f2414d692d3`, describes baseline daily vessel counts at U.S. principal ports (2015–2023), 745,704 vessel origin-destination trajectories, 1,927 port–cyclone interaction records, meteorological/tide variables, port responses, and resilience/network metrics. Its README declares a [Creative Commons Attribution 4.0 International licence](https://creativecommons.org/licenses/by/4.0/). The associated 2026 paper describes a dataset spanning 145 U.S. principal ports and 90 tropical cyclones from 2015–2023: [Kuai et al., *U.S. port disruptions under tropical cyclones: Resilience analysis by harnessing multiple-source dataset*](https://doi.org/10.1016/j.trd.2026.105420).

The pinned interaction resource is `Tropical_cyclone_interaction/interaction_data.csv`, with declared byte length `643047` and Git blob SHA-1 `21beb5b7673918143a0ab108b03acc3cccaeb330`. The exact UTF-8 bytes retrieved from the pinned raw URL were independently hashed using Git's blob preimage format; the resulting Git blob SHA-1 matched the repository object. The full source CSV has not been copied into Symtropy. A specific source profile documents observed counts, temporal coverage and sentinel imbalance: [CyPort interaction source profile](./CYPORT_INTERACTION_SOURCE_PROFILE_V0_1.json).

Observed in that pinned interaction file: 56 columns and 1,927 rows, 90 distinct storm IDs and 122 distinct port IDs; the `total_impact` field is zero in 1,654 rows (85.83%) and nonzero in 273 (14.17%). Recovery fields use a zero/NA sentinel in 1,650 rows. That sentinel count is not proof that recovery did not occur and does not automatically imply right-censoring. These distinctions need to be resolved against the source data dictionary before defining labels or scoring.

This is the strongest candidate for the first historical vertical slice because the repository combines baseline time series and event-linked exposure/response data. It provides the best opportunity in the sources found so far to test a real temporal trajectory and examine transfer across ports. **It is not yet admitted as a validation corpus.** The schema and initial source profile have now been inspected; raw-versus-derived field lineage, temporal semantics, sentinel definitions, event-window rules and component-specific use terms still need review. A top-level CC BY declaration must not be assumed to erase separate restrictions or attribution obligations for every upstream source or included component.

The unit of evaluation must be the **cyclone episode**, not each of the 1,927 interactions. Group all affected ports, nearby/unaffected ports, observations, reports and derived labels belonging to one storm episode before splitting. Because related storms can share port histories and seasonal conditions, add temporal and port-transfer evaluation when sample support permits. Published resilience/network columns should be treated as derived features unless their construction and independence from the target have been established; recompute target metrics from the underlying observations wherever possible.

### Secondary candidate: Port Disruption Database

The [Port_Disruption_database repository](https://github.com/jasperverschuur/Port_Disruption_database), pinned at `29cfb82b68bce25bc93df44a240913a6c3420ef1`, describes a workbook with event/port/date descriptors, reported reduction/shutdown/recovery durations, coarse severity categories and vessel-activity time series for affected **and unaffected** ports.

The inspected GitHub root listing does not display a `LICENSE` file, and the README does not establish the licence. Therefore the manifest marks this source `unverified`, blocks redistribution/use of the workbook bytes, and permits metadata inspection only until rights and attribution are confirmed. **Public visibility is not a data licence.** No workbook bytes, digest, byte length or capture timestamp are included.

This candidate may broaden event families and provide controls, but it remains secondary until its workbook semantics, source lineage, resolution and permissions are reviewed. Vessel activity must remain a specifically named proxy; it is not automatically cargo tonnage, terminal service, waiting time or supply delivered.

### Supplementary baseline candidate: U.S. BTS Port Throughput Metrics

The [Data.gov entry for BTS Port Throughput Metrics](https://catalog.data.gov/dataset/port-throughput-metrics) identifies public dataset view `8sfc-juwb`, modified on 23 February 2026, with a USA public-domain label. Its landing page is [data.transportation.gov/d/8sfc-juwb](https://data.transportation.gov/d/8sfc-juwb). This is a candidate for baseline context, not incident truth by default. Inspect the actual resource's temporal granularity, port coverage, revision behavior, null profile and fields before use. The mutable view identifier alone does not freeze content.

## 2. The paradigm shift: evaluate against history without leaking the future

A port disruption is an episode, not a random set of independent rows. Time points from the same event, affected ports, control ports, revised reports and labels derived from eventual recovery all carry shared information.

Use the **independent disruption episode** as the split unit:
- Freeze temporal train/calibration/test windows before fitting or tuning.
- Place every port/time-point/derived label tied to one cyclone in the same split.
- Never allow a report published after the decision cutoff into features that supposedly existed at the time.
- Use leave-one-port-out as a separate, demanding transfer test, only when the independent port count and data support it.
- Prevent shared observations of the same port and overlapping event windows from leaking across splits.
- Record source revisions and retrospective corrections explicitly instead of silently backfilling them.

A candidate unaffected port is not automatically a valid counterfactual. Explain why it is comparable, whether the hazard or network spillover reached it, which common shocks affect both, and why it was retained or excluded. Treat a non-affected port as a potential control with explicit assumptions—not as proof of what the affected port would have done without the disruption.

For CyPort, start with cyclone-grouped evaluation and report both (a) held-out storm performance and (b) spatial transfer by port where feasible. Avoid fitting any model on derived published resilience metrics then evaluating against those same metrics as if they were independent observations.

## 3. Episode schema and operational definitions

Every episode must carry a stable ID, immutable source references, affected entities, control entities or a reason controls are unavailable, hazard class, event start/end or an explicit unknown reason, decision-time cutoff, impact definition, recovery definition, observation references, label quality and split assignment.

For every observation distinguish:
- **event time:** when the real-world phenomenon occurred;
- **publication/issue time:** when the source stated the fact;
- **retrieval time:** when the collector acquired the bytes;
- **availability-to-decision time:** the earliest defensible point the simulated decision process could have known the fact.

These are not interchangeable. A retrospective report can label an outcome for evaluation but cannot become a feature of the earlier decision.

Recovery periods may be right-censored when the observation window ends before recovery is seen. Store this as `right_censored` with the end of the observation window, not a recovery time of zero or proof the port never recovered. Missing time points or absent controls remain missing; they do not become zero service.

Retain raw observations, derived labels and any published resilience metrics as separate layers. Every derived value should identify its inputs, formula/version, time window and whether it shares inputs with the target being evaluated.

## 4. Baselines and validation protocol

Start with at least two transparent baselines:
1. **Persistence/no new information:** carry forward the last justified observation only for a declared horizon and only when the field semantics permit it.
2. **Static capacity-loss/recovery baseline:** use a simple predeclared model built only from inputs available at the decision cutoff.

Then compare the candidate method on exactly the same held-out episodes, windows and metric definitions. Report per-service trajectory error, integrated unmet-service error, onset/duration error, observed time-to-recovery error, right-censored count, and strata by hazard, port, data-quality state and forecast lead where relevant.

Do not pool each trajectory sample into an apparently huge independent sample count. The independent storm/episode and port counts are the important constraints. Proposed corpus targets in the manifest are at least 10 independent episodes across three disruption classes for a pilot, and at least 30 independent episodes before broader empirical claims. CyPort's paper reports 90 tropical cyclones, but those should not automatically be assumed independent or sufficient for every statistical test. These are practical research gates, not universal statistical-sufficiency guarantees. Report uncertainty and leave-one-storm/port failures visible.

Before scoring:
- Pin and hash exact source resource bytes and metadata responses.
- Review licence, attribution, and component-level upstream rights.
- Inspect the data dictionary, event windows, field units, missingness and known revisions.
- Freeze operational target definitions, baselines, split assignment and scoring rules.
- Lock the holdout before fitting/tuning.
- Have a domain reviewer assess whether a vessel-activity proxy supports the service claim being made.

## 5. What this work establishes—and does not

The registry records three source candidates with separate roles:
1. CyPort: primary candidate after component-level rights and schema review.
2. Port Disruption Database: secondary candidate, rights unresolved and data use blocked.
3. BTS Port Throughput Metrics: supplementary baseline context, not event truth.

The executable manifest checker validates source metadata, immutable-revision requirements where available, declared licence state, capture status, split/censoring semantics and staged research targets. It does not download or parse the dataset resources.

This PR contains no captured historical observations, no source workbook bytes, no resource digests, no computed empirical model errors and no validated physical model. The registry's successful validation means only that the proposed intake contract is coherent.

## 6. Next execution sequence

1. Validate and freeze the three-source registry.
2. Capture CyPort's actual resources from the pinned repository revision, recording each file's exact bytes, path, SHA-256, byte length, retrieval time and licence/attribution metadata.
3. Inspect raw-versus-derived fields and establish cyclone episode IDs before computing any metric.
4. Create a canonical Rust importer with source-to-file/row/column lineage, time semantics, missingness and right-censoring.
5. Freeze cyclone-grouped train/calibration/test partitions; perform port-transfer evaluation only when supportable.
6. Establish persistence and transparent recovery baselines; publish adverse and counterexample results.
7. Inspect the legacy disruption workbook's rights and semantics separately. Do not import or redistribute its bytes while rights remain unresolved.
8. Add model-based policy comparisons only after the historical baseline and holdout protocol are reproducible.

**Acceptance ceiling:** source registry and validation design only. No historical data capture, rights clearance of third-party components, predictive superiority, counterfactual proof, physical safety or operational authority is claimed.
