# Orbital Truss Surface-Support Envelope Certificate v0.1

**Status:** executable synthetic envelope contract; occlusion remains NOT_ASSESSED  
**Implementation owner:** #7136  
**Related centre-sampled reproduction:** #7002  
**Claim ceiling:** idealized geometric range/FOV/incidence/GSD envelope only

## 1. Paradigm shift

A voxel-centre ray is not a certificate for the entire feature support. Treat the support region as a first-class geometric object with stable identity, a proven enclosure and a separately justified normal cone. The first implementation certifies only the sensing envelope; it deliberately does not claim full visibility because support-wide occlusion is not yet assessed.

## 2. Conservative bounds

For support S contained in a proven ball B(c,r), sensor position p, centre distance d0=||p-c||, and d0>r:

- minimum range lower bound: d_min=d0-r;
- maximum range upper bound: d_max=d0+r;
- ray-direction spread: delta=asin(r/d0);
- centre FOV angle eta0=acos(a dot (c-p)/d0), with bounds [max(0,eta0-delta), eta0+delta];
- centre incidence angle alpha0=acos(n dot (p-c)/d0); for a input-asserted normal cone of half-angle epsilon_n, bounds are [max(0,alpha0-delta-epsilon_n), alpha0+delta+epsilon_n];
- GSD lower/upper bounds, when incidence bounds are in [0,pi/2):

  GSD_lower = [2 tan(theta/2)/w] * d_min / cos(alpha_min)

  GSD_upper = [2 tan(theta/2)/w] * d_max / cos(alpha_max)

The implementation uses the paper's GSD model but evaluates a conservative range/angle envelope over the support enclosure. Results are fixed-point integers (nm, nrad, pm/pixel) for byte-stable cross-language comparison.

A whole-support envelope pass requires both support and normal-cone bounds to be asserted, the full distance interval within range, the FOV upper bound inside half-FOV, the incidence upper bound below maximum obliquity and 90 degrees, and the GSD upper bound below the declared limit.

A whole-support failure is reported only when a conservative bound proves every point in the support violates at least one admissible constraint. If an upper bound merely crosses a limit, the result is INDETERMINATE, not an invented failure.

## 3. Outcome semantics

- CERTIFIED_WITHIN_SENSING_ENVELOPE: the entire declared support is bounded inside range, FOV, incidence and GSD constraints.
- PROVEN_OUTSIDE_SENSING_ENVELOPE: a conservative bound proves the whole support violates a constraint.
- INDETERMINATE: support-bound assertion, normal-cone assertion or interval margins are insufficient to certify either side.
- INVALID_INPUT: malformed identifiers, frame, units, vectors, camera model or limits.

Every result is occlusion_status=NOT_ASSESSED. No result can be interpreted as full support visibility, and no result can count toward full-support coverage. The follow-up must add a conservative support-ray/occlusion certificate with explicit tangency, blocker, target-cell and partial-support semantics.

## 4. Frozen corpus

The 13 controls are:
1. whole support inside the sensing envelope;
2. whole support too far;
3. whole support too near;
4. whole support outside FOV;
5. FOV upper-bound indeterminacy;
6. incidence upper-bound indeterminacy;
7. whole support incidence failure;
8. GSD upper-bound indeterminacy;
9. whole support GSD failure;
10. support enclosure not asserted;
11. normal cone not asserted;
12. sensor lies inside the support enclosure;
13. malformed non-unit optical axis.

The corpus is replayed by a Rust CLI and an independently authored Python standard-library oracle. Tests also check rigid-frame invariance, support-radius monotonicity, normal-cone monotonicity and the prohibition on full-visibility claims while occlusion is unassessed.

## 5. Metrics and next extension

Report zero false envelope certificates on frozen controls, indeterminate fraction, proven-outside fraction, GSD-bound tightness, FOV/incidence margins and runtime. The next extension is conservative occlusion volume/cone reasoning checked against dense patch sampling and independent ray casting. Dense sampling is a comparison oracle, not a proof of full-support visibility.

Physical calibration, defect findability, NDE/POD, route reachability and structural acceptance remain outside this contract.


**Input-precondition limitation:** enclosing_ball_asserted=true and normal_cone_asserted=true are caller assertions. This evaluator validates the provided numeric parameters but does not independently prove that the ball encloses the support or that the normal cone encloses every surface normal. Every certificate therefore reports precondition_proofs_verified_independently=false. A future evidence adapter must bind each assertion to a separately verified geometry/support proof before any stronger claim is made.
