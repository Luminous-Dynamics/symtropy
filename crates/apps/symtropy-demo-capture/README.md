# symtropy-demo-capture

A small capture scheduler shared by Symtropy Bevy demos. With
SYMTROPY_DEMO_CAPTURE_DIR unset, CapturePlugin is a no-op. When enabled, it
schedules PNG screenshots and writes a sibling versioned receipt:

- <stem>_t1.5.png, <stem>_t4.0.png, ... — captured screenshots
- <stem>.capture-receipt.json — byte hashes, PNG decode checks, dimensions, and declared run provenance

Each image is capped at 256 MiB and 20 million decoded pixels. Known oversized files
are rejected before being read into memory, oversized image dimensions are rejected
before pixel allocation, and files that grow past the byte cap during reading are
rejected as well. Hashing and decoding operate on the same bounded byte buffer.

The receipt distinguishes two separate results:

- capture_status: complete only if every expected path exists, has a PNG signature,
  decodes successfully, and matches declared dimensions when dimensions are provided.
- qualification_status: qualified_metadata only if source, fixture digest, seed/tick,
  quality profile, camera/view identity, renderer version, render-settings digest,
  adapter, driver/backend, dimensions, OS, and toolchain metadata are present and valid.

A complete capture with incomplete metadata can still exit successfully: it is a
valid file capture but must not be presented as a qualified performance comparison.
A missing, unreadable, invalid, undecodable, or dimension-mismatched image—or a
failed receipt write—produces a non-zero process exit. The receipt's claim ceiling
explicitly excludes visual quality, physical correctness, and authenticity of the
caller-declared metadata.

## Enable a capture

Set the output directory in the environment:

```sh
export SYMTROPY_DEMO_CAPTURE_DIR=target/render-captures
```

Then register the plugin in the selected demo app:

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(MyDemoPlugin)
    .add_plugins(symtropy_demo_capture::CapturePlugin::new("scene-light-01"))
    .run();
```

Launch that demo through its normal Cargo command. The capture plugin does not
create a window or scene by itself; it observes the host application's primary window.

The schedule defaults to 1.5, 4.0, and 7.0 seconds of real time. Override it with
comma-separated seconds in SYMTROPY_DEMO_CAPTURE_TIMES. SYMTROPY_DEMO_CAPTURE_EXIT_AT
can override the requested exit time; the scheduler still waits for captures to
flush and for the configured timeout policy.

## Supply comparison metadata

The plugin deliberately does not invent GPU or fixture information. Set these
optional environment variables in the caller / benchmark harness:

| Variable | Meaning |
|---|---|
| SYMTROPY_DEMO_CAPTURE_COMMIT | Full source commit SHA |
| SYMTROPY_DEMO_CAPTURE_FIXTURE_ID | Stable reference-scene identifier |
| SYMTROPY_DEMO_CAPTURE_FIXTURE_SHA256 | SHA-256 of the frozen fixture/asset manifest |
| SYMTROPY_DEMO_CAPTURE_SIMULATION_SEED | Declared deterministic seed |
| SYMTROPY_DEMO_CAPTURE_SIMULATION_TICK | Simulation tick / replay point represented |
| SYMTROPY_DEMO_CAPTURE_QUALITY_PROFILE | Named rendering profile |
| SYMTROPY_DEMO_CAPTURE_CAMERA_ID | Stable camera/view identifier and path profile |
| SYMTROPY_DEMO_CAPTURE_RENDERER_VERSION | Explicit engine/renderer version (for example, Bevy version) |
| SYMTROPY_DEMO_CAPTURE_RENDER_SETTINGS_SHA256 | SHA-256 of canonical render-settings bytes |
| SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER | Explicit GPU adapter identity |
| SYMTROPY_DEMO_CAPTURE_GPU_DRIVER | Explicit driver/version string |
| SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND | For example, Vulkan or DX12 |
| SYMTROPY_DEMO_CAPTURE_WIDTH_PX | Expected PNG width; checked against decoded image |
| SYMTROPY_DEMO_CAPTURE_HEIGHT_PX | Expected PNG height; checked against decoded image |
| SYMTROPY_DEMO_CAPTURE_OS | OS/runtime label |
| SYMTROPY_DEMO_CAPTURE_RUSTC_VERSION | Exact rustc version string |

Missing values remain null and are listed in qualification_issues. Fixture SHA-256
is syntax-checked but the plugin does not resolve or hash the original fixture
manifest; the runner must bind that digest to the actual frozen fixture bytes.
GPU identity and driver strings are caller-declared, not hardware-attested.

## Receipt example (shape)

```json
{
  "schema_version": 1,
  "crate_name": "symtropy-demo-capture",
  "capture_stem": "scene-light-01",
  "capture_status": "complete",
  "capture_complete": true,
  "qualification_status": "qualified_metadata",
  "expected_image_count": 1,
  "images": [
    {
      "scheduled_time_secs": 1.5,
      "path": "target/render-captures/scene-light-01_t1.5.png",
      "status": "verified_png",
      "byte_len": 12345,
      "sha256": "..."
    }
  ],
  "claim_ceiling": "png-decode-byte-integrity-and-declared-provenance-only; not-visual-quality-or-physical-validation"
}
```

The exact receipt includes every schema field, including all declared metadata,
detected image dimensions, and qualification issues. SHA-256 protects the image
bytes against unnoticed changes after the run; it does not say whether the image
looks correct.
