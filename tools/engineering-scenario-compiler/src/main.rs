use engineering_scenario_compiler::{
    compile_with_artifacts, ArtifactRole, Disposition, SuppliedArtifact, MAX_SOURCE_BYTES,
    MAX_UPSTREAM_CONFIG_BYTES, MAX_UPSTREAM_MESH_BYTES,
};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn read_bounded(path: &Path, maximum: usize, role: &str) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("could not inspect {role} path: {error}"))?;
    if !metadata.file_type().is_file() {
        return Err(format!("{role} path must identify a regular file"));
    }
    if metadata.len() > maximum as u64 {
        return Err(format!("{role} exceeds the {maximum} byte CLI limit"));
    }
    let bytes = std::fs::read(path)
        .map_err(|error| format!("could not read {role}: {error}"))?;
    if bytes.len() > maximum {
        return Err(format!("{role} exceeded the {maximum} byte CLI limit while reading"));
    }
    Ok(bytes)
}

fn usage(program: &OsStr) {
    eprintln!(
        "usage: {} <scenario.toml> [--solver-config <path>] [--mesh <path>]",
        program.to_string_lossy()
    );
}

fn option_path(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
    slot: &mut Option<PathBuf>,
) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("{option} may only be specified once"));
    }
    let path = args
        .next()
        .ok_or_else(|| format!("{option} requires a path"))?;
    slot.replace(PathBuf::from(path));
    Ok(())
}

fn parse_options<I>(
    args: I,
) -> Result<(Option<PathBuf>, Option<PathBuf>), String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let mut config_path = None;
    let mut mesh_path = None;
    while let Some(argument) = args.next() {
        let result = match argument.to_str() {
            Some("--solver-config") => {
                option_path(&mut args, "--solver-config", &mut config_path)
            }
            Some("--mesh") => option_path(&mut args, "--mesh", &mut mesh_path),
            _ => return Err("unknown CLI option".to_owned()),
        };
        result?;
    }
    Ok((config_path, mesh_path))
}

fn run() -> Result<ExitCode, String> {
    let mut args = std::env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(source_path) = args.next() else {
        usage(&program);
        return Ok(ExitCode::from(64));
    };

    let (config_path, mesh_path) = match parse_options(args) {
        Ok(paths) => paths,
        Err(error) => {
            usage(&program);
            eprintln!("{error}");
            return Ok(ExitCode::from(64));
        }
    };

    let source = read_bounded(
        Path::new(&source_path),
        MAX_SOURCE_BYTES,
        "scenario source",
    )?;
    let mut owned_artifacts = Vec::with_capacity(2);
    if let Some(path) = config_path {
        owned_artifacts.push((
            ArtifactRole::SolverConfig,
            read_bounded(Path::new(&path), MAX_UPSTREAM_CONFIG_BYTES, "solver config")?,
        ));
    }
    if let Some(path) = mesh_path {
        owned_artifacts.push((
            ArtifactRole::Mesh,
            read_bounded(Path::new(&path), MAX_UPSTREAM_MESH_BYTES, "mesh")?,
        ));
    }
    let borrowed_artifacts: Vec<SuppliedArtifact<'_>> = owned_artifacts
        .iter()
        .map(|(role, bytes)| SuppliedArtifact {
            role: *role,
            bytes,
        })
        .collect();

    let report = compile_with_artifacts(&source, &borrowed_artifacts);
    let exit = match report.disposition {
        Disposition::PlanReady => ExitCode::SUCCESS,
        Disposition::Invalid => ExitCode::from(2),
        Disposition::Blocked => ExitCode::from(3),
        Disposition::Unsupported => ExitCode::from(4),
    };
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("could not serialize compiler report: {error}"))?;
    println!("{json}");
    Ok(exit)
}

fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(66)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args<'a, 'b>(values: &'a [&'b str]) -> impl Iterator<Item = OsString> + 'a {
        values.iter().map(OsString::from)
    }

    #[test]
    fn parses_each_optional_artifact_path_once() {
        let (config, mesh) = parse_options(args(&[
            "--solver-config", "solver.cfg", "--mesh", "grid.su2",
        ])).expect("valid options parse");
        assert_eq!(config, Some(PathBuf::from("solver.cfg")));
        assert_eq!(mesh, Some(PathBuf::from("grid.su2")));
    }

    #[test]
    fn rejects_unknown_duplicate_and_missing_option_values() {
        assert!(parse_options(args(&["--unknown"])).is_err());
        assert!(parse_options(args(&["--mesh"])).is_err());
        assert!(parse_options(args(&[
            "--mesh", "one.su2", "--mesh", "two.su2",
        ])).is_err());
        assert!(parse_options(args(&[
            "--solver-config", "one.cfg", "--solver-config", "two.cfg",
        ])).is_err());
    }

    #[test]
    fn bounded_reader_rejects_directories_before_reading() {
        assert!(read_bounded(Path::new("."), 1024, "test artifact")
            .expect_err("directory is not a regular file")
            .contains("regular file"));
    }
}
