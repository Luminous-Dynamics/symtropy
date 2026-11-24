# EHD-PIV structure-preservation metrics

Dependency-free Rust reference kernel for deterministic 2D velocity-field comparisons.

## Purpose

Separate global vector-field error from physical structure preservation. The scorecard reports vector coverage, relative L2, vorticity error, weak-structure recall/precision/Dice, sign agreement, signed amplitude bias and false vorticity support in reference-defined background.

Missing vectors are not treated as zero. The vector and derivative denominators retain the declared reference-domain counts, so selective deletion cannot improve a single global score without reducing explicit coverage or structure-retention metrics.

## Definitions and limitations

- Vorticity is computed as `dv/dx - du/dy` using centered finite differences on a five-point stencil.
- A stencil is evaluated only when its center and four neighbors belong to the declared evaluation domain.
- Reference vectors must be present everywhere in the declared domain. Missing reconstruction vectors reduce coverage and weak-structure retention.
- Weak structures are benchmark estimands defined by a frozen reference-vorticity magnitude band. These thresholds do not constitute a universal physical vortex detector.
- Strong reference-vorticity cells outside the weak band are neither weak positives nor background false-support opportunities.
- The precision/recall score counts retained weak structures as true positives and prediction-created support in reference-defined background as false positives.
- All metrics are on a 2D grid. No 3D, image-pair or physical EHD claim is implied.

## Run

    cargo fmt --manifest-path tools/ehd-piv-structure-preservation/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-structure-preservation/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-structure-preservation/Cargo.toml --all-targets -- -D warnings
