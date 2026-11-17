use quadruped_static_support_oracle::{ContactPoint, SupportFrame, SupportSnapshot, evaluate};

fn main() {
    let snapshot = SupportSnapshot {
        robot_id: "quadruped:reference".to_owned(),
        morphology_revision: "morphology:v1".to_owned(),
        support_frame: SupportFrame {
            frame_id: "robot:base".to_owned(),
            plane_id: "floor:reference".to_owned(),
            revision: 1,
        },
        com_projection_m: [0.0, 0.0],
        com_uncertainty_m: 0.01,
        required_margin_m: 0.1,
        contacts: vec![
            contact("front-left", [-0.5, 0.3]),
            contact("front-right", [0.5, 0.3]),
            contact("rear-left", [-0.5, -0.3]),
            contact("rear-right", [0.5, -0.3]),
        ],
    };

    let result = evaluate(&snapshot);
    println!("suite=QUADRUPED-STATIC-SUPPORT-ORACLE-001");
    println!("disposition={:?}", result.disposition);
    println!("hull_area_m2={:?}", result.hull_area_m2);
    println!(
        "nominal_signed_margin_m={:?}",
        result.nominal_signed_margin_m
    );
    println!("conservative_margin_m={:?}", result.conservative_margin_m);
    println!("required_margin_m={}", result.required_margin_m);
    println!("note=analytic static support only; no dynamic stability or actuation authority");
}

fn contact(foot_id: &str, position_m: [f64; 2]) -> ContactPoint {
    ContactPoint {
        foot_id: foot_id.to_owned(),
        frame_id: "robot:base".to_owned(),
        plane_id: "floor:reference".to_owned(),
        frame_revision: 1,
        position_m,
        uncertainty_radius_m: 0.01,
    }
}
