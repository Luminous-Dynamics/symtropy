fn main() {
    let revision = std::env::var("SOURCE_REVISION").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=BUILD_SOURCE_REVISION={revision}");
    println!("cargo:rerun-if-env-changed=SOURCE_REVISION");
}
