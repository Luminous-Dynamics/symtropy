//! Manufactured-solution mesh-convergence verification for the pinned CCX truss profile.
//!
//! Generates a deterministic 1D axial-bar deck for u(x)=U sin(pi*x/L), verifies that the
//! deck bytes match the requested mesh, parses every U output node, checks nodal values,
//! then measures normalized L2 and H1-seminorm errors of the piecewise-linear FE field.
//! The receipt is numerical code-verification evidence only, not physical validation.

use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs;

const YOUNGS_MODULUS_PA: f64 = 210.0e9;
const AREA_M2: f64 = 0.01;
const LENGTH_M: f64 = 1.0;
const AMPLITUDE_M: f64 = 1.0e-6;
const TARGET_TIME: f64 = 1.0;
const PI: f64 = std::f64::consts::PI;
const NODE_RELATIVE_ERROR_LIMIT: f64 = 5.0e-6;
const TRANSVERSE_DISPLACEMENT_LIMIT_M: f64 = 1.0e-12;

// Eight-point Gauss-Legendre quadrature on [-1,1].
const GAUSS_X: [f64; 8] = [
    -0.960_289_856_497_536_3,
    -0.796_666_477_413_626_7,
    -0.525_532_409_916_329,
    -0.183_434_642_495_649_8,
    0.183_434_642_495_649_8,
    0.525_532_409_916_329,
    0.796_666_477_413_626_7,
    0.960_289_856_497_536_3,
];
const GAUSS_W: [f64; 8] = [
    0.101_228_536_290_376_3,
    0.222_381_034_453_374_5,
    0.313_706_645_877_887_3,
    0.362_683_783_378_362,
    0.362_683_783_378_362,
    0.313_706_645_877_887_3,
    0.222_381_034_453_374_5,
    0.101_228_536_290_376_3,
];

