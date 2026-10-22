use ccx_mms_p1_element_oracle::run_global_mesh_conditioning_sweep;
use std::env;
use std::process::ExitCode;

fn is_lowercase_sha40(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn main() -> ExitCode {
    let source_sha = env::var("MMS_BENCH_SOURCE_SHA").unwrap_or_default();
    let checkout_sha = env::var("MMS_BENCH_CHECKOUT_SHA").unwrap_or_default();
    if !is_lowercase_sha40(&source_sha) {
        eprintln!("MMS_BENCH_SOURCE_SHA must be a 40-character lowercase Git SHA");
        return ExitCode::FAILURE;
    }
    if !is_lowercase_sha40(&checkout_sha) {
        eprintln!("MMS_BENCH_CHECKOUT_SHA must be a 40-character lowercase Git SHA");
        return ExitCode::FAILURE;
    }

    let sweep = match run_global_mesh_conditioning_sweep() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("global mesh conditioning sweep failed to execute: {error}");
            return ExitCode::FAILURE;
        }
    };
    let global_conditioning_pass = sweep.windows(2).all(|pair| {
        pair[1].cells_per_side > pair[0].cells_per_side
            && pair[1].constrained_global_condition_number
                > pair[0].constrained_global_condition_number
    });
    let records = sweep
        .iter()
        .map(|point| {
            format!(
                concat!(
                    "{{\"cells_per_side\":{},",
                    "\"free_dof_count\":{},",
                    "\"smallest_eigenvalue\":{:.17e},",
                    "\"largest_eigenvalue\":{:.17e},",
                    "\"constrained_global_condition_number\":{:.17e}}}"
                ),
                point.cells_per_side,
                point.free_dof_count,
                point.smallest_eigenvalue,
                point.largest_eigenvalue,
                point.constrained_global_condition_number
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    println!(
        concat!(
            "{{\"schema\":\"ccx-mms-p1-global-condition-v1\",",
            "\"source_sha\":\"{}\",\"checkout_sha\":\"{}\",",
            "\"solver_executed\":false,",
            "\"qualification_pass\":false,",
            "\"global_conditioning_pass\":{},",
            "\"sweep\":[{}]}}"
        ),
        source_sha,
        checkout_sha,
        global_conditioning_pass,
        records
    );
    if global_conditioning_pass
        && sweep.last().is_some_and(|point| {
            point.constrained_global_condition_number > 20.0
        })
    {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
