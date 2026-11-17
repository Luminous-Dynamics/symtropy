# WaterLoop design assurance and parametric sizing v0.1

**Date:** 17 November 2026  
**Parent:** [WATERLOOP-DESIGN-ASSURANCE-001 #5749](https://github.com/Luminous-Dynamics/symtropy/issues/5749)  
**Architecture parent:** [WATERLOOP-MODULAR-TRAIN-001 #5713](https://github.com/Luminous-Dynamics/symtropy/issues/5713)  
**Implementation:** `tools/water-service-t0`, WaterLoop physical T0 cases 035–042  
**Evidence class:** conceptual design + synthetic sizing arithmetic  
**Claim ceiling:** design-document consistency and preliminary calculations only; no component selection, treatment performance, safety certification or build authorization

## 1. Engineering objective

The modular pilot skid is now treated as a machine-checkable design contract rather than a folder of individually plausible documents:

`module interfaces → P&ID tags → BOM categories → FMEA hazards → challenge scenarios → sizing basis → exact revision receipt`

The design-audit tests consume the current source artefacts. They report the exact file and JSON path for missing or inconsistent references. They do not rewrite any source document.

This work extends the seven-file pilot-skid architecture package in [PR #5715](https://github.com/Luminous-Dynamics/symtropy/pull/5715). It does not create a second module registry, instrument registry, water ledger, BOM owner, asset registry or quality authority.

## 2. Design boundaries carried forward

The first demonstrator remains:
- an unoccupied, contained test-cell skid;
- source-separated bathroom light-greywater only, with any basin input separately admitted;
- kitchen/dishwasher, toilet/urinal/bidet, faeces/urine-contaminated laundry, paint/solvent/vehicle/workshop and unknown mixed drains excluded;
- separate raw equalisation/quarantine and treated-product hold;
- interchangeable treatment cassettes, one candidate train per comparison run;
- a closed toilet-flush emulator as the first intended-use test boundary;
- no potable cross-connection and no automatic product release;
- all reject, purge, overflow, backwash, solids and spent media accounted as separate residual paths.

The WRC TT 746/17 guidance identifies lower-risk shower/bath and selected basin streams as preferable, lists source exclusions, and states untreated greywater should not be stored longer than 24 hours (otherwise it should be treated). This is a conservative source/storage screening reference, not a validated residence-time design for every treatment process. The complete report and municipal/site review remain necessary before any use decision. [WRC TT 746/17](https://www.wrc.org.za/mdocs-posts/tt-746-17/) [WRC safety guidance summary](https://www.wrc.org.za/wp-content/uploads/mdocs/WW%20Sept-Oct%202018_Water%20reuse.pdf)

The 2025 national standards require a Water Services Institution planning to collect and use greywater beyond a household boundary to identify and manage risks in its wastewater risk-abatement plan, using WRC guidance. That institutional scope is not automatically the same as an individual household demonstrator; the boundary must be reviewed against the actual project/operator. [GN 6292, 6 June 2025, Regulation 12](https://www.dws.gov.za/Projects/wueew/documents/webinfo/20250606_GazettedNor.pdf)

For Johannesburg, the 2023 Water Services By-laws state that water from a source other than the Council supply must meet the prescribed standards for its intended purpose. The current consolidated by-law, any amendments, site-specific approvals and qualified professional interpretation must be checked before installation. [City of Johannesburg Water Services By-laws 2023](https://joburg.org.za/documents_/Documents/By-Laws/2023/City%20of%20Johannesburg%20Water%20Services%20Bylaws_%202023.pdf)

SANS 10252-1 Annex D is a relevant local design reference for cross-connection control, backflow prevention and permanent non-potable labelling. The exact current edition and applicable clauses must be obtained and reviewed by the responsible plumbing professional; this document is not a compliance interpretation or plumbing design. [SANS 10252-1:2018 extract](https://www.mosselbay.gov.za/storage/documents/cb942e034d7e086691fa18d41a15b553.pdf)

NSF/ANSI 350 is a useful international independent-testing/certification comparator for onsite residential/commercial reuse systems. It is not substituted for South African requirements and no NSF certification is claimed. [NSF/ANSI 350 overview](https://www.nsf.org/knowledge-library/nsf-ansi-standard-350-certification-water-reuse-treatment-systems)

## 3. Cross-file design audit

The Rust unit-test audit checks five current JSON artefacts:
1. module interface control;
2. pilot-skid P&ID tag schedule;
3. preliminary BOM;
4. starter FMEA;
5. synthetic greywater challenge profiles.

It requires:
- unique module IDs and nonempty required ports, requirements and failure states;
- all module references in P&ID, BOM and FMEA to resolve to the module interface source;
- every P&ID tag to have a function, boundary, interlocks and corresponding BOM category;
- every BOM item to retain null quantity and only `not_specified` / `candidate_only` procurement state before design freeze;
- every module to be represented in BOM and hazard coverage;
- every FMEA hazard to identify failure, effect, detection, safe state and verification;
- source scenarios to refer to an existing source profile;
- kitchen, unknown and over-age source controls to remain rejected/indeterminate/held;
- no module, sensor or synthetic profile to create a water-quality release decision.

The audit is deliberately fail-closed. It reports artefact/path violations; it does not attempt to guess a missing interface or auto-correct the design.

## 4. Parametric sizing arithmetic

The calculator accepts explicitly admitted design inputs. The corpus values are synthetic, hand-calculable examples only. It does not choose a safety factor, tank, pump, membrane, UV reactor, ozone generator, treatment train or pipe size.

### 4.1 Required process rate

For total raw feed (V_{in}) over a periodic design period and a declared number of process-available hours (T_a):

`Q_required = V_in / T_a`

This is a required average throughput during available hours. It must be compared to a separately supplied maximum validated process rate. If required rate exceeds that maximum, the schedule is infeasible; a larger buffer cannot repair an undersized treatment process.

### 4.2 Idealised raw equalisation volume

For each declared time interval, compute net inventory change:

`ΔV_i = V_in,i − Q_process × Δt_i`

Let (c_j) be the cumulative sum of `ΔV_i` at interval boundaries. For a periodic schedule with zero net inventory change:

`V_equalisation,ideal = max(c_j) − min(c_j)`

`V_initial,ideal = max(0, −min(c_j))`

This assumes discrete inflow is available at the start of each interval and the process draws at its declared rate in available intervals. It is an ideal minimum working inventory at the chosen time resolution, not gross tank capacity. It excludes:
- freeboard and overflow design;
- dead volume, heel and tank geometry;
- variable-density/temperature effects;
- process yield, reject, backwash, purge and sludge;
- fouling/cleaning downtime beyond the declared availability schedule;
- peak safety factors and uncertainty allowances;
- non-periodic multi-day storms, source outages or occupancy change.

Those must be added by the design owner using measured feed profiles, process yield and professional review.

### 4.3 Treated-service storage

For one explicitly named use profile:

`V_base = essential_daily_demand × autonomy_hours / 24`

`V_usable = V_base × (1 + reserve_fraction)`

`V_gross = V_usable / usable_fraction`

The usable fraction is an input from the vessel/design owner. This calculation does not infer fixture demand, grant water-quality approval, combine potable and non-potable demand, or say that the system can operate through the specified outage. It only calculates inventory implied by the declared demand/autonomy assumptions.

### 4.4 Pump hydraulic duty

For steady incompressible water flow:

`P_hydraulic = ρ × g × Q × H / η`

`e_hydraulic = ρ × g × H / (η × 3.6×10^6) kWh/m³`

where (Q) is in m³/s, (H) is total dynamic head in metres, (eta) is the declared wire-to-water efficiency, (ρ) is density in kg/m³ and (g) is gravitational acceleration in m/s².

The total dynamic head must be provided by the hydraulic design owner and account for static lift and relevant friction/minor losses. The efficiency is an explicit assumption, not a selected pump curve. The result excludes standby/controls, UV, ozone, aeration, heat, recirculation and other electrical loads. It is not a pump selection or electrical design.

## 5. Synthetic known-answer examples — cases 035–042

| Case | Input/condition | Expected result |
|---|---|---|
| 035 | 180 L arrives across 24 h; process available 12 h; max rate 20 L/h | 15 L/h required; ideal buffer 30 L; ideal start inventory 30 L |
| 036 | Same feed, maximum rate 14 L/h | Infeasible; required rate exceeds maximum; no equalisation size credited |
| 037 | 80 L/day essential use, 72 h autonomy, 20% reserve, 90% usable fraction | 288 L usable; 320 L gross |
| 038 | 10 L/min, 12 m TDH, 55% wire-to-water efficiency | 35.66 W hydraulic input; 0.05943 kWh/m³ idealised hydraulic energy |
| 039 | Pump efficiency 120% | Reject nonphysical input |
| 040 | Zero process-available hours | Reject; no finite required throughput |
| 041 | Design inputs not admitted | Indeterminate; no sizing outputs |
| 042 | Synthetic sizing requested as process qualification | Reject claim promotion |

These values are useful to check arithmetic and interface behaviour. They are not recommended design sizes for a household, facility or actual site.

## 6. Design-for-manufacture / assembly / serviceability (DFM/DFA)

The preliminary BOM remains a category list. The next physical review should examine:

### Frame, handling and containment
- assembled, empty, wet and service-lifted mass for every module;
- centre of gravity, lifting points, transport restraint and floor loading;
- welded/bolted frame access, corrosion protection and replaceable mounting rails;
- drip tray/bund capacity and a contained drain route;
- drainable low points and cleanouts so no stagnant pockets are hidden.

### Replaceable process cassettes
- isolate, drain, disconnect, remove and replace each cassette without disturbing unrelated modules;
- keyed couplings and physical identification prevent reversed or incorrect connection;
- tool and lifting requirements for wet media, membrane housings, GAC and UV hardware;
- access to membrane integrity tests, UV sleeve cleaning, GAC sampling/change, biological media and solids removal;
- sample points remain accessible without opening an unsafe process boundary.

### Materials and fabrication
- exact grade compatibility with feed, cleaning agents, temperature, UV exposure, ozone exposure, pressure and expected life;
- seal/elastomer compatibility and chemical ageing;
- weld procedure/inspection and dimensional inspection;
- pressure/leak test specifications from the responsible qualified designer;
- serial/lot tracking for vessel, pump, valve, seals, membrane, UV source and instrumentation;
- no generic material name is accepted as proof of compatibility.

### Electrical and controls
- physical separation of wet process and electrical enclosure;
- enclosure/IP rating selected from actual environment and washdown review;
- isolation, overcurrent protection, earthing, lockout and emergency stop;
- local safe state remains effective with network/cloud/Symthaea/Mycelix unavailable;
- no software signal can bypass independent water-quality release.

### Service and lifecycle
- cleaning, maintenance, consumable replacement, repair, downtime and decommissioning steps;
- locally available alternate suppliers where safety and compatibility permit;
- standard tools, replaceable wear parts and accessible service information;
- labour time and lifting burden measured before a “one-person maintenance” claim;
- residual/spent media disposal route and material end-of-life recorded.

No dimensions, wall thicknesses, pressure classes, electrical ratings, UV dose, ozone dose, membrane flux, pump SKU, materials grade or procurement quantity is fixed by this conceptual DFM checklist.

## 7. Release gates

1. Design document audit passes.
2. Independent structural/plumbing/electrical/process review closes the relevant hazards.
3. Dry assembly and fit-up prove access, lifting, module replacement and wet/dry separation.
4. Clean-water hydraulic test verifies leaks, pressure, flow, level and water balance with admitted instruments.
5. Contaminated-feed test requires approved containment, sampling, health and residual routes.
6. Treatment cassettes run same-batch comparative trials with identical feed, demand, end-use profile and measurement boundary.
7. At least 90-day continuous operation and seasonal/temperature replication are stretch validation objectives, not v0 requirements or current claims.
8. Any occupied/public installation requires the applicable local design, approvals and professional sign-off.

## 8. Nonclaims

The 42-case evaluator and design audit do not qualify physical vessel size, pump choice, treatment performance, disinfection, microbial/chemical safety, reclaimed-water quality, potable water, code compliance, installed cost, service reliability, process availability or manufacturing readiness. `qualification_status` remains `NotRun`; evidence remains synthetic/conceptual. No purchasing, fabrication, plumbing installation or commissioning is authorized by these files.
