# Contact-graph separator response messages — 2026-11-01

## Scope

This note records the next step after the body-generated three-contact star witness in [PR #3281](https://github.com/Luminous-Dynamics/symtropy/pull/3281): represent each eliminated leaf as a regime-conditioned response message over the separator impulse.

The initial implementation is test-only and handles strict sticking leaves with symmetric positive-definite 3x3 self-blocks. It does not implement general graph decomposition, interval response regions, sliding-message root sets, or rank-deficient fibres.

## Message model

Partition a contact island into separator variables p_S and an interior leaf p_L:

\[
W=\begin{bmatrix}C&B\\B^T&A\end{bmatrix},\qquad
q=\begin{bmatrix}q_S\\q_L\end{bmatrix}.
\]

For a strictly sticking interior contact:

\[
u_L=q_L+B^Tp_S+Ap_L=0.
\]

The leaf response is therefore the affine map

\[
p_L(p_S)=-A^{-1}(q_L+B^Tp_S).
\]

The response exists as an admissible sticking message only on the subset of separator impulses where:

\[
p_{n,L}>0,\qquad
\mu_Lp_{n,L}-\|p_{t,L}\|>0,\qquad
u_L=0.
\]

The current prototype evaluates this partial response map at one separator impulse and returns the impulse, post-contact residual, and strict disk margin. A later interval implementation should enclose the separator-domain region itself and preserve its boundary.

The Schur interface contribution is

\[
W_{\rm eff}=C-BA^{-1}B^T,\qquad
q_{\rm eff}=q_S-BA^{-1}q_L.
\]

The implementation uses a fixed-size Cholesky solve for SPD A; it does not form an explicit inverse. Rank-deficient A must be represented by a compatible multiplier fibre in a separate rank-aware message type, never silently pseudoinverted.

## Current executable qualification

The four-body/three-contact star fixture has A, B, C, D as four translational bodies with mass 2I:
- separator contact (B,C);
- leaf 1 (A,B);
- leaf 2 (C,D).

The 9x12 Jacobian and 12x12 inverse mass produce W=JM^-1J^T, while free contact velocity is q=Jv_free. The leaf-to-leaf contact block is structurally zero because those contacts share no body. The separator-to-leaf blocks are -1/2 I.

Each leaf is represented by a StickingLeafMessage. For separator impulse p_S=(1,-1/2,0), both messages return leaf impulse (1,0,0), zero leaf velocity, and strict disk margin 1/2. Their Schur composition gives W_eff=1/2 I and q_eff=(-1/2,3/4,0). The separator solves as a strictly sliding branch with tau=1; the original full-island energy change is -7/8.

A separate message-level unit test uses non-identity A and non-isotropic B, checks the block-Schur terms, and checks the pointwise response. Boundary-disk and non-SPD self-block controls are rejected.

## Message composition rules

1. A message is tied to a declared contact ordering, local law, friction coefficient, block data, and input digest.
2. Messages expose a response relation, not a preferred or nearest impulse. Every admissible response branch must be retained.
3. Message domains combine by intersection on separator variables. Empty intersections are certified only if every local exclusion is proved; otherwise status is INCONCLUSIVE.
4. The reduced interface law is composed from Schur terms only on patches where each eliminated regime is valid.
5. Every reconstructed global candidate is verified against the original unreduced W,q, all contact-wise normal/KKT/disk conditions, and full-island energy.
6. Contact permutation, separator ordering and equivalent block-coordinate transformations must preserve physical outcomes and response-set identity.
7. Typed message results should eventually distinguish COMPLETE_FOR_DECLARED_DOMAIN, PARTIAL, BOUNDARY_ONLY, and INCONCLUSIVE.

## Next implementation steps

- Replace pointwise response checks with interval boxes over separator impulse and certify the strict sticking domain.
- Compose three or more messages and verify that branch count / coverage matches a brute-force full-island reference on small fixtures.
- Add a leaf with multiple admissible local branches and ensure the message retains all branches.
- Add regime-conditioned separating leaves.
- Add rank-deficient leaf blocks with multiplier-fibre and unique physical-response semantics.
- Add balanced/unbalanced tree decompositions and measure separator width, fill-in, message count, and branch joins.
- Test local message generators from exact-Coulomb FBF / SOCCP candidate methods, with the independent verifier remaining separate.

## Claims not yet established

This does not establish a treewidth complexity theorem, an arbitrary-N contact solver, global branch completeness, a continuous-time trajectory certificate, or a performance win over production engines. Dense graphs and large separators may be slower than global solving. The current witness only demonstrates one body-generated star branch and its pointwise sticking-leaf messages.


## Separating-leaf response message — 2026-11-02

The next test-only increment [#3286](https://github.com/Luminous-Dynamics/symtropy/pull/3286) adds a regime-conditioned separating response:

```text
p_L = 0
u_L(p_S) = q_L + B^T p_S
```

The message has exactly zero Schur operator and zero free-velocity correction because its impulse is identically zero. It still retains the full local post-velocity, including tangential components. The zero-radius Coulomb disk forces the tangential impulse to zero; tangential velocity need not vanish.

A typed pointwise classification distinguishes:

- `StrictlySeparating`: the normal velocity is positive beyond the declared tolerance;
- `BoundaryWithinTolerance`: the normal velocity is zero or cannot be separated from zero at the pointwise tolerance; this is not a strict branch;
- `SignoriniViolation`: the zero-impulse hypothesis produces strictly negative normal velocity.

A Signorini violation only rejects the separating message at that separator impulse. It does not establish that the full island has no solution; another local regime or another separator branch may remain admissible.

The new mixed-regime fixture derives J, M^-1, W and q from the same four-body chain and body free-velocity vector. One leaf is strictly sticking (the affine message above), one leaf is strictly separating (zero impulse), and the separator is strictly sliding. It replays the original 9x9 contact equations, then independently reconstructs body post-velocity as

[
v^+=v^-+M^{-1}J^Tp
]

and checks (Jv^+) against the contact-space velocity. The frozen energy change is (-1/3).

### Important limitation

This remains a floating-point point-query prototype. The tolerance-based boundary label is not an interval proof. The next sound extension is a box classifier that returns strict separation only when the entire interval enclosure of (u_n) is positive, Signorini violation only when the entire enclosure is negative, and boundary/inconclusive when it intersects zero. A box touching zero must not be promoted to a strict regime.


## Outward-enclosed separator boxes — 2026-11-02

The follow-on [#3288](https://github.com/Luminous-Dynamics/symtropy/pull/3288) extends the pointwise separating message to axis-aligned boxes of separator impulses.

For the affine normal response

[
u_{n,L}(p_S)=q_{n,L}+b_{n,L}^{T}p_S,
]

the prototype evaluates the full box image using conservative binary64 interval addition and multiplication. Each operation widens the rounded endpoint by one adjacent representable value. A result is:

- `StrictlySeparating` only if the complete normal-velocity enclosure has lower bound greater than zero;
- `SignoriniViolation` only if the complete enclosure has upper bound less than zero;
- `BoundaryOrInconclusive` whenever zero is included.

The message also retains interval enclosures for tangential post-velocity, with its impulse fixed at zero. Overflow, non-finite values, and malformed/inverted boxes return errors rather than guessed signs.

The tests use dyadic boxes whose exact normal-velocity ranges are known:
- separator normal impulse in [1, 5/4] gives [3/8, 1/2];
- in [3, 13/4] gives [-5/8, -1/2];
- in [15/8, 17/8] crosses zero;
- the zero-radius box at 2 lies exactly on the transition and remains boundary/inconclusive.

This is a conservative pointwise-box certificate for binary64 inputs interpreted as exact represented values. It is not a correlated uncertainty model for geometry, mass or friction parameters, and it is not a global contact-island completeness proof.


## Outward-enclosed sticking response boxes — 2026-11-02

The follow-on [#3290](https://github.com/Luminous-Dynamics/symtropy/pull/3290) adds an interval affine response for strict-sticking leaves. Unlike a pointwise Cholesky response, it encloses the exact solution of the represented binary64 3x3 self-block and right-hand sides using interval Gaussian elimination.

For
[
A p_L = -(q_L+B^Tp_S),
]
the implementation:
- requires exact symmetry of the represented A matrix;
- checks every unpivoted interval pivot is strictly positive (the no-pivot path is valid for SPD A);
- computes interval enclosures for (a=-A^{-1}q_L) and (K=-A^{-1}B^T);
- propagates these coefficients over a separator impulse box;
- replays the original local velocity equation as an interval and requires every component enclosure to contain zero;
- bounds the complete Coulomb margin (mu p_{n,L}-|p_{t,L}|).

It returns strict sticking only when the whole normal-load and disk-margin enclosures are strictly positive. A box wholly outside the normal/disk domain is reported as outside this *local regime hypothesis*; this is not a global no-solution result. Boxes touching a boundary or unresolved pivots remain boundary/inconclusive or errors.

The frozen nonidentity message uses (A=operatorname{diag}(2,1,1)), (B=operatorname{diag}(-1/2,1/4,1/4)), (q_L=(-3/2,1/8,0)), (mu=1/2), with exact affine centre (a=(3/4,-1/8,0)), (K=operatorname{diag}(1/4,-1/4,-1/4)). The tests include a strict nonzero-width box, an exact zero-radius Coulomb boundary, an outside-disk box, a normal-load crossing box, and malformed/non-SPD/singular-pivot controls.

This closes an important architectural gap in the interval separator messages: both strict sticking and strict separation now have a route to whole-box regime certification, while sliding still needs direction-root/branch enclosures. It does not yet compose messages into a globally complete tree-decomposition proof or propagate correlated geometry/mass/friction uncertainty.


### Dyadic oracle details for the sticking-box lane

The strict interior separator box now uses dyadic endpoints:
- normal separator impulse ([7/8,9/8]), giving exact normal-load image ([31/32,33/32]);
- first tangent separator impulse ([-1/2,-3/8]);
- second tangent separator impulse ([-1/8,1/8]).

The exact affine impulse centre/map is (a=(3/4,-1/8,0)), (K=operatorname{diag}(1/4,-1/4,-1/4)). The tangent components lie in ([-1/32,0]) and ([-1/32,1/32]); the exact normal load stays strictly positive and the entire Coulomb disk margin has a strict positive lower bound. Boundary and outside-disk controls use dyadic separator coordinates 3/2 and 13/8 respectively. This keeps the frozen local oracle rational without decimal-to-rational reconstruction ambiguity.


## Coverage-preserving separator regime partitions — 2026-11-02

The next increment [#3294](https://github.com/Luminous-Dynamics/symtropy/pull/3294) adds deterministic bounded subdivision for separator boxes that cross the affine separating boundary.

For the normal response (u_{n,L}=q_{n,L}+b_{n,L}^Tp_S), each node is classified with the outward interval oracle. Strict-positive and strict-negative boxes are retained as certified local-regime cells. Boundary/inconclusive boxes are bisected along a deterministic axis with nonzero normal-response coefficient. The shared split face is retained in both closed children; their union therefore contains the entire parent without gaps.

Every leaf stores its exact binary64 bounds, depth and binary lineage. The independent coverage verifier reconstructs the deterministic split tree from the root box and message, checks that every leaf matches its reconstructed bounds, rejects duplicate or ancestor/descendant paths, and requires descendants in both children of every split node. The negative control drops a child and must fail the verifier.

Depth, node-budget, and no-representable-midpoint limits produce retained unresolved frontier cells. These remain part of the cover; they are not silently dropped and do not become NO-SOLUTION claims. The partition reports coverage preservation separately from whether every cell received a strict regime classification.

The frozen dyadic case (u_n=1-p_{S,n}/2), (p_{S,n}in[1,3]), crosses the transition at (p_{S,n}=2). A depth-3 partition retains two strict-separating cells, two strict Signorini-defeating cells for the separating hypothesis, and two boundary cells adjacent to the transition.
