# BHPT Teukolsky radial fixture producer

This is an external reference-data adapter. It runs in Wolfram Mathematica with the Black Hole Perturbation Toolkit (BHPT) paclets; it is not production Symtropy code.

## Reference APIs

- BHPT Teukolsky docs: https://bhptoolkit.org/modules/teukolsky/
- BHPT SpinWeightedSpheroidalHarmonics docs: https://bhptoolkit.org/modules/spinweightedspheroidalharmonics/
- BHPT convention definitions: https://bhptoolkit.org/conventions.html

The exporter uses documented calls:

- `TeukolskyRadial[s, ell, m, a, omega]`, followed by the returned object's `In` and `Up` radial functions;
- `SpinWeightedSpheroidalEigenvalue[s, ell, m, a omega]`.

BHPT names the spheroidal eigenvalue `lambda` and uses it in the radial Teukolsky equation. The exporter therefore writes `angular_eigenvalue_convention = teukolsky_radial_lambda`. It does not relabel this value as a generic spheroidal `A`.

## Run

1. Install and pin the BHPT `Teukolsky` and `SpinWeightedSpheroidalHarmonics` paclets. Record the exact versions/revisions used.
2. Open the `.wl` file in Mathematica after loading those paclets.
3. Call `BHPTExportTeukolskyRadialFixture[config]` with the required fields shown in the example at the bottom of the source file.
4. Run the Rust validator on the emitted file:

```sh
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- validate-radial bhpt-s-2-l2-m2-a03-w05.json
```

The exporter requires an explicit `FourierConvention` and `AngularBranchId`. Verify these against the exact package version and the branch-continuation path used; the sample values are illustrative, not an assertion that every engine/configuration uses that convention.

## Important scope and limitations

- The exporter emits homogeneous radial `In`/`Up` samples and the associated Teukolsky radial `lambda`. It does not emit QNM frequencies, source amplitudes, point-particle fluxes, a retarded sourced waveform, or QNM family labels.
- Radial derivatives are taken from the returned numerical functions using Mathematica's `Derivative[1]`. This path must be checked on the exact installed paclet/runtime before treating the resulting fixture as reference evidence.
- The package's native `In`/`Up` amplitude normalizations are preserved; no normalization fitting is applied.
- Precision bits are supplied by the caller and must reflect the precision actually used by the paclets. Supplying arbitrary-precision metadata does not promote machine-precision calculations.
- The producer script hash is recorded when Mathematica executes the file directly. If the function is copied into a notebook, the hash is absent and must be supplied/recorded through the external provenance process.
- The adapter source has not been run against a live Mathematica/BHPT installation in this environment. The emitted file must pass the Rust schema validator and be independently reviewed before entering the golden corpus.

Do not use this exporter as a substitute for the separate QNM sequence exporter or the source/flux reference pipeline.