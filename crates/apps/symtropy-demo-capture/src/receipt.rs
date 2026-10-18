// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Machine-readable visual-capture receipts.
//!
//! A receipt records capture integrity and declared provenance separately.
//! A verified PNG is not proof that a renderer is visually correct or that
//! the simulation is physically valid.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Cursor, ErrorKind, Read, Write},
    path::{Path, PathBuf},
};

const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const MAX_IMAGE_PIXELS: u64 = 20_000_000;
const CLAIM_CEILING: &str = "png-decode-byte-integrity-and-declared-provenance-only; not-visual-quality-or-physical-validation";

/// Metadata supplied by the benchmark runner. Missing values remain explicit;
/// the receipt is not qualified for comparisons until required fields exist.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CaptureMetadata {
    pub source_commit: Option<String>,
    pub fixture_id: Option<String>,
    pub fixture_sha256: Option<String>,
    pub simulation_seed: Option<String>,
    pub simulation_tick: Option<String>,
    pub quality_profile: Option<String>,
    pub camera_id: Option<String>,
    pub renderer_version: Option<String>,
    pub render_settings_sha256: Option<String>,
    pub gpu_adapter: Option<String>,
    pub gpu_driver: Option<String>,
    pub render_backend: Option<String>,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    pub operating_system: Option<String>,
    pub rustc_version: Option<String>,
}

impl CaptureMetadata {
    /// Read optional metadata from documented environment variables.
    ///
    /// No value is inferred from the host: missing adapter, driver, dimensions,
    /// or fixture information stays None and prevents qualification.
    pub fn from_env() -> Self {
        Self {
            source_commit: env_value("SYMTROPY_DEMO_CAPTURE_COMMIT"),
            fixture_id: env_value("SYMTROPY_DEMO_CAPTURE_FIXTURE_ID"),
            fixture_sha256: env_value("SYMTROPY_DEMO_CAPTURE_FIXTURE_SHA256")
                .map(|value| value.to_ascii_lowercase()),
            simulation_seed: env_value("SYMTROPY_DEMO_CAPTURE_SIMULATION_SEED"),
            simulation_tick: env_value("SYMTROPY_DEMO_CAPTURE_SIMULATION_TICK"),
            quality_profile: env_value("SYMTROPY_DEMO_CAPTURE_QUALITY_PROFILE"),
            camera_id: env_value("SYMTROPY_DEMO_CAPTURE_CAMERA_ID"),
            renderer_version: env_value("SYMTROPY_DEMO_CAPTURE_RENDERER_VERSION"),
            render_settings_sha256: env_value("SYMTROPY_DEMO_CAPTURE_RENDER_SETTINGS_SHA256")
                .map(|value| value.to_ascii_lowercase()),
            gpu_adapter: env_value("SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER"),
            gpu_driver: env_value("SYMTROPY_DEMO_CAPTURE_GPU_DRIVER"),
            render_backend: env_value("SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND"),
            width_px: env_value("SYMTROPY_DEMO_CAPTURE_WIDTH_PX")
                .and_then(|value| value.parse().ok()),
            height_px: env_value("SYMTROPY_DEMO_CAPTURE_HEIGHT_PX")
                .and_then(|value| value.parse().ok()),
            operating_system: env_value("SYMTROPY_DEMO_CAPTURE_OS"),
            rustc_version: env_value("SYMTROPY_DEMO_CAPTURE_RUSTC_VERSION"),
        }
    }

