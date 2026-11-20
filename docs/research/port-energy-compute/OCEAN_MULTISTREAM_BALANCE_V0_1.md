# Ocean Multi-stream Hydraulic Balance v0.1

**Status:** synthetic software qualification fixture and control-volume contract only.  
**Owner workstream:** OCEAN-MULTISTREAM-BALANCE-001 (opened as a separate public Symtropy issue; linked from the PR discussion)  
**PR:** [#6322](https://github.com/Luminous-Dynamics/symtropy/pull/6322)  
**Date:** 2026-11-20

## 1. Why the two-meter shortcut is not the system model

A flow comparison between one intake and one outfall is valid only for a narrow, declared single-pass boundary with no side streams and no inventory change. Real seawater cooling and treatment systems can have recirculation, makeup, purge/bleed, bypass, filter backwash, cleaning, leakage, evaporation, condensate, multiple outlets and storage/holdup changes.

Ignoring any of those paths can make a system appear to close when it does not. A missing stream must not silently become zero.

The design shift is from **two meters that agree** to a **typed physical control-volume graph**. Each material path is an explicit edge with an evidence record. Each connected boundary port is used exactly once; a not-applicable port needs a reason. Each control volume has an explicit inventory-change measurement. The complete network is then checked both locally and at its external system boundary.

## 2. Graph and conservation contract

The fixture models two connected control volumes:
- `CV-COOLER-A`: primary seawater heat-exchanger loop;
- `CV-TREATMENT-B`: secondary treatment, purge and return train.

Seven directed streams are declared:
- external seawater intake;
- internal transfer A → B;
- evaporation (modelled from a declared synthetic psychrometric input set);
- leakage to secondary containment;
- treated seawater return;
- purge/bleed;
- filter backwash.

Every stream has a source endpoint, destination endpoint and typed direction. An internal transfer is represented once; it appears as outflow from the source control volume and inflow to the destination. It cancels from the aggregate external boundary and is not double-counted as a new system intake/outfall.

For control volume (i), the signed balance is:

`R_i = Σ(incoming mass flow) - Σ(outgoing mass flow) - dM_i/dt`

where (dM_i/dt) is the measured/modelled inventory accumulation rate. Positive accumulation means mass is being stored in the control volume.

The aggregate external boundary is:

`R_system = Σ(external inflows) - Σ(external outflows) - Σ_i(dM_i/dt)`

Internal transfers cancel by construction. The aggregate is calculated from external edges and inventory change, not by counting internal piping twice.

### 2.1 Port completeness

A control volume lists its ports and their direction:
- `connected`: exactly one stream endpoint must reference it;
- `not_applicable`: no stream may reference it and a non-empty justification is mandatory.

Every stream endpoint must resolve to an existing control volume/port or a named external node. The source side must connect to an `out` port; the destination side must connect to an `in` port. Unknown nodes, duplicate port identities, reused ports, missing streams, unreferenced connected ports and invalid direction are rejected.

This is a machine-checkable completeness contract, not a claim that the fixture itself represents a field P&ID.

### 2.2 Inventory / storage change

Every control volume carries an explicit storage-inventory measurement with:
- signed mass change over the interval;
- independent and common-mode standard uncertainty;
- exact interval and clock;
- meter/calibration identity and validity window;
- QC, rights and evidence state.

A zero inventory change is a measurement in this fixture—not a default. A missing inventory term, stale calibration or unsupported origin blocks closure. Modelled inventory change must be a separately qualified extension; it cannot be passed off as a metered observation.

## 3. Evidence and stream semantics

Each stream is one of two origins:

**Metered:** requires a sensor, calibration identity whose validity covers the full interval, per-observation clock offset, raw evidence reference, QC, rights and exact UTC window.

**Modelled:** requires method identity, model version, all input references and a validation state. It must not carry a fictional sensor or calibration certificate. Modelled evaporation remains visibly modelled and its uncertainty is included.

The fixture is entirely synthetic. Its per-record `synthetic_record_reference` values are **references to test fixture records, not cryptographic hashes**. The CLI separately computes SHA-256 over the exact bundle-file bytes and includes that digest and byte length in its receipt. The file hash establishes byte identity only; it does not authenticate a sensor, a real acquisition chain or a calibration certificate.

All samples use the same declared UTC interval in this initial steady-flow fixture. A transient plant needs stream-specific hydraulic residence-time and inventory alignment; it must not use this steady-window profile without extension.

## 4. Uncertainty and covariance

For each control-volume residual:
- independent standard uncertainties are combined by root-sum-square;
- common-mode uncertainty contributions within a declared correlation group are summed algebraically with the signed incidence coefficient, then squared;
- storage uncertainty is converted to a rate by dividing inventory uncertainty by interval duration;
- common-mode storage uncertainty joins the same group accumulator as correlated flow meters where declared.

This preserves cancellation. The same internal transfer measurement has opposite incidence signs in CV-A and CV-B and cancels from the global boundary. Shared meter-scale error similarly combines by signed contribution, not by pretending every meter is independent.

Unknown correlation is rejected. A declared common-mode uncertainty without a correlation-group ID, an undeclared group, or a group attached to zero common-mode uncertainty is rejected. Real systems may need additional covariance terms for density conversion, common pressure/temperature references, shared sampling/lab batches, calibration standards and correlated process-model errors.

### 4.1 Mass basis and density

The fixture stores flows as **kg/s** so that different temperature/salinity streams can be compared on a mass basis without silently assuming equal density. A future volumetric meter adapter must convert (Q_v) to (dot m = ho Q_v), carrying density uncertainty and covariance. It may not simply add m³/s values when water density or composition differs.

### 4.2 Closure status is not a permit

The proposed (|R|/u(R) ≤ 2) threshold is a synthetic metrology screen in this fixture, not a regulatory or ecological limit. A closed mass balance says that declared streams/inventory agree within a stated uncertainty model. It does not establish:
- permitted discharge;
- water-quality compliance;
- absence of pollutants;
- ecological safety;
- protected-area compatibility; or
- site clearance.

The CLI keeps environmental compliance and ecological impact `NOT_ASSESSED` and site clearance `UNKNOWN_BLOCKED`.

## 5. Synthetic fixture result

The fixture's control volumes are balanced by construction:

| Quantity | CV-A | CV-B | Aggregate external boundary |
|---|---:|---:|---:|
| Incoming mass flow | 10.00 kg/s | 9.80 kg/s | 10.00 kg/s |
| Outgoing mass flow | 10.00 kg/s | 9.80 kg/s | 10.00 kg/s |
| Storage accumulation | 0.00 kg/s | 0.00 kg/s | 0.00 kg/s |
| Residual | 0.00 kg/s | 0.00 kg/s | 0.00 kg/s |

The internal A→B transfer is 9.8 kg/s and is counted once at each adjacent control volume, then cancels at the global boundary. Evaporation is 0.1 kg/s and explicitly modelled; leakage is 0.1 kg/s; return is 9.4 kg/s; purge is 0.25 kg/s; backwash is 0.15 kg/s.

The global uncertainty calculation combines five external metered streams, the modelled evaporation term, two explicit inventory uncertainties and the shared meter-scale group. The internal transfer's uncertainty is not added a second time to the global boundary.

These are synthetic values chosen to exercise the schema and conservation equations. They are not representative design flow rates, metered flows or environmental operating conditions.

## 6. Qualification and regression cases

The Rust verifier contains positive and negative cases for:
- valid two-control-volume and global closure;
- a missing purge stream / connected port;
- an internal port direction mismatch;
- modelled evaporation missing its method/version/input references;
- an unknown common-mode correlation group;
- a connected stream set to zero as a shortcut;
- expired meter calibration;
- a not-applicable port without justification;
- interval duration that disagrees with the UTC window and invalid calendar dates;
- internal transfer cancellation from the global uncertainty ledger;
- shared scale-error cancellation at the aggregate boundary; and
- a common storage/flow covariance group.

The verifier is strict `serde(deny_unknown_fields)`. Tests are authored but have not been compiled or executed in this environment because `rustc` and `cargo` are unavailable.

## 7. Reproduction

From the repository root, with a Rust 2024 toolchain and Cargo:

```bash
cargo test --manifest-path tools/ocean-multistream-balance-verifier/Cargo.toml
cargo run --manifest-path tools/ocean-multistream-balance-verifier/Cargo.toml
```

Expected receipt status: `SYNTHETIC_MULTI_STREAM_BALANCE_VALID_NO_SITE_OR_PERMIT_CLAIM`.

## 8. Next physical qualification steps

1. Start from a controlled, revisioned P&ID and site water-flow diagram.
2. Walk every port physically and identify all intake, return, purge, bleed, backwash, cleaning, bypass, drain, leak and condensate paths.
3. Meter each relevant stream or document the validated model and independent uncertainty basis for unmetered/evaporative terms.
4. Measure inventory changes and establish the timing/hold-up model for each control volume.
5. Calibrate density, temperature, salinity and volumetric/mass flow measurement chains, including shared calibration dependencies.
6. Conduct step/tracer response tests for residence-time distributions and transient alignment.
7. Independently replay raw evidence and compare the control-volume and global receipts against a pinned reference implementation.
8. Only after physical conservation closes should thermal-plume, pollutant load, permit and ecological assessment interpret the discharge.

**Claim ceiling:** the fixture verifies software behavior on a synthetic, fully declared two-volume network. It does not establish an as-built P&ID, actual stream completeness, water use, cooling-system performance, discharge compliance, environmental impact or site clearance.
