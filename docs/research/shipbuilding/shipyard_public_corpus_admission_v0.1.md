# Shipyard public ship-block scheduling corpus admission — V0.1

**Status:** source-admission specification based on public landing-page metadata; raw workbook bytes have not yet been downloaded or parsed in this work item. No reproduction or benchmark win is claimed.  
**Parent:** [SHIPYARD-PUBLIC-CORPUS-001 #4204](https://github.com/Luminous-Dynamics/symtropy/issues/4204)  
**Composes:** [SHIPBUILD-CO-DESIGN-001 #2048](https://github.com/Luminous-Dynamics/symtropy/issues/2048), [SHIPYARD-SPACE-001 #2050](https://github.com/Luminous-Dynamics/symtropy/issues/2050), [shipyard recovery lab #2734](https://github.com/Luminous-Dynamics/symtropy/issues/2734)

## 1. Public sources and exact version identity

### English-language record

- Title: *Dataset of Ship Block Scheduling (English)*
- Provider: Mendeley Data
- Version: 1
- DOI: 10.17632/2bmbmbf8hm.1
- Landing page: https://data.mendeley.com/datasets/2bmbmbf8hm/1
- Published: 3 March 2026
- Metadata licence: CC BY 4.0
- Contributor shown on page: Shaozi Chen
- Institution shown: Shanghai Jiao Tong University
- Provider description: inputs and results adapted from real-world shipyard production data, with minor adjustments for anonymisation and experimental controllability.

### Related language/version record

- Title: *Dataset of Ship Block Scheduling*
- Provider: Mendeley Data
- Version: 1
- DOI: 10.17632/4ds6t24mx2.1
- Landing page: https://data.mendeley.com/datasets/4ds6t24mx2/1
- Published: 3 March 2026
- Metadata licence: CC BY 4.0

The two records are related by title, contributor, date and described data structure, but **must not be assumed byte-identical or statistically independent**. The downloaded workbooks, file manifests, mapping files and rows must be compared before deciding whether one is a translation of the other or a separate corpus.

## 2. What the public metadata establishes

The English-language landing page describes:
- raw block information;
- initial block-position/berth information;
- block-position/berth attributes and constraints;
- scheduling result workbooks for DDQN, EDDQN and heuristic policies;
- raw figure-source data and mapping-key text.

The described input attributes include block identity, vessel/type, dimensions and weight, process requirements and time, priority, berth/position identity, area/capacity, lifting capacity, height constraints and permitted block types. These are metadata summaries, **not a verified column schema**. Actual sheet names, columns, keys, units, missingness and formulae must be read from the downloaded bytes.

Published baseline files are described as:
- DDQN scheduling results;
- EDDQN scheduling results;
- earliest-start-time heuristic results;
- longest-processing-time heuristic results;
- resource-utilization-based heuristic results;
- response-time-based heuristic results;
- shortest-processing-time heuristic results;
- figure-source result data.

No result values are copied into this specification because the raw workbooks have not been admitted.

## 3. Published simplifying assumptions

The English record states the following model assumptions. They define the source reproduction boundary:

1. Block and position shapes use minimum bounding rectangles.
2. A position can process only one block at a time.
3. Labour is sufficient; absenteeism and labour anomalies are not modelled.
4. Every block size can find a suitable position.
5. Each block completes at one position.
6. All positions are empty at the start.

These assumptions are not to be silently relaxed or replaced during faithful reproduction. They also define the extension opportunities: workforce limits, realistic irregular geometry, position-transfer operations, unavailable capacity and non-empty initial states need separate variant identifiers and results.

## 4. Three distinct evidence tracks

### Track A — faithful source reproduction

Implement only the published abstraction after the actual source workbook is parsed. Reconstruct the exact source objective functions, capacity rules, time conventions, initial state and termination criteria. Match the source algorithm names and run protocols where sufficiently disclosed. Do not add geometry, crane, transfer, rework or labour features not present in the source model.

### Track B — independent result verification

Independently reconstruct schedule feasibility and objective values from the admitted inputs and exported schedule traces. The evaluator must not invoke the production scheduler's feasibility or objective helpers. If the published results are inconsistent with a literal reconstruction, report the discrepancy; do not repair the source numbers silently.

### Track C — physicality extensions

Add one constraint family at a time. Examples:
- workforce, trade skill, shift calendar and absenteeism;
- transport/flatcar/vehicle time, capacity and corridor use;
- cranes, lift capacity/reach and shared rigging resources;
- actual polygon footprints, swept access paths and blocked corridors;
- material readiness, procurement delay and rejected material;
- inspection hold points, NDT, nonconformance and rework;
- work-cell outage, disruption and recovery;
- revision/effectivity change invalidating dependent work.

Each extension receives a new variant ID, source/assumption statement, exact tiny oracle where tractable and separate score table. Track C is not source reproduction.

## 5. File admission and provenance manifest

For every downloaded file, record:
- DOI and dataset version;
- provider landing-page URL and retrieval timestamp;
- displayed licence and attribution string;
- exact filename and language;
- byte length and SHA-256;
- workbook sheet names, dimensions, hidden sheets, merged regions and formulas where relevant;
- parser/runtime version and extraction method;
- row counts per sheet and counts of blank/invalid/duplicate records;
- original labels and translated labels kept side by side;
- source-to-normalized field mapping with unit provenance;
- exclusions or corrections, each with reason and before/after record digest.

Original files are immutable inputs. Any normalization or correction produces a separate derived artifact and a new digest. Do not commit large third-party workbooks by default; use a documented download/reference plus content digest or an approved artifact-storage mechanism.

## 6. Initial schema sketch — pending workbook inspection

These are semantic buckets to investigate, not assumed source column names:

| Semantic bucket | Questions to answer from source |
|---|---|
| Block identity | Stable ID? Vessel and block type? Duplicate rows? |
| Geometry | Dimensions, shape assumptions, units and orientation? |
| Mass/handling | Weight, lift capacity, height limits and units? |
| Process | Processing time, precedence, earliest/latest start, deadlines? |
| Priority | Source priority meaning and tie-breaking? |
| Position/berth | Capacity, compatible types, initial state and availability? |
| Resource | Unary or cumulative capacity? Labour implicit or explicit? |
| Result trace | Start/end, assignment, transfer, sequence, objective components? |
| Algorithm protocol | Training seeds, episodes, stopping, compute budget, hardware? |
| Objective | Exact formula, units, weights and any normalization? |

Unknown units or semantics remain unknown. Do not infer from Chinese/English labels alone.

## 7. Baseline comparison protocol

The minimum named set is:
- the source-published DDQN and EDDQN result traces;
- earliest-start-time;
- longest-processing-time;
- resource-utilization-based;
- response-time-based;
- shortest-processing-time;
- one simple deterministic transparent baseline;
- a bounded exact MILP/CP oracle on tiny source-faithful cases.

Report by instance:
- feasibility and each violated constraint;
- source-defined makespan/completion metric;
- reconstructed position/berth utilization and its denominator;
- transfers/position changes if present in the source;
- energy/cost only when source fields and formula support it;
- runtime, memory, hardware and compute budget;
- timeouts and incomplete runs as outcomes, never silently dropped;
- objective gap to proven-optimal exact solutions only where exact optimality is established.

A baseline with a different objective, data variant, constraint set or compute budget is not an apples-to-apples winner.

## 8. Negative controls

- Missing file, sheet, row or required field is surfaced.
- Unknown unit or objective formula blocks numerical comparison.
- Duplicate IDs, orphan assignments or impossible initial state are rejected.
- Capacity and overlap errors are independently detected.
- Source result inconsistencies remain visible.
- English and Chinese files are not mixed or double-counted without reconciliation.
- Row order, locale and translated labels do not change record identity.
- A solver timeout is not reported as infeasibility or optimality.
- Online-policy extensions cannot receive future disturbance events before their timestamp.

## 9. Initial gates — proposed, not achieved

- 100% admitted files have a source/version/size/hash record.
- 100% rows are accounted for as valid, invalid, duplicate, unknown or excluded with reason.
- 100% numeric comparisons have verified units and exact objective definition.
- 100% imported schedules are independently checked against the source model.
- Exact agreement with a hand-checkable independent oracle on every supported tiny instance.
- Every reproduction mismatch is documented.
- Faithful reproduction and physicality extensions remain separately reportable.
- No schedule with a hard physical/source constraint violation is reported feasible.

## 10. Implementation principles

Prefer Rust-first, deterministic, dependency-audited parsing/normalization and evaluation. Before choosing an XLSX library, inspect the repository's existing parser dependencies and current file tooling; add the smallest supported dependency only after compatibility and maintenance review. Do not add a production Python runtime solely because source data happen to arrive as workbooks.

Keep:
- original file bytes immutable and hashed;
- parser version and mapping manifest pinned;
- outputs deterministic for a frozen input/toolchain where promised;
- no spreadsheet formulas evaluated as trusted source facts unless their semantics and cached values are independently checked;
- all data rights and attribution visible in any redistributed derivative.

## 11. Claim ceiling

A faithful parser or result reproduction validates only the admitted source abstraction and declared protocol. The dataset is anonymized/adjusted research data, not a complete live-yard execution record. Track C extensions are synthetic unless separately calibrated against admitted real observations. None of these results alone proves yard-wide productivity, safe crane lifts, welding acceptance, physical construction feasibility, vessel compliance or industrial superiority.
