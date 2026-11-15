use std::fmt::Write as _;
use std::io::{self, Write as _};

#[derive(Clone, Copy)]
struct Point {
    x: i64,
    y: i64,
    z: i64,
}

#[derive(Clone, Copy)]
enum Family {
    Linear,
    UpperSemicircle,
}

#[derive(Clone, Copy)]
enum Partition {
    Development,
    Holdout,
}

struct Lcg(u32);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    fn range(&mut self, upper_exclusive: u32) -> i64 {
        i64::from(self.next_u32() % upper_exclusive)
    }
}

fn distance(left: Point, right: Point) -> f64 {
    let dx = (left.x - right.x) as f64;
    let dy = (left.y - right.y) as f64;
    let dz = (left.z - right.z) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn point_segment_distance(point: Point, start: Point, end: Point) -> f64 {
    let ab = [
        (end.x - start.x) as f64,
        (end.y - start.y) as f64,
        (end.z - start.z) as f64,
    ];
    let ap = [
        (point.x - start.x) as f64,
        (point.y - start.y) as f64,
        (point.z - start.z) as f64,
    ];
    let denominator = ab.iter().map(|v| *v * *v).sum::<f64>();
    let parameter = if denominator == 0.0 {
        0.0
    } else {
        (ap.iter().zip(ab).map(|(&a, b)| a * b).sum::<f64>() / denominator).clamp(0.0, 1.0)
    };
    let closest_x = start.x as f64 + ab[0] * parameter;
    let closest_y = start.y as f64 + ab[1] * parameter;
    let closest_z = start.z as f64 + ab[2] * parameter;
    let dx = point.x as f64 - closest_x;
    let dy = point.y as f64 - closest_y;
    let dz = point.z as f64 - closest_z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn semicircle_distance(point: Point, radius: i64) -> f64 {
    let rho = (point.x as f64).hypot(point.y as f64);
    let mut theta = (point.y as f64).atan2(point.x as f64);
    if theta < 0.0 {
        theta += std::f64::consts::TAU;
    }
    let xy_distance = if theta <= std::f64::consts::PI {
        (rho - radius as f64).abs()
    } else {
        let left_dx = point.x as f64 + radius as f64;
        let right_dx = point.x as f64 - radius as f64;
        let dy = point.y as f64;
        left_dx.hypot(dy).min(right_dx.hypot(dy))
    };
    xy_distance.hypot(point.z as f64)
}

fn aabb_overlap(
    path_min: Point,
    path_max: Point,
    obstacle_min: Point,
    obstacle_max: Point,
) -> bool {
    path_min.x <= obstacle_max.x
        && obstacle_min.x <= path_max.x
        && path_min.y <= obstacle_max.y
        && obstacle_min.y <= path_max.y
        && path_min.z <= obstacle_max.z
        && obstacle_min.z <= path_max.z
}

fn point_json(point: Point) -> String {
    format!("{{\"x\":{},\"y\":{},\"z\":{}}}", point.x, point.y, point.z)
}

fn margins_json() -> &'static str {
    "{\"path_deviation\":{\"x\":0,\"y\":0,\"z\":0},\"localization\":{\"x\":0,\"y\":0,\"z\":0},\"tracking\":{\"x\":0,\"y\":0,\"z\":0},\"model_uncertainty\":{\"x\":0,\"y\":0,\"z\":0},\"numerical_enclosure\":{\"x\":0,\"y\":0,\"z\":0},\"stopping_reachability\":{\"x\":0,\"y\":0,\"z\":0},\"policy_clearance\":{\"x\":0,\"y\":0,\"z\":0}}"
}

fn make_case(
    family: Family,
    partition: Partition,
    index: usize,
    rng: &mut Lcg,
) -> (String, bool, bool, bool, bool) {
    let is_holdout = matches!(partition, Partition::Holdout);
    let family_name = match family {
        Family::Linear => "linear_segment",
        Family::UpperSemicircle => "upper_semicircle",
    };
    let family_code = match family {
        Family::Linear => "LINE",
        Family::UpperSemicircle => "ARC",
    };
    let partition_name = if is_holdout { "holdout" } else { "development" };
    let partition_code = if is_holdout { "VALIDATION" } else { "DEV" };
    let case_id = format!("GEN-{family_code}-{partition_code}-{:04}", index + 1);

    let moving_radius = 20 + rng.range(81);
    let obstacle_radius = 10 + rng.range(61);

    let (
        start,
        end,
        path_extra,
        obstacle,
        analytic_distance,
        endpoint_distance,
        chord_distance,
        candidate,
    ) = match family {
        Family::Linear => {
            let length = if is_holdout {
                3_000 + rng.range(3_001)
            } else {
                200 + rng.range(2_800)
            };
            let start = Point {
                x: -length,
                y: 0,
                z: 0,
            };
            let end = Point {
                x: length,
                y: 0,
                z: 0,
            };
            let obstacle = Point {
                x: -length - 500 + rng.range((2 * length + 1_001) as u32),
                y: rng.range(1_201) - 600,
                z: rng.range(801) - 400,
            };
            let analytic = point_segment_distance(obstacle, start, end);
            let endpoint = distance(obstacle, start).min(distance(obstacle, end));
            let minimum = Point {
                x: -length - moving_radius,
                y: -moving_radius,
                z: -moving_radius,
            };
            let maximum = Point {
                x: length + moving_radius,
                y: moving_radius,
                z: moving_radius,
            };
            let obstacle_min = Point {
                x: obstacle.x - obstacle_radius,
                y: obstacle.y - obstacle_radius,
                z: obstacle.z - obstacle_radius,
            };
            let obstacle_max = Point {
                x: obstacle.x + obstacle_radius,
                y: obstacle.y + obstacle_radius,
                z: obstacle.z + obstacle_radius,
            };
            (
                start,
                end,
                "\"arc_radius_mm\":0,\"arc_z_mm\":0".to_owned(),
                obstacle,
                analytic,
                endpoint,
                analytic,
                aabb_overlap(minimum, maximum, obstacle_min, obstacle_max),
            )
        }
        Family::UpperSemicircle => {
            let radius = if is_holdout {
                2_000 + rng.range(3_001)
            } else {
                300 + rng.range(1_700)
            };
            let degrees = rng.range(360) - 90;
            let angle = (degrees as f64).to_radians();
            let radial = (radius + rng.range(1_001) - 500).max(50);
            let obstacle = Point {
                x: ((radial as f64) * angle.cos()).round() as i64,
                y: ((radial as f64) * angle.sin()).round() as i64,
                z: rng.range(501) - 250,
            };
            let start = Point {
                x: -radius,
                y: 0,
                z: 0,
            };
            let end = Point {
                x: radius,
                y: 0,
                z: 0,
            };
            let analytic = semicircle_distance(obstacle, radius);
            let endpoint = distance(obstacle, start).min(distance(obstacle, end));
            let chord = point_segment_distance(obstacle, start, end);
            let minimum = Point {
                x: -radius - moving_radius,
                y: -moving_radius,
                z: -moving_radius,
            };
            let maximum = Point {
                x: radius + moving_radius,
                y: radius + moving_radius,
                z: moving_radius,
            };
            let obstacle_min = Point {
                x: obstacle.x - obstacle_radius,
                y: obstacle.y - obstacle_radius,
                z: obstacle.z - obstacle_radius,
            };
            let obstacle_max = Point {
                x: obstacle.x + obstacle_radius,
                y: obstacle.y + obstacle_radius,
                z: obstacle.z + obstacle_radius,
            };
            (
                start,
                end,
                format!("\"arc_radius_mm\":{radius},\"arc_z_mm\":0"),
                obstacle,
                analytic,
                endpoint,
                chord,
                aabb_overlap(minimum, maximum, obstacle_min, obstacle_max),
            )
        }
        };

    let contact = analytic_distance <= (moving_radius + obstacle_radius) as f64;
    let endpoint_contact = endpoint_distance <= (moving_radius + obstacle_radius) as f64;
    let chord_contact = chord_distance <= (moving_radius + obstacle_radius) as f64;
    assert!(
        !contact || candidate,
        "analytic collision lacks an AABB candidate: {case_id}"
    );

    let primitive_id = if is_holdout {
        "tool-validation"
    } else {
        "tool-development"
    };
    let mut json = String::new();
    write!(
        &mut json,
        concat!(
            "{{",
            "\"case_id\":\"{}\",",
            "\"partition\":\"{}\",",
            "\"path\":{{\"family\":\"{}\",\"start_mm\":{},\"end_mm\":{},{}}},",
            "\"moving_radius_mm\":{},",
            "\"obstacle\":{{\"center_mm\":{},\"radius_mm\":{}}},",
            "\"margins_mm\":{},",
            "\"expected_primitive_ids\":[\"{}\"],",
            "\"present_primitive_ids\":[\"{}\"],",
            "\"scoreable\":true,",
            "\"purpose\":\"Deterministic {} fixture for {}; analytic distance is evaluated over the full supported path domain.\",",
            "\"expected\":{{",
            "\"analytic_physical_collision\":{},",
            "\"endpoints_only_collision\":{},",
            "\"chord_collision\":{},",
            "\"physical_envelope_candidate\":{},",
            "\"stopping_envelope_candidate\":{},",
            "\"coordination_envelope_candidate\":{},",
            "\"validation_disposition\":\"ADMIT\"",
            "}}",
            "}}"
        ),
        case_id,
        partition_name,
        family_name,
        point_json(start),
        point_json(end),
        path_extra,
        moving_radius,
        point_json(obstacle),
        obstacle_radius,
        margins_json(),
        primitive_id,
        primitive_id,
        partition_name,
        family_name,
        contact,
        endpoint_contact,
        chord_contact,
        candidate,
        candidate,
        candidate,
    )
    .expect("writing to String cannot fail");

    (
        json,
        contact,
        !endpoint_contact && contact,
        !chord_contact && contact,
        !contact && candidate,
    )
}

fn generate() -> (String, usize, usize, usize, usize, usize) {
    let mut rng = Lcg(0xc0c5_7a11);
    let mut cases = Vec::with_capacity(260);
    let mut collisions = 0;
    let mut endpoint_misses = 0;
    let mut chord_misses = 0;
    let mut broad_phase_false_positives = 0;

    for family in [Family::Linear, Family::UpperSemicircle] {
        for partition in [Partition::Development, Partition::Holdout] {
            let count = if matches!(partition, Partition::Development) {
                100
            } else {
                30
            };
            for index in 0..count {
                let (json, contact, endpoint_miss, chord_miss, broad_phase_false_positive) =
                    make_case(family, partition, index, &mut rng);
                if contact {
                    collisions += 1;
                }
                if endpoint_miss {
                    endpoint_misses += 1;
                }
                if chord_miss {
                    chord_misses += 1;
                }
                if broad_phase_false_positive {
                    broad_phase_false_positives += 1;
                }
                cases.push(json);
            }
        }
    }

    let mut output = String::from("[\n");
    for (index, case) in cases.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str("  ");
        output.push_str(case);
    }
    output.push_str("\n]\n");

    (
        output,
        cases.len(),
        collisions,
        endpoint_misses,
        chord_misses,
        broad_phase_false_positives,
    )
}

