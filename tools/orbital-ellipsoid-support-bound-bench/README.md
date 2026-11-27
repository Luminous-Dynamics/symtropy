# Orbital directional ellipsoid support bound v0.1

This component computes an outward fixed-point upper bound for the along-track support radius of a keep-out ellipsoid:

    rho_y = (y_hat^T K y_hat)^(-1/2)

It is a narrow adapter for the first-order eccentric passive-safety branch. It does not duplicate the generic SPD, matrix conditioning, SVD or operator-norm implementation owned by Symtropy #4818.

## Contract

The input is a strictly positive rational lower enclosure:

    q_lower = n / d <= y_hat^T K y_hat

where q_lower is expressed in m^-2, the direction is unit-normalized in the exact declared frame, and the evidence providers bind the same:
- terminal action and TAI epoch;
- protected target, keep-out profile and geometry revision;
- exact K matrix SHA-256 and matrix convention;
- frame identity/revision and coordinate transform;
- along-track direction SHA-256 and convention;
- length unit and fixed-point arithmetic contract.

The matrix SPD provider, direction-unit provider and directional-quadratic-lower provider must all be current, well-formed, and bound to the same scope ID. A digest is a reference, not source-byte authentication.

## Outward conversion

The output is in micrometres. The smallest integer R satisfying

    R^2 * n >= (1_000_000)^2 * d

is returned as rho_y_upper_um.

The implementation computes:

    T = ceil((1_000_000)^2 * d / n)
    R = ceil(sqrt(T))

using checked u128 multiplication, exact integer ceiling division and a division-based integer square-root search. No floating-point square root can round the result down. The receipt includes T, floor(sqrt(T)), whether T is a perfect square, the q numerator/denominator, exact scope, matrix/direction digests and provider identities.

Only metre input units and the unit-along-track-positive-y direction convention are supported in v0.1. Unsupported units/conventions are rejected instead of being silently converted.

## 36-case synthetic corpus

The frozen corpus covers:
- unit sphere and diagonal-ellipsoid examples;
- exact square and non-square rational thresholds;
- outward ceiling at rational boundaries;
- monotonicity as q_lower increases;
- zero numerator/denominator and huge denominators;
- u128 overflow;
- malformed matrix/direction digests;
- wrong unit and direction conventions;
- provider failure, stale, revoked and unknown states;
- provider-scope mismatch and invalid evidence metadata;
- action/target/geometry/frame/transform identity in the receipt.

The independent Python oracle uses arbitrary-precision integers and an independently written binary-search square root, then emulates u128 checked limits. The Rust and Python outputs must match as complete JSON objects.

## Reproduce

    cargo fmt --manifest-path tools/orbital-ellipsoid-support-bound-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/orbital-ellipsoid-support-bound-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/orbital-ellipsoid-support-bound-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/orbital-ellipsoid-support-bound-bench/fixtures/ellipsoid-support-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/orbital-ellipsoid-support-bound-bench/Cargo.toml --bin orbital-ellipsoid-support-bound-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/orbital-ellipsoid-support-bound-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

## Claim boundary

A BOUND_READY result establishes only the inverse-square-root rounding implication of the supplied rational q_lower and evidence references. It does not derive q_lower from raw K and y_hat, independently prove K positive definite, prove direction normalization, validate the geometry or authenticate source bytes. It does not establish orbital collision safety, passive safety, flight readiness or maneuver authority.

The receipt hard-codes source_bytes_authenticated=false, physical_geometry_validated=false, and authority_granted=false.

Research parent: Symtropy #7697. Downstream consumer: #7687/#7698. Generic matrix/SPD machinery: #4818.
