use ccx_mms_p1_element_oracle::run_patch_probe;
use std::env;
use std::process::ExitCode;

fn is_sha40(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn main() -> ExitCode {
    let source_sha = env::var("MMS_BENCH_SOURCE_SHA").unwrap_or_default();
    let checkout_sha = env::var("MMS_BENCH_CHECKOUT_SHA").unwrap_or_default();
    if !is_sha40(&source_sha) || !source_sha.bytes().all(|b| !b.is_ascii_uppercase()) {
        eprintln!("MMS_BENCH_SOURCE_SHA must be a 40-character lowercase Git SHA");
        return ExitCode::FAILURE;
    }
    if !is_sha40(&checkout_sha) || !checkout_sha.bytes().all(|b| !b.is_ascii_uppercase()) {
        eprintln!("MMS_BENCH_CHECKOUT_SHA must be a 40-character lowercase Git SHA");
        return ExitCode::FAILURE;
    }

    let probe = match run_patch_probe() {
        Ok(report) => report,
        Err(error) => {
            eprintln!("patch probe failed to execute: {error}");
            return ExitCode::FAILURE;
        }
    };
    let passed = probe.passed();
    println!(
        concat!(
            "{{\"schema\":\"ccx-mms-p1-element-patch-v1\",",
            "\"source_sha\":\"{}\",\"checkout_sha\":\"{}\",",
            "\"solver_executed\":false,",
            "\"elements\":{},\"interior_nodes\":{},",
            "\"max_strain_error\":{:.17e},",
            "\"max_stiffness_symmetry_defect\":{:.17e},",
            "\"max_interior_force_linf\":{:.17e},",
            "\"max_rigid_mode_force_linf\":{:.17e},",
            "\"element_patch_pass\":{},",
            "\"qualification_pass\":false}}"
        ),
        source_sha,
        checkout_sha,
        probe.elements,
        probe.interior_nodes,
        probe.max_strain_error,
        probe.max_stiffness_symmetry_defect,
        probe.max_interior_force_linf,
        probe.max_rigid_mode_force_linf,
        passed
    );
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
