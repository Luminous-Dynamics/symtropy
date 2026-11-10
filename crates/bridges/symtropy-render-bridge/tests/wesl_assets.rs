use std::{fs, path::Path};

use bevy::shader::Shader;

const WESL_ASSETS: &[&str] = &[
    "nd_slicing.wesl",
    "holographic.wesl",
    "telemetry_visualization.wesl",
    "crt_consciousness.wesl",
];

#[test]
fn migrated_custom_shaders_parse_as_bevy_wesl_assets() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository_root = crate_dir.join("../../..");
    let asset_dir = repository_root.join("assets/shaders");

    for file_name in WESL_ASSETS {
        let path = asset_dir.join(file_name);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

        assert!(
            !source.contains("#import") && !source.contains("#ifdef") && !source.contains("#endif"),
            "{} still contains the removed shader-preprocessor syntax",
            path.display()
        );

        let shader = Shader::from_wesl(
            source,
            format!("assets/shaders/{file_name}"),
        );

        assert!(
            !shader.imports.is_empty(),
            "{} should declare imports recognized by Bevy 0.20's WESL loader",
            path.display()
        );
    }
}
