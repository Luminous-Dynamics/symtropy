# PWR-TX-MFG-001 — Manufacturing digital thread and process-quality benchmark for electrical equipment

**Status:** executable trace-ledger scaffold and research protocol. The checked-in ledgers are synthetic. No factory integration, measured yield improvement, product conformity or process-capability result is claimed.

## The paradigm shift: co-design the product and the factory

Transformer engineering normally treats design optimization, process planning, factory execution, inspection/testing and service as separate activities. That can produce a design that is electrically attractive but difficult to wind, stack, dry, weld, fill, test, repair or source reliably.

The proposed system closes the loop:

1. Product and component design publish a revisioned bill of materials, tolerances, material requirements, process-critical characteristics and product-specific quality gates.
2. Process planning maps each revision to an approved route, machines, tooling, recipes, operator qualifications, control limits and inspection/test plans.
3. Manufacturing execution emits immutable event records for receipt, transformation, inspection, test, rework and release.
4. The quality system links every measurement/test event to its actual evidence artifact and records non-conformance, disposition and rework.
5. The factory digital twin estimates queue, setup, machine, energy, yield and quality consequences, but its recommendations remain inside approved process windows.
6. Field/service observations flow back to product and process engineering with the relevant component/lot/route genealogy.

This is not one giant factory twin. Compose reusable product, component, machine/cell, inspection/test and facility twins. ISO 23247-5:2026 addresses the digital thread across product lifecycle stages; ISO 23247-6:2026 describes integrated, unified and federated digital-twin composition. Official previews: https://www.iso.org/standard/87425.html and https://www.iso.org/standard/87426.html.

NIST's 2026 manufacturing digital-twin workshop report highlights interoperability, verification/validation/uncertainty quantification (VVUQ), cybersecurity and workforce readiness as challenges that must be addressed alongside model capability: https://doi.org/10.6028/NIST.IR.8620.

## Current research anchors

- Yang et al. (2025), *Research on optimisation of transformer manufacturing process and construction of smart factory based on digital twin technology*, describe a transformer factory digital-twin architecture and report a production accuracy error below 0.02 mm in their factory case, alongside improved order-delay performance. The result is a paper-specific claim and should not be treated as a universal industry benchmark: https://doi.org/10.2478/amns-2025-0012.
- IEC 60404-3:2022 defines single-sheet-tester measurement methods for AC magnetic properties of grain-oriented and non-oriented electrical steel strip/sheet at power frequencies. It is a useful traceability anchor for electrical-steel lot characterization, not a complete core manufacturing acceptance plan: https://webstore.iec.ch/en/publication/66641.
- IEC TR 62271-322:2026 describes digital technologies over the lifecycle of high-voltage switchgear and controlgear, including IoT, edge/cloud, digital twins, AI and cybersecurity: https://webstore.iec.ch/en/publication/87013.
- The IEC 60076 family and applicable product-specific standards contain transformer-specific test requirements. The actual current edition, national adoption, purchaser specification and routine/type/special test scope must be mapped by product and jurisdiction; this ledger does not prescribe or replace that test plan.

## Product-family coverage

Use one shared event/genealogy envelope, but separate product-family engineering and test profiles.

| Family | Example process/quality modules | Do not assume |
|---|---|---|
| Liquid-immersed and dry-type power/distribution transformers | Electrical-steel receipt/characterization, core cutting and stacking, winding, insulation, active-part assembly, drying/impregnation, tank/enclosure, oil process where applicable, final electrical/mechanical routine tests | A single route, cooling model or test profile covers dry and liquid equipment |
| Reactors, inductors and magnetic components | Core/material characterization, winding, gap/stack geometry, impregnation, inductance, impedance, loss, thermal tests | Electrical-steel SST rules apply unchanged to ferrite or every magnetic material |
| Switchgear, breakers and controlgear | Busbar processing, contact/material traceability, mechanism assembly, enclosure/insulation, functional and dielectric test evidence, firmware/configuration trace | Transformer tests can qualify switchgear |
| Busbars, conductors and cable assemblies | Conductor/material lots, forming, joining, surface treatment, insulation/sheath, resistance and applicable mechanical/electrical checks | The electrical test plan is identical for busbar, cable and winding products |
| Motors and generators | Lamination/core, winding, impregnation, rotor/stator assembly, balance, insulation and performance testing | Transformer process gates establish rotating-machine qualification |
| Power-conversion assemblies | Semiconductor/module lots, thermal interface, busbar/laminate, cooling system, PCB/control, firmware, dielectric/EMC and functional tests | A component-level pass automatically qualifies the assembled system |

