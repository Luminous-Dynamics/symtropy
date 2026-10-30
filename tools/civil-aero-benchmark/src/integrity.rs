//! Exact-byte source integrity checks for retained benchmark reference data.
//!
//! Hashing is delegated to a locally installed SHA-256 utility through stdin. This keeps the
//! production crate dependency-free while binding the digest to the exact in-memory bytes that
//! are subsequently parsed. Supported commands: sha256sum or shasum -a 256.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn sha256_hex(bytes: &[u8]) -> Result<String, String> {
    let mut last_not_found = None;
    for (program, args) in [
        ("sha256sum", Vec::<&str>::new()),
        ("shasum", vec!["-a", "256"]),
    ] {
        let mut child = match Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                last_not_found = Some(format!("{program}: {error}"));
                continue;
            }
            Err(error) => return Err(format!("cannot start {program}: {error}")),
        };

        {
            let mut child_stdin = child
                .stdin
                .take()
                .ok_or_else(|| format!("{program}: stdin pipe was not created"))?;
            child_stdin
                .write_all(bytes)
                .map_err(|error| format!("cannot provide source bytes to {program}: {error}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|error| format!("cannot collect {program} output: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{program} failed while hashing source bytes: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let digest = String::from_utf8(output.stdout)
            .map_err(|_| format!("{program} returned non-UTF-8 digest output"))?
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("{program} returned an empty digest"))?
            .to_ascii_lowercase();
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("{program} returned a malformed SHA-256 digest"));
        }
        return Ok((digest, byte_count));
    }

    Err(format!(
        "no SHA-256 utility found; install sha256sum or shasum (-a 256). {}",
        last_not_found.unwrap_or_default()
    ))
}



/// Hash a file through a streaming stdin pipe without loading the entire file into memory.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let source = File::open(path)
        .map_err(|error| format!("cannot open source file {}: {error}", path.display()))?;
    sha256_reader(source).map(|(digest, _byte_count)| digest)
}

fn sha256_reader<R: Read>(mut source: R) -> Result<(String, u64), String> {
    let mut last_not_found = None;
    for (program, args) in [
        ("sha256sum", Vec::<&str>::new()),
        ("shasum", vec!["-a", "256"]),
    ] {
        let mut child = match Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                last_not_found = Some(format!("{program}: {error}"));
                continue;
            }
            Err(error) => return Err(format!("cannot start {program}: {error}")),
        };

        let byte_count = {
            let mut child_stdin = child
                .stdin
                .take()
                .ok_or_else(|| format!("{program}: stdin pipe was not created"))?;
            std::io::copy(&mut source, &mut child_stdin)
                .map_err(|error| format!("cannot stream source file to {program}: {error}"))?
        };

        let output = child
            .wait_with_output()
            .map_err(|error| format!("cannot collect {program} output: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{program} failed while hashing source bytes: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let digest = String::from_utf8(output.stdout)
            .map_err(|_| format!("{program} returned non-UTF-8 digest output"))?
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("{program} returned an empty digest"))?
            .to_ascii_lowercase();
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("{program} returned a malformed SHA-256 digest"));
        }
        return Ok((digest, byte_count));
    }

    Err(format!(
        "no SHA-256 utility found; install sha256sum or shasum (-a 256). {}",
        last_not_found.unwrap_or_default()
    ))
}

/// Verify exact file bytes by checking metadata length and streaming the actual file contents.
pub fn verify_source_file(
    path: &Path,
    source_name: &str,
    expected_byte_length: u64,
    expected_sha256: &str,
) -> Result<(), String> {
    let file = File::open(path)
        .map_err(|error| format!("{source_name}: cannot open source file: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("{source_name}: cannot read file metadata: {error}"))?;
    if !metadata.is_file() {
        return Err(format!("{source_name}: expected a regular file"));
    }
    if expected_sha256.len() != 64
        || !expected_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(format!("{source_name}: expected SHA-256 is malformed"));
    }
    let (actual, actual_byte_length) = sha256_reader(file)?;
    if actual_byte_length != expected_byte_length {
        return Err(format!(
            "{source_name}: byte length mismatch: expected {expected_byte_length}, got {actual_byte_length}"
        ));
    }
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(format!(
            "{source_name}: SHA-256 mismatch: expected {}, got {actual}",
            expected_sha256.to_ascii_lowercase()
        ));
    }
    Ok(())
}

/// Verify exact in-memory bytes before parsing or attaching source provenance.
pub fn verify_source_bytes(
    bytes: &[u8],
    source_name: &str,
    expected_byte_length: usize,
    expected_sha256: &str,
) -> Result<(), String> {
    if bytes.len() != expected_byte_length {
        return Err(format!(
            "{source_name}: byte length mismatch: expected {expected_byte_length}, got {}",
            bytes.len()
        ));
    }
    if expected_sha256.len() != 64 || !expected_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(format!("{source_name}: expected SHA-256 is malformed"));
    }
    let actual = sha256_hex(bytes)?;
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(format!(
            "{source_name}: SHA-256 mismatch: expected {}, got {actual}",
            expected_sha256.to_ascii_lowercase()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_abc_vector() {
        assert_eq!(
            sha256_hex(b"abc").expect("SHA-256 utility available"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn streaming_file_hash_matches_in_memory_hash() {
        let path = std::env::temp_dir().join(format!(
            "civil-aero-integrity-stream-{}.bin",
            std::process::id()
        ));
        let bytes = b"streaming hash test payload";
        std::fs::write(&path, bytes).expect("write test source");
        let digest = sha256_hex(bytes).expect("in-memory hash");
        assert_eq!(sha256_file(&path).expect("streaming file hash"), digest);
        verify_source_file(&path, "stream fixture", bytes.len() as u64, &digest)
            .expect("file bytes, length and digest agree");
        std::fs::remove_file(path).expect("remove test source");
    }

    #[test]
    fn byte_length_mismatch_fails_before_hashing() {
        let error = verify_source_bytes(b"abc", "fixture", 4, &"0".repeat(64))
            .expect_err("wrong byte count rejected");
        assert!(error.contains("byte length mismatch"));
    }

    #[test]
    fn digest_mismatch_fails_closed() {
        let error = verify_source_bytes(
            b"abc",
            "fixture",
            3,
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .expect_err("wrong digest rejected");
        assert!(error.contains("SHA-256 mismatch"));
    }
}
