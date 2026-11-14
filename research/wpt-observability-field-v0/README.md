# WPT observability-aware field-design score oracle v0

**Status:** synthetic, one-axis known-truth scoring fixture. It contains no electromagnetic solver, measured hardware, physical transfer-efficiency model, exposure model, controller or flight capability.

## Research purpose

This executable seed tests the hypothesis in issue #4197: a candidate field/configuration can deliver a high useful-service proxy while leaving materially distinct receiver poses indistinguishable; a different synthetic signature map may improve local information and global pose separation while carrying a service/leakage trade-off.

This is a mathematical software oracle for a fixed lateral pose grid. It is not a reproduction of Li et al.'s 350 W / 85 kHz hardware, field model, measured sweeps or position-indexed Pareto lookup table.

## Synthetic fixture contract

- Pose grid: -100, -50, 0, +50 and +100 mm on one lateral axis.
- Global alias test separation: 100 mm.
- Two normalized sensor features per pose; each feature has declared standard deviation 0.1.
- A pose pair is an alias when its normalized signature distance is at or below 1.5.
- Local scalar Fisher information uses finite differences per sample and reports the minimum across the grid.
- Service and off-target values are dimensionless synthetic indices. Values 350 and 315 are not watts; values 35 and 32 are not field-strength or exposure units.
- All sensor signatures are invented known-truth fixture values.

## Candidate set

1. position_lut_analogue: symmetric signature map; high service index, but distinct negative/positive lateral poses alias.
2. balanced_current_analogue: same symmetric signature, with a different synthetic service/off-target trade-off.
3. information_bearing_candidate: adds an odd-symmetric signature component, distinguishing the sampled lateral poses while lowering the service index.
4. off_target_violation_control: uses the informative signature but exceeds the synthetic off-target ceiling and must be rejected.

Candidate names are optimizer-family analogies, not faithful implementations of published algorithms or coil designs.

## Pareto semantics

The evaluator keeps service index, minimum Fisher information, global alias margin, unique sample fraction and off-target index separate. It enforces the service floor and off-target ceiling before Pareto filtering. No weighted scalar can compensate for a hard constraint failure.

## Commands

    cargo test --locked --manifest-path tools/wpt-observability-field-oracle/Cargo.toml
    cargo run --locked --manifest-path tools/wpt-observability-field-oracle/Cargo.toml -- research/wpt-observability-field-v0/synthetic-designs.tsv

## Claim ceiling

A passing result establishes arithmetic and alias/constraint dispositions for this frozen synthetic one-dimensional dataset only. It does not establish a real WPT observation map, physical localization accuracy, field uniformity/leakage, measured energy delivery, exposure safety, hardware feasibility or SOTA superiority. Physical model and sensor/noise inputs must come from independently qualified modality-specific owners before physical interpretation.
