# Locomotion Sensory-Gate Oracle V1

A dependency-free Rust known-answer challenge for the modality- and state-dependent proprioceptive-gating hypothesis in Symtropy #3519.

## Falsifiable signature

The input is a complete 2×2×2 cell-level design:

- sensor modality: position-encoding vs movement-encoding;
- movement context: passive vs self-generated;
- presynaptic gate: blocked vs intact.

The target signature is deliberately selective:

1. Position-encoding activity remains comparatively stable across active/passive context and gate state.
2. Movement-encoding activity is suppressed during self-generated movement when the presynaptic gate is intact.
3. Blocking the presynaptic gate removes that movement-channel suppression.
4. Passive movement does not produce the same movement-channel suppression.

The oracle tests this pattern in a declared activity metric. It is Phase I of the broader sensory architecture: vibration-encoding FeCO club neurons and their intersegmental/ascending exteroceptive pathways (Lee et al. 2025) are a separate channel in #3519 and are not represented as a third modality in this V1 test. CxHP8 position/limit feedback is also separate. It is not a calcium-imaging simulator, a connectome model, a statistical power calculation or a claim that every proprioceptor behaves this way.

## API

- Enumerates the eight conditions with stable typed identities.
- Encodes the full 8-term factorial response surface using -1/+1 coding.
- Diagnoses design rank and recovers coefficients only when the full design is identifiable.
- Classifies the selective-gating signature using preregistered suppression and non-effect tolerances.
- Returns separate report flags; generic/global sensory damping cannot pass by matching only the movement-channel response.

Repeated measurements must first be analyzed with individual/session-aware statistics. The oracle accepts one cell-level estimate per condition and rejects duplicates.

## Negative controls

Synthetic tests include:
- selective position-vs-movement gating with active/passive controls;
- global damping that incorrectly suppresses position-encoding signals;
- movement suppression that persists when the presynaptic gate is blocked;
- movement suppression under passive stimulation;
- rank-deficient/missing-cell designs, duplicate conditions, non-finite outcomes and invalid thresholds;
- exact recovery of known factorial coefficients.

## Source grounding and evidence boundary

The test signature is derived from Dallmann et al., Nature (2025), Selective presynaptic inhibition of leg proprioception in behaving Drosophila, DOI 10.1038/s41586-025-09554-2, and is complementary to the CxHP8 position/limit-reflex results in Proprioceptive limit detectors contribute to sensorimotor control of the Drosophila leg (Nature Communications, 2026).

Source files, connectome materializations, evidence classes and animal/session identities must be pinned before using measured data. This crate contains synthetic fixtures only. Passing its tests qualifies software behavior, not a biological circuit.

## Run

    cargo fmt --manifest-path research/locomotion-sensory-gate-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-sensory-gate-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-sensory-gate-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings
