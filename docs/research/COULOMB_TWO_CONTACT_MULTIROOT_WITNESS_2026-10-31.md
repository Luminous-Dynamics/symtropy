# Two-contact coupled Coulomb multiple-solution witness

**Status:** frozen test-only equation witness; complete two-contact solver not implemented.  
**Research issue:** [#3187 — PHYS-MULTICONTACT-COULOMB-001](https://github.com/Luminous-Dynamics/symtropy/issues/3187).  
**Single-contact parent:** [#3168](https://github.com/Luminous-Dynamics/symtropy/pull/3168).  
**Qualification authority:** [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Why this witness matters

Two contacts sharing generalized degrees of freedom cannot be treated as two independent 3x3 solves. Their complete 6x6 Delassus matrix contains cross-contact blocks that couple the impulses.

This fixture is a concrete falsification target for the next solver. It has:
- a symmetric positive-definite 6x6 Delassus matrix;
- nonzero off-diagonal cross-contact blocks;
- two loaded contacts;
- three distinct admissible solutions for the same free velocity and friction coefficients;
- one contact sticking in all solutions, while the other has one sticking and two different sliding branches.

The three solutions are verified against the unreduced contact law in a double-precision independent prototype and frozen as Rust tests. The fixture test does not search for solutions or prove that these are the only solutions.

## Frozen input

Contact-coordinate ordering is (n1,t11,t12,n2,t21,t22).

Free relative velocity:

\`q = (-0.10770302, 1.49149027, -0.95561905, -1.0, 0.1, -0.1)\`.

Friction coefficients:

\`mu_1 = 1.383289774658335\`, \`mu_2 = 1.0\`.

The full symmetric Delassus matrix is:

\`\`\`text
[ 0.5949743200, -0.3905462000, -0.6354242600, -0.0000399246589471, -0.0012577367209219,  0.0025740231899637]
[-0.3905462000,  1.6505234000,  0.8091008900,  0.0004817979761919,  0.0006435466245356, -0.0002079266799144]
[-0.6354242600,  0.8091008900,  0.9452273500,  0.0000582870846214,  0.0003367426363655,  0.0001951070898771]
[-0.0000399246589471, 0.0004817979761919, 0.0000582870846214, 1, 0, 0]
[-0.0012577367209219, 0.0006435466245356, 0.0003367426363655, 0, 1, 0]
[ 0.0025740231899637, -0.0002079266799144, 0.0001951070898771, 0, 0, 1]
\`\`\`

The smallest eigenvalue in the generating prototype was approximately 0.0786717464. The cross-contact block Frobenius norm is approximately 0.0030088978, so this is not a block-diagonal system.

## Three distinct admissible impulses

The post-impulse velocity is u=q+Wp. For each contact, normal complementarity is enforced; sticking has zero tangential velocity and a strict interior Coulomb margin; sliding has the Coulomb boundary and u_t + tau j_t = 0 with tau>0.

| Branch | Contact 1 | Contact 2 | Kinetic-energy change |
|---|---|---|---:|
| A | Sticking | Sticking | -8.1526697111 |
| B | Sliding, tau=0.1985877354 | Sticking | -5.6978488251 |
| C | Sliding, tau=3.8800159788 | Sticking | -1.2013304004 |

The independent nonlinear prototype's maximum residual norms were approximately 7.1e-16, 3.1e-16 and 4.6e-16 respectively. The impulses are separated by order-one distances, so they are not duplicate representations of a single solution.

Both contacts have positive normal impulses in all three branches. The second contact remains strictly inside its friction disk, while the first contact changes between stick and two distinct sliding directions. Cross-contact energy terms are nonzero and change sign across the branches.

## What the Rust fixture checks

- full 6x6 symmetry and Cholesky positive definiteness;
- nonzero cross-contact block;
- three distinct impulse vectors;
- both normal complementarity branches;
- contact-specific sticking and sliding KKT conditions;
- strict Coulomb-disk margins where applicable;
- strictly negative island kinetic-energy change;
- nonzero cross-contact energy contribution.

## Claim ceiling

This is an equation-level multiple-solution witness and a regression fixture only. It does not implement a two-contact root solver, prove completeness of the three-solution set, certify a global uniqueness or no-solution condition, assemble W from geometry, or establish engine performance. The exact-head Rust workflow must pass before the Rust fixture is considered qualified.
