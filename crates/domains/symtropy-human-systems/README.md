# Symtropy Human Systems — first bounded subsystem

**Status:** synthetic mathematical reference; not human-validated and not a clinical model.  
**Parent programme:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625).

This isolated Rust crate implements a narrow part of the classic two-process sleep-regulation model:

- **Process S:** a bounded homeostatic state that rises exponentially during wake and decays exponentially during sleep.
- **Process C:** a periodic circadian component represented by a configurable Fourier/harmonic series.

The implementation exposes these components separately. It does not convert them into a sleep/wake decision, alertness score, performance prediction, diagnosis, or treatment recommendation.

## Equations

For a fixed wake interval of length \(\Delta t\):

\[
S_{t+\Delta t}=1-(1-S_t)e^{-\Delta t/\tau_w}.
\]

For a fixed sleep interval:

\[
S_{t+\Delta t}=S_t e^{-\Delta t/\tau_s}.
\]

The configurable circadian component is:

\[
C(t)=\sum_{i=1}^{5} a_i\sin\left(\frac{2\pi i}{\tau}(t+\phi)\right).
\]

The implementation takes elapsed time in hours, and the caller supplies the wake/sleep time constants, circadian period, phase offset and harmonic coefficients. The optional reference five-harmonic coefficients are attributed to the model form reported by Achermann and colleagues' individualized performance implementation; they are not per-person fitted parameters. The wake and sleep time constants are deliberately required inputs rather than silently populated with a universal human default.

Primary mathematical references:
- Borbély (1982), *A two process model of sleep regulation*, Human Neurobiology 1:195–204.
- Borbély (2016), *The two-process model of sleep regulation: a reappraisal*, Journal of Sleep Research, DOI [10.1111/jsr.12371](https://doi.org/10.1111/jsr.12371).
- Achermann et al. (2008), *Individualized performance prediction of sleep-deprived individuals with the two-process model*, Journal of Applied Physiology, DOI [10.1152/japplphysiol.00877.2007](https://doi.org/10.1152/japplphysiol.00877.2007).

## Scope and limits

- This is the mathematical S/C reference layer, not a complete sleep physiology model.
- The model does not identify sleep onset/wake thresholds, NREM/REM stages, slow-wave activity, sleep inertia, naps, sleep disorders, medication effects or individual vulnerability.
- The circadian component is a fixed periodic function in v0.1. It is **not** a light-entrained oscillator and has no melatonin, temperature, feeding or shift-work adaptation mechanism.
- Process-S time constants and circadian phase must be independently specified/calibrated for an intended context. The reference coefficients are not individual parameter estimates.
- The code has not yet been fitted to MMASH or validated against Sleep-EDF. Synthetic tests only qualify the equations and numerical implementation, not their accuracy for any specific human.
- A state of Process S or C is not a medical measurement or a safe-to-work determination.

## Run

This crate is isolated from the root Symtropy workspace; no root workspace or ecological LifeSim semantics are changed.

```bash
cargo fmt --manifest-path crates/domains/symtropy-human-systems/Cargo.toml -- --check
cargo test --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --locked
cargo clippy --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --all-targets --locked -- -D warnings
```

## What comes next

1. Validate the implementation against analytic solutions and step-partition equivalence.
2. Compare the model form with a pinned reference implementation.
3. Build local-only public-data adapters for subject-held-out Sleep-EDF / MMASH evaluation.
4. Add light-driven circadian phase dynamics only as a separately versioned mechanism with explicit inputs and testable parameters.
5. Couple outputs to Symthaea through a qualified event-time / missingness / precision observation envelope (see [Symthaea #12407](https://github.com/Luminous-Dynamics/symthaea/issues/12407)).

No raw participant data or dataset-specific parameters are included in this crate.
