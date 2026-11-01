# Exact physical two-contact Delassus rank deficiency and impulse equivalence

**Parent:** [#3217 — PHYS-CONTACT-GEOMETRY-RANK-001](https://github.com/Luminous-Dynamics/symtropy/issues/3217).  
**Synthetic full-rank counterpart:** [#3188](https://github.com/Luminous-Dynamics/symtropy/pull/3188), [#3200](https://github.com/Luminous-Dynamics/symtropy/pull/3200).  
**Qualification authority:** [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Geometry and rigid-body model

One unit-mass rigid body has principal inertia
`I = diag(1,2,3)` in generalized twist coordinates
`(v_x,v_y,v_z,omega_x,omega_y,omega_z)`. It touches a plane with normal
`e_x` at two points
`r_1=(0,0,0)` and `r_2=(0,1,0)`. Both contact frames are the orthonormal
basis `(n,t_1,t_2)=(e_x,e_y,e_z)`.

For a twist `(v,omega)`, point velocity is
`v + omega x r`. Consequently the exact contact Jacobian in ordering
`(n_1,t_11,t_12,n_2,t_21,t_22)` is

```text
J = [
  [1,0,0, 0, 0, 0],
  [0,1,0, 0, 0, 0],
  [0,0,1, 0, 0, 0],
  [1,0,0, 0, 0,-1],
  [0,1,0, 0, 0, 0],
  [0,0,1, 1, 0, 0]
]
```

The inverse generalized mass matrix is exactly
`diag(1,1,1,1,1/2,1/3)`. The verifier constructs
`W = J M^{-1} J^T` using exact rational arithmetic. It obtains

```text
W = [
  [1,0,0, 1, 0, 0],
  [0,1,0, 0, 1, 0],
  [0,0,1, 0, 0, 1],
  [1,0,0, 4/3,0, 0],
  [0,1,0, 0, 1, 0],
  [0,0,1, 0, 0, 2]
]
```

Exact elimination gives `rank(J)=rank(W)=5`. Because `M^{-1}` is positive
definite, `W` is positive semidefinite. Its exact one-dimensional impulse
nullspace is spanned by

```text
n = [0, 1, 0, 0, -1, 0]^T
W n = 0.
```

The null mode redistributes equal-and-opposite tangent-1 impulses at two
points separated along that same tangent direction. It changes neither the
resultant force nor the generalized torque.

## A continuum of admissible sticking impulses

Choose
`p_0=(2,0,0,2,0,0)^T` and `q=-Wp_0`. Exactly,

```text
q = [-4, 0, 0, -14/3, 0, 0]^T.
```

For friction coefficients `mu_1=mu_2=1/2), define

```text
p(alpha) = p_0 + alpha*n
         = [2, alpha, 0, 2, -alpha, 0]^T.
```

For every `alpha in (-1,1)`:

- both normal impulses equal 2 and are strictly positive;
- both post-contact velocities are exactly zero, since
  `q + W p(alpha) = q + W p_0 + alpha W n = 0`;
- each tangential impulse magnitude is `|alpha| < 1 = mu_i*lambda_i`,
  so both contacts stick strictly inside their Coulomb disks;
- normal complementarity is exact because `lambda_i > 0` and `u_ni=0`;
- the island energy change is exactly `Delta K = -26/3`, independent of
  `alpha`.

The disk margins are both `1-alpha^2`, so the open interval is analytically
admissible. The integration test evaluates several exact rational values of
`alpha`, verifies the full rational equations and exact energy, and proves
the common-response relation through `W n = 0` and `q^T n = 0`.

This is an infinite family of distinct contact-impulse vectors but one
generalized response. Any API that promises a unique impulse for this input is
mathematically wrong; the correct object is an admissible impulse set or an
equivalence class in `R^6 / ker(W)`.

## Three non-collinear contacts: a three-dimensional impulse family

The same rigid body can touch the plane at three non-collinear points
`r_1=(0,0,0)`, `r_2=(0,1,0)`, and `r_3=(0,0,1)`, using the same
`(e_x,e_y,e_z)` contact frame at each. The resulting exact
`J in Q^(9x6)` has full column rank six, but
`W=J M^{-1} J^T in Q^(9x9)` has rank six and nullity three.

Three independent exact contact-impulse null vectors are:

```text
n_a = [0,-1, 0, 0, 1, 0, 0, 0, 0]^T
n_b = [0,-1,-1, 0, 0, 1, 0, 1, 0]^T
n_c = [0, 0,-1, 0, 0, 0, 0, 0, 1]^T
```

For `p_0=(2,0,0,2,0,0,2,0,0)^T`, set `q=-Wp_0`. The exact free
contact velocity is `q=(-6,0,0,-20/3,0,0,-7,0,0)^T`. With
`mu_i=1/2`, the three-parameter family

```text
p(alpha,beta,gamma) = p0 + alpha*n_a + beta*n_b + gamma*n_c
```

is strictly admissible on the open cube
`(alpha,beta,gamma) in (-1/4,1/4)^3`. Each normal impulse remains two,
all three contact post-velocities remain exactly zero, and every tangential
impulse remains strictly inside its unit-radius Coulomb disk. The total energy
change is exactly `-59/3` throughout the cube.

The verifier constructs the 9x6 Jacobian and 9x9 Delassus operator using
exact rationals, proves the three null vectors lie in `ker(J^T)=ker(W)`,
and checks eight rational cube corners. The analytic coordinate bounds show
the entire open cube is disk-admissible. This is a three-dimensional continuum
of force allocations for a single rigid-body response, not three isolated
solutions.

## Paradigm-shift implications

1. **Separate response uniqueness from impulse uniqueness.** A unique body
   velocity update does not imply unique per-point contact-force allocation.
2. **Make rank and nullspace first-class data.** Receipts should contain
   certified rank bounds, nullspace basis/intervals, and the effect of null
   directions on physical observables.
3. **Return sets where mechanics gives sets.** A point estimate can be a
   representative, but it must not be mislabeled as the unique physical
   impulse. Preserve the admissible set's extent and friction-boundary
   intersections.
4. **Do not regularize away ambiguity silently.** Compliance, minimum-norm
   selection or warm-starting may pick one representative, but each changes
   or augments the selection rule. Record it explicitly.
5. **Use two complementary benchmark families.** Synthetic SPD tests stress
   algebraic multi-root behaviour; geometry-derived PSD tests stress
   physically real constraint redundancy and nullspace ambiguity.

## Qualification boundaries

This fixture proves a physically realizable rank-five Delassus matrix and a
continuum of strictly admissible sticking impulses for that exact geometry.
It is not a complete rank-deficient island solver. Arbitrary redundant
multi-contact islands, nullspace intersections with Coulomb cones, near-rank
uncertainty and contact creation/destruction still require separate certified
coverage. The full-rank synthetic two-contact results in #3200 remain a
different mathematical lane and are not presented as geometry-generated.
