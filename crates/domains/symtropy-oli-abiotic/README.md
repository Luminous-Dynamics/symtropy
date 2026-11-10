# symtropy-oli-abiotic

Deterministic synthetic abiotic mechanism generators for the Open-World Life & Intelligence benchmark.

## First generator: Gray–Scott reaction–diffusion

Generator ID: \`OLI-ABIOTIC-GRAY-SCOTT-001\`.

Evidence class: \`SYNTHETIC_KNOWN_TRUTH\`.

The reference implements the two-field Gray–Scott reaction–diffusion update on a periodic 2D grid. The initial perturbation is deterministic and seeded. The reference profile uses a 64×64 grid, the canonical seeded patch, explicit Euler integration and 500 steps.

A matched **diffusion-only** control uses the same initial condition, timestep and diffusion coefficients while disabling reaction/feed/kill terms.

The benchmark reports spatial concentration means/variances and extrema. It does not infer life from patterns, and does not treat visual spot formation as reproduction or heredity.

## Numerical contract

For the explicit 2D five-point Laplacian, the configured diffusion update is rejected if \(D\,dt > 1/4\). Invalid/non-finite parameters and concentration values outside the declared [0, 1] physical range are rejected transactionally: a failed step does not partially mutate the grid.

The reaction and forcing parameters are separately declared. Removing feed is an intervention whose outcome is measured, not assumed.

## Run

\`\`\`bash
cargo test -p symtropy-oli-abiotic --locked
\`\`\`

## Claim ceiling

This is one discrete synthetic reaction–diffusion mechanism. It establishes only that the declared model can produce spatial organization under the tested update rules and parameter profile. It does not establish that natural reaction–diffusion systems are non-living, or that any observed physical pattern is life, reproduction, agency, intelligence or consciousness.
