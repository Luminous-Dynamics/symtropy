//! Audit one exact-schema CSV mass ledger and emit deterministic screening-only properties.
//!
//! Usage from the repository root:
//! cargo run --locked --manifest-path tools/civil-aero-benchmark/Cargo.toml --bin mass_properties_audit -- <fixture.csv>

use civil_aero_benchmark::mass_properties::{
    MassGeometry, MassItem, MassItemCategory, MassLedger,
};
use std::fs;

const HEADER: &str =
    "configuration_sha256,item_id,category,mass_kg,geometry_kind,x_m,y_m,z_m,Ixx,Iyy,Izz,Ixy,Ixz,Iyz";
const OUTPUT_HEADER: &str =
    "configuration_sha256,status,evidence_level,total_mass_kg,known_mass_subtotal_kg,cg_x_m,cg_y_m,cg_z_m,Ixx,Iyy,Izz,Ixy,Ixz,Iyz,unknown_mass_item_ids,unknown_geometry_item_ids";

fn main() {
    if let Err(error) = run() {
        eprintln!("mass_properties_audit_error={error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args();
    let _program = args.next();
    let path = args
        .next()
        .ok_or_else(|| "usage: mass_properties_audit <fixture.csv>".to_owned())?;
    if args.next().is_some() {
        return Err("expected exactly one fixture path".to_owned());
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path))?;
    let ledger = parse_ledger(&content)?;
    let report = ledger.evaluate()?;

    println!("{OUTPUT_HEADER}");
    let status = match report.status {
        civil_aero_benchmark::mass_properties::MassPropertiesStatus::Complete => "complete",
        civil_aero_benchmark::mass_properties::MassPropertiesStatus::Indeterminate => {
            "indeterminate"
        }
    };
    let cg = report.center_of_gravity_m;
    let inertia = report.inertia_tensor_about_cg_kg_m2;
    let row = [
        report.configuration_sha256,
        status.to_owned(),
        report.evidence_level.to_owned(),
        format_option(report.total_mass_kg),
        format!("{:.12}", report.known_mass_subtotal_kg),
        format_option(cg.map(|value| value[0])),
        format_option(cg.map(|value| value[1])),
        format_option(cg.map(|value| value[2])),
        format_option(inertia.map(|value| value[0])),
        format_option(inertia.map(|value| value[1])),
        format_option(inertia.map(|value| value[2])),
        format_option(inertia.map(|value| value[3])),
        format_option(inertia.map(|value| value[4])),
        format_option(inertia.map(|value| value[5])),
        report.unknown_mass_item_ids.join(";"),
        report.unknown_geometry_item_ids.join(";"),
    ];
    println!("{}", row.join(","));
    Ok(())
}

fn parse_ledger(content: &str) -> Result<MassLedger, String> {
    let mut lines = content.lines();
    if lines.next() != Some(HEADER) {
        return Err("input header does not match the exact mass-properties schema".to_owned());
    }

    let mut configuration_sha256: Option<String> = None;
    let mut items = Vec::new();
    for (zero_based, line) in lines.enumerate() {
        let line_number = zero_based + 2;
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 14 {
            return Err(format!(
                "line {line_number}: expected 14 comma-separated fields, got {}",
                fields.len()
            ));
        }

        let config = fields[0].trim();
        if config.is_empty() || !is_sha256_hex(config) {
            return Err(format!(
                "line {line_number}: configuration_sha256 must be 64 hexadecimal characters"
            ));
        }
        match &configuration_sha256 {
            Some(existing) if existing != config => {
                return Err(format!(
                    "line {line_number}: mixed configuration identities in one mass ledger"
                ));
            }
            None => configuration_sha256 = Some(config.to_owned()),
            _ => {}
        }

        let id = fields[1].trim();
        let category = parse_category(fields[2].trim())
            .ok_or_else(|| format!("line {line_number}: unknown mass category"))?;
        let mass_kg = parse_optional_f64(fields[3], "mass_kg", line_number)?;
        let geometry_kind = fields[4].trim();
        let x = parse_optional_f64(fields[5], "x_m", line_number)?;
        let y = parse_optional_f64(fields[6], "y_m", line_number)?;
        let z = parse_optional_f64(fields[7], "z_m", line_number)?;
        let tensor = [
            parse_optional_f64(fields[8], "Ixx", line_number)?,
            parse_optional_f64(fields[9], "Iyy", line_number)?,
            parse_optional_f64(fields[10], "Izz", line_number)?,
            parse_optional_f64(fields[11], "Ixy", line_number)?,
            parse_optional_f64(fields[12], "Ixz", line_number)?,
            parse_optional_f64(fields[13], "Iyz", line_number)?,
        ];

        let geometry = match geometry_kind {
            "point" => {
                if tensor.iter().any(Option::is_some) {
                    return Err(format!(
                        "line {line_number}: point mass must not declare intrinsic inertia"
                    ));
                }
                MassGeometry::PointMass {
                    position_m: [
                        x.ok_or_else(|| format!("line {line_number}: point mass requires x_m"))?,
                        y.ok_or_else(|| format!("line {line_number}: point mass requires y_m"))?,
                        z.ok_or_else(|| format!("line {line_number}: point mass requires z_m"))?,
                    ],
                }
            }
            "rigid_body" => {
                let mut values = [0.0_f64; 6];
                for (index, value) in tensor.into_iter().enumerate() {
                    values[index] = value.ok_or_else(|| {
                        format!("line {line_number}: rigid_body requires all six inertia entries")
                    })?;
                }
                MassGeometry::RigidBody {
                    center_of_mass_m: [
                        x.ok_or_else(|| format!("line {line_number}: rigid_body requires x_m"))?,
                        y.ok_or_else(|| format!("line {line_number}: rigid_body requires y_m"))?,
                        z.ok_or_else(|| format!("line {line_number}: rigid_body requires z_m"))?,
                    ],
                    inertia_tensor_about_center_kg_m2: values,
                }
            }
            "unknown" => {
                if x.is_some() || y.is_some() || z.is_some() || tensor.iter().any(Option::is_some)
                {
                    return Err(format!(
                        "line {line_number}: unknown geometry must not include coordinates or inertia"
                    ));
                }
                MassGeometry::Unknown
            }
            _ => {
                return Err(format!(
                    "line {line_number}: geometry_kind must be point, rigid_body, or unknown"
                ));
            }
        };

        items.push(MassItem {
            id: id.to_owned(),
            category,
            mass_kg,
            geometry,
        });
    }

    let configuration_sha256 = configuration_sha256
        .ok_or_else(|| "mass ledger contains no item records".to_owned())?;
    Ok(MassLedger {
        configuration_sha256,
        items,
    })
}

fn parse_category(value: &str) -> Option<MassItemCategory> {
    match value {
        "primary_structure" => Some(MassItemCategory::PrimaryStructure),
        "secondary_structure" => Some(MassItemCategory::SecondaryStructure),
        "propulsion" => Some(MassItemCategory::Propulsion),
        "energy_storage" => Some(MassItemCategory::EnergyStorage),
        "electrical_distribution" => Some(MassItemCategory::ElectricalDistribution),
        "thermal_management" => Some(MassItemCategory::ThermalManagement),
        "flight_controls" => Some(MassItemCategory::FlightControls),
        "avionics" => Some(MassItemCategory::Avionics),
        "landing_gear" => Some(MassItemCategory::LandingGear),
        "interior_and_furnishings" => Some(MassItemCategory::InteriorAndFurnishings),
        "safety_equipment" => Some(MassItemCategory::SafetyEquipment),
        "fuel_and_fluids" => Some(MassItemCategory::FuelAndFluids),
        "payload" => Some(MassItemCategory::Payload),
        "other" => Some(MassItemCategory::Other),
        _ => None,
    }
}

fn parse_optional_f64(value: &str, field: &str, line_number: usize) -> Result<Option<f64>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<f64>()
        .map_err(|error| format!("line {line_number}: invalid {field}: {error}"))?;
    if !parsed.is_finite() {
        return Err(format!("line {line_number}: {field} must be finite"));
    }
    Ok(Some(parsed))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn format_option(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.12}"))
        .unwrap_or_default()
}
