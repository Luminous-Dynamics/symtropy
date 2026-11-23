// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic benchmark tournament for contact-manifold reduction policies.
//!
//! The output is CSV so policy adapters can be compared on identical frozen
//! candidate geometry, friction assumptions and wrench-direction sets.

use wrench_support_oracle::general::{
    ContactFrame, ContactWrenchModel, ExactContactSelectorConfig,
    GreedyContactSelectorConfig, select_exact_contact_subset, select_greedy_contact_subset,
};
use wrench_support_oracle::{
    ContactPatch, ContactPoint, ExactSelectorConfig, GreedySelectorConfig,
    WrenchDirection, WrenchSupportReport, evaluate_retained_set, select_deepest_area_subset,
    select_exact_subset, select_greedy_subset,
};

fn axes() -> Vec<WrenchDirection> {
    vec![
        WrenchDirection {
            id: "fx-positive".into(),
            components: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        WrenchDirection {
            id: "fx-negative".into(),
            components: [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        WrenchDirection {
            id: "fy-positive".into(),
            components: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        },
        WrenchDirection {
            id: "fy-negative".into(),
            components: [0.0, -1.0, 0.0, 0.0, 0.0, 0.0],
        },
        WrenchDirection {
            id: "fz-positive".into(),
            components: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        },
        WrenchDirection {
            id: "mx-positive".into(),
            components: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        },
        WrenchDirection {
            id: "mx-negative".into(),
            components: [0.0, 0.0, 0.0, -1.0, 0.0, 0.0],
        },
        WrenchDirection {
            id: "my-positive".into(),
            components: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        },
        WrenchDirection {
            id: "my-negative".into(),
            components: [0.0, 0.0, 0.0, 0.0, -1.0, 0.0],
        },
        WrenchDirection {
            id: "mz-positive".into(),
            components: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        },
        WrenchDirection {
            id: "mz-negative".into(),
            components: [0.0, 0.0, 0.0, 0.0, 0.0, -1.0],
        },
    ]
}

fn planar_point(id: &str, x: f64, y: f64, depth: f64) -> ContactPoint {
    ContactPoint {
        id: id.into(),
        position: [x, y, 0.0],
        depth,
    }
}

fn print_planar(
    scenario: &str,
    policy: &str,
    candidate_count: usize,
    report: &WrenchSupportReport,
    direction_count: usize,
) {
    let support_evaluations =
        report.subsets_evaluated * direction_count as u64 * 2;
    println!(
        "planar,{scenario},{policy},{candidate_count},{},{:.9},{:.9},{},{},{}",
        report.retained_point_ids.len(),
        report.maximum_normalized_deficit,
        report.mean_normalized_deficit,
        report.subsets_evaluated,
        support_evaluations,
        report.retained_point_ids.join("|")
    );
}

fn run_planar_scenario(
    name: &str,
    patch: ContactPatch,
    keep: usize,
    include_best_n: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let exact = select_exact_subset(
        &patch,
        ExactSelectorConfig {
            keep,
            maximum_subsets: 1_000_000,
        },
    )?;
    print_planar(name, "exact-oracle", patch.points.len(), &exact, patch.directions.len());

    let greedy = select_greedy_subset(
        &patch,
        GreedySelectorConfig {
            keep,
            maximum_evaluations: 100_000,
        },
    )?;
    print_planar(name, "greedy", patch.points.len(), &greedy, patch.directions.len());

    if include_best_n {
        let best_n = select_deepest_area_subset(&patch, keep)?;
        print_planar(name, "symtropy-best-n", patch.points.len(), &best_n, patch.directions.len());
    }

    // Sanity-check the exported retained-set evaluator against the exact report.
    let evaluated = evaluate_retained_set(&patch, &exact.retained_point_ids)?;
    if (evaluated.maximum_normalized_deficit - exact.maximum_normalized_deficit).abs() > 1.0e-12 {
        return Err("retained-set evaluator disagrees with exact selector".into());
    }
    Ok(())
}

fn general_contact(
    id: &str,
    position: [f64; 3],
    normal: [f64; 3],
    tangent_u_hint: [f64; 3],
    friction_axes: [f64; 2],
    load_bounds: [f64; 2],
) -> ContactFrame {
    ContactFrame {
        id: id.into(),
        position,
        normal,
        tangent_u_hint,
        friction_u: friction_axes[0],
        friction_v: friction_axes[1],
        min_normal_fraction: load_bounds[0],
        max_normal_fraction: load_bounds[1],
    }
}

fn run_general_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let model = ContactWrenchModel {
        contacts: vec![
            general_contact(
                "a",
                [-1.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [0.45, 0.25],
                [0.0, 1.0],
            ),
            general_contact(
                "b",
                [1.0, -1.0, 0.0],
                [0.0, 0.08, 0.997],
                [1.0, 0.0, 0.0],
                [0.35, 0.2],
                [0.0, 1.0],
            ),
            general_contact(
                "c",
                [-1.0, 1.0, 0.0],
                [0.08, 0.0, 0.997],
                [0.0, 1.0, 0.0],
                [0.25, 0.4],
                [0.0, 1.0],
            ),
            general_contact(
                "d",
                [1.0, 1.0, 0.0],
                [0.04, 0.04, 0.9984],
                [1.0, -1.0, 0.0],
                [0.3, 0.3],
                [0.0, 1.0],
            ),
            general_contact(
                "e",
                [0.0, 1.5, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [0.2, 0.5],
                [0.0, 1.0],
            ),
        ],
        cone_sides: 32,
        characteristic_length: 1.0,
    };
    let directions = axes();
    let exact = select_exact_contact_subset(
        &model,
        &directions,
        1.0e-6,
        ExactContactSelectorConfig {
            keep: 3,
            maximum_subsets: 1_000_000,
        },
    )?;
    println!(
        "general,varied-normal-anisotropic,exact-oracle,{},{},{:.9},{:.9},{},{},{}",
        model.contacts.len(),
        exact.retained_contact_ids.len(),
        exact.maximum_normalized_deficit,
        exact.mean_normalized_deficit,
        exact.contact_sets_evaluated,
        exact.support_evaluations,
        exact.retained_contact_ids.join("|")
    );

    let greedy = select_greedy_contact_subset(
        &model,
        &directions,
        1.0e-6,
        GreedyContactSelectorConfig {
            keep: 3,
            maximum_evaluations: 100_000,
        },
    )?;
    println!(
        "general,varied-normal-anisotropic,greedy,{},{},{:.9},{:.9},{},{},{}",
        model.contacts.len(),
        greedy.retained_contact_ids.len(),
        greedy.maximum_normalized_deficit,
        greedy.mean_normalized_deficit,
        greedy.contact_sets_evaluated,
        greedy.support_evaluations,
        greedy.retained_contact_ids.join("|")
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "model,scenario,policy,candidates,retained,max_normalized_deficit,mean_normalized_deficit,sets_evaluated,support_evaluations,retained_ids"
    );

    let asymmetric = ContactPatch {
        points: vec![
            planar_point("p0", -0.8053331255487975, -1.0022880202777662, 0.1),
            planar_point("p1", -0.00663309450661842, 0.24817392184595977, 0.9),
            planar_point("p2", -0.9469860377145608, -1.4553152497193032, 0.2),
            planar_point("p3", -0.0866003133286175, 0.6847299845497847, 0.8),
            planar_point("p4", 1.25580147532063, 0.37660201720639197, 0.3),
        ],
        directions: axes(),
        friction_coefficient: 0.6,
        characteristic_length: 1.0,
        support_floor: 0.1,
    };
    run_planar_scenario("asymmetric-five", asymmetric, 3, true)?;

    let thin_patch = ContactPatch {
        points: vec![
            planar_point("a0", -2.0, -0.2, 0.2),
            planar_point("a1", -2.0, 0.2, 0.3),
            planar_point("b0", -1.2, -0.2, 0.4),
            planar_point("b1", -1.2, 0.2, 0.5),
            planar_point("c0", 1.2, -0.2, 0.6),
            planar_point("c1", 1.2, 0.2, 0.7),
            planar_point("d0", 2.0, -0.2, 0.8),
            planar_point("d1", 2.0, 0.2, 0.9),
        ],
        directions: axes(),
        friction_coefficient: 0.3,
        characteristic_length: 2.0,
        support_floor: 1.0e-6,
    };
    run_planar_scenario("thin-eight", thin_patch, 4, true)?;

    run_general_scenario()?;
    Ok(())
}
