//! Verifies an evidence manifest against the actual bytes of artifacts.
//! A matching digest proves byte consistency with this manifest, not provenance, authority, or scientific validity.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process;

const HEADER: [&str; 4] = ["artifact_id", "role", "relative_path", "sha256"];

#[derive(Clone, Debug)]
struct Artifact {
    artifact_id: String,
    role: String,
    relative_path: String,
    expected_sha256: String,
}

#[derive(Clone, Debug)]
struct ReceiptRow {
    artifact_id: String,
    role: String,
    relative_path: String,
    expected_sha256: String,
    observed_sha256: String,
    status: String,
    detail: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("evidence-receipt: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(format!(
            "usage: {} <artifact-root> <manifest.csv> <receipt.csv>",
            args.first().map(String::as_str).unwrap_or("evidence-receipt")
        ));
    }
    let root = fs::canonicalize(&args[1])
        .map_err(|error| format!("cannot resolve artifact root {}: {error}", args[1]))?;
    if !root.is_dir() {
        return Err(format!("artifact root {} is not a directory", args[1]));
    }
    let manifest_bytes = fs::read(&args[2])
        .map_err(|error| format!("cannot read manifest {}: {error}", args[2]))?;
    let manifest_text = std::str::from_utf8(&manifest_bytes)
        .map_err(|_| "manifest must be UTF-8".to_string())?;
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    let artifacts = parse_manifest(manifest_text, &args[2])?;
    let (rows, all_pass) = verify_artifacts(&root, &artifacts);
    write_receipt(&args[3], &manifest_sha256, &rows)?;
    if !all_pass {
        return Err(format!(
            "one or more artifacts failed verification; receipt written to {}; manifest_sha256={manifest_sha256}",
            args[3]
        ));
    }
    eprintln!(
        "evidence integrity passed: artifacts={}, manifest_sha256={manifest_sha256}; hashes are not signatures and do not authenticate provenance or scientific validity",
        rows.len()
    );
    Ok(())
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{context}: empty value is not allowed"))
    } else {
        Ok(value.to_string())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn safe_relative_path(value: &str) -> bool {
    if value.is_empty()
        || value.contains('\\')
        || value.contains(':')
        || value.contains('"')
        || value.contains(',')
    {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn parse_manifest(input: &str, source: &str) -> Result<Vec<Artifact>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{source}: missing CSV header"))?
        .trim_end_matches('\r');
    let columns: Vec<&str> = header.split(',').collect();
    if columns.as_slice() != HEADER.as_slice() {
        return Err(format!(
            "{source}: expected header [{}], got [{}]",
            HEADER.join(","),
            columns.join(",")
        ));
    }

    let mut artifacts = Vec::new();
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for (index, raw_line) in lines.enumerate() {
        let line_number = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            return Err(format!("{source}:{line_number}: blank rows are not allowed"));
        }
        if line.contains('"') {
            return Err(format!(
                "{source}:{line_number}: quoted CSV fields are not supported"
            ));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != HEADER.len() {
            return Err(format!(
                "{source}:{line_number}: expected {} fields, found {}",
                HEADER.len(),
                fields.len()
            ));
        }
        let artifact_id = nonempty(fields[0], "artifact_id")?;
        let role = nonempty(fields[1], "role")?;
        let relative_path = nonempty(fields[2], "relative_path")?;
        let expected_sha256 = nonempty(fields[3], "sha256")?.to_ascii_lowercase();
        if !safe_relative_path(&relative_path) {
            return Err(format!(
                "{source}:{line_number}: relative_path must be a safe relative path beneath artifact-root"
            ));
        }
        if !valid_sha256(&expected_sha256) {
            return Err(format!(
                "{source}:{line_number}: sha256 must contain exactly 64 hexadecimal characters"
            ));
        }
        if !ids.insert(artifact_id.clone()) {
            return Err(format!(
                "{source}:{line_number}: duplicate artifact_id {artifact_id:?}"
            ));
        }
        if !paths.insert(relative_path.clone()) {
            return Err(format!(
                "{source}:{line_number}: duplicate relative_path {relative_path:?}"
            ));
        }
        artifacts.push(Artifact {
            artifact_id,
            role,
            relative_path,
            expected_sha256,
        });
    }
    if artifacts.is_empty() {
        return Err(format!("{source}: at least one artifact is required"));
    }
    Ok(artifacts)
}

fn verify_one(root: &Path, artifact: &Artifact) -> Result<String, String> {
    let joined: PathBuf = root.join(&artifact.relative_path);
    let resolved = fs::canonicalize(&joined).map_err(|error| {
        format!(
            "cannot resolve artifact {}: {error}",
            artifact.relative_path
        )
    })?;
    if !resolved.starts_with(root) {
        return Err(format!(
            "artifact {} resolves outside artifact-root",
            artifact.relative_path
        ));
    }
    let metadata = fs::metadata(&resolved)
        .map_err(|error| format!("cannot inspect artifact {}: {error}", artifact.relative_path))?;
    if !metadata.is_file() {
        return Err(format!(
            "artifact {} is not a regular file",
            artifact.relative_path
        ));
    }
    let bytes = fs::read(&resolved)
        .map_err(|error| format!("cannot read artifact {}: {error}", artifact.relative_path))?;
    Ok(sha256_hex(&bytes))
}

fn verify_artifacts(root: &Path, artifacts: &[Artifact]) -> (Vec<ReceiptRow>, bool) {
    let mut rows = Vec::with_capacity(artifacts.len());
    let mut all_pass = true;
    for artifact in artifacts {
        match verify_one(root, artifact) {
            Ok(observed) if observed == artifact.expected_sha256 => rows.push(ReceiptRow {
                artifact_id: artifact.artifact_id.clone(),
                role: artifact.role.clone(),
                relative_path: artifact.relative_path.clone(),
                expected_sha256: artifact.expected_sha256.clone(),
                observed_sha256: observed,
                status: "PASS".to_string(),
                detail: "digest matches manifest".to_string(),
            }),
            Ok(observed) => {
                all_pass = false;
                rows.push(ReceiptRow {
                    artifact_id: artifact.artifact_id.clone(),
                    role: artifact.role.clone(),
                    relative_path: artifact.relative_path.clone(),
                    expected_sha256: artifact.expected_sha256.clone(),
                    observed_sha256: observed,
                    status: "FAIL".to_string(),
                    detail: "digest mismatch".to_string(),
                });
            }
            Err(error) => {
                all_pass = false;
                rows.push(ReceiptRow {
                    artifact_id: artifact.artifact_id.clone(),
                    role: artifact.role.clone(),
                    relative_path: artifact.relative_path.clone(),
                    expected_sha256: artifact.expected_sha256.clone(),
                    observed_sha256: String::new(),
                    status: "FAIL".to_string(),
                    detail: error,
                });
            }
        }
    }
    (rows, all_pass)
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn write_receipt(path: &str, manifest_sha256: &str, rows: &[ReceiptRow]) -> Result<(), String> {
    let mut output = String::from(
        "manifest_sha256,artifact_id,role,relative_path,expected_sha256,observed_sha256,status,detail\n",
    );
    for row in rows {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            manifest_sha256,
            csv_escape(&row.artifact_id),
            csv_escape(&row.role),
            csv_escape(&row.relative_path),
            row.expected_sha256,
            row.observed_sha256,
            row.status,
            csv_escape(&row.detail),
        ));
    }
    fs::write(path, output).map_err(|error| format!("cannot write receipt {path}: {error}"))
}

// FIPS 180-4 SHA-256 implementation; kept dependency-free for the isolated research workspace.
fn sha256_hex(input: &[u8]) -> String {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
        0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
        0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let bit_length = (input.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(input.len() + 72);
    padded.extend_from_slice(input);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());

    let mut state = INITIAL;
    for block in padded.chunks_exact(64) {
        let mut words = [0u32; 64];
        for index in 0..16 {
            let offset = index * 4;
            words[index] = u32::from_be_bytes([
                block[offset],
                block[offset + 1],
                block[offset + 2],
                block[offset + 3],
            ]);
        }
        for index in 16..64 {
            let x = words[index - 15];
            let y = words[index - 2];
            let sigma0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let sigma1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            words[index] = words[index - 16]
                .wrapping_add(sigma0)
                .wrapping_add(words[index - 7])
                .wrapping_add(sigma1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut output = String::with_capacity(64);
    for word in state {
        output.push_str(&format!("{word:08x}"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "transformer-evidence-receipt-{label}-{}-{nonce}",
            process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn sha256_matches_standard_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn manifest_path_must_be_safe_and_digest_must_be_well_formed() {
        let traversal = "artifact_id,role,relative_path,sha256\nx,dataset,../outside.csv,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n";
        assert!(parse_manifest(traversal, "manifest").unwrap_err().contains("safe relative path"));
        let malformed = "artifact_id,role,relative_path,sha256\nx,dataset,data.csv,not-a-hash\n";
        assert!(parse_manifest(malformed, "manifest").unwrap_err().contains("64 hexadecimal"));
    }

    #[test]
    fn verifies_matching_bytes_and_rejects_tampering() {
        let root = temporary_root("tamper");
        let artifact_path = root.join("dataset.csv");
        fs::write(&artifact_path, b"source bytes\n").unwrap();
        let digest = sha256_hex(b"source bytes\n");
        let manifest = format!("artifact_id,role,relative_path,sha256\ndataset,dataset,dataset.csv,{digest}\n");
        let artifacts = parse_manifest(&manifest, "manifest").unwrap();
        let canonical_root = fs::canonicalize(&root).unwrap();
        let (rows, all_pass) = verify_artifacts(&canonical_root, &artifacts);
        assert!(all_pass);
        assert_eq!(rows[0].status, "PASS");

        fs::write(&artifact_path, b"tampered source bytes\n").unwrap();
        let (rows, all_pass) = verify_artifacts(&canonical_root, &artifacts);
        assert!(!all_pass);
        assert_eq!(rows[0].status, "FAIL");
        assert_ne!(rows[0].observed_sha256, rows[0].expected_sha256);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_duplicate_artifact_ids_and_paths() {
        let digest = "a".repeat(64);
        let duplicate_id = format!(
            "artifact_id,role,relative_path,sha256\na,dataset,a.csv,{digest}\na,split,b.csv,{digest}\n"
        );
        assert!(parse_manifest(&duplicate_id, "manifest").unwrap_err().contains("duplicate artifact_id"));
        let duplicate_path = format!(
            "artifact_id,role,relative_path,sha256\na,dataset,a.csv,{digest}\nb,split,a.csv,{digest}\n"
        );
        assert!(parse_manifest(&duplicate_path, "manifest").unwrap_err().contains("duplicate relative_path"));
    }

    #[test]
    fn rejects_a_symlink_that_resolves_outside_the_root() {
        let root = temporary_root("symlink");
        let outside = root.with_extension("outside");
        fs::write(&outside, b"outside bytes").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&outside, root.join("escape.csv")).unwrap();
            let artifact = Artifact {
                artifact_id: "escape".to_string(),
                role: "dataset".to_string(),
                relative_path: "escape.csv".to_string(),
                expected_sha256: sha256_hex(b"outside bytes"),
            };
            let canonical_root = fs::canonicalize(&root).unwrap();
            assert!(verify_one(&canonical_root, &artifact)
                .unwrap_err()
                .contains("outside artifact-root"));
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(outside);
    }
}
