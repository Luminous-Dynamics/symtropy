use std::path::PathBuf;
use std::process::Command;

const MANIFEST_SHA256: &str =
    "7945986581c4c7b07b8cdb8e970434c11a8023b19e2b05a3e059607a3bb410cd";

fn invoke(expected: &str) -> std::process::Output {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("valid-manifest.json");
    Command::new(env!("CARGO_BIN_EXE_shipyard-benchmark-audit"))
        .args(["verify-digest", manifest.to_str().expect("fixture path is UTF-8"), expected])
        .output()
        .expect("audit CLI should start")
}

#[test]
fn verify_digest_accepts_matching_exact_file_bytes() {
    let output = invoke(MANIFEST_SHA256);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("VALID SHA-256"));
}

#[test]
fn verify_digest_rejects_a_mismatched_hash() {
    let output = invoke(&"0".repeat(64));
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("SHA-256 mismatch"));
}

#[test]
fn verify_digest_rejects_malformed_hash_arguments() {
    let output = invoke("not-a-digest");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("64 hexadecimal characters"));
}
