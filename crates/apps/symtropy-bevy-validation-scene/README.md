# Symtropy Bevy validation scene

Minimal executable smoke scene for the permissive Symtropy Bevy distribution.

## Raster baseline

```sh
cargo run -p symtropy-bevy-validation-scene
```

## Bevy 0.20 realtime path-tracing preview

The validation scene has an opt-in Solari profile for GPUs/backends that
support Bevy 0.20's experimental ray-query path:

```sh
cargo run -p symtropy-bevy-validation-scene --features solari-preview
```

The default command remains the conventional Bevy renderer. The Solari profile
uses the same deterministic procedural material scene and camera fixture,
but adds `SolariPlugins`, `RaytracingMesh3d`, `SolariLighting`,
`CameraMainTextureUsages::STORAGE_BINDING`, and `Msaa::Off`.

This is an experimental visual comparison path, not the default runtime. It
requires a GPU/backend with ray-query support; capture adapter, driver, backend,
frame-time percentiles, VRAM, and image receipts before comparing it with raster
output. Solari support is not assumed on every platform, and no AAA-quality or
performance result is implied by the example.
