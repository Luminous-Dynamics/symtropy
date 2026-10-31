# Interval continuation and admissibility of Coulomb sliding branches

**Status:** test-only interval branch verifier; exact-head Rust validation pending.  
**Date:** 2026-10-31  
**Parent:** [#3179 interval root-family certificate](https://github.com/Luminous-Dynamics/symtropy/pull/3179).  
**Research contract:** [#3181 PHYS-CONTACT-BRANCH-CERT-001](https://github.com/Luminous-Dynamics/symtropy/issues/3181).  
**Qualification authority:** [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## From root counts to moving solution branches

#3179 can certify that a bounded input box has an invariant number of distinct finite real angular roots. That still does not prove which roots correspond to valid Coulomb impulses. A polynomial root can violate the nonnegative disk KKT multiplier, so counting all roots as physical contact solutions would be wrong.

This increment builds an interval continuation bracket for each simple representative root, then evaluates the original sliding impulse law over that bracket and the full physical input box.

## Uniform root bracket

For every exact representative Sturm root, the verifier deterministically widens a rational interval T = [t_lo,t_hi] until it finds:

- strict opposite signs for the interval polynomial at t_lo and t_hi, uniformly over every W/q/mu in the input box;
- an interval evaluation of P'(T) that excludes zero;
- a bracket disjoint from every other continued-root bracket.

The endpoint sign change proves that every parameter tuple has at least one root inside T. The derivative exclusion proves that this root is unique and simple in T. Pairwise disjoint brackets, together with #3179's invariant full root count, account for every finite real angular root in the family.

The routine starts from exact representative Sturm intervals, uses exact rational widening factors, caps bracket width by neighboring root separation, and returns INCONCLUSIVE if any branch cannot be bracketed within the fixed exponent budget.

## Sliding impulse interval evaluation

For each uniform root bracket T, the verifier constructs exact rational intervals for

- cos(theta) = (1 - t^2)/(1 + t^2);
- sin(theta) = 2t/(1 + t^2);
- D = W_nn - mu * (W_n1*cos(theta) + W_n2*sin(theta));
- lambda_n = -q_n / D;
- j_t = -mu * lambda_n * (cos(theta), sin(theta));
- u_t = q_t + W_tn*lambda_n + W_tt*j_t.

It then encloses:

1. normal denominator D;
2. normal impulse lambda_n;
3. disk KKT multiplier tau = -(j_t dot u_t)/(j_t dot j_t);
4. tangent orthogonality s_perp dot u_t;
5. Coulomb disk boundary residual j_t dot j_t - (mu*lambda_n)^2;
6. full impulse kinetic-energy change DeltaK = p^T q + 1/2 p^T W p.

The impulse is constructed on the Coulomb disk boundary; the interval residual must contain zero. The uniform root bracket proves the unreduced angular equation for the actual root; the independent orthogonality interval must also contain zero. The full energy interval retains normal/tangent cross-work terms.

## Branch outcomes

- **AdmissibleThroughout:** D excludes zero, lambda_n has a strictly positive lower bound, tau has a strictly positive lower bound (away from the stick/slip boundary), the disk and tangent residual enclosures contain zero, and the kinetic-energy upper bound is non-positive.
- **InadmissibleThroughout:** the complete interval proves a negative normal impulse, a negative disk multiplier, or strictly positive kinetic-energy change.
- **Inconclusive:** any required sign is unresolved, D may vanish, the normal impulse may touch zero, tau may touch zero, or the interval residual enclosure is inconsistent.

A branch with tau touching zero is deliberately not called sliding throughout: it may be crossing the stick/slip boundary. Repeated roots, root folds and projective theta = pi transitions are outside this first simple-branch verifier and remain fail-closed.

## Frozen tests

- The narrow regular SPD box used in #3179 yields two pairwise-disjoint simple angular-root brackets.
- Endpoint signs are opposite over the entire input family for both branches.
- P'(T) excludes zero for both branches.
- One branch has a strictly negative disk multiplier throughout and is rejected as inadmissible.
- The other has positive normal impulse, strictly positive disk multiplier, tangent/disk residual enclosures containing zero and non-positive kinetic-energy change throughout the box.
- A family whose leading coefficient may cross the projective boundary cannot produce uniform root brackets.

## Evidence and claim boundary

This verifies simple finite sliding branches over the declared exact dyadic W/q/mu box. It does not yet classify separating or sticking branches, complete multi-regime solution multiplicity, physical assembly of W/q from rigid-body data, measurement coverage, contact manifolds, multi-contact islands, production time stepping, or industry performance.

All results remain subordinate to #460. No runtime SOTA or unconditional uniqueness claim is made.
