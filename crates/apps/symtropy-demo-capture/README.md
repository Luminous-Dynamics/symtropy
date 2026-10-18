# Symtropy visual capture and regression metrics

This package contains two related tools:

- `CapturePlugin` creates PNG screenshots and receipt JSON from a Bevy app.
- `symtropy-visual-diff` compares two receipt-bound PNGs using explicit regression budgets.

The receipt validates artifact bytes and binds them to caller-declared metadata. The diff tool requires both receipts to be complete and metadata-qualified, then verifies that the selected PNG hashes and dimensions are actually listed by their corresponding receipts.

Neither tool establishes physical correctness, hardware attestation, source authenticity, or perceptual quality. The diff metrics are raw RGBA8 sRGB code-value differences, not a human-perception model.

## Comparison example

Capture both versions of the same frozen fixture with identical:
- fixture and asset manifest digest;
- simulation seed and tick;
- quality profile, camera/view identity and canonical render-settings digest;
- GPU adapter, driver, backend, output dimensions, OS and Rust toolchain.

Use different source commits and renderer-version identifiers as needed.

```sh
cargo run --release -p symtropy-demo-capture --bin symtropy-visual-diff -- \
  --baseline target/baseline/scene_t1.5.png \
  --baseline-receipt target/baseline/scene.capture-receipt.json \
  --candidate target/candidate/scene_t1.5.png \
  --candidate-receipt target/candidate/scene.capture-receipt.json \
  --output target/diff/scene.json \
  --diff-image target/diff/scene-heatmap.png \
  --diff-image target/diff/scene-heatmap.png \
  --pixel-threshold 8 \
  --max-mean-channel-delta 1.0 \
  --max-changed-pixel-fraction 0.01 \
  --max-p95-pixel-delta 8
```

The example thresholds are illustrative policy inputs, not validated recommendations. Choose budgets after measuring repeat captures on the target adapter. Keep thresholds and the report in version control or bind them to the benchmark suite when using CI gates.

For a Bevy migration comparison only, add `--allow-renderer-version-change`. It allows the renderer version label to differ; it does not permit mismatched fixture, camera, settings digest, resolution, adapter, driver, render backend, OS, or Rust version.

## Metrics

The JSON report contains:
- SHA-256 of baseline and candidate PNG byte streams, validated against their receipts;
- optional difference heatmap path, byte digest and dimensions when `--diff-image` is supplied;
- optional difference heatmap path, byte digest and dimensions when `--diff-image` is supplied;
- image dimensions and the distinct source commit / renderer version for each image;
- receipt context that had to match for a comparison to be considered valid;
- mean absolute and root-mean-square RGBA channel delta in 8-bit code values;
- 95th percentile of each pixel's maximum RGBA channel delta;
- number and fraction of pixels whose maximum RGBA channel delta exceeds the configured pixel threshold;
- the exact user-declared budget and whether all three conditions passed.

Exit codes:
- `0`: all metrics are within the declared budget;
- `1`: valid, comparable inputs, but at least one metric exceeds budget (the report is still written);
- `2`: invalid or incomparable inputs, invalid receipt or image, or report-write failure.

The verifier rejects incomplete/unqualified receipts, missing or mismatched metadata, image bytes not recorded by the receipt, invalid PNGs, mismatched dimensions, oversized artifacts, and images above the decoded-pixel limit. It refuses to write a report over one of the input artifacts.

## Important interpretation limits

- Cross-GPU byte-level equality is not a fair default. Run strict visual regression on a pinned adapter/backend; use separate baselines and explicitly reviewed budgets for other adapters.
- A threshold pass only says the computed code-value differences fit the chosen limits. It does not establish that the candidate looks better.
- A changed-pixel fraction is not perceptually weighted and may overreact to antialiasing, temporal noise, or small geometry changes. Keep it alongside mean and tail metrics, and add perceptual metrics only after independently validating their failure modes.
- Receipts contain caller-declared GPU, driver, fixture and render settings metadata. The tool checks its syntax and equality, not whether the caller told the truth.
