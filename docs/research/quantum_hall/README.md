# Quantum Hall research benchmarks

## Spherical Laughlin V1 gap

The machine-readable proof-aware benchmark registry is:

- `laughlin_v1_gap_benchmark_v1.json`

It compares separately:
- OpenAI Math Family 269's original Lean-formalized unperturbed bound;
- Qiang Gao's improved exact-rational SDP certificate and reported (1/5) spherical gap;
- Scheer–Khalaf's independent parametric-SDP / rank-stability certificate;
- finite-size SU(2)-resolved exact diagonalization.

The registry keeps exact theorem bounds, numerical SDP candidates, and finite-size spectra as distinct evidence classes. It records the coefficient-one pair-projector convention and the (Q=3(N-1)) shift.

## Ownership

- Symtropy finite-size numerical benchmark: [#4529](https://github.com/Luminous-Dynamics/symtropy/issues/4529)
- Symthaea formal scope audit: [#11407](https://github.com/Luminous-Dynamics/symthaea/issues/11407)
- Symthaea improved certificate verification / formalization: [#11422](https://github.com/Luminous-Dynamics/symthaea/issues/11422)
- Mycelix evidence lineage: [#6027](https://github.com/Luminous-Dynamics/mycelix/issues/6027)

This directory currently specifies benchmarks only. It contains no Luminous spectral computation, certificate replay, or improved Lean proof.
