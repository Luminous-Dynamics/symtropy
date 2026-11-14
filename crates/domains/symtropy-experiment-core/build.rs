// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn digest_file(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|bytes| sha256_hex(&bytes))
}

fn emit_value(name: &str, value: Option<String>) {
    // Build-script rustc-env values cannot contain arbitrary line breaks; discard malformed data.
    let value = value.unwrap_or_default();
    if value.contains('\n') || value.contains('\r') {
        println!("cargo:rustc-env={name}=");
    } else {
        println!("cargo:rustc-env={name}={value}");
    }
}

fn source_revision() -> Option<String> {
    let value = env::var("SYMTROPY_SOURCE_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .or_else(|_| env::var("CI_COMMIT_SHA"))
        .ok()?;
    let value = value.trim();
    // Accept a full or abbreviated hexadecimal Git object ID, not arbitrary free-form text.
    if !(7..=64).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(value.to_ascii_lowercase())
}

fn rustc_version() -> Option<String> {
    let rustc = env::var_os("RUSTC")?;
    let output = Command::new(rustc).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8(output.stdout).ok()?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_owned())
}

fn main() {
    println!("cargo:rerun-if-env-changed=SYMTROPY_SOURCE_REVISION");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=CI_COMMIT_SHA");
    println!("cargo:rerun-if-env-changed=RUSTC");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"));
    let package_manifest = manifest_dir.join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", package_manifest.display());
    emit_value(
        "SYMTROPY_BUILD_PACKAGE_MANIFEST_SHA256",
        digest_file(&package_manifest),
    );

    let workspace_lock = manifest_dir
        .ancestors()
        .map(|directory| directory.join("Cargo.lock"))
        .find(|candidate| candidate.is_file());
    if let Some(lock) = workspace_lock {
        println!("cargo:rerun-if-changed={}", lock.display());
        emit_value("SYMTROPY_BUILD_CARGO_LOCK_SHA256", digest_file(&lock));
    } else {
        // Published or standalone package builds may have no workspace lockfile.
        emit_value("SYMTROPY_BUILD_CARGO_LOCK_SHA256", None);
    }

    emit_value("SYMTROPY_BUILD_SOURCE_REVISION", source_revision());
    emit_value("SYMTROPY_BUILD_RUSTC_VERSION", rustc_version());
    emit_value(
        "SYMTROPY_BUILD_TARGET",
        env::var("TARGET").ok().filter(|value| !value.trim().is_empty()),
    );
    emit_value(
        "SYMTROPY_BUILD_HOST",
        env::var("HOST").ok().filter(|value| !value.trim().is_empty()),
    );
}
