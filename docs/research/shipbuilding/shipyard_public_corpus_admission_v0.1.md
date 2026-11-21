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


## 12. Current research landscape and matched comparison tracks (8 November 2026)

The source-faithful dataset is an entry point, not a complete representation of shipyard planning. Recent literature covers materially different decision problems. These works are comparators for the *relevant track*, not interchangeable leaderboard rows.

### Relevant research comparators

- **Irregular block placement under disruption.** Wang, Yin and Khan, *Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding*, IET Collaborative Intelligent Manufacturing (first published 11 July 2026; DOI 10.1049/cim2.70066). The paper describes a mixed-integer model for planar storage-yard utilization and vehicle scheduling, irregular polygon geometry, workstation/vehicle disruptions, and a two-stage ABC + ALNS recovery method. The study reports shipyard-case experiments and sensitivity analysis. This is a strong Track C comparator, not a source-faithful Track A baseline.
- **Transporter dispatch.** *Crystal graph convolutional neural network-based reinforcement learning for adaptive shipyard block transportation scheduling*, Engineering Applications of Artificial Intelligence 177 (1 August 2026), DOI 10.1016/j.engappai.2026.114959. Its stated objectives include block transport tardiness and empty travel. Compare it only on a shared transporter assignment/dispatch definition, not the source dataset's berth-only abstraction.
- **Painting bottleneck.** *Scheduling the block painting process in shipbuilding with group completion constraints*, International Journal of Naval Architecture and Ocean Engineering 18 (2026), DOI 10.1016/j.ijnaoe.2026.100777. It considers parallel-machine painting, detour-path and group completion-time constraints, using simulated annealing guided by truncated branch-and-bound and comparing with a MILP model. It is a painting-process comparator, not a generic block-berth benchmark.
- **Assembly-line scheduling.** *Scheduling optimization of hull block assembly line using constraint programming and discrete-event simulation*, IJNAOE 17 (2025), DOI 10.1016/j.ijnaoe.2025.100675, supports a CP + DES comparison for a flow-shop/assembly-line track.
- **Adaptive stockyard scheduling.** Zhong et al., *Integrated dynamic scheduling of shipyard blocks considering adaptive adjustment in stockyard layout*, Engineering Optimization 58 (2026 issue 2), DOI 10.1080/0305215X.2025.2466829, reports 50 sets of actual block data, four road types and four stockyard types, and a paper-reported 56% reduction in interference-block moves. The article's data availability statement says raw/processed data cannot currently be shared. Treat the reported 56% as a result in that study's own problem and protocol—not as a direct target or comparable score for this public corpus.

The 2026 review by Bordbar et al. (JMSE 14(4), 396; DOI 10.3390/jmse14040396) reinforces the methodological point: hybrid simulation + optimization is credible in bounded industrial settings, while longitudinal yard-wide execution validation and public datasets remain limited. Therefore, our differentiation should be evidence-linked performance under real constraints—not an “AI” label or a single makespan number.

### Three matched benchmark tracks

**Track A — source-faithful reproduction**
- Inputs: only the exact public input workbooks and source-defined initial state.
- Constraints: preserve minimum-bounding-rectangle representation, unary berth, sufficient labour, one-position-per-block and empty initial positions.
- Comparators: published EDDQN/DDQN and five published heuristics, plus transparent deterministic rules and CP/MILP exact or bounded oracle for tiny instances.
- Outcomes: source objective(s), makespan, cost and position switches only after the formulas, units and denominators are reconstructed from source files.
- Purpose: establish whether we can reproduce the published result, not whether the model solves a real yard.

**Track B — dynamic capacity and workforce**
- Add one constraint family per version: trade/crew calendars, absenteeism, crane/rigging availability, transporter capacity, aisle/corridor reservations, material readiness, inspection holds, rework and shift boundaries.
- Inject timestamped disturbances: berth closure, crane fault, delayed material, rejected inspection, unplanned absence and urgent priority work.
- Compare frozen-plan execution, rolling-horizon CP/MILP, dispatch heuristics, ALNS, and learned policies under the same observation horizon and compute budget.
- Score feasible completion, weighted tardiness, recovery time, schedule nervousness, disruption propagation, resource utilization, unserved work and objective regret against clairvoyant/exact small-instance references. A policy must not see future disturbances before they occur.

**Track C — geometric and logistics realism**
- Replace rectangles only in this explicitly new variant: polygonal footprints, permitted orientations, swept transport paths, crane reach/lift envelopes, access corridors, turning radii and blocking/interference dependencies.
- Use exact feasibility oracles on small cases and independently validated geometric predicates. Heuristic/learned results must be replayed through the same independent checker.
- Report usable area, interference moves, empty travel, total travel, lift/transfer operations, blocked access, safety-envelope violations and recovery from spatial disruptions.
- Compare against the ABC/ALNS and graph-based/RL research above only where the geometry, transporter and disturbance definitions can be matched. Do not average incomparable papers into one leaderboard.

### Performance and evidence gates

Proposed targets (not measured results):
1. **Hard feasibility:** zero accepted overlapping/over-capacity/incompatible assignments and zero unpermitted access/lift-envelope violations.
2. **Tiny-instance optimality:** exact agreement with the independent oracle on all supported tiny instances; report proven optimality gaps and timeouts separately.
3. **Fair improvement:** paired per-instance improvement over the strongest feasible classical baseline under the same wall-clock, hardware, memory and information budget. Report median, p95, worst case, confidence intervals and all failed runs—not only the best seed.
4. **Disturbance robustness:** lower time-to-feasible-recovery and disruption regret without increasing hard violations or schedule churn; measure performance by disturbance type and severity.
5. **Generalization:** hold out complete instance families, yard layouts, disturbance sequences and scale ranges—not random rows from the same scenario.
6. **Operational fidelity:** replay against event traces with event-time/as-of semantics; planned, simulated, forecast and measured execution remain separate.
7. **Reproducibility:** immutable input manifests, hashes, parser/method/toolchain versions, deterministic seeds and independent result verification.

No arbitrary target such as “10% faster” is meaningful until Track A has been faithfully reproduced and a strong matched baseline exists. A credible SOTA claim requires superiority on the same inputs, constraints, objective, compute budget and information available at decision time. A solver that is faster but infeasible, sees future data, or omits transport/inspection work is not better.

