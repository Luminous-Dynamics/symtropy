# Exact Sturm certificate for the coupled two-contact slide/stick stratum

**Status:** exact-binary64, test-only root-completeness certificate for one active regime stratum.  
**Parent research:** [#3187 — PHYS-MULTICONTACT-COULOMB-001](https://github.com/Luminous-Dynamics/symtropy/issues/3187).  
**Frozen physical witness:** [#3188](https://github.com/Luminous-Dynamics/symtropy/pull/3188).  
**Single-contact exact oracle reused:** [#3175](https://github.com/Luminous-Dynamics/symtropy/pull/3175).  
**Qualification authority:** [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Result

For the frozen symmetric positive-definite 6x6 Delassus matrix, free velocity and friction coefficients in #3188, the active regime

- contact 1: sliding on its Coulomb disk;
- contact 2: sticking strictly inside its Coulomb disk;
- both normal contacts active;

has exactly **two admissible finite angular roots**.

The exact angular equation has four distinct real roots. Two have positive contact-1 disk multipliers and dissipative island energy; two have negative disk multipliers and are rejected by the original KKT law. The active 4x4 linear system has two singular real angular directions, but the exact Cramer/determinant gcd is one, so neither singular direction admits a compatible right-hand side. The quartic has a nonzero leading coefficient, excluding a further projective root at theta = pi.

This certifies the slide/stick stratum only. Combined with the separately reconstructed stick/stick branch in #3188, it gives three independently validated solutions of the full coupled witness, but it does **not** yet rule out solutions in the other separating/sticking/sliding regime combinations.

## Derivation

Use contact-coordinate ordering (n1,t11,t12,n2,t21,t22), with
`u = q + W p`, `mu_1 = 1.383289774658335`, and `mu_2 = 1`.

For contact 1 sliding, set
`t = tan(theta/2)`,
`cos(theta) = (1-t^2)/(1+t^2)`,
`sin(theta) = 2t/(1+t^2)`,
and
`j_1 = -mu_1 lambda_1 (cos(theta), sin(theta))`.

Let `z = (lambda_1, lambda_2, j_21, j_22)`. The four active equations
`u_n1 = u_n2 = u_t21 = u_t22 = 0`
form a 4x4 linear system
`M(t) z = -q_active`.
Multiplying by `1+t^2` yields a polynomial matrix. Cramer's rule gives exact rational functions for z except where the active-system determinant is zero.

The remaining contact-1 sliding KKT condition is
`(-sin(theta), cos(theta)) dot u_t1 = 0`.
After clearing the common denominator and removing the nonzero factor `(1+t^2)^3`, the resulting equation is a quartic. The Rust test derives this polynomial from the exact dyadic values of every binary64 entry in W, q and mu; it does not copy the production coefficient array or parse decimal approximations.

One positive rescaling of the exact quartic has descending integer coefficients:

```text
354230366598080575799982824851724076880938481411469601482315845498400615844946703953
-178101948869788234772896573997090551849327913387647160564960521183479738480824441486803
453195116402293956003399567551845718983340011631026695060683529683403798012675209738540
596124074428065072231250961750444925355871437315446913278576446286450600372891409300947
153210118233836060880109112548202749527444549826296001149007606778391442322888031008337
```

The test uses the existing exact-rational Sturm implementation from #3175 to count distinct real roots and isolate each in a disjoint rational interval with exact non-root endpoints and a unit Sturm count. The quartic is square-free.

## Certified angular roots and physical classification

The following decimal values are rounded display values for the exact isolating intervals; the actual tests retain rational interval endpoints.

| Root | t = tan(theta/2) | theta (rad) | lambda1 | lambda2 | tau1 | lambda2^2 - ||j2||^2 | Delta K | Result |
|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | -0.618116116740922 | -1.107267570174 | 2.635473725159 | 1.028018929667 | +0.265441678569 | 1.011635089287 | -5.181481718268 | admissible |
| 2 | -0.391407757835296 | -0.746154794338 | 0.247596019815 | 1.004700183840 | +4.709435913523 | 0.997348100323 | -1.098282552488 | admissible |
| 3 | 3.573880097738590 | 2.595931474046 | 0.162331877580 | 0.996831829343 | -8.683044692876 | 0.977249537000 | -0.098468009475 | reject: tau1 < 0 |
| 4 | 500.221304363265 | 3.137594428568 | 1.952113658886 | 0.951102374802 | -1.923644844140 | 0.893378394642 | +8.452041898769 | reject: tau1 < 0 and energy increases |

For every interval, exact interval evaluation establishes:

- positive normal impulses at both contacts;
- normal complementarity and contact-2 tangential sticking equations (these are the four equations used to construct z);
- a strictly interior contact-2 Coulomb disk;
- a contact-1 sliding impulse on its Coulomb boundary by parameterization;
- a disk multiplier interval that excludes zero;
- the sign of the full-island kinetic-energy change.

The two admissible intervals have strictly positive tau1 and strictly negative Delta K. The other two have strictly negative tau1, so they violate the original sliding KKT law even though both have positive normal loads and contact 2 remains inside its disk.

## Contact-permuted slide/stick orientation

A second exact test permutes the contact coordinate blocks with
`(n_1,t_{11},t_{12}) \leftrightarrow (n_2,t_{21},t_{22})`. This preserves
the physical island and its exact SPD property while making original contact
2 the sliding contact and original contact 1 the sticking contact. The
friction coefficients are permuted with the contact roles:
`mu_sliding = 1`, `mu_sticking = 1.383289774658335`.

The independently reconstructed quartic is square-free and has exactly two
finite real roots. The singular active-system determinant again has two real
directions and is coprime with a Cramer replacement numerator. Exact rational
interval KKT evaluation classifies both real roots as inadmissible because
their sliding disk multipliers are strictly negative:

| Root | t = tan(theta/2) | theta (rad) | sliding lambda | sticking lambda | tau_sliding | Delta K | Result |
|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | -0.734574431368000 | -1.267110859726 | 1.05874557 | 6.96146185 | -1.706693969635 | -6.799397310032 | reject: tau < 0 |
| 2 | 1.38413544400421 | 1.890293451636 | 1.06435846 | 8.71957774 | -0.100329364468 | -8.604101219431 | reject: tau < 0 |

Both roots have positive normal impulses, strict sticking-disk margin and
dissipative energy, so the negative multiplier is the decisive KKT rejection.
This is useful: the reverse orientation is not dismissed because a root
solver failed to find a branch; it is excluded after isolating every real
angular root in that stratum.

Together, the two exact orientation-specific results are:

- original contact 1 sliding / original contact 2 sticking: four angular roots,
  exactly two admissible;
- original contact 2 sliding / original contact 1 sticking: two angular roots,
  zero admissible.

Neither result covers slide/slide, separating contacts, simultaneous active-set
boundaries, or uncertain parameter families.

## Singular and projective directions

The determinant of the 4x4 active linear system has two real zeros after removing the known nonzero half-angle scaling factor. At a singular angle, a compatible solution would force every Cramer replacement determinant to vanish. The exact polynomial gcd of the reduced determinant and the first replacement determinant is the constant one. Therefore neither singular direction can satisfy the active equations.

The exact quartic's leading coefficient is nonzero. Hence the angular equation does not have a root at the projective half-angle point t = infinity (theta = pi). No finite root or projective angular direction is silently omitted in this active stratum.

## Executable checks

The new test reuses the existing `BigRational`/Sturm machinery in `coupled_friction_sturm.rs` and checks:

1. The full 6x6 W is exactly symmetric and positive definite by exact leading-principal-minor (Sylvester) tests on the exact dyadic entries.
2. Independent polynomial construction reduces to a square-free quartic.
3. Exact Sturm isolation finds four finite real roots.
4. The active determinant has exactly two real singular directions and its reduced polynomial is coprime with a Cramer replacement numerator.
5. Exact rational interval arithmetic classifies the four roots as `[admissible, admissible, rejected, rejected]`.
6. The nonzero leading coefficient excludes a projective root at infinity.

The dedicated workflow runs rustfmt on the modified verifier, this exact test, the existing exact single-contact Sturm suite, and the single-contact physical regression suite.

## Claim ceiling

This is a mathematical certificate for one frozen 6x6 binary64 Delassus input and one regime stratum. It does not solve every two-contact regime, prove that the full contact problem has exactly three solutions, cover arbitrary uncertain parameter families, or establish an engine-level/SOTA result. The other regime combinations, folds/active-set boundaries and general contact islands remain open work. All final qualification evidence stays under #460.
