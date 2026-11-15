# Power Fabric Module Contract v0.1

**Status:** interface research contract; not a product specification, wiring standard, protection design or installation approval  
**Date:** 2026-11-14  
**Owner issue:** [POWER-FABRIC-MODULE-CONTRACT-001 #5159](https://github.com/Luminous-Dynamics/symtropy/issues/5159)  
**System owners:** [#3400](https://github.com/Luminous-Dynamics/symtropy/issues/3400), [#3409](https://github.com/Luminous-Dynamics/symtropy/issues/3409), [#3418](https://github.com/Luminous-Dynamics/symtropy/issues/3418)  
**Human-facing consumer:** [#5158](https://github.com/Luminous-Dynamics/symtropy/issues/5158)

## 1. Purpose

Define a vendor-neutral description of a power-system module so that heterogeneous generation, storage, conversion, protection, distribution, thermal and sensing equipment can be composed and compared without pretending their internal physics are identical.

This contract describes **interfaces, capabilities, constraints, evidence, serviceability and identity**. It does not implement a universal internal physics model and does not grant a compatibility result the meaning of a safety approval.

The service boundary work in PR #3419 remains authoritative for delivered-service semantics such as net AC, regulated DC and propulsion work. This contract expands the module's port and qualification description around that service layer.

## 2. Core design rules

1. **Typed ports:** AC, DC, thermal, measurement, control and protection interfaces are distinct types.
2. **Evidence-bound claims:** every capability is tied to a particular asset/configuration, operating envelope and evidence class.
3. **Unknown is not compatible:** missing, stale, contradictory or out-of-scope data returns UNKNOWN or BLOCKED, never optimistic compatibility.
4. **Distinct internals:** reciprocating engines, linear generators, turbines, fuel cells, batteries and converters retain their own internal energy ledgers and source-specific models.
5. **Whole-boundary compatibility:** voltage match alone is insufficient. Check power direction, current, frequency/phase, transients, protection, grounding, thermal support, auxiliaries, environment, modes and jurisdiction.
6. **Protection stays independent:** no Symthaea, Symtropy or Mycelix module contract can override local protection.
7. **Human serviceability:** identity, labels, isolation limits, stored-energy hazards and authorized service role must be explicit.
8. **Replaceability is conditional:** a replaceable module still needs a qualified isolation and verification process. Do not assume hot swapping.

## 3. Module identity

Each module declares the fields that exist and explicitly marks unavailable fields unknown:

| Field | Meaning |
|---|---|
| `module_id` | Stable identifier within the Power Fabric graph |
| `module_class` | Source, storage, conversion, protection, distribution, load, thermal, measurement or control |
| `manufacturer` / `model` | Manufacturer and exact model identity |
| `hardware_revision` | Installed hardware revision |
| `firmware_revision` | Firmware/software revision if present |
| `configuration_digest` | Digest of relevant configuration |
| `asset_identity` | Serial/lot/site asset identity where available |
| `declared_claims` | Manufacturer/nameplate claims, kept distinct from measured results |
| `evidence_refs` | Claim-specific evidence IDs and provenance |
| `environment` | Declared site and environmental applicability |
| `lifecycle_state` | Proposed, installed, unavailable, quarantined, removed or retired |
| `service_profile` | Inspection/maintenance boundary and role requirements |

A catalogue model is not the same thing as an installed physical asset. The design graph must bind to an exact asset when a physical installation is claimed.

## 4. Typed ports

Port values require unit-bearing quantities and explicit direction. A port declaration has an identifier, type, directionality, operating envelope, connection constraints, evidence references and applicability.

### AC port

Declare nominal and permitted voltage, phase count, frequency, current and power envelopes, permitted import/export direction, power-factor range, P/Q/S capability where relevant, waveform-quality evidence and operating modes.

### DC port

Declare nominal and permitted voltage range, polarity, current and power envelopes, allowed direction, ripple/quality envelope, isolation/grounding semantics and pre-charge/inrush constraints where applicable.

An AC port and DC port are incompatible even if both happen to report the same numeric voltage.

### Thermal port

Declare whether the module rejects, absorbs or stores thermal energy; operating temperature range; heat transfer capability and limitations; fluid/pressure/flow semantics where applicable; ambient limits; thermal interfaces; auxiliary energy required for thermal operation.

A module requiring active cooling is not thermally compatible merely because it has a liquid port. The cooling capacity, temperature range, flow conditions, pumping power and failure response must be supported by evidence and a compatible thermal module.

### Measurement port

Declare the measurand, physical location/boundary, unit, range, accuracy/uncertainty, cadence, timestamp semantics, calibration identity/expiry, quality flags, freshness limit, missing-data behavior and provenance.

Derived estimates are not physical measurements. A stale or invalid measurement cannot support a current-state safety claim.

### Control port

Declare command types, allowed modes, caller roles, preconditions, bounds, expiry/timeout, authentication expectations, local fallback and response evidence. A control port does not itself grant authority to command equipment.

### Protection/isolation port

Declare the protection and isolation function, applicable device/product evidence, coordination references, declared interruption/withstand envelopes, grounding/earthing interface, isolation boundary, interlocks and residual-energy limitations. A generic Boolean `is_safe` or `is_isolated` field is prohibited.

## 5. Operating envelope and modes

Declare separately:
- continuous, overload, transient and derated envelopes;
- import/export and charge/discharge directions;
- ramp, start, stop, restart and minimum stable operating constraints;
- supported grid-connected, grid-forming, grid-following, islanded and black-start modes;
- environmental and altitude/temperature derating;
- required fuel, cooling, ventilation, reference source and auxiliary power;
- safe shutdown and loss-of-communications behavior;
- fault and degraded operating behavior.

Unsupported modes are explicit. "Inverter" does not imply grid-forming. "Battery" does not imply black-start. A semiconductor switch does not imply a qualified DC circuit breaker. A product brochure is not independent field evidence.

## 6. Evidence class

Use an explicit claim-level evidence enum:

| Class | Meaning |
|---|---|
| Unknown | No admissible evidence for the claim |
| ManufacturerDeclared | Datasheet or manufacturer statement |
| Modelled | Simulation or analytical model output |
| Measured | Physical measurement with declared method and instrumentation |
| IndependentlyVerified | Independent test/review supports the specific claim |
| FieldQualified | Evidence from a declared field configuration and operating envelope |

The enum is not a universal ranking in which every claim automatically progresses to a higher level. Evidence must be fit for the specific claim. A measured voltage does not qualify inverter fault behavior. A third-party emission test does not qualify net electrical efficiency. A laboratory test does not establish every field condition.

Bind each evidence object to:
- module/asset and configuration identity;
- claim and operating mode;
- test procedure/profile and operating point;
- time window and source;
- measurement boundary;
- instrument and calibration identity;
- uncertainty and limitations;
- raw/derived data digest and reviewer;
- evidence class and applicability envelope.

## 7. Compatibility evaluation

A compatibility engine returns one of:

- `CompatibleWithinDeclaredEnvelope`
- `Incompatible(reason_codes)`
- `UnknownInsufficientEvidence(reason_codes)`
- `CompatibleOnlyWithDeclaredAdapterOrProtection(reason_codes)`
- `StaleConfigurationRequiresReconciliation(reason_codes)`

Evaluate at least:
1. Port type and direction.
2. Voltage, current, power and polarity.
3. AC frequency, phase and waveform constraints.
4. Continuous, transient and derated envelopes.
5. Operating mode and command authority.
6. Protection, grounding/earthing and isolation.
7. Thermal limits and auxiliary energy.
8. Environment and installation context.
9. Connector/keying and mechanical fit evidence.
10. Firmware/configuration and protocol compatibility.
11. Jurisdiction, product approval and utility constraints.
12. Evidence class, freshness and claim applicability.

A "compatible" result means only that the declared interfaces overlap under the declared assumptions. It is not a complete system design, installation approval, interconnection approval, certification or permission to energize.

## 8. Example: synthetic module port

This is a schema illustration, **not real equipment data or a construction specification**:

```json
{
  "module_id": "synthetic-battery-converter-01",
  "module_class": "conversion",
  "configuration_digest": "sha256:synthetic-example-only",
  "ports": [
    {
      "port_id": "dc-1",
      "port_type": "dc",
      "direction": "bidirectional",
      "voltage": {
        "unit": "V",
        "minimum": null,
        "nominal": null,
        "maximum": null,
        "status": "unknown"
      },
      "polarity": "declared-by-qualified-product-record",
      "evidence_class": "unknown"
    },
    {
      "port_id": "ac-1",
      "port_type": "ac",
      "direction": "bidirectional",
      "frequency": {
        "unit": "Hz",
        "status": "unknown"
      },
      "phase": {
        "status": "unknown"
      },
      "evidence_class": "unknown"
    }
  ],
  "compatibility_status": "unknown_insufficient_evidence"
}
```

Null is intentional. Missing ratings must not be filled with plausible-looking defaults.

## 9. Required hostile fixtures

- Numerically equal AC/DC voltages are rejected as a type mismatch.
- Polarity mismatch or disjoint voltage envelopes reject compatibility.
- Missing current or transient rating blocks the relevant claim.
- Nameplate-only evidence cannot satisfy a requirement for independent verification.
- Stale firmware/configuration invalidates relevant compatibility.
- Thermal support is insufficient when capacity, temperature or auxiliary-power constraints do not match.
- A grid-following inverter cannot be relabelled grid-forming.
- A battery discharge trajectory cannot be treated as fresh generation without initial/final stored energy and charge-source accounting.
- Hidden alternate feeds block any isolation/service claim.
- Missing protection, grounding, connector or jurisdiction evidence returns UNKNOWN/BLOCKED.
- A failed communications link cannot disable independent local protection.
- A component replacement changes the installed asset graph and affected view/evidence digests.
- Incompatible converter/connector combinations cannot be made compatible by a UI warning.
- A physical connector that can mate is not automatically an electrically or functionally compatible interface.

## 10. Benchmarks and acceptance

Use a frozen, versioned synthetic catalogue and matched integration cases. Compare manual datasheet interpretation and bespoke vendor integration with the typed evaluator.

Measure:
- critical incompatibility detection recall;
- false-accept rate (primary safety metric);
- false-reject rate and reason-code quality;
- percentage of claims bound to exact evidence;
- time/expert effort to compose a valid set;
- hidden integration assumptions found;
- stale-configuration detection;
- change-impact completeness;
- vendor and AC/DC/thermal interoperability;
- evidence-class misclassification.

Hard gates:
- zero false accepts in the frozen critical incompatibility suite;
- zero Unknown-to-Compatible coercions;
- zero evidence-class promotions;
- zero "installation approved", "safe to energize" or certification claims from this contract alone.

Numeric convenience/performance targets should be preregistered against the manual baseline. Do not tune a threshold after looking at candidate performance.

## 11. Integration boundaries

- **Symtropy:** deterministic schema validation, compatibility checks, energy/thermal service trace replay and reason-code receipts.
- **Symthaea:** read-only explanation, diagnosis, forecasting and proposal generation against verified module/evidence records.
- **Mycelix:** role/authority, approval and provenance where federated coordination is required.
- **Physical protection:** hardware-local, qualified and independent from the software layers.
- **Qualified practitioners and authorities:** installation, safety validation, approval, commissioning and certification.

Keep internal model ownership specific: source topology models own conversion physics; storage models own electrochemical/thermal behavior; power-electronics and protection owners own their qualified device/system behavior. This contract describes their connection, not their physics.

## 12. First implementation sequence

1. Create Rust enums/structs and versioned serialization for identity, typed ports, envelopes, evidence and compatibility results.
2. Add AC/DC/thermal/measurement/control/protection fixtures with unknown-by-default fields.
3. Implement pure deterministic compatibility evaluation with stable reason codes.
4. Add hostile fixtures and independent oracle tests.
5. Bind module contracts to #3409's topology variants and #3419's delivered-service traces without merging their internal physics.
6. Export graph summaries to #5158's human-facing design compiler.
7. Build a small frozen catalogue and benchmark against manual integration.
8. Obtain independent electrical/power-electronics review before representing real equipment or mapping legal/product requirements.

**Definition of done:** a heterogeneous set of synthetic and evidence-classified modules is composable only when typed interfaces and declared constraints match; unknown/incompatible cases fail closed with useful reasons; and every compatibility and delivered-service claim is traceable to the exact module, configuration and evidence digest.
