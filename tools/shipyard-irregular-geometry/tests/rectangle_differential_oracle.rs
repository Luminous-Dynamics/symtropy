//! Differential oracle: exact polygon predicates must agree with the independently defined
//! strict-area overlap semantics for 512 generated axis-aligned rectangles.

use shipyard_irregular_geometry::{Point2, SimplePolygon};
use shipyard_space_time::Rect;

const CASES: usize = 512;

fn next_u32(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn pick(state: &mut u32, bound: u32) -> u32 {
    next_u32(state) % bound
}

fn rectangle_polygon(rect: Rect) -> SimplePolygon {
    SimplePolygon::new(vec![
        Point2 {
            x: rect.min_x,
            y: rect.min_y,
        },
        Point2 {
            x: rect.max_x,
            y: rect.min_y,
        },
        Point2 {
            x: rect.max_x,
            y: rect.max_y,
        },
        Point2 {
            x: rect.min_x,
            y: rect.max_y,
        },
    ])
    .expect("generated rectangle polygon must be valid")
}

fn generated_rect(state: &mut u32) -> Rect {
    let min_x = i64::from(pick(state, 8));
    let min_y = i64::from(pick(state, 8));
    Rect {
        min_x,
        min_y,
        max_x: min_x + 1 + i64::from(pick(state, 4)),
        max_y: min_y + 1 + i64::from(pick(state, 4)),
    }
}

#[test]
fn generated_rectangle_pairs_match_strict_area_reference() {
    let mut state = 0x1a77_2026_u32;
    let mut cases = 0usize;
    let mut overlaps = 0usize;
    let mut disjoint_or_touching = 0usize;

    for _ in 0..CASES {
        let left = generated_rect(&mut state);
        let right = generated_rect(&mut state);
        let expected = left.overlaps(right);
        let actual = rectangle_polygon(left)
            .interior_overlaps(&rectangle_polygon(right))
            .expect("rectangular polygons should validate and triangulate");

        assert_eq!(actual, expected, "left={left:?}, right={right:?}");
        if expected {
            overlaps += 1;
        } else {
            disjoint_or_touching += 1;
        }
        cases += 1;
    }

    assert_eq!(cases, CASES);
    assert!(overlaps > 0);
    assert!(disjoint_or_touching > 0);
}