The family names above are architecture scopes. Before production use, each requires a separate route revision, acceptance plan, calibrated measurement system and approved evidence source.

## Executable manufacturing trace ledger

The binary `manufacturing-ledger` validates the initial common digital-thread envelope:

`product_family,product_id,serial_number,component_id,component_type,component_revision,component_lot_id,parent_component_id,event_id,sequence_no,event_type,operation_code,station_id,recipe_sha256,start_utc,end_utc,disposition,energy_kwh,scrap_kg,evidence_artifact_id,evidence_sha256`

Run the synthetic transformer example:

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin manufacturing-ledger -- \
  research/transformer-bench/fixtures/manufacturing-ledger-synthetic.csv \
  /tmp/transformer-manufacturing-summary.csv
```

Run the adjacent product-family example:

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin manufacturing-ledger -- \
  research/transformer-bench/fixtures/manufacturing-ledger-multi-family-synthetic.csv \
  /tmp/electrical-components-manufacturing-summary.csv
```

The validator checks unique event IDs, stable product/serial and component identity, parent references and genealogy cycles, contiguous per-component sequence numbers, strictly ordered non-overlapping per-component event times, allowed event/disposition types, required process/rework recipe hashes, numeric energy/scrap, and syntactically valid SHA-256 evidence references. It summarizes event counts, failures, rework, energy, scrap and the count of pass-marked final-release events.

The ledger is deliberately not a route-completeness or release-authority system. It does not know which tests are mandatory for a particular rating, design, jurisdiction or customer; it does not verify that evidence files exist; it does not judge a test result against approved acceptance limits; and it does not authorize dispatch. Use the separate evidence-receipt tool to verify listed artifact bytes and add a revisioned, independently reviewed route/test-plan manifest before any conformity or release use.

## Manufacturing optimization benchmark

Establish a product-family and process-route baseline before selecting an optimizer. Do not pool materially different product mixes, machine types, material grades, plants, shifts or customer test requirements into a single rate.

### Core outcome metrics

- First-pass yield (FPY) at each operation and final test; rolled throughput yield (RTY) across the route.
- Rework incidence and rework labor-hours per accepted unit.
- Scrap mass, high-value material yield and offcut utilization per accepted unit.
- Energy (kWh) and peak demand per accepted unit; report energy source and allocation method.
- Lead time median/P90, queue time, on-time-in-full delivery, setup/changeover time and work-in-process.
- OEE components (availability, performance and quality) with the exact measurement boundary.
- Defect escape, final-test retest, warranty/field-return rate and failure mode by product/lot/route revision.
- Traceability coverage for critical components/material lots and completeness of required inspection/test artifacts.
- Process capability (Cp/Cpk or appropriate non-normal/stability method) for declared critical-to-quality characteristics, only after measurement-system analysis and statistical process stability are established.
- Lifecycle cost and embodied impacts with declared allocation boundaries.

### Process windows and causal evidence

For each high-impact process, track the input variables that are physically relevant and controllable. Examples include core-cut length/angle/burr, stacking pressure and step-lap registration; winding tension, conductor temperature and turn placement; drying/vacuum/impregnation profile; tank weld procedure and leak-test result; and test-bench calibration/configuration. These are candidate variables to investigate, not universal acceptance limits.

Use designed experiments, blocked/stratified by machine, shift, lot and product revision, to identify robust process windows. Do not learn correlations from an unstratified factory log and treat them as causal. Use measurement-system analysis, control charts, capability analysis, independent validation lots and approved engineering change control before changing a critical recipe.

