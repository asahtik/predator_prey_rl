use bevy::prelude::Vec2;

fn reorder_vertices(mut r: [Vec2; 4]) -> [Vec2; 4] {
    let v01 = r[1] - r[0];
    let v02 = r[2] - r[0];
    let v03 = r[3] - r[0];
    if v01.dot(v03).abs() > 1e-5 {
        if v01.dot(v02).abs() > 1e-5 {
            r.swap(1, 2);
        } else {
            r.swap(3, 2);
        }
    }
    r
}

fn project(ax: Vec2, point: Vec2) -> f32 {
    let ax = ax.normalize();
    point.dot(ax)
}

/**
 * 0--------1
 * |        |
 * |        |
 * |        |
 * 3--------2
 */
pub fn sat2d(b1: [Vec2; 4], b2: [Vec2; 4]) -> bool {
    let center1 = (b1[0] + b1[1] + b1[2] + b1[3]) / 4.0;
    let center2 = (b2[0] + b2[1] + b2[2] + b2[3]) / 4.0;
    let r1 = (b1[0] - center1).length();
    let r2 = (b2[0] - center2).length();
    let dist = (center1 - center2).length();
    if dist > r1 + r2 {
        // If the bounding boxes are far away from each other, they can't collide
        return false;
    }

    let b1 = reorder_vertices(b1);
    let b2 = reorder_vertices(b2);

    let _w = (b1[1] - b1[0]).dot(b1[3] - b1[0]).abs() > 1e-8;
    assert!(
        (b1[1] - b1[0]).dot(b1[3] - b1[0]).abs() <= 1e-5,
        "Edges of bounding box 1 are not orthogonal ({:?}, {})",
        b1,
        (b1[1] - b1[0]).dot(b1[3] - b1[0])
    );
    assert!(
        (b2[1] - b2[0]).dot(b2[3] - b2[0]).abs() <= 1e-5,
        "Edges of bounding box 2 are not orthogonal ({:?}, {})",
        b2,
        (b2[1] - b2[0]).dot(b2[3] - b2[0])
    );

    let axes = [b1[1] - b1[0], b1[3] - b1[0], b2[1] - b2[0], b2[3] - b2[0]];

    for ax in axes {
        let ax_b1p0 = project(ax, b1[0]);
        let ax_b1p1 = project(ax, b1[1]);
        let ax_b1p2 = project(ax, b1[2]);
        let ax_b1p3 = project(ax, b1[3]);
        let ax_b1_min = ax_b1p0.min(ax_b1p1).min(ax_b1p2).min(ax_b1p3);
        let ax_b1_max = ax_b1p0.max(ax_b1p1).max(ax_b1p2).max(ax_b1p3);

        let ax_b2p0 = project(ax, b2[0]);
        let ax_b2p1 = project(ax, b2[1]);
        let ax_b2p2 = project(ax, b2[2]);
        let ax_b2p3 = project(ax, b2[3]);
        let ax_b2_min = ax_b2p0.min(ax_b2p1).min(ax_b2p2).min(ax_b2p3);
        let ax_b2_max = ax_b2p0.max(ax_b2p1).max(ax_b2p2).max(ax_b2p3);

        if ax_b1_min.max(ax_b2_min) >= ax_b1_max.min(ax_b2_max) {
            return false;
        }
    }
    true
}

fn get_segment_intercept(from1: Vec2, to1: Vec2, from2: Vec2, to2: Vec2) -> Option<Vec2> {
    let x1 = from1.x;
    let y1 = from1.y;
    let x2 = to1.x;
    let y2 = to1.y;
    let x3 = from2.x;
    let y3 = from2.y;
    let x4 = to2.x;
    let y4 = to2.y;

    let x1mx3 = x1 - x3;
    let y3my4 = y3 - y4;
    let y1my3 = y1 - y3;
    let x3mx4 = x3 - x4;
    let x1mx2 = x1 - x2;
    let y1my2 = y1 - y2;

    let den = x1mx2 * y3my4 - y1my2 * x3mx4;
    let t = (x1mx3 * y3my4 - y1my3 * x3mx4) / den;
    let u = (x1mx3 * y1my2 - y1my3 * x1mx2) / den;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(Vec2::new(x1 + t * (x2 - x1), y1 + t * (y2 - y1)))
    } else {
        None
    }
}

pub fn seg_box_intersect(from: Vec2, to: Vec2, mut b: [Vec2; 4]) -> Option<Vec2> {
    let center = (b[0] + b[1] + b[2] + b[3]) / 4.0;
    if (center - from).length() <= (center - b[0]).length() {
        return Some(from);
    }
    let mut closest = 0;
    let mut closest_dist = (from - b[0]).length();
    for (i, e) in b.iter().enumerate().skip(1) {
        let dist = (from - *e).length();
        if dist < closest_dist {
            closest_dist = dist;
            closest = i;
        }
    }
    if closest != 0 {
        b.swap(0, closest);
    }
    b = reorder_vertices(b);
    let segments = [(b[0], b[1]), (b[0], b[3])];
    assert!(
        (b[1] - b[0]).dot(b[3] - b[0]).abs() <= 1e-5,
        "Segments not orthogonal"
    );
    let mut closest_dist = 1e10f32;
    let mut closest_int = None;
    for seg in segments {
        if let Some(intercept) = get_segment_intercept(from, to, seg.0, seg.1) {
            let dist = (from - intercept).length();
            if dist < closest_dist {
                closest_dist = dist;
                closest_int = Some(intercept);
            }
        }
    }
    closest_int
}

#[cfg(test)]
mod sat_tests {
    use bevy::prelude::Vec2;