#[derive(Clone, Debug)]
struct NodeDisplacement {
    id: usize,
    ux_m: f64,
    uy_m: f64,
    uz_m: f64,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn supported_mesh(elements: usize) -> bool {
    [4, 8, 16, 32].contains(&elements)
}

fn nodal_load(elements: usize, node_index: usize) -> f64 {
    let h = LENGTH_M / elements as f64;
    let k = PI / LENGTH_M;
    let ea_u = YOUNGS_MODULUS_PA * AREA_M2 * AMPLITUDE_M;
    if node_index == 0 || node_index == elements {
        ea_u * (k - (k * h).sin() / h)
    } else {
        let x = node_index as f64 * h;
        2.0 * ea_u * (1.0 - (k * h).cos()) / h * (k * x).sin()
    }
}

fn expected_deck(elements: usize) -> Result<String, String> {
    if !supported_mesh(elements) {
        return Err(format!(
            "unsupported mesh size {elements}; expected one of 4, 8, 16, 32 elements"
        ));
    }
    let nodes = elements + 1;
    let h = LENGTH_M / elements as f64;
    let mut deck = String::new();
    deck.push_str("*HEADING\nPHYS-MMS-001 sine manufactured axial-bar solution\n*NODE\n");
    for node in 0..=elements {
        let x = node as f64 * h;
        deck.push_str(&format!(
            "{},{:.15e},0.000000000000000e0,0.000000000000000e0\n",
            node + 1,
            x
        ));
    }

    deck.push_str("*ELEMENT,TYPE=T3D2,ELSET=BAR\n");
    for element in 0..elements {
        deck.push_str(&format!("{},{},{}\n", element + 1, element + 1, element + 2));
    }

    deck.push_str("*NSET,NSET=ALL\n");
    for first in (1..=nodes).step_by(16) {
        let last = (first + 15).min(nodes);
        let ids = (first..=last)
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        deck.push_str(&ids);
        deck.push('\n');
    }
    deck.push_str(&format!("*NSET,NSET=SUPPORT\n1,{}\n", nodes));
    deck.push_str("*MATERIAL,NAME=STEEL\n*ELASTIC\n2.100000000000000e11,0.3\n");
    deck.push_str("*SOLID SECTION,ELSET=BAR,MATERIAL=STEEL\n1.000000000000000e-2\n");
    deck.push_str("*BOUNDARY\n");
    deck.push_str("1,1,3,0.0\n");
    for node in 2..nodes {
        deck.push_str(&format!("{node},2,3,0.0\n"));
    }
    deck.push_str(&format!("{nodes},1,3,0.0\n"));
    deck.push_str("*STEP\n*STATIC\n1.0,1.0,1.0e-5,1.0\n*CLOAD\n");
    for node_index in 0..=elements {
        deck.push_str(&format!(
            "{},{},{:.15e}\n",
            node_index + 1,
            1,
            nodal_load(elements, node_index)
        ));
    }
    deck.push_str("*NODE PRINT,NSET=ALL\nU\n*END STEP\n");
    Ok(deck)
}

fn parse_finite(text: &str, context: &str) -> Result<f64, String> {
    let normalized = text.trim().replace(['D', 'd'], "E");
    let value = normalized
        .parse::<f64>()
        .map_err(|_| format!("{context}: invalid numeric value {text:?}"))?;
    if !value.is_finite() {
        return Err(format!("{context}: non-finite numeric value {text:?}"));
    }
    Ok(value)
}

fn parse_node_displacements(
    dat: &str,
    elements: usize,
) -> Result<(f64, Vec<NodeDisplacement>), String> {
    let lines: Vec<&str> = dat.lines().collect();
    let headers: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, raw)| {
            raw.trim()
                .to_ascii_lowercase()
                .starts_with("displacements (vx,vy,vz) for set ")
                .then_some(index)
        })
        .collect();
    if headers.len() != 1 {
        return Err(format!(
            "expected exactly one displacement section, observed {}",
            headers.len()
        ));
    }

    let header_index = headers[0];
    let header = lines[header_index].trim();
    let prefix = "displacements (vx,vy,vz) for set ";
    let rest = header
        .get(prefix.len()..)
        .ok_or_else(|| "displacement header prefix is malformed".to_owned())?;
    let lower = rest.to_ascii_lowercase();
    let time_marker = " and time ";
    let marker_index = lower
        .find(time_marker)
        .ok_or_else(|| "displacement header omitted output time".to_owned())?;
    let set_name = rest[..marker_index].trim();
    if !set_name.eq_ignore_ascii_case("ALL") {
        return Err(format!(
            "displacement output set must be ALL, observed {set_name:?}"
        ));
    }
    let observed_time = parse_finite(
        &rest[marker_index + time_marker.len()..],
        "displacement header time",
    )?;
    if (observed_time - TARGET_TIME).abs() > 1.0e-8 * TARGET_TIME.max(1.0) {
        return Err(format!(
            "displacement output time {observed_time:.17e} differs from target {TARGET_TIME:.17e}"
        ));
    }

    let mut rows = Vec::new();
    let mut saw_data = false;
    for (index, raw) in lines.iter().enumerate().skip(header_index + 1) {
        let line = raw.trim();
        if line.is_empty() {
            if saw_data {
                break;
            }
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let node_id = match fields.first().copied().unwrap_or_default().parse::<usize>() {
            Ok(id) => id,
            Err(_) if saw_data => break,
            Err(_) => {
                return Err(format!(
                    "unexpected content following displacement header on .dat line {}",
                    index + 1
                ));
            }
        };
        if fields.len() != 4 {
            return Err(format!(
                "displacement row on .dat line {} must contain node ID and 3 components",
                index + 1
            ));
        }
        rows.push(NodeDisplacement {
            id: node_id,
            ux_m: parse_finite(fields[1], "ux")?,
            uy_m: parse_finite(fields[2], "uy")?,
            uz_m: parse_finite(fields[3], "uz")?,
        });
        saw_data = true;
    }

    if rows.len() != elements + 1 {
        return Err(format!(
            "expected {} displacement rows, observed {}",
            elements + 1,
            rows.len()
        ));
    }
    for (index, row) in rows.iter().enumerate() {
        if row.id != index + 1 {
            return Err(format!(
                "node output must be ordered and unique: row {index} carried node ID {}",
                row.id
            ));
        }
    }
    Ok((observed_time, rows))
}

fn normalized_errors(
    elements: usize,
    rows: &[NodeDisplacement],
) -> Result<(f64, f64, f64), String> {
    if rows.len() != elements + 1 {
        return Err("displacement row count does not match mesh size".to_owned());
    }
    let mut max_nodal_error: f64 = 0.0;
    for (index, row) in rows.iter().enumerate() {
        let x = LENGTH_M * index as f64 / elements as f64;
        let exact = AMPLITUDE_M * (PI * x / LENGTH_M).sin();
        max_nodal_error = max_nodal_error.max((row.ux_m - exact).abs() / AMPLITUDE_M);
        if row.uy_m.abs() > TRANSVERSE_DISPLACEMENT_LIMIT_M
            || row.uz_m.abs() > TRANSVERSE_DISPLACEMENT_LIMIT_M
        {
            return Err(format!(
                "transverse displacement exceeds the pinned axial profile at node {}",
                row.id
            ));
        }
    }

    let h = LENGTH_M / elements as f64;
    let mut l2_squared = 0.0;
    let mut h1_squared = 0.0;
    for element in 0..elements {
        let left = LENGTH_M * element as f64 / elements as f64;
        let right = left + h;
        let u0 = rows[element].ux_m / AMPLITUDE_M;
        let u1 = rows[element + 1].ux_m / AMPLITUDE_M;
        let slope = (u1 - u0) / h;
        for point in 0..GAUSS_X.len() {
            let x = 0.5 * (right - left) * GAUSS_X[point] + 0.5 * (right + left);
            let weight = 0.5 * h * GAUSS_W[point];
            let interpolation = u0 + (x - left) * slope;
            let exact = (PI * x / LENGTH_M).sin();
            let exact_derivative_scaled = (PI * x / LENGTH_M).cos();
            let l2_difference = interpolation - exact;
            let h1_difference = slope * LENGTH_M / PI - exact_derivative_scaled;
            l2_squared += weight * l2_difference * l2_difference / LENGTH_M;
            h1_squared += weight * h1_difference * h1_difference / LENGTH_M;
        }
    }
    if !max_nodal_error.is_finite()
        || !l2_squared.is_finite()
        || !h1_squared.is_finite()
        || l2_squared < 0.0
        || h1_squared < 0.0
    {
        return Err("non-finite or invalid manufactured-solution error".to_owned());
    }
    Ok((max_nodal_error, l2_squared.sqrt(), h1_squared.sqrt()))
}

