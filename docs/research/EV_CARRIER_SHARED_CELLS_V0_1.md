# EV-CARRIER-SHARED-CELLS-001 — mixed-model factory cell capacity screen v0.1

**Implementation:** tools/ev-passenger-cost-frontier/src/carrier_module_factory.rs  
**CLI:** ev_carrier_shared_cell_screen  
**Input mappings:** fixtures/ev_carrier_shared_cell_mappings_v0_1.json  
**Base factory:** EV-FACTORY-DESIGN-001 report  
**Module process routes:** EV-CARRIER-MODULE-MFG-001 fixture  
**Claim ceiling:** synthetic shared-station arithmetic only

## Paradigm shift: one factory, multiple product families

The dedicated-cell module screen (#6193) is an intentionally conservative baseline. It assumes each carrier pod, trailer or AMR process gets its own process cells. That gives a useful upper bound on incremental footprint and equipment, but does not answer whether low-volume module work can reuse the EV factory's receiving, electrical integration, calibration or EOL capacity.

This screen adds a **shared-cell alternative**. It does not create a second factory model: it consumes the same base factory report and module process catalogue, then evaluates a strict mapping between one named module process and one named base factory station.

A matching process label is not a compatibility proof. Reuse is allowed only according to an explicit compatibility state and evidence class.

## Formula

For base station j, with installed parallel cells C_j, productive minutes per cell-year P_j, base vehicle attempts A_v, and base process cycle t_vj:

M_available = C_j × P_j

M_base = A_v × t_vj

M_residual = M_available - M_base

For module process i, with required attempts A_m, module cycle t_mi, changeovers N_i, and setup time s_i:

M_module = A_m × t_mi + N_i × s_i

The amount that must be handled by supplemental cells is:

M_supplemental = max(0, M_module - max(0, M_residual))

C_supplemental = ceil(M_supplemental / P_j)

The mapping only passes its synthetic capacity screen if:

M_residual + C_supplemental,max × P_j >= M_module

The report separately gives the residual base minutes, module process minutes, changeover minutes, supplemental cells required, declared available supplemental cells, total capacity margin, and the number of base attempts that would have to be displaced if the shortfall were forced into the base line.

This is a station-capacity feasibility bound. It is not a detailed sequenced production schedule. For non-aggregated mixed-model sequences, changeover count and setup duration must be derived from an explicit lot/sequence schedule. The current fixture uses declared synthetic annual changeover counts.

## Compatibility and evidence semantics

- **synthetic_compatible:** arithmetic test only; requires a synthetic evidence class.
- **verified_compatible:** requires measured or independent-test evidence and a production-release-ready base factory before the result can be evidence-backed.
- **unknown:** the mapping is not permitted to credit base residual capacity; result remains indeterminate.
- **incompatible:** the proposed reuse is blocked, even if the arithmetic would fit.

The fixture includes three architectures across three factory archetypes:
- detachable UAV pod: synthetic harness/charger integration mapping to the base battery-drive station;
- AMR handoff: synthetic EOL mapping to the template-specific base EOL station;
- integrated drone bay: unknown HV/thermal compatibility and unknown capacity data.

The synthetic mappings do not prove that any EOL test, HV station, fire/thermal process, fixture, worker qualification or acceptance method is genuinely interchangeable.

## Outputs

For each site/template/demand/configuration/mapping, the CLI emits:
- base factory attempts, station cells, station available minutes and base station load;
- residual base station minutes;
- module attempts, module processing minutes and setup/changeover minutes;
- module workload that fits in residual minutes and excess work;
- required and maximum supplemental cells;
- actual combined capacity margin;
- dedicated-cell baseline, supplemental cells used and dedicated cells avoided;
- supplemental equipment capex, area and operator positions;
- base vehicle attempts displaced if an unresolved capacity shortfall were forced;
- compatibility status, evidence class/reference, missing fields and findings;
- inherited base factory release state/evidence gaps and an explicit false module release flag.

The module process catalogue remains the source for dedicated-cell cycle/yield/footprint/equipment/labor assumptions. The shared-cell screen only evaluates the mapped process slice; it does not waive the other module station processes.

## Run

First generate the base factory report and module process fixture (the exact-head workflow already runs the base factory screen). Then run:

    cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin ev_carrier_shared_cell_screen -- ev-factory-design-report.json tools/ev-passenger-cost-frontier/fixtures/ev_carrier_module_processes_v0_1.json tools/ev-passenger-cost-frontier/fixtures/ev_carrier_shared_cell_mappings_v0_1.json

The committed fixture has 3 site profiles × 3 factory templates × 4 volume cases and 9 template-specific mappings: 108 reported cases total. Independent fixture arithmetic predicts 72 synthetic-compatible arithmetic passes and 36 indeterminate integrated-bay cases. This is not a Rust test result or a measured plant result.

## Benchmark targets

- 100% of shared capacity maps name an exact module config, module process, factory template and base station.
- Unknown or incompatible mappings never credit base residual minutes.
- No shared station overbooks base workload plus module workload plus changeovers.
- Zero base good-vehicle output displacement for any passing shared-cell result.
- A higher module volume cannot reduce required supplemental cells unless the declared shift, availability, performance or station-capacity assumptions change.
- Zero setup/changeover work omitted from an admitted mapping.
- Input order permutations produce byte-equivalent deterministic JSON.
- Compare dedicated versus shared versus hybrid cell counts, capex, footprint, staffing and utilization under identical base factory inputs.
- Do not claim cost savings until the shared station compatibility, cycle time, setup/changeover, labor and equipment values are measured or independently tested.

## Industry context and differentiator

The IEA's Global EV Outlook 2026 reports that Daimler and Volvo can assemble battery-electric trucks on the same production lines as diesel models. Mixed-model EV production is therefore an existing industrial capability, not a novel claim by itself. The opportunity here is to quantify whether optional carrier modules can share qualified EV factory stations without displacing good base-vehicle output or weakening module-specific inspection and EOL requirements. https://www.iea.org/reports/global-ev-outlook-2026/manufacturing-and-trade

The same IEA report notes global electric truck production of roughly 440,000 units in 2025, with over 90% concentrated in China. Its battery chapter notes that battery production outside China remains more expensive and that average yield above 90% is important for competitiveness. These data support a manufacturing strategy focused on flexible lines, supplier resilience and saleable good output—not simply nameplate capacity. https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-batteries

## Non-claims

The fixture does not prove actual process equivalence, station availability, production scheduling, local supplier access, cycle times, setup times, labor availability, cost superiority, product safety, road homologation, airworthiness, production release or commercial feasibility. The base factory fixture is synthetic and not a South African factory quotation or engineering survey.