    fn qualification_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        for (name, value) in [
            ("source_commit", self.source_commit.as_ref()),
            ("fixture_id", self.fixture_id.as_ref()),
            ("simulation_seed", self.simulation_seed.as_ref()),
            ("simulation_tick", self.simulation_tick.as_ref()),
            ("quality_profile", self.quality_profile.as_ref()),
            ("camera_id", self.camera_id.as_ref()),
            ("renderer_version", self.renderer_version.as_ref()),
            ("gpu_adapter", self.gpu_adapter.as_ref()),
            ("gpu_driver", self.gpu_driver.as_ref()),
            ("render_backend", self.render_backend.as_ref()),
            ("operating_system", self.operating_system.as_ref()),
            ("rustc_version", self.rustc_version.as_ref()),
        ] {
            if value.is_none_or(|value| value.trim().is_empty()) {
                issues.push(format!("missing_{name}"));
            }
        }
        if self.width_px.is_none_or(|value| value == 0) {
            issues.push("missing_or_invalid_width_px".to_owned());
        }
        if self.height_px.is_none_or(|value| value == 0) {
            issues.push("missing_or_invalid_height_px".to_owned());
        }
        if self
            .source_commit
            .as_deref()
            .is_some_and(|value| !(is_sha1(value) || is_sha256(value)))
        {
            issues.push("invalid_source_commit".to_owned());
        }
        match self.fixture_sha256.as_deref() {
            Some(value) if is_sha256(value) => {}
            Some(_) => issues.push("invalid_fixture_sha256".to_owned()),
            None => issues.push("missing_fixture_sha256".to_owned()),
        }
        match self.render_settings_sha256.as_deref() {
            Some(value) if is_sha256(value) => {}
            Some(_) => issues.push("invalid_render_settings_sha256".to_owned()),
            None => issues.push("missing_render_settings_sha256".to_owned()),
        }
        issues
    }
}

/// Result for one expected screenshot path, including explicit failures.
#[derive(Clone, Debug, Serialize)]
pub struct ImageReceipt {
    pub scheduled_time_secs: Option<f32>,
    pub path: Option<String>,
    pub status: String,
    pub byte_len: Option<u64>,
    pub sha256: Option<String>,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
}

/// Versioned receipt written next to the PNGs.
#[derive(Clone, Debug, Serialize)]
pub struct CaptureReceipt {
    pub schema_version: u32,
    pub crate_name: &'static str,
    pub crate_version: &'static str,
    pub capture_stem: String,
    pub capture_status: &'static str,
    pub capture_complete: bool,
    pub qualification_status: &'static str,
    pub qualification_issues: Vec<String>,
    pub expected_image_count: usize,
    pub images: Vec<ImageReceipt>,
    pub metadata: CaptureMetadata,
    pub claim_ceiling: &'static str,
}

impl CaptureReceipt {
    /// Build a deterministic receipt over the expected schedule and output paths.
    pub fn build(
        stem: impl Into<String>,
        scheduled_times: &[f32],
        paths: &[String],
        metadata: CaptureMetadata,
    ) -> Self {
        let count = scheduled_times.len().max(paths.len());
        let mut images = Vec::with_capacity(count);
        let mut capture_complete = scheduled_times.len() == paths.len();

        for index in 0..count {
            let scheduled_time_secs = scheduled_times.get(index).copied();
            let path = paths.get(index).cloned();
            let image = inspect_image(
                scheduled_time_secs,
                path.as_deref(),
                metadata.width_px,
                metadata.height_px,
            );
            if image.status != "verified_png" {
                capture_complete = false;
            }
            images.push(image);
        }

        let qualification_issues = metadata.qualification_issues();
        let qualification_complete = qualification_issues.is_empty();

        Self {
            schema_version: 1,
            crate_name: env!("CARGO_PKG_NAME"),
            crate_version: env!("CARGO_PKG_VERSION"),
            capture_stem: stem.into(),
            capture_status: if capture_complete {
                "complete"
            } else {
                "incomplete"
            },
            capture_complete,
            qualification_status: if qualification_complete {
                "qualified_metadata"
            } else {
                "incomplete_metadata"
            },
            qualification_issues,
            expected_image_count: scheduled_times.len(),
            images,
            metadata,
            claim_ceiling: CLAIM_CEILING,
        }
    }
}

/// Write JSON to a temporary file and rename it into place to avoid leaving a
/// partially written receipt after a process interruption.
pub fn write_capture_receipt(
    directory: impl AsRef<Path>,
    stem: &str,
    receipt: &CaptureReceipt,
) -> io::Result<PathBuf> {
    let directory = directory.as_ref();
    fs::create_dir_all(directory)?;
    let final_path = directory.join(format!("{stem}.capture-receipt.json"));
    let temporary_path = directory.join(format!(
        ".{stem}.capture-receipt.{}.tmp",
        std::process::id()
    ));
    let bytes = serde_json::to_vec_pretty(receipt)
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;

    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);

        // Unix rename replaces atomically. Windows does not replace an
        // existing destination, so remove an older receipt only on Windows.
        #[cfg(windows)]
        if final_path.exists() {
            fs::remove_file(&final_path)?;
        }
        fs::rename(&temporary_path, &final_path)?;
        Ok(final_path.clone())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}

