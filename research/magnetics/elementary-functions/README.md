# Magnetic elementary-function qualification (v0.1)

This isolated Rust crate evaluates `scioxide-validated = 0.1.0` as a candidate provider for the elementary interval functions needed by Symtropy's magnetic design and encoder-angle work.

It is a **qualification harness**, not a production backend switch. The existing `magnetic-interval-ad` kernel remains the independent algebraic interval/AD reference.

## Why this candidate

The provider publishes decorated interval arithmetic and elementary enclosures for `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `hypot`, and more. Its `ElementaryEnclosure` preserves up to two disjoint pieces and exposes domain status and whether gaps may have been filled. Its documented trigonometric argument reduction is Payne–Hanek.

That is a better starting point than writing argument reduction and polynomial error bounds from scratch. It is still an external dependency whose source, assumptions, floating-point environment, target support and output semantics must be independently qualified.

## Semantics retained by this harness

- Do not collapse disjoint pieces into a hull and then forget the gap. The probe exposes piece count and `gaps_filled`.
- Do not treat `PartiallyDefined` or `Inconclusive` as a fully defined function range.
- A tangent interval crossing one pole has two pieces and a partial-domain status; more complicated pole cases can degrade to an entire/inconclusive range.
- `atan2` crossing the negative real axis has two pieces near -π and +π; boxes containing the origin require explicit partial-domain treatment.
- `asin`/`acos` boxes partly outside [-1,1] must retain their partial-domain status.
- Backend errors, unsupported FPU state and numerical-instability outcomes remain errors.

## Floating-point environment gate

The provider's documentation requires round-to-nearest-even, gradual underflow (no flush-to-zero), masked floating-point exceptions, and x86-64, x86 with SSE2, or AArch64. It checks the environment around numerical operations/callbacks. Unsupported targets or FPU configurations return an error.

The three-platform CI matrix is therefore a real compatibility gate, not just a source-format check. A platform pass is evidence for that runner's tested configuration only; it does not prove every deployment environment satisfies the FPU assumptions.

## Authored adversarial probe cases

- sin interval containing π/2 must enclose +1;
- cos interval containing zero must enclose +1;
- tan interval containing π/2 must remain partial with two pieces;
- atan2 box crossing the negative real axis must retain the two angular pieces;
- atan2 box containing the origin must not be labelled fully defined;
- asin box partly outside [-1,1] must remain partial;
- invalid infinite interval endpoints must be rejected.

These tests exercise the provider's documented API/semantics. They are not an independent proof of its implementation. A later independent harness should compare exact point enclosures to CRlibm directed-rounding results and interval images to MPFR/Arb references.

## Integration decision

Do not route Symtropy production/design certificates through this provider until:
1. exact crate source/version/license/dependency graph is frozen;
2. the Linux/macOS/Windows matrix passes on exact heads;
3. point values are independently checked against correctly rounded references;
4. periodic extrema, branch cuts, partial domains, gap-filling and large arguments are tested;
5. derivative rules are separately validated over whole boxes;
6. the FPU environment assumptions are checked at the execution boundary and included in the receipt.

CRlibm is retained as a scalar directed-rounding comparison candidate, not an interval-image replacement. Correctly rounded point functions do not, on their own, establish interval range semantics.

## Claim ceiling

This probe demonstrates how to call and inspect the candidate library. It does not establish that its outputs are safe for Symtropy certification, add interval derivatives, qualify magnetic physical models, or authorize a production backend switch.
