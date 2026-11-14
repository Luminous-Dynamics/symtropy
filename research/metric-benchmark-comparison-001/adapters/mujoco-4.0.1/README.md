# MuJoCo 4.0.1 matched Delassus operator adapter

This is an operator-level comparator for [PHYS-METRIC-BENCHMARK-001 #4805](https://github.com/Luminous-Dynamics/symtropy/issues/4805). It does **not** claim equivalence of the MuJoCo forward integrator and Symtropy's constrained midpoint step.

## Frozen mapping

The MJCF model has six translational generalized velocities: three orthogonal slide joints on each of two point-mass bodies, masses 1.3 kg and 2.1 kg, a 1.2 m site-to-site spatial tendon, and one equality/tendon constraint fixing the tendon to its compiled initial length. The model has no collision geoms and zero gravity. The force covector is the same fixture force `[0.18, -0.07, 0.11, 0, 0, 0]`.

This reproduces the six-dimensional mass matrix and one-row fixed-distance Jacobian at the frozen initial configuration. It compares (M), (J), (Y=JM^{-1/2}), and (W=JM^{-1}J^T). It does not compare the full discrete step or the solver's force law.

## Exact release pin

- MuJoCo release tag: `4.0.1`.
- Release commit: `4ed093a9f06b170bd1564942fa37e1bb907d93e3`.
- Linux x86-64 release archive SHA-256: `8df1308b2900a4ed6c18717ebbc9d66d53ab3fe7a4da860a7d5b21f94607299d`.

The workflow verifies the archive digest before extracting it. This exact version matters because 4.0.1 fixed a sparse Cholesky bug in the 4.0.0 Newton Hessian path.

## Capture procedure

The harness:
1. loads the exact MJCF file through the MuJoCo C API;
2. sets solver PGS and enables `mjENBL_DIAGEXACT`, which ensures the whitened Jacobian and exact Delassus diagonal are allocated;
3. runs `mj_forward` without integrating time;
4. reconstructs dense (M), (J), and (Y) from the official sparse representations;
5. independently computes (W=JM^{-1}J^T) using its own dense Cholesky solve;
6. computes (YY^T) independently and rejects the capture if the direct and whitened results differ by more than (10^{-10}) relative;
7. records native `efc_diagA`, `efc_R`, the constraint force, and the constraint residual separately.

The native diagonal is reported as a distinct MuJoCo diagnostic; it is not silently substituted for the independently calculated (W). The native `efc_R) is also reported separately because the solver's regularized matrix `efc_AR) is not the same object as the unregularized Delassus Gram (W).

## Run locally on Linux x86-64

```sh
curl -fL -o mujoco-4.0.1-linux-x86_64.tar.gz \
  https://github.com/google-deepmind/mujoco/releases/download/4.0.1/mujoco-4.0.1-linux-x86_64.tar.gz
echo "8df1308b2900a4ed6c18717ebbc9d66d53ab3fe7a4da860a7d5b21f94607299d  mujoco-4.0.1-linux-x86_64.tar.gz" | sha256sum -c -
tar -xzf mujoco-4.0.1-linux-x86_64.tar.gz
gcc -std=c11 -O2 -Wall -Wextra \
  -I./mujoco-4.0.1/include \
  research/metric-benchmark-comparison-001/adapters/mujoco-4.0.1/delassus_capture.c \
  -L./mujoco-4.0.1/lib -Wl,-rpath,"$PWD/mujoco-4.0.1/lib" -lmujoco -lm \
  -o mujoco-dumbbell-capture
./mujoco-dumbbell-capture \
  research/metric-benchmark-comparison-001/adapters/mujoco-4.0.1/dumbbell.xml \
  mujoco-operator-capture.json
```

## Claim boundary

The capture compares a matched mass/Jacobian/Delassus operator only. MuJoCo's equality/tendon solver uses its own constraint regularization and PGS solution. We do not claim its discrete state update equals the Symtropy midpoint step, and no industry speed ranking is produced. The result schema pins the engine version and exact six-DOF/one-row/no-contact mapping and fails closed on other model shapes.
