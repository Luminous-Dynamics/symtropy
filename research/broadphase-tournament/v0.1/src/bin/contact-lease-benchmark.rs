use broadphase_tournament::contact_reuse_lease::{
    ContactFeatureIdentity, ContactGeometryLease, ContactLeaseDisposition,
    RelativeMotionEnvelope, SeparationInterval, evaluate_contact_geometry_lease,
};
use incremental_aabb_tree_reference::{ProxyId, ProxyPair};
use std::hint::black_box;
use std::time::Instant;

#[derive(Clone, Copy)]
enum Scenario {
    IdleReuse,
    LowRelativeMotion,
    MotionRefresh,
    RotationRefresh,
    FeatureRefresh,
    AmbiguousGap,
    Separated,
    GenerationRefresh,
    MissingFeature,
}

impl Scenario {
    const ALL: [Self; 9] = [
        Self::IdleReuse,
        Self::LowRelativeMotion,
        Self::MotionRefresh,
        Self::RotationRefresh,
        Self::FeatureRefresh,
        Self::AmbiguousGap,
        Self::Separated,
        Self::GenerationRefresh,
        Self::MissingFeature,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::IdleReuse => "idle_reuse",
            Self::LowRelativeMotion => "low_relative_motion",
            Self::MotionRefresh => "motion_refresh",
            Self::RotationRefresh => "rotation_refresh",
            Self::FeatureRefresh => "feature_refresh",
            Self::AmbiguousGap => "ambiguous_gap",
            Self::Separated => "separated",
            Self::GenerationRefresh => "generation_refresh",
            Self::MissingFeature => "missing_feature",
        }
    }
}

#[derive(Default)]
struct Counts {
    reuse: u64,
    ended: u64,
    refresh: u64,
    unsupported: u64,
    errors: u64,
}

fn run_scenario<const D: usize>(scenario: Scenario, iterations: usize) -> (Counts, u128) {
    let mut counts = Counts::default();
    let start = Instant::now();

    for index in 0..iterations {
        let left_id = (index as u64).saturating_mul(2);
        let right_id = left_id.saturating_add(1);
        let Some(pair) = ProxyPair::new(ProxyId(left_id), ProxyId(right_id)) else {
            counts.errors += 1;
            continue;
        };
        let feature = ContactFeatureIdentity {
            feature_a: 17,
            feature_b: 29,
        };
        let mut lease = ContactGeometryLease {
            pair,
            generations: [3, 4],
            shape_revisions: [7, 8],
            feature_identity: Some(feature),
            feature_margin_lower_bound: 0.5,
            normal_variation_upper_bound_rad: 0.01,
            normal_variation_limit_rad: 0.05,
            updated_gap: SeparationInterval {
                lower: -0.005,
                upper: 0.005,
            },
            contact_slop: 0.01,
            maximum_penetration: 0.02,
        };
        let mut translation = [0.0; D];
        let mut angle = 0.0;
        let mut second_plane_angle = 0.0;
        let mut generations = [3, 4];
        let mut current_feature = Some(feature);

        match scenario {
            Scenario::IdleReuse => {}
            Scenario::LowRelativeMotion => translation[0] = 0.1,
            Scenario::MotionRefresh => translation[0] = 0.6,
            Scenario::RotationRefresh => {
                angle = 0.6;
                if D >= 4 {
                    second_plane_angle = 0.45;
                }
            },
            Scenario::FeatureRefresh => {
                current_feature = Some(ContactFeatureIdentity {
                    feature_a: 17,
                    feature_b: 30,
                });
            }
            Scenario::AmbiguousGap => lease.updated_gap.upper = 0.02,
            Scenario::Separated => {
                lease.updated_gap = SeparationInterval {
                    lower: 0.02,
                    upper: 0.03,
                };
            }
            Scenario::GenerationRefresh => generations[1] += 1,
            Scenario::MissingFeature => current_feature = None,
        }

        let mut relative_rotation_delta_abs_upper = [[0.0; D]; D];
        if D >= 2 {
            let cosine_delta = angle.cos() - 1.0;
            let sine = angle.sin();
            relative_rotation_delta_abs_upper[0][0] = cosine_delta.abs();
            relative_rotation_delta_abs_upper[0][1] = sine.abs();
            relative_rotation_delta_abs_upper[1][0] = sine.abs();
            relative_rotation_delta_abs_upper[1][1] = cosine_delta.abs();
        }
        if D >= 4 {
            let cosine_delta = second_plane_angle.cos() - 1.0;
            let sine = second_plane_angle.sin();
            relative_rotation_delta_abs_upper[2][2] = cosine_delta.abs();
            relative_rotation_delta_abs_upper[2][3] = sine.abs();
            relative_rotation_delta_abs_upper[3][2] = sine.abs();
            relative_rotation_delta_abs_upper[3][3] = cosine_delta.abs();
        }
        let motion = RelativeMotionEnvelope {
            relative_translation_delta: translation,
            relative_rotation_delta_abs_upper,
            maximum_lever_arm_radius: 2.0,
            rounding_allowance: 1.0e-12,
        };
        match evaluate_contact_geometry_lease(
            black_box(&lease),
            black_box(pair),
            black_box(generations),
            black_box([7, 8]),
            black_box(current_feature),
            black_box(&motion),
        ) {
            Ok(decision) => match black_box(decision.disposition) {
                ContactLeaseDisposition::ReuseCertified => counts.reuse += 1,
                ContactLeaseDisposition::ContactEnded => counts.ended += 1,
                ContactLeaseDisposition::RefreshRequired(_) => counts.refresh += 1,
                ContactLeaseDisposition::UnsupportedGeometry => counts.unsupported += 1,
            },
            Err(_) => counts.errors += 1,
        }
    }

    (counts, start.elapsed().as_nanos())
}

fn run_dimension<const D: usize>(iterations: usize) {
    for scenario in Scenario::ALL {
        let (counts, elapsed_ns) = run_scenario::<D>(scenario, iterations);
        let ns_per_check = elapsed_ns as f64 / iterations.max(1) as f64;
        println!(
            "{D},{},{iterations},{},{},{},{},{errors},{elapsed_ns},{ns_per_check:.3}",
            scenario.name(),
            counts.reuse,
            counts.ended,
            counts.refresh,
            counts.unsupported,
            errors = counts.errors,
        );
        assert_eq!(
            counts.reuse
                + counts.ended
                + counts.refresh
                + counts.unsupported
                + counts.errors,
            iterations as u64,
            "outcome accounting must cover every requested check"
        );
        assert_eq!(counts.errors, 0, "unexpected lease evaluation errors");
    }
}

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100_000);
    println!(
        "dimension,scenario,requested_checks,reuse_certified,contact_ended,refresh_required,unsupported,errors,elapsed_ns,ns_per_check"
    );
    run_dimension::<2>(iterations);
    run_dimension::<3>(iterations);
    run_dimension::<4>(iterations);
}
