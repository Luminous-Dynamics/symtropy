# SOFiSTiK BE25 Published-Input Audit v0.1

**Status:** audit executable; BE25 benchmark reproduction is blocked.  
**Tracking:** [issue #2617](https://github.com/Luminous-Dynamics/symtropy/issues/2617).  
**Implementation draft:** this branch adds `sofistik_be25_audit`, `sofistik-be25-input-audit`, a frozen request fixture, mutation gates and a deterministic receipt.

## Source values

The [official SOFiSTiK 2026 BE25 PDF](https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be25.pdf) gives the Bernoulli-beam formula

\[
f=\frac{\lambda^2}{2\pi}\sqrt{\frac{EI}{\mu L^4}},
\qquad \mu=\frac{\gamma A}{g}
\]

and reports the following Table 2 fields: E=200 MPa, gamma=25 kN/m³, A=1 cm², I=0.1 cm⁴, L=1 m, and mu=0.025 t/m. Table 3/4 references are 0.457 Hz (simple cantilever), 2.003 Hz (cantilever with simply supported end), 1.283 Hz (simply supported), and 2.907 Hz (both ends fixed). The same PDF states that the numeric model uses ten Bernoulli beam elements and a consistent mass matrix.

The audit applies the printed units literally; it does not silently replace MPa with GPa, or infer a preferred mass from the target frequency.

## Conversion conflict

- Printed density-derived mass: \(\gamma A/g=(25{,}000\,\mathrm{N/m^3})(10^{-4}\,\mathrm{m^2})/(9.80665\,\mathrm{m/s^2})=0.2549290532\,\mathrm{kg/m}\).
- Printed line mass: \(0.025\,\mathrm{t/m}=25\,\mathrm{kg/m}\).
- Ratio: \(25/0.2549290532=98.0665\).

Using literal E=200 MPa, I=0.1 cm⁴ = 10⁻⁹ m⁴, L=1 m, and lambda=1.875 gives a cantilever analytical frequency of:

| Interpretation | Mass per length | Predicted first frequency | Relative error to 0.457 Hz |
|---|---:|---:|---:|
| Printed mu | 25 kg/m | 0.0500458 Hz | about 89.05% low |
| Derived from gamma*A/g | 0.254929 kg/m | 0.495596 Hz | about 8.45% high |
| Published reference | not implied by stated fields | 0.457 Hz | reference |

Inverting the same analytical equation for each published reference frequency implies about **0.300 kg/m** across all four cases, roughly 17.7% above the value from gamma*A/g and 83 times smaller than the printed 25 kg/m. Rounding of the published frequencies explains the small spread around 0.300 kg/m; it does not reconcile the two stated mass fields.

The numerical audit also evaluates all four boundary-condition cases, so the discrepancy cannot disappear through choosing a different lambda value. It emits the literal inputs, both predictions, per-case relative errors, and the mass implied by each reference value.

## Why this is an audit, not a benchmark pass

The candidates are contradictory. The implementation therefore emits `audit-complete` with disposition `blocked-until-source-input-resolved`; **this is not a benchmark pass**. The analysis does not tell us whether the PDF has a typo, the input-deck convention changes a quantity, or a different value is used in the shipped run.

Required next evidence:

1. Retrieve and independently inspect the officially attached ZIP/input file `eigenvalue analysis.dat`.
2. Record the actual input values and unit conventions, with the archive/input file digest and source attribution.
3. Keep three separate manifests until resolved: printed mu, gamma*A/g-derived mu, and the actual input-deck mass.
4. Recompute the four analytical references from the source-consistent input deck.
5. Compare against a ten-element consistent-mass Euler-Bernoulli beam implementation with mode pairing, frequency convergence and residual checks.

Until that is done, do not label the BE25 baseline reproducible or use it to calibrate solver accuracy.

## V&V boundary

This is analytical unit reconciliation and software-test-sensitivity evidence only. It is not a structural solver result, physical validation, uncertainty quantification study, safety assessment or code-compliance result. The broader process should follow the currently reaffirmed [ASME V&V 10-2019 (R2025)](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019), keeping verification, validation and uncertainty separate.