### Aspirational improvement gates

These are relative improvement targets for a controlled pilot, not universal industry standards:

- 100% trace coverage for declared critical material/component lots and mandatory inspection/test evidence in the pilot scope.
- 10–20% reduction in rework labor-hours per accepted unit against a matched baseline.
- 10% reduction in scrap mass per accepted unit without lower reliability or test performance.
- 5–10% reduction in energy per accepted unit, with comparable product mix and metering boundaries.
- 10–15% reduction in median setup/queue time at the targeted bottleneck, without a worse P90 lead time or quality escape rate.
- No deterioration in critical final-test failures, safety escapes, field failure or warranty outcomes. Any critical escape is a pilot stop and review trigger.
- For stable, adequately sampled critical dimensions where normal capability indices are valid, target Cpk ≥1.33 as an initial process goal; it is not a substitute for product-specific tolerance or qualification.

Report uncertainty, denominators, confidence intervals, excluded work and baseline window for every metric. The factory pilot should use a controlled rollout or other defensible counterfactual. A before/after comparison without product-mix and time effects is not enough to claim causality.

## Paradigm shifts to prioritize

1. **Manufacturability-aware product optimization.** Optimize electromagnetic performance, losses, thermal behavior, mass, embodied carbon and lifecycle cost together with tolerances, material yield, winding/stacking difficulty, process capability, tooling availability and testability. A theoretically superior design that cannot be made repeatably is not a superior product.
2. **Lot-to-field digital thread.** Bind steel/copper/insulation/component lots to process events, recipes, measurement records, routine/type/special tests, product serials and field outcomes. Enable a quality engineer to trace a field issue to all affected serials without assuming supplier identity from an unverified label.
3. **Quality-by-design and active process learning.** Use value-of-information to choose the next coupon, core stack, winding trial or test sample that most reduces uncertainty about a critical quality characteristic. Compare against random and classical DOE baselines; count scrap and destructive test cost.
4. **Federated factory twins.** Keep product, machine, process-cell, metrology, test-bench and utility twins separate with versioned contracts; compose them where necessary. This follows ISO 23247-5/-6 better than assuming one central twin can own every supplier and machine data model.
5. **Energy, material and carbon as production objectives.** Track energy and scrap per accepted unit, not merely per machine-hour. Preserve quality and safety as hard gates rather than trading them away for throughput.
6. **Human-approved adaptive control.** A digital twin may recommend setpoint changes inside a qualified process window. Out-of-window or safety-relevant changes require engineering review, controlled change, requalification and fail-safe rollback.

## Next implementation sequence

1. Freeze the ledger contract and add route/test-plan manifest validation with exact product-family and revision matching.
2. Add measurement records with units, calibration certificate ID, uncertainty, acceptance-limit revision and evidence receipt. Do not store only PASS/FAIL when raw numeric values are available.
3. Add per-operation route completeness checks and non-conformance/rework state transitions.
4. Add process KPI evaluation with stratification, uncertainty and counterfactual comparisons.
5. Create manufacturing digital twins for core cutting/stacking and winding first; these couple most directly to transformer electromagnetic performance and material yield.
6. Extend the common digital-thread to switchgear, busbars and magnetic components with separate product-specific profiles.
7. Only after independent offline validation should any process parameter be proposed for closed-loop control.

## References

- Yang et al. (2025), transformer manufacturing process optimization and smart factory digital twins: https://doi.org/10.2478/amns-2025-0012
- NISTIR 8620 (2026), Digital Twins Workshops Summary Report: https://doi.org/10.6028/NIST.IR.8620
- ISO 23247-5:2026, digital thread for manufacturing digital twins: https://www.iso.org/standard/87425.html
- ISO 23247-6:2026, digital twin composition: https://www.iso.org/standard/87426.html
- IEC 60404-3:2022, electrical-steel magnetic-property measurement with a single-sheet tester: https://webstore.iec.ch/en/publication/66641
- IEC TR 62271-322:2026, digital technologies across the switchgear/controlgear lifecycle: https://webstore.iec.ch/en/publication/87013
