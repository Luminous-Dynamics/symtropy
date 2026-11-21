use std::process::ExitCode;

use civil_aero_benchmark::task_conformance::{
    ObservedProtocolValue, ProtocolConstraint, ProtocolObservation, ProtocolRequirement,
    audit_protocol, report_to_csv,
};

const PARTICIPANT_METADATA_REF: &str = concat!(
    "https://github.com/Drag-Prediction-Workshop/DPW8-Buffet/blob/",
    "4d7582633a2bed4f4fe1fe933af9a9b6f3e2ae43/",
    "TestCase1a/014_IISc/01_SU2_Symmetric_BC/",
    "DPW8-AePW4_ForceMoment_v5.dat",
);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("DPW-8 protocol conformance audit rejected: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    if std::env::args().len() != 1 {
        return Err("usage: dpw8_protocol_conformance_demo".to_owned());
    }

    let requirements = vec![
        ProtocolRequirement {
            key: "mach".to_owned(),
            description: "V1a freestream Mach number".to_owned(),
            constraint: ProtocolConstraint::NumericRange {
                minimum: 0.73,
                maximum: 0.73,
                absolute_tolerance: 0.0,
                unit: "1".to_owned(),
            },
        },
        ProtocolRequirement {
            key: "chord_reynolds_number".to_owned(),
            description: "V1a chord Reynolds number".to_owned(),
            constraint: ProtocolConstraint::NumericRange {
                minimum: 3_000_000.0,
                maximum: 3_000_000.0,
                absolute_tolerance: 0.0,
                unit: "1".to_owned(),
            },
        },
        ProtocolRequirement {
            key: "static_temperature_kelvin".to_owned(),
            description: "V1a static temperature".to_owned(),
            constraint: ProtocolConstraint::NumericRange {
                minimum: 271.0,
                maximum: 271.0,
                absolute_tolerance: 0.0,
                unit: "K".to_owned(),
            },
        },
        ProtocolRequirement {
            key: "angle_of_attack_deg".to_owned(),
            description: "V1a prescribed steady-polar angles".to_owned(),
            constraint: ProtocolConstraint::ExactText {
                expected: "1.36;1.50;2.50;3.00;3.10".to_owned(),
                case_sensitive: true,
            },
        },
        ProtocolRequirement {
            key: "sidewall_boundary_condition".to_owned(),
            description: "V1a sidewall boundary condition".to_owned(),
            constraint: ProtocolConstraint::ExactText {
                expected: "periodic".to_owned(),
                case_sensitive: false,
            },
        },
        ProtocolRequirement {
            key: "grid_level_count".to_owned(),
            description: "V1a grid-family count (four required, six desirable)".to_owned(),
            constraint: ProtocolConstraint::MinimumDesirableCount {
                minimum: 4,
                desirable: 6,
            },
        },
        ProtocolRequirement {
            key: "reproduction_artifact_identity".to_owned(),
            description: "Geometry, mesh, executable, and solver-input byte identities".to_owned(),
            constraint: ProtocolConstraint::ExactText {
                expected: "geometry_mesh_solver_build_and_config_byte_pins_present".to_owned(),
                case_sensitive: true,
            },
        },
    ];

    let observations = vec![
        ProtocolObservation {
            key: "mach".to_owned(),
            value: Some(ObservedProtocolValue::Number(0.73)),
            source_reference: PARTICIPANT_METADATA_REF.to_owned(),
            note: "Value transcribed from the pinned participant submission metadata".to_owned(),
        },
        ProtocolObservation {
            key: "chord_reynolds_number".to_owned(),
            value: Some(ObservedProtocolValue::Number(3_000_000.0)),
            source_reference: PARTICIPANT_METADATA_REF.to_owned(),
            note: "Value transcribed from the pinned participant submission metadata".to_owned(),
        },
        ProtocolObservation {
            key: "static_temperature_kelvin".to_owned(),
            value: Some(ObservedProtocolValue::Number(271.0)),
            source_reference: PARTICIPANT_METADATA_REF.to_owned(),
            note: "Value transcribed from the pinned participant submission metadata".to_owned(),
        },
        ProtocolObservation {
            key: "angle_of_attack_deg".to_owned(),
            value: Some(ObservedProtocolValue::Text(
                "1.36;1.50;2.50;3.00;3.10".to_owned(),
            )),
            source_reference: PARTICIPANT_METADATA_REF.to_owned(),
            note: "Five angles transcribed from participant metadata".to_owned(),
        },
        ProtocolObservation {
            key: "sidewall_boundary_condition".to_owned(),
            value: Some(ObservedProtocolValue::Text("symmetric".to_owned())),
            source_reference: PARTICIPANT_METADATA_REF.to_owned(),
            note: "Symmetric observed; periodic required. No physical equivalence inferred"
                .to_owned(),
        },
        ProtocolObservation {
            key: "grid_level_count".to_owned(),
            value: Some(ObservedProtocolValue::Count(4)),
            source_reference: PARTICIPANT_METADATA_REF.to_owned(),
            note: "Four levels meet the minimum; six are desirable".to_owned(),
        },
    ];

    let report = audit_protocol(
        "AIAA-DPW8-V1A-ONERA-OAT15A",
        "DPW8-V1a-ONERA-OAT15A",
        "participant-014.01-commit-4d7582633a2bed4f4fe1fe933af9a9b6f3e2ae43",
        &requirements,
        &observations,
    )?;

    let conforming = report
        .findings
        .iter()
        .filter(|finding| finding.status.as_str() == "conforms")
        .count();
    let deviations = report
        .findings
        .iter()
        .filter(|finding| finding.status.as_str() == "deviates")
        .count();
    let minimum_only = report
        .findings
        .iter()
        .filter(|finding| finding.status.as_str() == "meets_minimum_below_desirable")
        .count();
    let unknown = report
        .findings
        .iter()
        .filter(|finding| finding.status.as_str() == "unknown")
        .count();

    print!("{}", report_to_csv(&report));
    eprintln!(
        concat!(
            "dpw8_protocol_conformance_known_answer overall_status={} findings={} ",
            "conforms={} deviations={} minimum_below_desirable={} unknown={} ",
            "no_solver_execution=true no_physical_validation=true"
        ),
        report.overall_status,
        report.findings.len(),
        conforming,
        deviations,
        minimum_only,
        unknown
    );
    Ok(())
}