fn env_value(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn is_sha1(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn inspect_image(
    scheduled_time_secs: Option<f32>,
    path: Option<&str>,
    expected_width: Option<u32>,
    expected_height: Option<u32>,
) -> ImageReceipt {
    const MAX_PNG_BYTES: u64 = 256 * 1024 * 1024;

    let Some(path) = path else {
        return image_failure(scheduled_time_secs, None, "missing_capture_path");
    };

    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return image_failure(scheduled_time_secs, Some(path), "unreadable"),
    };

    // Reject known oversized files before reading them into memory. The
    // bounded read below still handles a file that grows after this metadata check.
    if let Ok(metadata) = file.metadata()
        && metadata.len() > MAX_PNG_BYTES
    {
        return ImageReceipt {
            scheduled_time_secs,
            path: Some(path.to_owned()),
            status: "size_limit_exceeded".to_owned(),
            byte_len: Some(metadata.len()),
            sha256: None,
            width_px: None,
            height_px: None,
        };
    }

    // Hash and decode the exact same bounded byte buffer. Reopening a path for
    // decode after hashing could otherwise make the digest and dimensions refer
    // to different file contents if the path were replaced between operations.
    let mut bytes = Vec::new();
    let mut limited = file.take(MAX_PNG_BYTES + 1);
    if limited.read_to_end(&mut bytes).is_err() {
        return image_failure(scheduled_time_secs, Some(path), "read_error");
    }
    let byte_len = bytes.len() as u64;
    if byte_len > MAX_PNG_BYTES {
        return ImageReceipt {
            scheduled_time_secs,
            path: Some(path.to_owned()),
            status: "size_limit_exceeded".to_owned(),
            byte_len: None,
            sha256: None,
            width_px: None,
            height_px: None,
        };
    }
    if byte_len == 0 {
        return ImageReceipt {
            scheduled_time_secs,
            path: Some(path.to_owned()),
            status: "empty".to_owned(),
            byte_len: Some(0),
            sha256: None,
            width_px: None,
            height_px: None,
        };
    }

    let digest = format!("{:x}", Sha256::digest(&bytes));
    if !bytes.starts_with(&PNG_SIGNATURE) {
        return ImageReceipt {
            scheduled_time_secs,
            path: Some(path.to_owned()),
            status: "invalid_png_signature".to_owned(),
            byte_len: Some(byte_len),
            sha256: Some(digest),
            width_px: None,
            height_px: None,
        };
    }

    let (declared_width, declared_height) = match image::ImageReader::with_format(
        Cursor::new(bytes.as_slice()),
        image::ImageFormat::Png,
    )
    .into_dimensions()
    {
        Ok(dimensions) => dimensions,
        Err(_) => {
            return ImageReceipt {
                scheduled_time_secs,
                path: Some(path.to_owned()),
                status: "decode_error".to_owned(),
                byte_len: Some(byte_len),
                sha256: Some(digest),
                width_px: None,
                height_px: None,
            };
        }
    };
    let pixel_count = u64::from(declared_width).checked_mul(u64::from(declared_height));
    if declared_width == 0
        || declared_height == 0
        || pixel_count.is_none_or(|count| count > MAX_IMAGE_PIXELS)
    {
        return ImageReceipt {
            scheduled_time_secs,
            path: Some(path.to_owned()),
            status: "pixel_limit_exceeded".to_owned(),
            byte_len: Some(byte_len),
            sha256: Some(digest),
            width_px: Some(declared_width),
            height_px: Some(declared_height),
        };
    }

    let decoded = match image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) {
        Ok(image) => image,
        Err(_) => {
            return ImageReceipt {
                scheduled_time_secs,
                path: Some(path.to_owned()),
                status: "decode_error".to_owned(),
                byte_len: Some(byte_len),
                sha256: Some(digest),
                width_px: None,
                height_px: None,
            };
        }
    };
    drop(bytes);

    let width_px = decoded.width();
    let height_px = decoded.height();
    if width_px != declared_width || height_px != declared_height {
        return ImageReceipt {
            scheduled_time_secs,
            path: Some(path.to_owned()),
            status: "dimension_instability".to_owned(),
            byte_len: Some(byte_len),
            sha256: Some(digest),
            width_px: Some(width_px),
            height_px: Some(height_px),
        };
    }
    if expected_width.is_some_and(|expected| expected != width_px)
        || expected_height.is_some_and(|expected| expected != height_px)
    {
        return ImageReceipt {
            scheduled_time_secs,
            path: Some(path.to_owned()),
            status: "dimension_mismatch".to_owned(),
            byte_len: Some(byte_len),
            sha256: Some(digest),
            width_px: Some(width_px),
            height_px: Some(height_px),
        };
    }

    ImageReceipt {
        scheduled_time_secs,
        path: Some(path.to_owned()),
        status: "verified_png".to_owned(),
        byte_len: Some(byte_len),
        sha256: Some(digest),
        width_px: Some(width_px),
        height_px: Some(height_px),
    }
}

