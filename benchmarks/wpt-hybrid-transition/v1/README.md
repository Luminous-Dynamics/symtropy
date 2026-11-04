# WPT hybrid mode-transition energy validator v1

**Status:** research software, deterministic synthetic known-truth fixtures. No physical WPT switchgear or resonant network has been qualified by this tool.

## Research gap

The parent programme separately checks complete inductance-matrix passivity, frequency-dependent multiport positive-realness, transient-model error, segment handover, and source-to-load energy accounting. None of those results alone certifies the state jump when a circuit changes topology or state dimension.

A resonant mode can be passive during continuous operation while an invalid reset map copies, drops, duplicates, or invents stored energy at a switching event. Therefore a mode transition is a separate energy event with its own model, state-map and energy receipt.

## Prior art and WPT relevance

- Frasca et al., *Linear Passive Networks With Ideal Switches: Consistent Initial Conditions and State Discontinuities*, IEEE Transactions on Circuits and Systems I (2010), DOI 10.1109/TCSI.2010.2052511. The work studies admissible switched-network states and energy-minimizing state reinitialization.
- Restrepo et al., *Passivity preserving energy-aware design for multi-dimensional switched systems*, Automatica (November 2025), DOI 10.1016/j.automatica.2025.112496. The work develops hybrid energy tanks to handle energy jumps when switched-system state dimension changes.
- Li et al., *Dual-Mode Switching with Coupling Reversal for High-Misalignment Tolerance and CV Output in AGV WPT Systems*, IEEE Transactions on Transportation Electrification (early access 12 June 2026), DOI 10.1109/TTE.2026.3703005. A 500 W experimental WPT prototype switches between LCC-S and LCC-S-S modes; the paper reports 90.28% maximum DC-DC efficiency. This is a low-power transition-design reference, not an EV-scale efficiency comparator.

The contribution here is not a new theory of hybrid passivity. It is a WPT-specific, source-bound, machine-checkable event contract that can be connected to the existing physical model and route ledger.

## Event contract

Each transition binds two already-qualified modes and a declared state mapping. The total stored-energy inventory is:

    H_total = H_physical + H_energy_tank

For one switching window, define signed net energy entering the modeled boundary as positive:

    Delta H_total = E_port,net + W_switch,net - E_event,dissipated + residual

The validator computes the residual and a conservative absolute error bound by summing the declared deterministic uncertainty bounds. These bounds are not standard deviations or confidence intervals. A separately digested tolerance policy is mandatory; the combined numerical decision tolerance is capped at 0.5% of the declared energy scale. The energy-generation inequality uses only the conservative measurement/model uncertainty plus a 64-epsilon floating-point guard, so a loose residual tolerance cannot excuse proven energy creation.

The separate non-generation inequality is:

    Delta H_total <= E_port,net + W_switch,net

A PASSIVE_TRANSITION_CERTIFIED disposition requires both adjacent modes to carry PASSIVE_CERTIFIED receipts, a valid topology/state mapping, nonnegative declared dissipation, a complete energy-tank receipt when applicable, a residual interval fully contained by the declared decision tolerance, and no proven energy-generation violation.

A positive residual is never reinterpreted as an unknown source. A large negative residual is not silently called safe: it is ENERGY_BALANCE_UNRESOLVED until missing event loss or measurement boundaries are explained.

## State mapping rules

- IDENTITY is allowed only when the source and target dimensions match.
- CHARGE_FLUX_CONSISTENT, ENERGY_MINIMIZING and DECLARED_PHYSICAL_RESET require a source-bound mapping digest and exact source/target dimensions.
- ENERGY_TANK requires a complete pre/post tank-energy receipt. Tank energy is included in the total storage inventory, so it cannot be spent twice.
- Missing state, mode or energy evidence fails closed.
- This v1 validator accepts supplied stored-energy values. It does not derive capacitor charge, inductor flux, Hamiltonians, or reset maps from raw circuit states.

## Dispositions

- PASSIVE_TRANSITION_CERTIFIED: energy balance closes within the declared conservative uncertainty/tolerance interval.
- ENERGY_GENERATING_REJECTED: the conservative lower bound proves stored-energy growth beyond port input and switching work.
- ENERGY_GENERATION_UNRESOLVED: the energy-generation interval overlaps a possible positive jump; uncertainty is too large to rule out generation.
- ENERGY_BALANCE_UNRESOLVED: generation has been bounded out, but the complete energy balance does not close tightly enough to certify.
- ENERGY_TANK_RESERVE_UNRESOLVED: a conservative pre/post energy-tank reserve lower bound is negative; a wide tolerance cannot make a potentially negative reserve safe.
- MODE_UNQUALIFIED: at least one adjacent mode lacks PASSIVE_CERTIFIED upstream evidence.
- INVALID_INPUT: required fields, dimensions, digests, units-by-contract, nonnegative quantities, event identity/time or mapping semantics are invalid.

These labels are event-level software dispositions, not proof of hardware stability, safety, EMC, exposure compliance or standards conformance.

## Known-truth corpus

The JSONL corpus includes energy-conserving, dissipative, externally powered, energy-tank-funded, energy-generating, uncertainty-overlap and unqualified-mode cases. The main negative control deliberately gives both adjacent modes a PASSIVE_CERTIFIED mode status while the state reset creates unexplained stored energy. Static per-mode passivity alone must not admit this transition.

## Run

    cargo test --locked -p wpt-hybrid-transition-validator
    cargo run --locked -p wpt-hybrid-transition-validator -- benchmarks/wpt-hybrid-transition/v1/known-truth.jsonl

The CLI compares every expected disposition in the frozen JSONL corpus and rejects duplicate event identifiers.

## Next research increments

1. Add charge/flux-constraint residuals and state-vector reset receipts for explicit switched RLC models.
2. Compare naive copy/truncation, energy-minimizing reinitialization, common-storage maps and hybrid energy-tank policies.
3. Ingest a source-backed WPT mode-switching waveform when the complete waveform, component values, switch timing and measurement boundaries are available.
4. Couple the event receipt to the transient ROM trace and segment-level route energy ledger.
5. Add formal bounds or interval arithmetic before making continuous-state or hardware claims.

## Claim ceiling

Synthetic fixtures and validator tests do not demonstrate WPT performance. The 500 W experiment above is literature context only. No high-power switching, live grid, roadway, human exposure or physical hardware test is authorized or claimed.
