# Thermodynamic Ledger Reference V1 — Explicit Boundary Accounting

**Status:** research-only reference; no production ledger or physics behavior is modified  
**Symtropy baseline:** `Luminous-Dynamics/symtropy@556e78c6e93a63b1b734c9d1b9d3f91cdd7223ac`  
**Issue:** [THERMO-LEDGER-001 #1693](https://github.com/Luminous-Dynamics/symtropy/issues/1693)  
**Inspected production source:** `crates/domains/symtropy-consciousness-physics/src/thermodynamics.rs`, blob `56748ac7e43954e4f96a7fe0667db705dacf3cc5`  
**Inspected caller source:** `crates/domains/symtropy-consciousness-physics/src/coupling.rs`, blob `9ceb484c3960b78484fcd6bba259a75f0ebf05ac`

## Why this work exists

The current `ThermodynamicLedger::tick_balance` sets a local residual to `energy_in - energy_out` and treats its absolute value as a conservation error. That residual does not include the change in stored energy or necessarily all energy crossing the system boundary. The current call path also records regeneration with `record_dissipation(-effect)`, despite the field being described as energy dissipated as heat. The values are therefore being used as a signed net-flow convention under nonnegative-dissipation names.

These facts invalidate a claim that this particular residual proves system-wide energy conservation. They do not establish that the entire simulation is physically wrong; they establish that this residual alone cannot support that broader claim.

This PR creates a self-contained accounting reference and hostile fixture set before changing callers or API behavior.

## Boundary identity

For a clearly specified control volume and chosen energy store total:

[
R = E_{\mathrm{initial}} + \sum E_{\mathrm{boundary\ in}}
- \sum E_{\mathrm{boundary\ out}} - E_{\mathrm{final}}.
]

Under the accounting convention in this reference, the balance passes only when:

[
|R| \le \epsilon_r \max\left(1,
E_{\mathrm{initial}} + \sum E_{\mathrm{boundary\ in}}
+ \sum E_{\mathrm{boundary\ out}} + E_{\mathrm{final}}\right),
]

where the relative tolerance (\epsilon_r) is finite and lies in ((0,1]). This is a numerical bookkeeping criterion, not a physical-accuracy tolerance. Its actual value must be selected based on units, scale, rounding, integration method and the intended model—not copied blindly into all physics systems.

All stored energy and boundary-flow magnitudes must be finite and nonnegative. Direction is represented by which category contains an event, not by the sign of its magnitude. Regeneration is an inbound flow, not negative dissipation. Categories must be documented. Each event ID is unique within the balance record.

### Internal transfers

Transfers between stores inside the same control volume are not external boundary flows. If kinetic energy becomes thermal energy and both stores are inside the boundary, the total internal stored-energy change must account for that transfer while the transfer itself does not add energy to the system. If the receiver is outside the boundary, the event must instead be modeled as an outgoing boundary flow. Boundary placement determines the correct accounting semantics.

### What this identity does and does not establish

It can establish that an implementation matches a declared finite bookkeeping identity for its reference inputs. It does not establish that:

- every real energy reservoir is represented by the model;
- an entity's stored-energy field captures kinetic, potential, chemical, thermal and other relevant stores;
- all external work, radiation, force coupling, sinks and sources are measured or modeled correctly;
- units or coefficients match empirical physics;
- the simulator obeys physical conservation outside the explicitly tested boundary.

A residual of zero can occur when important flows or stores have been omitted consistently. Model-boundary completeness requires a separate inventory and empirical/model validation.

## Frozen cases and reference oracle

The v1 corpus contains **24 cases: 12 valid-domain examples and 12 invalid-domain inputs**. It includes:

- closed, stationary boundaries and balanced inflow/outflow;
- regeneration represented as a positive boundary inflow;
- internal kinetic-to-thermal transfer without fake external energy;
- counterexamples where equal inflow and outflow hide a changed final stored-energy total;
- balanced and unbalanced records just inside/outside the numerical tolerance;
- small-scale arithmetic, multiple sources/sinks and every permitted flow category;
- negative, NaN, infinite and overflowed energy values;
- negative stored-energy totals, duplicated event IDs, unknown internal stores, self-transfers, zero tolerance and undocumented categories.

The independent Rust test is deliberately separated from `ThermodynamicLedger`. It checks the frozen contract without claiming that the production code implements it. Its deterministic property grid constructs hundreds of balance cases, then perturbs final storage by an amount well above tolerance to ensure the check fails closed.

Five semantic mutation controls cover dropping the storage delta, encoding regeneration as negative dissipation, counting an internal transfer as an external flow, accepting duplicated/undocumented events, and omitting numeric domain/overflow validation.

## Next production work — gated, not included here

After the reference qualification passes, the next implementation change should be a separate PR. It should audit every call to `record_action`, `record_dissipation`, `record_phi_change` and `tick_balance`; explicitly select which energy stores lie inside the boundary; identify the source and destination for each transfer; and document units. Then:

1. add checked APIs that reject negative/non-finite magnitudes and handle accumulation overflow explicitly;
2. represent regeneration and imported energy with their own typed inbound-flow API, not negative dissipation;
3. count internal transfers without double-counting them as external work;
4. retain legacy residual fields as explicitly named local diagnostics until all boundary terms are represented;
5. measure long-horizon numeric drift and compare against analytically known accounting examples;
6. verify the exact migration of every production call site and preserve compatibility semantics deliberately.

No whole-system conservation theorem, physics calibration claim or production behavior change is implied by this reference.

## Proposed benchmark gates

| Measure | Initial research gate |
|---|---|
| Corpus | 12/12 valid cases accepted; 12/12 invalid cases rejected |
| Numerical identity | residual agrees with independent fixture within declared tolerance; pass/fail equals residual-vs-tolerance rule |
| Deterministic property sweep | every constructed balanced case passes; every perturbed case above tolerance fails |
| Event integrity | zero duplicate event identifiers accepted |
| Flow domains | zero negative or non-finite magnitudes accepted |
| Energy categories | zero regeneration encoded as negative dissipation |
| Internal transfer semantics | zero transfers counted as net boundary inflow/outflow |
| Overflow | all tested sum/residual/tolerance overflow cases reject explicitly |
| Production correspondence | no claim until a separate adapter and call-site migration are tested |
| Physical validity | no claim from this ledger reference alone |

The numeric fixture uses common abstract units for reference arithmetic only. Production thresholds must be justified against the simulation's actual magnitude distributions and time integration—not optimized to make the existing residual pass.

## Source

- [THERMO-LEDGER-001 issue #1693](https://github.com/Luminous-Dynamics/symtropy/issues/1693)
- [Production thermodynamics source at the baseline SHA](https://github.com/Luminous-Dynamics/symtropy/blob/556e78c6e93a63b1b734c9d1b9d3f91cdd7223ac/crates/domains/symtropy-consciousness-physics/src/thermodynamics.rs)
- [Coupling caller at the baseline SHA](https://github.com/Luminous-Dynamics/symtropy/blob/556e78c6e93a63b1b734c9d1b9d3f91cdd7223ac/crates/domains/symtropy-consciousness-physics/src/coupling.rs)
