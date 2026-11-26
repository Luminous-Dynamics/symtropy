# Magnetic interval automatic differentiation (v0.1)

A zero-dependency Rust reference for evaluating algebraic/rational expression graphs over a bounded parameter box and enclosing every first partial derivative over that entire box.

## Why it exists

The spatial certifier (#7642 / #7639) requires derivative absolute supremum bounds that hold throughout each exact cell. Pointwise automatic differentiation, finite differences and sampled maximum gradients do not provide that guarantee.

This crate implements interval forward-mode automatic differentiation (IAD). Each input parameter is an interval. Its dual-number derivative is itself an interval. The ordinary forward-mode sum/product/quotient/chain rules are evaluated with outward-rounded interval arithmetic, yielding derivative enclosures for every point in the Cartesian input box.

The method is established validated-numerics mathematics, not a new invention. This is a Rust-first bridge between interval AD evidence and the magnetic spatial-certification workflow.

## Validated arithmetic subset

v0.1 deliberately implements:
- IEEE-754 binary64 intervals with outward expansion around each basic arithmetic result;
- addition, subtraction, multiplication, reciprocal and division;
- square root with a checked real-domain condition; a box straddling the negative domain is rejected rather than clipped;
- a dedicated interval-square operation avoids the dependency overestimate of X*X when X crosses zero, and the dual-number square uses the exact chain rule 2X·X';
- forward-mode interval dual numbers with add, subtract, multiply, divide, negate, integer powers and square root.

It intentionally does **not** call platform `sin`, `cos`, `log`, `atan`, or `atan2` and call them rigorous. These require a separately validated elementary-function implementation with proven range reduction and rounding error. The first magnetic application target is algebraic/rational dipole and multipole kernels, plus square-root geometry, where the supported expression can be written without those functions.

## Result semantics

For parameter intervals X_i and expression f, IAD returns an interval F(X) and derivative intervals D_i(X) intended to enclose f and each partial derivative for every input in the complete Cartesian box.

The exported absolute derivative supremum is:

    L_i = max(abs(inf(D_i)), abs(sup(D_i)))

No finite supremum is emitted if a derivative enclosure is unbounded. Division by an interval containing zero gives an unbounded enclosure. Square-root domain errors return an explicit error; a square-root box touching zero yields unbounded derivative enclosures because the derivative may be singular there.

The output binds the expression, parameter order/units/box, cell ID, input digest, source/artifact/environment identities, frame, operating condition, model set, geometry/material and functional requirement. Those identities are provenance links, not cryptographic authentication.

## Rounding model

The reference core assumes ordinary Rust binary64 arithmetic uses IEEE-754 round-to-nearest semantics without fast-math transformations. For finite arithmetic endpoints it moves one representable value outward after each primitive operation. Exact zero products and exact zero additions are preserved where the mathematical result is exactly zero. Overflow and indeterminate extended-real endpoint cases become unbounded intervals.

The implementation is a research reference, not an IEEE 1788 conformance claim, a proof-certified compiler, or a formally verified implementation. Its enclosure contract must be independently challenged before product qualification. The deterministic tests are necessary controls, not a proof of all possible floating-point behaviour.

## Handoff to continuous spatial certification (#7639 / #7642)

For a direct mapping into `CellMarginEvidence`, require all of the following:

- IAD `cell_id` equals the spatial cell ID, and its parameter bounds are the exact same cell box.
- IAD frame, operating condition, model set, solver run, geometry/material digests and requirement ID equal the spatial evidence scope.
- Parameter order is explicitly mapped to spatial axes. For the current XYZ spatial contract use parameter IDs `x,y,z` with length units of metres, or provide a separately reviewed axis permutation mapping.
- The IAD derivative suprema are in the same output-units-per-metre expected by the spatial margin contract. A unit mismatch is a hard rejection, not an implicit conversion.
- The IAD record ID is retained as the source for the three derivative-bound record identities; the spatial record must preserve the exact derivative component association (X/Y/Z).

**Do not use `value_enclosure` as the centre margin interval.** It encloses the expression over the complete parameter box. The spatial certifier requires an independent centre sample or separately qualified centre oracle, with its own sample ID, digest and measurement uncertainty, at the exact box centre. These are different mathematical objects.

The bridge is valid only when the derivative expression is the derivative of the exact scalar functional margin required by the spatial certifier. Field-component derivatives or derivatives of an unrelated objective cannot be passed off as margin derivatives.

If the IAD result is unbounded, outside the supported expression domain, has an identity mismatch, or cannot establish the required units, the spatial cell remains unresolved.

## Centred mean-value enclosure

`certify_expression_centered` adds a second, independently valid enclosure of the same expression over the same box:

    f(X) ⊆ f(c) + Σ_i D_i(X) · (X_i - c_i)

where c is the representable box centre and D_i(X) are the interval partial derivatives already evaluated over the complete box. The implementation intersects this enclosure with the natural interval extension:

    F_tight(X) = F_natural(X) ∩ F_mean_value(X)

If either enclosure is invalid, non-finite or the intersection is empty, certification fails closed. The natural enclosure is retained separately in the result, and the reported enclosure method is explicit. The intersection cannot be wider than either valid enclosure.

This can reduce dependency overestimation for repeated-variable expressions (for example x-x over a wide interval), but it does not solve every dependency problem. The derivative intervals may themselves be wide, and the method requires a differentiable expression over the complete convex box. Singular or nonsmooth boundaries remain unresolved.

## Dependency overestimation

Plain interval AD can be conservative. Repeated appearances of the same variable lose dependency information (the classic interval dependency problem). v0.1 favours sound enclosure over tightness. Follow-on work should compare:
- ordinary interval forward-mode AD;
- centred/mean-value forms using interval derivatives;
- Taylor models with interval remainders;
- adaptive box subdivision.

Never repair an over-wide enclosure by replacing it with a sampled gradient or by dropping difficult corners. A box is either finitely enclosed, or it stays unresolved.

## Benchmark plan

On identical algebraic/rational magnetic kernels and parameter boxes, compare:
1. sampled pointwise AD maxima (heuristic baseline, not a certificate);
2. interval forward-mode AD;
3. centred/mean-value enclosures;
4. Taylor-model enclosures with matched order and remainder contract.

Report enclosure validity against analytic derivative ranges, derivative interval width, overestimation ratio, finite/inconclusive cell rate, subdivision count, runtime, allocations and memory. First target: **zero false finite bounds** on all deterministic challenge cases. Optimize enclosure width and speed only after soundness controls survive.

## Claim ceiling

This crate validates supplied algebraic expression graphs over parameter boxes for its supported operations. It does not execute Magpylib/JAX/PyTorch/FEM, validate the physical constitutive model, authenticate field-map provenance, prove a real encoder margin, estimate manufacturing yield or replace independent physical metrology.