fn image_failure(
    scheduled_time_secs: Option<f32>,
    path: Option<&str>,
    status: &str,
) -> ImageReceipt {
    ImageReceipt {
        scheduled_time_secs,
        path: path.map(str::to_owned),
        status: status.to_owned(),
        byte_len: None,
        sha256: None,
        width_px: None,
        height_px: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    fn test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "symtropy-capture-receipt-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("test directory should be created");
        directory
    }

    fn qualified_metadata() -> CaptureMetadata {
        CaptureMetadata {
            source_commit: Some("a".repeat(40)),
            fixture_id: Some("SCENE-LIGHT-01".to_owned()),
            fixture_sha256: Some("b".repeat(64)),
            simulation_seed: Some("7".to_owned()),
            simulation_tick: Some("120".to_owned()),
            quality_profile: Some("balanced-1080p".to_owned()),
            camera_id: Some("main-camera".to_owned()),
            renderer_version: Some("bevy-0.19.0".to_owned()),
            render_settings_sha256: Some("c".repeat(64)),
            gpu_adapter: Some("test-adapter".to_owned()),
            gpu_driver: Some("test-driver".to_owned()),
            render_backend: Some("vulkan".to_owned()),
            width_px: Some(1),
            height_px: Some(1),
            operating_system: Some("test-os".to_owned()),
            rustc_version: Some("rustc test".to_owned()),
        }
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        use image::ImageEncoder;
        let pixels = vec![127_u8; (width * height * 4) as usize];
        let file = File::create(path).expect("test PNG should be created");
        image::codecs::png::PngEncoder::new(file)
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .expect("test PNG should encode");
    }

    #[test]
    fn valid_png_bytes_produce_stable_sha256_receipt() {
        let directory = test_directory();
        let path = directory.join("frame.png");
        write_test_png(&path, 2, 1);
        let byte_len = fs::metadata(&path)
            .expect("PNG metadata should exist")
            .len();
        let path_string = path.to_string_lossy().into_owned();

        let mut metadata = qualified_metadata();
        metadata.width_px = Some(2);
        metadata.height_px = Some(1);
        let receipt = CaptureReceipt::build("test", &[1.5], &[path_string], metadata);
        assert!(receipt.capture_complete);
        assert_eq!(receipt.capture_status, "complete");
        assert_eq!(receipt.qualification_status, "qualified_metadata");
        assert_eq!(receipt.images.len(), 1);
        assert_eq!(receipt.images[0].status, "verified_png");
        assert_eq!(receipt.images[0].byte_len, Some(byte_len));
        assert_eq!(receipt.images[0].width_px, Some(2));
        assert_eq!(receipt.images[0].height_px, Some(1));
        assert_eq!(receipt.images[0].sha256.as_ref().map(String::len), Some(64));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_image_is_incomplete_not_a_successful_capture() {
        let directory = test_directory();
        let missing = directory.join("missing.png").to_string_lossy().into_owned();
        let receipt = CaptureReceipt::build("test", &[1.5], &[missing], qualified_metadata());

        assert!(!receipt.capture_complete);
        assert_eq!(receipt.capture_status, "incomplete");
        assert_eq!(receipt.images[0].status, "unreadable");

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn a_png_signature_is_required_even_when_bytes_exist() {
        let directory = test_directory();
        let path = directory.join("not-a-png.png");
        fs::write(&path, b"not actually png").expect("fixture should be written");
        let receipt = CaptureReceipt::build(
            "test",
            &[1.5],
            &[path.to_string_lossy().into_owned()],
            qualified_metadata(),
        );

        assert!(!receipt.capture_complete);
        assert_eq!(receipt.images[0].status, "invalid_png_signature");

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn truncated_png_with_valid_signature_fails_decode() {
        let directory = test_directory();
        let path = directory.join("truncated.png");
        fs::write(&path, PNG_SIGNATURE).expect("truncated fixture should be written");
        let receipt = CaptureReceipt::build(
            "test",
            &[1.5],
            &[path.to_string_lossy().into_owned()],
            qualified_metadata(),
        );

        assert!(!receipt.capture_complete);
        assert_eq!(receipt.images[0].status, "decode_error");
        assert!(receipt.images[0].sha256.is_some());

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn capture_integrity_and_provenance_qualification_are_separate() {
        let directory = test_directory();
        let path = directory.join("frame.png");
        write_test_png(&path, 1, 1);
        let receipt = CaptureReceipt::build(
            "test",
            &[1.5],
            &[path.to_string_lossy().into_owned()],
            CaptureMetadata::default(),
        );

        assert!(receipt.capture_complete);
        assert_eq!(receipt.capture_status, "complete");
        assert_eq!(receipt.qualification_status, "incomplete_metadata");
        assert!(!receipt.qualification_issues.is_empty());

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn declared_dimensions_must_match_decoded_png() {
        let directory = test_directory();
        let path = directory.join("frame.png");
        write_test_png(&path, 2, 1);
        let mut metadata = qualified_metadata();
        metadata.width_px = Some(1920);
        metadata.height_px = Some(1080);
        let receipt = CaptureReceipt::build(
            "test",
            &[1.5],
            &[path.to_string_lossy().into_owned()],
            metadata,
        );

        assert!(!receipt.capture_complete);
        assert_eq!(receipt.images[0].status, "dimension_mismatch");
        assert_eq!(receipt.images[0].width_px, Some(2));
        assert_eq!(receipt.images[0].height_px, Some(1));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_schedule_path_mismatch_is_not_hidden() {
        let directory = test_directory();
        let path = directory.join("frame.png");
        write_test_png(&path, 1, 1);
        let receipt = CaptureReceipt::build(
            "test",
            &[1.5, 4.0],
            &[path.to_string_lossy().into_owned()],
            qualified_metadata(),
        );

        assert!(!receipt.capture_complete);
        assert_eq!(receipt.expected_image_count, 2);
        assert_eq!(receipt.images.len(), 2);
        assert_eq!(receipt.images[1].status, "missing_capture_path");

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn receipt_file_is_json_and_replaces_earlier_run() {
        let directory = test_directory();
        let path = directory.join("frame.png");
        write_test_png(&path, 1, 1);
        let receipt = CaptureReceipt::build(
            "test",
            &[1.5],
            &[path.to_string_lossy().into_owned()],
            qualified_metadata(),
        );

        let first = write_capture_receipt(&directory, "test", &receipt)
            .expect("first receipt should be written");
        let second = write_capture_receipt(&directory, "test", &receipt)
            .expect("receipt should be replaceable");
        assert_eq!(first, second);
        let encoded: serde_json::Value =
            serde_json::from_slice(&fs::read(&second).expect("receipt should be readable"))
                .expect("receipt should contain JSON");
        assert_eq!(encoded["schema_version"], 1);
        assert_eq!(encoded["capture_status"], "complete");

        let _ = fs::remove_dir_all(directory);
    }
}
