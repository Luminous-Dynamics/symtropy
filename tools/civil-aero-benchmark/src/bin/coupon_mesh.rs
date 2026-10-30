use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process,
};

use civil_aero_benchmark::coupon::{CouponSpec, generate_coupon};

fn main() {
    if let Err(error) = run() {
        eprintln!("coupon generation rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut options = BTreeMap::<String, String>::new();
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            print_help();
            return Ok(());
        }
        if !matches!(
            flag.as_str(),
            "--chord-mm" | "--span-mm" | "--chordwise" | "--spanwise" | "--out" | "--manifest"
        ) {
            return Err(format!("unknown option {flag}; use --help"));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        if options.insert(flag.clone(), value).is_some() {
            return Err(format!("duplicate option {flag}"));
        }
    }

    let chord_mm = required(&options, "--chord-mm")?
        .parse::<f64>()
        .map_err(|_| "--chord-mm must be a number".to_owned())?;
    let span_mm = required(&options, "--span-mm")?
        .parse::<f64>()
        .map_err(|_| "--span-mm must be a number".to_owned())?;
    let chordwise_panels = required(&options, "--chordwise")?
        .parse::<usize>()
        .map_err(|_| "--chordwise must be an integer".to_owned())?;
    let spanwise_panels = required(&options, "--spanwise")?
        .parse::<usize>()
        .map_err(|_| "--spanwise must be an integer".to_owned())?;
    let out_path = PathBuf::from(required(&options, "--out")?);
    let manifest_path = PathBuf::from(required(&options, "--manifest")?);
    if out_path == manifest_path {
        return Err("--out and --manifest must refer to different files".to_owned());
    }
    if out_path.exists() || manifest_path.exists() {
        return Err(
            "refusing to overwrite an existing output; choose new paths or move old files"
                .to_owned(),
        );
    }

    let spec = CouponSpec {
        chord_mm,
        span_mm,
        chordwise_panels,
        spanwise_panels,
    };
    let mesh = generate_coupon(spec)?;
    let audit = mesh.audit()?;
    let stl = mesh.to_ascii_stl("civil_aero_nonflight_coupon")?;
    let manifest = mesh.json_manifest(spec, &out_path.to_string_lossy())?;

    if let Err(error) = write_new_file(&out_path, stl.as_bytes()) {
        let _ = fs::remove_file(&out_path);
        return Err(error);
    }
    if let Err(error) = write_new_file(&manifest_path, manifest.as_bytes()) {
        let _ = fs::remove_file(&out_path);
        let _ = fs::remove_file(&manifest_path);
        return Err(format!(
            "could not write manifest; removed partial artifacts: {error}"
        ));
    }

    println!(
        "generated={} manifest={} vertices={} triangles={} edges={} watertight={} signed_volume_mm3={:.6} evidence=geometry_generated_only flight_use=prohibited",
        out_path.display(),
        manifest_path.display(),
        audit.vertex_count,
        audit.triangle_count,
        audit.unique_edge_count,
        audit.watertight_two_manifold,
        audit.signed_volume_mm3
    );
    eprintln!(
        "NEXT: compute SHA-256 over the exact STL and manifest bytes; review the facility's dimensions, blockage/support requirements and measurement envelope. This tool does not validate wind-tunnel suitability or authorize a flight."
    );
    Ok(())
}

fn required<'a>(options: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, String> {
    options
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required option {name}; use --help"))
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create parent directory: {error}"))?;
        }
    }
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn print_help() {
    println!(
        "Generate a non-flight-critical NACA 0012 extruded ground-test coupon.\n\
         Required options (no dimensions are guessed):\n\
           --chord-mm MM       positive chord dimension in millimetres\n\
           --span-mm MM        positive span/extrusion dimension in millimetres\n\
           --chordwise N       chordwise panels, 16..=400\n\
           --spanwise N        spanwise panels, 1..=60\n\
           --out PATH          new ASCII STL output path\n\
           --manifest PATH     new JSON geometry/audit manifest path\n\
         WARNING: output is not flight-ready, structurally qualified, or certified for any wind tunnel.\n\
         The STL format does not encode units; keep its millimetre manifest with the file."
    );
}
