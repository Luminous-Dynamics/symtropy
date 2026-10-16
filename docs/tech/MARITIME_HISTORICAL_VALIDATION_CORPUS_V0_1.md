# Historical Maritime Validation Corpus v0.1

**Status:** source-discovery and corpus-design increment. No source bytes have been captured by this PR; no scored historical corpus or predictive model has been validated.  
**Purpose:** create a defensible bridge from synthetic contract tests to historical evidence without inventing observations, flattening time semantics or ignoring source-use constraints.

## 1. Research result: use two sources for different purposes

### Primary candidate: Port Disruption Database

The public [Port_Disruption_database repository](https://github.com/jasperverschuur/Port_Disruption_database) documents a workbook with event/port/date descriptors, reported reduction/shutdown/recovery days, coarse severity categories, and event-folder vessel-activity time series for affected **and unaffected** ports. The repository was pinned to immutable source revision `29cfb82b68bce25bc93df44a240913a6c3420ef1` for metadata inspection.

This is the strongest lead found so far for a historical incident corpus because it describes recovery durations and time series, not merely aggregate annual throughput. The repository README does not establish the licence in the inspected metadata, and common root `LICENSE` paths were not found. Therefore the manifest marks it `unverified`, bars redistribution, and permits metadata inspection only until rights and attribution are confirmed. **Public visibility is not a data licence.** The workbook has not been imported into this repository; no byte length, digest or capture timestamp is fabricated.

Vessel activity must remain a specifically named proxy. It is not automatically equivalent to cargo tonnage, terminal service, vessel waiting time or supply delivered. Inspect the workbook's sheet/column definitions, time resolution, missingness, event-window conventions, port matching and source references before calculating any model error.

### Supplementary candidate: U.S. BTS Port Throughput Metrics

The [Data.gov entry for BTS Port Throughput Metrics](https://catalog.data.gov/dataset/port-throughput-metrics) identifies a public dataset view `8sfc-juwb`, modified on 23 February 2026, with a USA public-domain label. Its landing page is [data.transportation.gov/d/8sfc-juwb](https://data.transportation.gov/d/8sfc-juwb). This is a candidate for baseline throughput context, not incident truth by default. The actual resource's temporal granularity, ports, revision behavior, null profile and fields must be inspected from captured bytes. The mutable view ID alone is not a content revision.

The machine-readable source registry pins the research repository revision where possible, distinguishes a source reference from captured bytes, records the unresolved licence question and blocks the leap from source discovery to validation evidence. Each later capture will need its own retrieval timestamp, exact response/resource length, SHA-256, content type, revision and use/attribution snapshot.

## 2. The paradigm shift: evaluate against history without leaking the future

A port disruption is an episode, not a random set of independent rows. Time points from the same event, affected ports, control ports, revised reports and labels derived from the eventual recovery all carry shared information.

Use the **episode** as the split unit:
- freeze temporal train/calibration/test windows before tuning;
- place every time point and affected/control port from one disruption into one split;
- never allow a report published after the decision cutoff into features that supposedly existed at the cutoff;
- treat leave-one-port-out as a separate, demanding transfer test, only when the available ports support it;
- record source revisions and retrospective corrections explicitly instead of silently backfilling them.

A candidate unaffected port is not automatically a valid counterfactual. Explain why it is comparable, whether the same hazard or network spillover reached it, which common shocks affect both, and why it was retained or excluded.

## 3. Episode schema and operational definitions

Every episode must carry a stable ID, immutable source references, affected entities, control entities or a reason controls are unavailable, hazard class, event start/end or an explicit unknown reason, decision-time cutoff, service/impact definition, recovery definition, observation references, label quality and split assignment.

For every observation distinguish:
- **event time**: when the real-world phenomenon occurred;
- **publication/issue time**: when the source stated the fact;
- **retrieval time**: when this collector acquired the bytes;
- **availability-to-decision time**: the earliest defensible point the simulated decision process could have known the fact.

These are not interchangeable. A historical report published after an event can label an outcome for evaluation but cannot become a feature of the prior operational decision.

Recovery periods may be right-censored when a source window ends before service recovery is observed. Store that as `right_censored` with the observation-window end. It is not a recovery time of zero or proof that the port never recovered. Similarly, missing time points or an absent control series remain missing; they do not become zero service.

## 4. Baselines and validation protocol

Start with at least two transparent baselines:
1. **Persistence/no new information:** the last justifiable observation carries forward only for a declared interval and only when the data's meaning permits it.
2. **Static capacity-loss/recovery baseline:** a simple predeclared model using only features available at the decision cutoff.

Then compare the candidate method on exactly the same held-out episodes, observation windows, metric definitions and scope. Report per-service trajectory error, integrated unmet-service error, disruption onset error, duration error, observed time-to-recovery error, right-censored episode count, and stratification by hazard, port and data-quality state.

Do not pool every observation into an apparently huge sample size. The independent episode count and port count are the important constraints. Proposed corpus targets in the manifest are 10 independent episodes across at least three disruption classes for a pilot, then at least 30 independent episodes before broader empirical claims. These are practical research gates, not universal statistical sufficiency guarantees. Report uncertainty around metrics and resist claims that exceed data coverage.

No historical scoring should start until:
- source terms, attribution and actual byte capture are reviewed;
- the dataset dictionary and operational definitions are pinned;
- episode/port/time leakage rules are implemented and independently checked;
- baseline definitions and scoring rules are frozen;
- the temporal holdout is protected from tuning;
- a domain reviewer has assessed whether the available proxy represents the claim being tested.

## 5. What this work does and does not establish

The new manifest is machine-readable and is intended for strict Rust contract validation. It registers two candidates, required capture metadata, minimum pilot targets, a split policy, label/time/censoring semantics and prohibited inferences.

It does **not** contain any historical observations, downloaded workbook rows, authentic source artifact digest, measured model errors, fitted parameters, or a physical/operational assessment. The manifest validation command will validate this source registry contract, not assert that historical validation has occurred.

## 6. Next execution sequence

1. Validate the source registry and freeze its exact bytes.
2. Capture only the BTS resource first if the actual metadata/resource and use terms remain consistent with its public-domain label; pin the full response, query, byte length and SHA-256.
3. Inspect the Port Disruption Database workbook's content and licence. If rights remain unresolved, do not commit or redistribute its bytes; seek explicit permission or use a rights-cleared source.
4. Build a Rust importer that emits canonical episode records with source-to-row/column provenance and explicit missing/censored states.
5. Run leakage checks on grouped episode IDs and preserve a holdout manifest before any baseline fitting.
6. Establish the baseline metrics and publish failure cases before integrating learned policy or dependency propagation.
7. Only make empirical claims that the specific event/port/metric coverage supports.

**Acceptance ceiling:** source registry and evaluation protocol only. No claim of captured bytes, licence clearance for the research workbook, historical validation, predictive superiority or maritime safety is made.