fn main() -> io::Result<()> {
    let (output, count, collisions, endpoint_misses, chord_misses, broad_phase_false_positives) =
        generate();
    if let Some(path) = std::env::args_os().nth(1) {
        std::fs::write(path, output)
    } else {
        io::stdout().write_all(output.as_bytes())?;
        eprintln!(
            "generated_cases={count} analytic_contacts={collisions} endpoint_misses={endpoint_misses} chord_misses={chord_misses} broad_phase_false_positives={broad_phase_false_positives} broad_phase_false_negatives=0"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_half_arc_endpoint_distance_uses_vertical_offset_once() {
        let point = Point {
            x: 0,
            y: -1_000,
            z: 100,
        };
        let expected = 2_010_000.0_f64.sqrt();
        assert!((semicircle_distance(point, 1_000) - expected).abs() < 1.0e-9);
    }

    #[test]
    fn checked_in_generated_supplement_is_byte_identical() {
        const CHECKED_IN: &str = include_str!("../../fixtures/generated-cases-v0.2.json");
        let (generated, _, _, _, _, _) = generate();
        assert_eq!(generated, CHECKED_IN);
    }

    #[test]
    fn combined_v0_2_manifest_has_exact_case_and_partition_counts() {
        const COMBINED: &str = include_str!("../../fixtures/construction-motion-oracle-v0.2.json");
        assert_eq!(COMBINED.matches("\"case_id\":").count(), 272);
        assert_eq!(
            COMBINED.matches("\"partition\": \"development\"").count(),
            212
        );
        assert_eq!(COMBINED.matches("\"partition\": \"holdout\"").count(), 60);
        assert_eq!(COMBINED.matches("\"case_id\": \"GEN-").count(), 260);
    }

    #[test]
    fn seed_generates_fixed_partition_cardinalities_and_no_aabb_false_negative() {
        let (json, count, collisions, endpoint_misses, chord_misses, broad_phase_false_positives) =
            generate();
        assert_eq!(count, 260);
        assert_eq!(json.matches("\"partition\":\"development\"").count(), 200);
        assert_eq!(json.matches("\"partition\":\"holdout\"").count(), 60);
        assert!(collisions > 0);
        assert!(endpoint_misses > 0);
        assert!(chord_misses > 0);
        assert_eq!(collisions, 7);
        assert_eq!(endpoint_misses, 7);
        assert_eq!(chord_misses, 2);
        assert_eq!(broad_phase_false_positives, 16);
        assert!(!json.contains("\"analytic_physical_collision\":true,\"endpoints_only_collision\":false,\"chord_collision\":false,\"physical_envelope_candidate\":false"));
    }
}
