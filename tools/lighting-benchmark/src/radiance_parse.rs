use lighting_benchmark::radiance::{
    parse_rtrace_legacy_rgb_irradiance, MeasurementPoint, RtraceParseError,
};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

struct Args {
    grid_tsv: PathBuf,
    raw_output: PathBuf,
    normalized_output: PathBuf,
    diagnostics_output: PathBuf,
    coordinate_tolerance_m: f64,
}

fn usage() -> &'static str {
    "Usage: lighting-rtrace-parse --grid-tsv GRID.tsv --raw-output STDOUT.raw \
     --normalized-output SAMPLES.json --diagnostics-output DIAGNOSTICS.json \
     --coordinate-tolerance-m METRES"
}

fn parse_args() -> Result<Args, String> {
    let mut grid_tsv = None;
    let mut raw_output = None;
    let mut normalized_output = None;
    let mut diagnostics_output = None;
    let mut coordinate_tolerance_m = None;
    let mut iter = env::args().skip(1);
    while let Some(key) = iter.next() {
        if key == "-h" || key == "--help" {
            return Err(usage().to_owned());
        }
        let value = iter.next().ok_or_else(|| format!("missing value for {key}"))?;
        match key.as_str() {
            "--grid-tsv" => grid_tsv = Some(PathBuf::from(value)),
            "--raw-output" => raw_output = Some(PathBuf::from(value)),
            "--normalized-output" => normalized_output = Some(PathBuf::from(value)),
            "--diagnostics-output" => diagnostics_output = Some(PathBuf::from(value)),
            "--coordinate-tolerance-m" => {
                coordinate_tolerance_m = Some(
                    value.parse::<f64>().map_err(|_| "invalid coordinate tolerance".to_owned())?,
                )
            }
            _ => return Err(format!("unknown argument {key}\n{}", usage())),
        }
    }
    let args = Args {
        grid_tsv: grid_tsv.ok_or_else(|| "missing --grid-tsv".to_owned())?,
        raw_output: raw_output.ok_or_else(|| "missing --raw-output".to_owned())?,
        normalized_output: normalized_output.ok_or_else(|| "missing --normalized-output".to_owned())?,
        diagnostics_output: diagnostics_output.ok_or_else(|| "missing --diagnostics-output".to_owned())?,
        coordinate_tolerance_m: coordinate_tolerance_m
            .ok_or_else(|| "missing --coordinate-tolerance-m".to_owned())?,
    };
    if !args.coordinate_tolerance_m.is_finite() || args.coordinate_tolerance_m <= 0.0 {
        return Err("coordinate tolerance must be positive and finite".to_owned());
    }
    if args.normalized_output == args.diagnostics_output {
        return Err("normalized and diagnostics outputs must differ".to_owned());
    }
    Ok(args)
}

fn read_grid(path: &Path) -> Result<Vec<MeasurementPoint>, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("read grid: {error}"))?;
    let mut points = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            return Err(format!("blank grid row at line {}", line_number + 1));
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(format!("expected four TSV fields at line {}", line_number + 1));
        }
        let grid_index = fields[0].parse::<usize>()
            .map_err(|_| format!("invalid grid index at line {}", line_number + 1))?;
        let x_m = fields[1].parse::<f64>()
            .map_err(|_| format!("invalid x coordinate at line {}", line_number + 1))?;
        let y_m = fields[2].parse::<f64>()
            .map_err(|_| format!("invalid y coordinate at line {}", line_number + 1))?;
        let z_m = fields[3].parse::<f64>()
            .map_err(|_| format!("invalid z coordinate at line {}", line_number + 1))?;
        points.push(MeasurementPoint { grid_index, x_m, y_m, z_m });
    }
    if points.is_empty() {
        return Err("grid has no points".to_owned());
    }
    Ok(points)
}

fn json_number(value: f64) -> String {
    format!("{value:.17e}")
}

fn write_new(path: &Path, content: &str) -> Result<(), String> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)
        .map_err(|error| format!("create {}: {error}", path.display()))?;
    file.write_all(content.as_bytes())
        .map_err(|error| format!("write {}: {error}", path.display()))?;
    file.sync_all().map_err(|error| format!("sync {}: {error}", path.display()))?;
    Ok(())
}

fn render_normalized(samples: &[lighting_benchmark::radiance::RadianceGridSample]) -> String {
    let mut out = String::from("[\n");
    for (index, sample) in samples.iter().enumerate() {
        if index > 0 { out.push_str(",\n"); }
        out.push_str(&format!(
            "  {{\"grid_index\":{},\"x_m\":{},\"y_m\":{},\"z_m\":{},\"illuminance_lx\":{}}}",
            sample.grid_index,
            json_number(sample.expected_position_m[0]),
            json_number(sample.expected_position_m[1]),
            json_number(sample.expected_position_m[2]),
            json_number(sample.illuminance_lx),
        ));
    }
    out.push_str("\n]\n");
    out
}

fn render_diagnostics(samples: &[lighting_benchmark::radiance::RadianceGridSample]) -> String {
    let mut out = String::from("[\n");
    for (index, sample) in samples.iter().enumerate() {
        if index > 0 { out.push_str(",\n"); }
        out.push_str(&format!(
            "  {{\"grid_index\":{},\"echoed_origin_m\":[{},{},{}],\"irradiance_rgb\":[{},{},{}],\"conversion_id\":\"{}\"}}",
            sample.grid_index,
            json_number(sample.echoed_origin_m[0]),
            json_number(sample.echoed_origin_m[1]),
            json_number(sample.echoed_origin_m[2]),
            json_number(sample.irradiance_rgb[0]),
            json_number(sample.irradiance_rgb[1]),
            json_number(sample.irradiance_rgb[2]),
            sample.conversion_id,
        ));
    }
    out.push_str("\n]\n");
    out
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("lighting-rtrace-parse: {error}");
            std::process::exit(2);
        }
    };
    let expected = match read_grid(&args.grid_tsv) {
        Ok(points) => points,
        Err(error) => {
            eprintln!("lighting-rtrace-parse: {error}");
            std::process::exit(3);
        }
    };
    let raw = match fs::read_to_string(&args.raw_output) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("lighting-rtrace-parse: could not read raw solver output: {error}");
            std::process::exit(3);
        }
    };
    let samples = match parse_rtrace_legacy_rgb_irradiance(
        &raw, &expected, args.coordinate_tolerance_m,
    ) {
        Ok(samples) => samples,
        Err(error) => {
            eprintln!("lighting-rtrace-parse: {}", error.as_str());
            std::process::exit(exit_code_for_parse_error(error));
        }
    };
    if let Err(error) = write_new(&args.normalized_output, &render_normalized(&samples)) {
        eprintln!("lighting-rtrace-parse: {error}");
        std::process::exit(4);
    }
    if let Err(error) = write_new(&args.diagnostics_output, &render_diagnostics(&samples)) {
        eprintln!("lighting-rtrace-parse: {error}");
        std::process::exit(4);
    }
    println!("parsed {} samples with {}", samples.len(),
        samples.first().map_or("no-conversion", |sample| sample.conversion_id));
}

fn exit_code_for_parse_error(_error: RtraceParseError) -> i32 { 3 }
