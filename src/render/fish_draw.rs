use crate::simulation::fish::{KoiFish, KoiVariety};
use crate::simulation::ripple::Ripple;
use std::f32::consts::PI;
use tiny_skia::{FillRule, LineCap, Paint, Path, PathBuilder, PixmapMut, Stroke, Transform};

/// Helper creating an anti-aliased paint instance.
#[inline]
fn antialiased_paint() -> Paint<'static> {
    Paint {
        anti_alias: true,
        ..Default::default()
    }
}

/// Renders water ripples and stylized 2D koi fish onto a target pixel buffer.
pub struct FishDrawer;

impl FishDrawer {
    /// Renders expanding surface ripples.
    pub fn draw_ripple(pixmap: &mut PixmapMut, ripple: &Ripple) {
        let alpha = ripple.alpha();
        if alpha <= 0.01 {
            return;
        }

        // Outer primary wave crest
        let mut pb = PathBuilder::new();
        pb.push_circle(ripple.x, ripple.y, ripple.radius);
        if let Some(path) = pb.finish() {
            let mut paint = antialiased_paint();
            let a = (alpha * 210.0).clamp(0.0, 255.0) as u8;
            paint.set_color_rgba8(200, 235, 255, a);
            let stroke = Stroke {
                width: 2.4,
                line_cap: LineCap::Round,
                ..Default::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // Secondary inner ripple ring
        if ripple.radius > 16.0 {
            let mut pb2 = PathBuilder::new();
            pb2.push_circle(ripple.x, ripple.y, ripple.radius - 14.0);
            if let Some(path2) = pb2.finish() {
                let mut paint2 = antialiased_paint();
                let a2 = (alpha * 140.0).clamp(0.0, 255.0) as u8;
                paint2.set_color_rgba8(160, 215, 255, a2);
                let stroke2 = Stroke {
                    width: 1.6,
                    line_cap: LineCap::Round,
                    ..Default::default()
                };
                pixmap.stroke_path(&path2, &paint2, &stroke2, Transform::identity(), None);
            }
        }

        // Tertiary faint inner ripple ring
        if ripple.radius > 32.0 {
            let mut pb3 = PathBuilder::new();
            pb3.push_circle(ripple.x, ripple.y, ripple.radius - 28.0);
            if let Some(path3) = pb3.finish() {
                let mut paint3 = antialiased_paint();
                let a3 = (alpha * 75.0).clamp(0.0, 255.0) as u8;
                paint3.set_color_rgba8(130, 200, 245, a3);
                let stroke3 = Stroke {
                    width: 1.0,
                    line_cap: LineCap::Round,
                    ..Default::default()
                };
                pixmap.stroke_path(&path3, &paint3, &stroke3, Transform::identity(), None);
            }
        }
    }

    /// Renders a single koi fish with anatomical curves, fins, and distinctive patterning.
    pub fn draw_fish(pixmap: &mut PixmapMut, fish: &KoiFish) {
        let n = KoiFish::SPINE_SEGMENTS;
        if fish.spine.len() < n {
            return;
        }

        // 1. Calculate undulated spine points and normal vectors
        let mut points: Vec<(f32, f32)> = Vec::with_capacity(n);
        let mut normals: Vec<(f32, f32)> = Vec::with_capacity(n);

        for i in 0..n {
            let curr = fish.spine[i];
            let (tan_x, tan_y) = if i == 0 {
                let next = fish.spine[1];
                (curr.0 - next.0, curr.1 - next.1)
            } else if i == n - 1 {
                let prev = fish.spine[n - 2];
                (prev.0 - curr.0, prev.1 - curr.1)
            } else {
                let prev = fish.spine[i - 1];
                let next = fish.spine[i + 1];
                (prev.0 - next.0, prev.1 - next.1)
            };

            let len = (tan_x * tan_x + tan_y * tan_y).sqrt().max(0.001);
            let nx = -tan_y / len;
            let ny = tan_x / len;

            let lat_offset = fish.undulation_offset(i);
            points.push((curr.0 + nx * lat_offset, curr.1 + ny * lat_offset));
            normals.push((nx, ny));
        }

        // 2. Width profile along the spine (head to tail peduncle)
        let base_widths = [
            7.5,  // 0: snout / head
            11.0, // 1: cranium
            13.5, // 2: chest / pectoral girdle (widest)
            13.0, // 3: upper torso
            11.8, // 4: mid torso
            10.0, // 5: lower torso
            8.2,  // 6: flank
            6.5,  // 7: upper tail stock
            4.8,  // 8: mid tail stock
            3.2,  // 9: slender peduncle
            2.0,  // 10: tail stem
        ];

        let scale = fish.scale;
        let mut left_hull: Vec<(f32, f32)> = Vec::with_capacity(n);
        let mut right_hull: Vec<(f32, f32)> = Vec::with_capacity(n);

        for i in 0..n {
            let w = base_widths[i] * scale;
            let (px, py) = points[i];
            let (nx, ny) = normals[i];
            left_hull.push((px + nx * w, py + ny * w));
            right_hull.push((px - nx * w, py - ny * w));
        }

        // 3. Render Pectoral Fins (underneath body)
        Self::draw_pectoral_fins(pixmap, fish, &points, &normals);

        // 4. Render Ventral / Pelvic Fins
        Self::draw_ventral_fins(pixmap, fish, &points, &normals);

        // 5. Render Caudal (Tail) Fin (underneath tail tip)
        Self::draw_caudal_fin(pixmap, fish, &points, &normals);

        // 6. Render Fish Body Hull
        let body_path =
            Self::build_body_path(&points, &left_hull, &right_hull, fish.heading, scale);
        if let Some(path) = body_path {
            let mut paint = antialiased_paint();

            // Base body color
            let (r, g, b, a) = match fish.variety {
                KoiVariety::Kohaku | KoiVariety::Sanke | KoiVariety::Tancho => (248, 246, 242, 245),
                KoiVariety::YamabukiOgon => (238, 188, 42, 245),
                KoiVariety::Showa => (35, 36, 40, 245),
            };
            paint.set_color_rgba8(r, g, b, a);
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );

            // Subtle body outline for crisp definition
            let stroke = Stroke {
                width: 1.0,
                ..Default::default()
            };
            let mut stroke_paint = antialiased_paint();
            stroke_paint.set_color_rgba8(
                r.saturating_sub(40),
                g.saturating_sub(40),
                b.saturating_sub(40),
                120,
            );
            pixmap.stroke_path(&path, &stroke_paint, &stroke, Transform::identity(), None);
        }

        // 7. Render Distinctive Pattern Markings
        Self::draw_variety_patterns(pixmap, fish, &points, &left_hull, &right_hull);

        // 8. Render Dorsal Ridge & Eyes
        Self::draw_dorsal_ridge(pixmap, &points, scale);
        Self::draw_eyes(pixmap, &points[1], normals[1], scale);
    }

    /// Builds a smooth closed hull path around the spine.
    fn build_body_path(
        points: &[(f32, f32)],
        left: &[(f32, f32)],
        right: &[(f32, f32)],
        heading: f32,
        scale: f32,
    ) -> Option<Path> {
        let mut pb = PathBuilder::new();
        let snout = (
            points[0].0 + heading.cos() * 8.0 * scale,
            points[0].1 + heading.sin() * 8.0 * scale,
        );

        pb.move_to(snout.0, snout.1);

        // Curve down the left flank
        pb.quad_to(left[0].0, left[0].1, left[1].0, left[1].1);
        for i in 2..left.len() {
            let mid_x = (left[i - 1].0 + left[i].0) * 0.5;
            let mid_y = (left[i - 1].1 + left[i].1) * 0.5;
            pb.quad_to(left[i - 1].0, left[i - 1].1, mid_x, mid_y);
        }
        let last = left.len() - 1;
        pb.line_to(left[last].0, left[last].1);

        // Cross tail stem to right side
        pb.line_to(right[last].0, right[last].1);

        // Curve back up the right flank
        for i in (1..right.len()).rev() {
            let mid_x = (right[i].0 + right[i - 1].0) * 0.5;
            let mid_y = (right[i].1 + right[i - 1].1) * 0.5;
            pb.quad_to(right[i].0, right[i].1, mid_x, mid_y);
        }
        pb.quad_to(right[0].0, right[0].1, snout.0, snout.1);
        pb.close();

        pb.finish()
    }

    /// Renders translucent left and right pectoral fins with realistic sway.
    fn draw_pectoral_fins(
        pixmap: &mut PixmapMut,
        fish: &KoiFish,
        points: &[(f32, f32)],
        normals: &[(f32, f32)],
    ) {
        let anchor = points[2];
        let (nx, ny) = normals[2];
        let scale = fish.scale;

        // Fin oscillation: gentle swimming rhythm + burst flutter
        let flutter = if fish.fin_flutter > 0.0 {
            (fish.swim_phase * 2.5).sin() * 0.35
        } else {
            (fish.swim_phase * 0.8).sin() * 0.18
        };

        let fin_length = 26.0 * scale;
        let fin_spread = 15.0 * scale;

        let base_angle = fish.heading + PI; // Facing backward from head
        let left_angle = base_angle + 0.95 + flutter;
        let right_angle = base_angle - 0.95 - flutter;

        // Fin color (translucent white or golden tint)
        let (r, g, b) = match fish.variety {
            KoiVariety::YamabukiOgon => (245, 205, 80),
            _ => (240, 244, 250),
        };
        let mut paint = antialiased_paint();
        paint.set_color_rgba8(r, g, b, 140);

        // Left Fin
        let left_root = (anchor.0 + nx * 8.0 * scale, anchor.1 + ny * 8.0 * scale);
        let left_tip = (
            left_root.0 + left_angle.cos() * fin_length,
            left_root.1 + left_angle.sin() * fin_length,
        );
        let left_rear = (
            left_root.0 + (left_angle - 0.5).cos() * fin_spread,
            left_root.1 + (left_angle - 0.5).sin() * fin_spread,
        );

        let mut pb_left = PathBuilder::new();
        pb_left.move_to(left_root.0, left_root.1);
        pb_left.quad_to(
            left_root.0 + left_angle.cos() * (fin_length * 0.6) + nx * 6.0,
            left_root.1 + left_angle.sin() * (fin_length * 0.6) + ny * 6.0,
            left_tip.0,
            left_tip.1,
        );
        pb_left.quad_to(left_rear.0, left_rear.1, left_root.0, left_root.1);
        pb_left.close();

        if let Some(path) = pb_left.finish() {
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }

        // Right Fin
        let right_root = (anchor.0 - nx * 8.0 * scale, anchor.1 - ny * 8.0 * scale);
        let right_tip = (
            right_root.0 + right_angle.cos() * fin_length,
            right_root.1 + right_angle.sin() * fin_length,
        );
        let right_rear = (
            right_root.0 + (right_angle + 0.5).cos() * fin_spread,
            right_root.1 + (right_angle + 0.5).sin() * fin_spread,
        );

        let mut pb_right = PathBuilder::new();
        pb_right.move_to(right_root.0, right_root.1);
        pb_right.quad_to(
            right_root.0 + right_angle.cos() * (fin_length * 0.6) - nx * 6.0,
            right_root.1 + right_angle.sin() * (fin_length * 0.6) - ny * 6.0,
            right_tip.0,
            right_tip.1,
        );
        pb_right.quad_to(right_rear.0, right_rear.1, right_root.0, right_root.1);
        pb_right.close();

        if let Some(path) = pb_right.finish() {
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// Renders small pelvic / ventral fins along the lower flank.
    fn draw_ventral_fins(
        pixmap: &mut PixmapMut,
        fish: &KoiFish,
        points: &[(f32, f32)],
        normals: &[(f32, f32)],
    ) {
        let anchor = points[5];
        let (nx, ny) = normals[5];
        let scale = fish.scale;
        let fin_len = 12.0 * scale;

        let base_angle = fish.heading + PI;
        let mut paint = antialiased_paint();
        paint.set_color_rgba8(240, 244, 250, 110);

        // Left ventral fin
        let l_root = (anchor.0 + nx * 6.0 * scale, anchor.1 + ny * 6.0 * scale);
        let l_tip = (
            l_root.0 + (base_angle + 0.45).cos() * fin_len,
            l_root.1 + (base_angle + 0.45).sin() * fin_len,
        );
        let mut pb_l = PathBuilder::new();
        pb_l.move_to(l_root.0, l_root.1);
        pb_l.line_to(l_tip.0, l_tip.1);
        pb_l.line_to(l_root.0 + nx * 2.0, l_root.1 + ny * 2.0);
        pb_l.close();
        if let Some(p) = pb_l.finish() {
            pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        }

        // Right ventral fin
        let r_root = (anchor.0 - nx * 6.0 * scale, anchor.1 - ny * 6.0 * scale);
        let r_tip = (
            r_root.0 + (base_angle - 0.45).cos() * fin_len,
            r_root.1 + (base_angle - 0.45).sin() * fin_len,
        );
        let mut pb_r = PathBuilder::new();
        pb_r.move_to(r_root.0, r_root.1);
        pb_r.line_to(r_tip.0, r_tip.1);
        pb_r.line_to(r_root.0 - nx * 2.0, r_root.1 - ny * 2.0);
        pb_r.close();
        if let Some(p) = pb_r.finish() {
            pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    /// Renders the fan-shaped caudal (tail) fin trailing the fish.
    fn draw_caudal_fin(
        pixmap: &mut PixmapMut,
        fish: &KoiFish,
        points: &[(f32, f32)],
        normals: &[(f32, f32)],
    ) {
        let n = points.len();
        let stem = points[n - 1];
        let pre_stem = points[n - 2];
        let (nx, ny) = normals[n - 1];
        let scale = fish.scale;

        // Tail orientation vector
        let dir_x = stem.0 - pre_stem.0;
        let dir_y = stem.1 - pre_stem.1;
        let tail_angle = (dir_y).atan2(dir_x);

        let tail_len = 28.0 * scale;
        let tail_width = 16.0 * scale;

        // Trailing tip positions of the fan-shaped caudal fin
        let tip_top = (
            stem.0 + tail_angle.cos() * tail_len + nx * tail_width,
            stem.1 + tail_angle.sin() * tail_len + ny * tail_width,
        );
        let tip_bot = (
            stem.0 + tail_angle.cos() * tail_len - nx * tail_width,
            stem.1 + tail_angle.sin() * tail_len - ny * tail_width,
        );
        let tip_center = (
            stem.0 + tail_angle.cos() * (tail_len * 0.75),
            stem.1 + tail_angle.sin() * (tail_len * 0.75),
        );

        let mut pb = PathBuilder::new();
        pb.move_to(stem.0, stem.1);
        pb.quad_to(
            stem.0 + nx * (tail_width * 0.5),
            stem.1 + ny * (tail_width * 0.5),
            tip_top.0,
            tip_top.1,
        );
        pb.quad_to(tip_center.0, tip_center.1, tip_bot.0, tip_bot.1);
        pb.quad_to(
            stem.0 - nx * (tail_width * 0.5),
            stem.1 - ny * (tail_width * 0.5),
            stem.0,
            stem.1,
        );
        pb.close();

        let (r, g, b) = match fish.variety {
            KoiVariety::YamabukiOgon => (250, 210, 75),
            _ => (242, 245, 252),
        };

        let mut paint = antialiased_paint();
        paint.set_color_rgba8(r, g, b, 150);

        if let Some(path) = pb.finish() {
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// Renders variety-specific color patterns (Hi red, Sumi black, Tancho crown).
    fn draw_variety_patterns(
        pixmap: &mut PixmapMut,
        fish: &KoiFish,
        points: &[(f32, f32)],
        _left: &[(f32, f32)],
        _right: &[(f32, f32)],
    ) {
        let scale = fish.scale;
        let mut paint_red = antialiased_paint();
        paint_red.set_color_rgba8(228, 55, 30, 240); // Vibrant crimson / scarlet Hi

        let mut paint_black = antialiased_paint();
        paint_black.set_color_rgba8(26, 26, 28, 240); // Inky black Sumi

        let mut paint_white = antialiased_paint();
        paint_white.set_color_rgba8(248, 248, 248, 240);

        match fish.variety {
            KoiVariety::Kohaku => {
                // 3 distinctive scarlet Hi spots along head, mid-body, and peduncle
                Self::draw_spot(pixmap, points[1].0, points[1].1, 7.5 * scale, &paint_red);
                Self::draw_spot(pixmap, points[3].0, points[3].1, 9.5 * scale, &paint_red);
                Self::draw_spot(pixmap, points[6].0, points[6].1, 6.5 * scale, &paint_red);
            }
            KoiVariety::Sanke => {
                // Scarlet Hi patches + Black Sumi accents
                Self::draw_spot(pixmap, points[1].0, points[1].1, 7.0 * scale, &paint_red);
                Self::draw_spot(pixmap, points[3].0, points[3].1, 9.0 * scale, &paint_red);
                Self::draw_spot(pixmap, points[6].0, points[6].1, 6.0 * scale, &paint_red);

                Self::draw_spot(
                    pixmap,
                    points[2].0 + 3.0 * scale,
                    points[2].1,
                    4.2 * scale,
                    &paint_black,
                );
                Self::draw_spot(
                    pixmap,
                    points[5].0 - 3.0 * scale,
                    points[5].1,
                    4.5 * scale,
                    &paint_black,
                );
                Self::draw_spot(pixmap, points[8].0, points[8].1, 3.2 * scale, &paint_black);
            }
            KoiVariety::YamabukiOgon => {
                // Golden dorsal sheen highlight
                let mut paint_gold = antialiased_paint();
                paint_gold.set_color_rgba8(255, 220, 85, 210);
                Self::draw_spot(pixmap, points[2].0, points[2].1, 6.0 * scale, &paint_gold);
                Self::draw_spot(pixmap, points[4].0, points[4].1, 5.0 * scale, &paint_gold);
            }
            KoiVariety::Showa => {
                // Black base with scarlet and white markings
                Self::draw_spot(pixmap, points[1].0, points[1].1, 7.5 * scale, &paint_red);
                Self::draw_spot(pixmap, points[4].0, points[4].1, 8.5 * scale, &paint_red);
                Self::draw_spot(pixmap, points[7].0, points[7].1, 5.5 * scale, &paint_red);

                Self::draw_spot(
                    pixmap,
                    points[2].0 - 4.0 * scale,
                    points[2].1,
                    5.0 * scale,
                    &paint_white,
                );
                Self::draw_spot(
                    pixmap,
                    points[5].0 + 4.0 * scale,
                    points[5].1,
                    5.0 * scale,
                    &paint_white,
                );
            }
            KoiVariety::Tancho => {
                // Signature circular scarlet crown directly on the forehead
                let crown_center = points[0];
                let mut paint_crown = antialiased_paint();
                paint_crown.set_color_rgba8(218, 32, 22, 245);
                Self::draw_spot(
                    pixmap,
                    crown_center.0,
                    crown_center.1,
                    5.8 * scale,
                    &paint_crown,
                );
            }
        }
    }

    /// Renders a soft circular pattern spot.
    fn draw_spot(pixmap: &mut PixmapMut, x: f32, y: f32, radius: f32, paint: &Paint) {
        let mut pb = PathBuilder::new();
        pb.push_circle(x, y, radius);
        if let Some(path) = pb.finish() {
            pixmap.fill_path(&path, paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    /// Renders a subtle dorsal ridge line along the fish's spine.
    fn draw_dorsal_ridge(pixmap: &mut PixmapMut, points: &[(f32, f32)], scale: f32) {
        let mut pb = PathBuilder::new();
        pb.move_to(points[2].0, points[2].1);
        for pt in &points[3..=8] {
            pb.line_to(pt.0, pt.1);
        }
        if let Some(path) = pb.finish() {
            let stroke = Stroke {
                width: 1.4 * scale,
                line_cap: LineCap::Round,
                ..Default::default()
            };

            let mut paint = antialiased_paint();
            paint.set_color_rgba8(255, 255, 255, 75);
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    /// Renders subtle dark eyes on the sides of the cranium.
    fn draw_eyes(pixmap: &mut PixmapMut, cranium: &(f32, f32), normal: (f32, f32), scale: f32) {
        let eye_dist = 6.8 * scale;
        let eye_r = 1.4 * scale;

        let left_eye = (
            cranium.0 + normal.0 * eye_dist,
            cranium.1 + normal.1 * eye_dist,
        );
        let right_eye = (
            cranium.0 - normal.0 * eye_dist,
            cranium.1 - normal.1 * eye_dist,
        );

        let mut paint = antialiased_paint();
        paint.set_color_rgba8(20, 20, 24, 230);

        Self::draw_spot(pixmap, left_eye.0, left_eye.1, eye_r, &paint);
        Self::draw_spot(pixmap, right_eye.0, right_eye.1, eye_r, &paint);
    }
}
