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

`q = (-0.10770302, 1.49149027, -0.95561905, -1.0, 0.1, -0.1)`.

Friction coefficients:

`mu_1 = 1.383289774658335`, `mu_2 = 1.0`.

The full symmetric Delassus matrix is:

```text
[ 0.5949743200, -0.3905462000, -0.6354242600, -0.0015969863578854, -0.0503094688368766,  0.1029609275985498]
[-0.3905462000,  1.6505234000,  0.8091008900,  0.0192719190476772,  0.0257418649814222, -0.0083170671965776]
[-0.6354242600,  0.8091008900,  0.9452273500,  0.0023314833848562,  0.0134697054546187,  0.0078042835950851]
[-0.0015969863578854, 0.0192719190476772, 0.0023314833848562, 1, 0, 0]
[-0.0503094688368766, 0.0257418649814222, 0.0134697054546187, 0, 1, 0]
[ 0.1029609275985498, -0.0083170671965776, 0.0078042835950851, 0, 0, 1]
```

The smallest eigenvalue in the generating prototype was approximately 0.0707398084. The cross-contact block Frobenius norm is approximately 0.1203559111, so the cross-contact coupling is deliberate and materially larger than the original weak-coupling seed.

## Three distinct admissible impulses

The post-impulse velocity is u=q+Wp. For each contact, normal complementarity is enforced; sticking has zero tangential velocity and a strict interior Coulomb margin; sliding has the Coulomb boundary and u_t + tau j_t = 0 with tau>0.

| Branch | Contact 1 | Contact 2 | Kinetic-energy change |
|---|---|---|---:|
| A | Sticking | Sticking | -8.6104275288 |
| B | Sliding, tau=0.2654416784 | Sticking | -5.1814817181 |
| C | Sliding, tau=4.7094359135 | Sticking | -1.0982825525 |

The independent nonlinear prototype's maximum residual norms were below 1e-9, with the largest residual about 8.2e-10 in the second branch. The impulses are separated by order-one distances, so they are not duplicate representations of a single solution.

Both contacts have positive normal impulses in all three branches. The second contact remains strictly inside its friction disk, while the first contact changes between stick and two distinct sliding directions. Cross-contact energy contributions are nonzero in all three branches (approximately -2.16458, -0.19618, and -0.002633 respectively).

## Reduced-normal-map insight for the next solver

Reorder the island coordinates conceptually into normal and tangent blocks,
\[
W=\begin{bmatrix}W_{nn}&W_{nt}\\W_{tn}&W_{tt}\end{bmatrix},
\qquad p=(\lambda,j),\qquad q=(q_n,q_t).
\]
Because the full Delassus matrix is SPD, its tangential principal block
\(H=W_{tt}\) is SPD. For any fixed normal-load vector \(\lambda\ge0\), the
tangential response is the unique minimizer
\[
j^*(\lambda)=\arg\min_{\|j_i\|\le\mu_i\lambda_i}
\left[\tfrac12j^THj+(q_t+W_{tn}\lambda)^Tj\right].
\]
Its KKT system is
\[
Hj+q_t+W_{tn}\lambda+
\operatorname{diag}(\tau_1I_2,\tau_2I_2)j=0,
\quad \tau_i\ge0,
\quad \tau_i(\|j_i\|^2-\mu_i^2\lambda_i^2)=0.
\]
Strict convexity makes these conditions sufficient for the unique tangential
minimizer at that \(\lambda\). The remaining coupled problem is the
two-dimensional normal complementarity map
\[
g(\lambda)=q_n+W_{nn}\lambda+W_{nt}j^*(\lambda),
\quad \lambda\ge0,\quad g(\lambda)\ge0,\quad
\lambda^Tg(\lambda)=0.
\]

The three frozen witnesses have positive normal load at both contacts and
satisfy the tangential QP KKT conditions; their multiplicity therefore appears
as distinct roots of the reduced normal map, not as ambiguity in the inner
tangent minimizer. This suggests a future architecture: solve the small
strongly-convex tangential QP (or its two-multiplier dual) for each trial
\(\lambda\), then enumerate normal active sets and certify the outer roots
with interval methods. Do not assume the reduced normal map is monotone; the
non-associated normal/friction coupling is exactly where multiple branches
can arise.

This is a solver-design deduction from the frozen fixture, not an implemented
two-contact algorithm or a completeness proof.

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