    fn permute_test(b1: [Vec2; 4], b2: [Vec2; 4], intersect: bool) {
        let sb1 = [
            [b1[0], b1[1], b1[2], b1[3]],
            [b1[0], b1[2], b1[1], b1[3]],
            [b1[0], b1[1], b1[3], b1[2]],
            [b1[0], b1[3], b1[1], b1[2]],
            [b1[0], b1[3], b1[2], b1[1]],
            [b1[1], b1[2], b1[3], b1[0]],
            [b1[1], b1[3], b1[2], b1[0]],
            [b1[1], b1[2], b1[0], b1[3]],
            [b1[1], b1[0], b1[2], b1[3]],
            [b1[1], b1[0], b1[3], b1[2]],
            [b1[2], b1[3], b1[0], b1[1]],
            [b1[2], b1[0], b1[3], b1[1]],
            [b1[2], b1[3], b1[1], b1[0]],
            [b1[2], b1[1], b1[3], b1[0]],
            [b1[2], b1[1], b1[0], b1[3]],
            [b1[3], b1[0], b1[1], b1[2]],
            [b1[3], b1[1], b1[0], b1[2]],
            [b1[3], b1[0], b1[2], b1[1]],
            [b1[3], b1[2], b1[0], b1[1]],
            [b1[3], b1[2], b1[1], b1[0]],
        ];
        let sb2 = [
            [b2[0], b2[1], b2[2], b2[3]],
            [b2[0], b2[2], b2[1], b2[3]],
            [b2[0], b2[1], b2[3], b2[2]],
            [b2[0], b2[3], b2[1], b2[2]],
            [b2[0], b2[3], b2[2], b2[1]],
            [b2[0], b2[2], b2[3], b2[1]],
            [b2[1], b2[2], b2[3], b2[0]],
            [b2[1], b2[3], b2[2], b2[0]],
            [b2[1], b2[2], b2[0], b2[3]],
            [b2[1], b2[0], b2[2], b2[3]],
            [b2[1], b2[0], b2[3], b2[2]],
            [b2[1], b2[3], b2[0], b2[2]],
            [b2[2], b2[3], b2[0], b2[1]],
            [b2[2], b2[0], b2[3], b2[1]],
            [b2[2], b2[3], b2[1], b2[0]],
            [b2[2], b2[1], b2[3], b2[0]],
            [b2[2], b2[1], b2[0], b2[3]],
            [b2[2], b2[0], b2[1], b2[3]],
            [b2[3], b2[0], b2[1], b2[2]],
            [b2[3], b2[1], b2[0], b2[2]],
            [b2[3], b2[0], b2[2], b2[1]],
            [b2[3], b2[2], b2[0], b2[1]],
            [b2[3], b2[2], b2[1], b2[0]],
            [b2[3], b2[1], b2[0], b2[0]],
        ];

        for (b1, b2) in sb1.into_iter().zip(sb2.into_iter()) {
            assert!(
                super::sat2d(b1, b2) == intersect,
                "b1: {:?}, b2: {:?}",
                b1,
                b2
            );
        }
    }

    #[test]
    fn test_sat1() {
        let b1 = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let b2 = [
            Vec2::new(1.5, 0.5),
            Vec2::new(2.5, 0.5),
            Vec2::new(2.5, 1.5),
            Vec2::new(1.5, 1.5),
        ];
        permute_test(b1, b2, false);
    }

    #[test]
    fn test_sat2() {
        let b1 = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let b2 = [
            Vec2::new(0.5, 0.5),
            Vec2::new(1.5, 0.5),
            Vec2::new(1.5, 1.5),
            Vec2::new(0.5, 1.5),
        ];
        permute_test(b1, b2, true);
    }

    #[test]
    fn test_sat3() {
        let b1 = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let b2 = [
            Vec2::new(0.5, 0.5),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.5, 0.5),
            Vec2::new(1.0, 0.0),
        ];
        permute_test(b1, b2, true);
    }

    #[test]
    fn test_sat4() {
        let b1 = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        let b2 = [
            Vec2::new(0.5, 0.5),
            Vec2::new(1.5, 0.5),
            Vec2::new(1.5, 1.5),
            Vec2::new(0.5, 1.5),
        ];
        permute_test(b1, b2, true);
    }

    #[test]
    fn test_sat5() {
        let b1 = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 4.0),
            Vec2::new(0.0, 4.0),
        ];
        let b2 = [
            Vec2::new(1.5, 0.5),
            Vec2::new(2.5, 0.5),
            Vec2::new(2.5, 4.5),
            Vec2::new(1.5, 4.5),
        ];
        permute_test(b1, b2, false);
    }

    #[test]
    fn test_intercept1() {
        let b = [
            Vec2::new(0.5, 0.5),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.5, 0.5),
            Vec2::new(1.0, 0.0),
        ];
        let s = (Vec2::new(0.0, 0.0), Vec2::new(0.5, 5.0));
        assert!(super::seg_box_intersect(s.0, s.1, b).is_none());
    }

    #[test]
    fn test_intercept2() {
        let b = [
            Vec2::new(0.5, 0.5),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.5, 0.5),
            Vec2::new(1.0, 0.0),
        ];
        let s = (Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.5));
        assert!(super::seg_box_intersect(s.0, s.1, b).is_some());
    }

    #[test]
    fn test_intercept3() {
        let b = [
            Vec2::new(0.5, 0.5),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.5, 0.5),
            Vec2::new(1.0, 0.0),
        ];
        let s = (Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.5));
        assert!(super::seg_box_intersect(s.0, s.1, b).is_some());
    }
}
