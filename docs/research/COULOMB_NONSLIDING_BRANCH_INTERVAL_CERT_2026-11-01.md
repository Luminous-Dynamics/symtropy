# Separating and sticking branch interval certificates

**Research issue:** [PHYS-CONTACT-REGIME-INTERVAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/3191)  
**Parent:** [interval sliding branch certificates, PR #3182](https://github.com/Luminous-Dynamics/symtropy/pull/3182)  
**Shared authority:** [PHYS-VALID-06, issue #460](https://github.com/Luminous-Dynamics/symtropy/issues/460)

## Scope

This increment adds test-only exact-rational interval verifiers for two single-contact regimes that are separate from the sliding quartic: zero-impulse separation and unconstrained sticking. It complements but does not replace the sliding branch certificate from #3182.

The verifier works over the existing ContactInputBox of exact dyadic intervals for the independent symmetric entries of the 3x3 Delassus matrix W, free velocity q, and friction coefficient mu.

## Separating branch

For p=0, tangential impulse is zero and the normal post-impulse velocity is q_n. The branch is admissible over the entire box exactly when the box proves q_n >= 0. A q_n box crossing zero is unresolved; q_n strictly negative throughout proves only that the separating branch is inadmissible. It does not rule out an active branch.

The verifier also requires the existing conservative Gershgorin lower bound to prove SPD for the declared model family and rejects negative friction intervals.

## Sticking branch

The unconstrained full-contact impulse is p = -W^-1 q. The implementation computes the symmetric 3x3 adjugate and determinant using exact rational interval arithmetic. It requires both the interval Gershgorin lower bound and a strictly positive interval determinant before dividing.

For the candidate to be certified as sticking throughout the box:
- the normal impulse interval has a strictly positive lower bound;
- the squared Coulomb disk margin (mu*lambda_n)^2 - j_1^2 - j_2^2 has a strictly positive lower bound;
- independently reconstructed post-contact velocity q + Wp encloses zero in all components;
- the full impulse energy interval p^T q + 0.5 p^T Wp has an upper bound no greater than zero.

No square-root or floating tolerance is used for the disk test. An exact stick/slip boundary or a box that may cross it remains inconclusive. Strictly outside-disk candidates are rejected as sticking; they may still belong to a sliding branch.

## Frozen tests

- Uniformly separating/grazing boxes and a strictly negative q_n box that rejects the separating branch.
- A separating box that crosses q_n=0 remains inconclusive.
- A narrow SPD sticking box certifies positive normal load, strict disk interior, zero-compatible post velocity, and non-positive island energy.
- An exact stick/slip-boundary point remains inconclusive.
- A sticking normal-impulse box that crosses zero remains inconclusive.
- A strictly outside-disk unconstrained impulse is rejected as sticking.
- Ninety-six deterministic exact-dyadic point inputs are checked against the enclosing interval outputs for normal impulse, squared disk margin, and energy.
- Failure to prove SPD returns inconclusive for both non-sliding verifiers.

## Mathematical claim boundary

These functions certify the existence/admissibility of an individual separating or strictly interior sticking branch over the declared single-contact W/q/mu box. They do not prove that these are the only branches, that the sliding branch set is complete, or that all regimes have been exhaustively classified. They do not yet certify body-to-W assembly, multi-contact islands, uncertain geometry or trajectories.

All work remains under the existing evidence authority #460. It is not a production solver or SOTA claim.

## Validation

Formatting, compilation and Rust tests are delegated to the exact-head GitHub Actions workflow for this commit. No local Rust test pass is claimed.