fn verify_case(
    elements: usize,
    deck: &[u8],
    dat: &[u8],
    source_sha: &str,
    build_sha: &str,
    executable_sha: &str,
) -> serde_json::Value {
    let mut failure = None;
    let mut nodal_error = None;
    let mut l2_error = None;
    let mut h1_error = None;
    let mut observed_time = None;
    let expected = expected_deck(elements);
    match &expected {
        Ok(expected_deck_bytes) => {
            if deck != expected_deck_bytes.as_bytes() {
                failure = Some("input deck bytes differ from the deterministic manufactured fixture".to_owned());
            }
        }
        Err(error) => failure = Some(error.clone()),
    }
    if failure.is_none() {
        match std::str::from_utf8(dat) {
            Ok(dat_text) => match parse_node_displacements(dat_text, elements) {
                Ok((time, rows)) => {
                    observed_time = Some(time);
                    match normalized_errors(elements, &rows) {
                        Ok((nodal, l2, h1)) => {
                            nodal_error = Some(nodal);
                            l2_error = Some(l2);
                            h1_error = Some(h1);
                            if nodal > NODE_RELATIVE_ERROR_LIMIT {
                                failure = Some(format!(
                                    "maximum normalized nodal error {nodal:.6e} exceeds {:.6e}",
                                    NODE_RELATIVE_ERROR_LIMIT
                                ));
                            }
                        }
                        Err(error) => failure = Some(error),
                    }
                }
                Err(error) => failure = Some(error),
            },
            Err(_) => failure = Some(".dat output is not valid UTF-8/ASCII".to_owned()),
        }
    }
    let status = if failure.is_none() { "PASS" } else { "FAIL" };
    json!({
        "schema_version": "ccx-mms-sine-convergence-v0.1",
        "benchmark_id": "PHYS-MMS-001",
        "case_id": format!("PHYS-MMS-001-N{elements}"),
        "elements": elements,
        "nodes": elements + 1,
        "source_sha": source_sha,
        "build_manifest_sha256": build_sha,
        "solver_executable_sha256": executable_sha,
        "deck_sha256": sha256_hex(deck),
        "dat_sha256": sha256_hex(dat),
        "target_time": TARGET_TIME,
        "observed_time": observed_time,
        "analytic_solution": "u(x)=U*sin(pi*x/L); -EA*u''(x)=EA*(pi/L)^2*U*sin(pi*x/L); u(0)=u(L)=0",
        "load_discretization": "exact consistent nodal loads integral(N_i*EA*(pi/L)^2*U*sin(pi*x/L) dx)",
        "amplitude_m": AMPLITUDE_M,
        "nodal_max_normalized_error": nodal_error,
        "nodal_error_limit": NODE_RELATIVE_ERROR_LIMIT,
        "l2_interpolation_error_normalized": l2_error,
        "h1_seminorm_error_normalized": h1_error,
        "quadrature": "8-point Gauss-Legendre per element",
        "numerical_contract_status": status,
        "failure_reason": failure,
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "engineering_use_authorized": false,
        "claim_ceiling": "code-verification evidence for one linear 1D manufactured-solution family and this exact solver/input/output byte set; not experimental validation or a safety claim"
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("generate") if args.len() == 3 => {
            let elements: usize = args[2].parse()?;
            print!("{}", expected_deck(elements).map_err(std::io::Error::other)?);
            Ok(())
        }
        Some("verify") if args.len() == 8 => {
            let elements: usize = args[2].parse()?;
            let deck_path = &args[3];
            let dat_path = &args[4];
            let source_sha = &args[5];
            let build_sha = &args[6];
            let executable_sha = &args[7];
            let deck = fs::read(deck_path)?;
            let dat = fs::read(dat_path)?;
            let report = verify_case(
                elements,
                &deck,
                &dat,
                source_sha,
                build_sha,
                executable_sha,
            );
            println!("{}", serde_json::to_string(&report)?);
            Ok(())
        }
        _ => Err(
            "usage: mms_convergence generate <4|8|16|32> | verify <4|8|16|32> <deck> <dat> <source-sha> <build-sha256> <executable-sha256>"
                .into(),
        ),
    }
}
