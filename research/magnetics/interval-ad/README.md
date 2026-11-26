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