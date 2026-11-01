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
