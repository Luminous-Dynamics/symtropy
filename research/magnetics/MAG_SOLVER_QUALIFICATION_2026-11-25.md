# Magnetic solver qualification and external solver tournament — 25 November 2026

## Key research finding

The strongest immediate external comparator is not a generic commercial FEA package alone. The Magpylib ecosystem now provides a serious open solver ladder:
- Magpylib 6.1 analytical sources for homogeneous magnetization;
- magpylib-material-response face-charge for coupled linear isotropic soft materials;
- cell/volume methods for anisotropic susceptibility and B-H curves;
- FEM as a separate numerical formulation and cross-check;
- refinement/error-estimation workflows.

As checked on 25 November 2026, the upstream material-response changelog shows v0.14.0 dated 20 November 2026. Its adaptive cross-approximation update reports up to 10x faster whole solves in documented examples while keeping field blocks within 1e-8 of dense blocks. The package explicitly warns that it is experimental and can have breaking API changes. Treat it as a pinned research comparator, not an Luminous production dependency.

## Why this changes our roadmap

Do not write a new general-purpose demagnetization solver before testing this upstream solver family. The more valuable Luminous work is to:
1. preserve our separate full-vector analytical cuboid derivation and independent quadrature;
2. build a matched geometry / material / observer-grid benchmark;
3. compare face-charge and FEM where the material assumptions support them;
4. route anisotropic and nonlinear-BH cases only to methods that actually support them;
5. bind every output to version, source revision, source archive, environment, inputs, refinement history and output digest;
6. compare solver predictions with calibrated physical field maps only after measurement uncertainty is available.

## Model applicability, not solver marketing

The upstream face-charge guide documents a linear, uniform, isotropic susceptibility model and warns that anisotropic susceptibility and BHCurve cases require a cell/volume method. It also provides refinement studies; close/touching geometries may require substantially more resolution. Therefore, benchmark datasets must separate:
- homogeneous hard magnets;
- linear isotropic soft material;
- anisotropic soft material;
- nonlinear B-H materials;
- physical measurement evidence.

A solver that returns a number outside its admitted model class must be rejected or marked unsupported, not silently compared.

## Proposed benchmark families

| Family | Reference comparison | Main risk |
|---|---|---|
| Homogeneous cuboid | Symtropy surface-integral + quadrature vs Magpylib analytical | Formula/edge convention mismatch |
| Rigid finite assembly | Symtropy superposition vs Magpylib | Pose/polarization transforms |
| Magnet + linear isotropic yoke | Face-charge vs FEM | Panel convergence and near-gap error |
| Magnet + anisotropic material | Cell method vs FEM | Constitutive tensor and mesh dependence |
| Nonlinear B-H circuit | Cell method vs FEM, with measured material curve | Saturation and operating point |
| Physical encoder ring | Held-out solver predictions vs 3D Hall map | Calibration, registration and material variation |

## Evidence discipline

Keep these separate:
- solver-reported numerical error estimate;
- observed change between refinements;
- inter-solver disagreement;
- measurement covariance / calibration uncertainty;
- model discrepancy;
- physical specimen variability.

Agreement between two numerical methods does not establish physical truth. A calibrated physical map is a separate evidence class. A solver may be qualified for one geometry/material regime without being qualified for another.

## Upstream references

- Magpylib 6.1 physics: https://magpylib.readthedocs.io/en/6.1.0/_pages/user_guide/guide_resources_01_physics.html
- Material-response repository (experimental status): https://github.com/magpylib/magpylib-material-response
- Face-charge scope and limits: https://magpylib-material-response.readthedocs.io/en/stable/guide/face_charge.html
- Changelog v0.14.0 (20 November 2026): https://magpylib-material-response.readthedocs.io/en/stable/changelog.html

## Evidence status

- Rust qualification contract: authored.
- Unit tests: authored, not executed in this environment.
- External package execution / exact version lock: not performed here.
- Analytical parity against Magpylib: not performed here.
- FEM / face-charge matched comparison: not performed here.
- Physical metrology: not performed here.
