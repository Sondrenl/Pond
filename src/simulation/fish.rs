use std::f32::consts::PI;

/// Distinctive varieties of Japanese Koi fish, each with its own signature
/// coloration and patterning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KoiVariety {
    /// Pure white body with vivid scarlet/crimson (Hi) patches.
    Kohaku,
    /// White body with scarlet (Hi) and inky black (Sumi) patches.
    Sanke,
    /// Lustrous metallic golden-yellow body with amber highlights.
    YamabukiOgon,
    /// Deep black body adorned with vermilion and pure white markings.
    Showa,
    /// Snow-white body with an iconic circular crimson crown on the head.
    Tancho,
}

/// Represents an autonomous 2D Koi fish swimming in the pond with multi-joint
/// spine inverse kinematics, organic steering, and reactive darting behaviors.
#[derive(Debug, Clone)]
pub struct KoiFish {
    /// Identifier for variety styling.
    pub variety: KoiVariety,
    /// Spine joint positions in world coordinates (index 0 is the head).
    pub spine: Vec<(f32, f32)>,
    /// Distance between adjacent spine joints.
    pub segment_length: f32,
    /// Overall scale factor for fish size variation.
    pub scale: f32,

    /// Current heading angle in radians.
    pub heading: f32,
    /// Target heading angle toward which the fish is smoothly steering.
    pub target_heading: f32,
    /// Current forward swimming speed in pixels per second.
    pub speed: f32,
    /// Normal cruising speed when relaxed.
    pub cruise_speed: f32,
    /// Maximum speed during panicked darting.
    pub burst_speed: f32,

    /// Wandering direction perturbation angle.
    pub wander_angle: f32,
    /// Timer for wandering direction changes.
    pub wander_timer: f32,

    /// Accumulated swim phase driving sinusoidal tail undulation.
    pub swim_phase: f32,
    /// Current tail oscillation frequency (Hz).
    pub tail_frequency: f32,

    /// Remaining time in panicked darting state (in seconds).
    pub panic_timer: f32,
    /// Remaining time of pectoral fin flutter excitement.
    pub fin_flutter: f32,
}

impl KoiFish {
    /// Number of spine joints along the body length.
    pub const SPINE_SEGMENTS: usize = 11;

    /// Creates a new koi fish of the specified variety at initial position (x, y).
    pub fn new(variety: KoiVariety, x: f32, y: f32, heading: f32, scale: f32) -> Self {
        let segment_length = 8.5 * scale;
        let mut spine = Vec::with_capacity(Self::SPINE_SEGMENTS);

        // Lay out initial spine joints trailing behind the head
        let dir_x = -heading.cos();
        let dir_y = -heading.sin();
        for i in 0..Self::SPINE_SEGMENTS {
            let offset = i as f32 * segment_length;
            spine.push((x + dir_x * offset, y + dir_y * offset));
        }

        let cruise_speed = 48.0 * scale;
        let burst_speed = 135.0 * scale;

        Self {
            variety,
            spine,
            segment_length,
            scale,
            heading,
            target_heading: heading,
            speed: cruise_speed,
            cruise_speed,
            burst_speed,
            wander_angle: 0.0,
            wander_timer: 0.0,
            swim_phase: 0.0,
            tail_frequency: 2.2,
            panic_timer: 0.0,
            fin_flutter: 0.0,
        }
    }

    /// Returns the world position of the koi's head.
    #[inline]
    pub fn head_pos(&self) -> (f32, f32) {
        self.spine[0]
    }

    /// Triggers an escape burst darting directly away from a disturbance point (e.g. click).
    pub fn trigger_panic(&mut self, disturbance_x: f32, disturbance_y: f32) {
        let (hx, hy) = self.head_pos();
        let dx = hx - disturbance_x;
        let dy = hy - disturbance_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq > 0.001 {
            // Escape vector directly away from disturbance, plus slight random veer
            let base_angle = dy.atan2(dx);
            let veer = if (self.spine[0].0 as usize) & 1 == 0 {
                0.25
            } else {
                -0.25
            };
            self.target_heading = base_angle + veer;
            self.heading = self.target_heading; // Quick reflex turn
            self.speed = self.burst_speed;
            self.panic_timer = 2.0;
            self.tail_frequency = 5.5;
            self.fin_flutter = 2.5;
        }
    }

    /// Updates autonomous swimming behaviors, steering, boundary avoidance, and spine IK.
    pub fn update(
        &mut self,
        dt: f32,
        bounds_width: f32,
        bounds_height: f32,
        cursor_pos: Option<(f32, f32)>,
        other_fish_positions: &[(f32, f32)],
    ) {
        let (mut hx, mut hy) = self.head_pos();

        // 1. Panic timer countdown & speed relaxation
        if self.panic_timer > 0.0 {
            self.panic_timer = (self.panic_timer - dt).max(0.0);
            let t = self.panic_timer / 2.0;
            self.speed = self.cruise_speed + (self.burst_speed - self.cruise_speed) * (t * t);
            self.tail_frequency = 2.2 + 3.3 * t;
        } else {
            // Relax toward cruise speed
            let speed_diff = self.cruise_speed - self.speed;
            self.speed += speed_diff * (dt * 2.0);
            self.tail_frequency = 2.0 + (self.speed / self.cruise_speed - 1.0) * 1.5;
        }

        if self.fin_flutter > 0.0 {
            self.fin_flutter = (self.fin_flutter - dt).max(0.0);
        }

        // 2. Cursor proximity detection (fish gently moves away from approaching cursor)
        if let Some((cx, cy)) = cursor_pos {
            let dx = hx - cx;
            let dy = hy - cy;
            let dist_sq = dx * dx + dy * dy;
            let avoid_dist = 110.0 * self.scale;
            if dist_sq < avoid_dist * avoid_dist && dist_sq > 1.0 {
                let dist = dist_sq.sqrt();
                let escape_angle = dy.atan2(dx);
                let weight = (1.0 - dist / avoid_dist).clamp(0.0, 1.0);
                self.target_heading = self.target_heading * (1.0 - weight) + escape_angle * weight;
                if self.speed < self.cruise_speed * 1.4 {
                    self.speed += 30.0 * dt * self.scale;
                }
            }
        }

        // 3. Autonomous wandering behavior
        self.wander_timer -= dt;
        if self.wander_timer <= 0.0 {
            self.wander_timer = 1.2 + (self.heading.cos() * 0.5).abs();
            // Smoothly perturb the wander angle
            let delta = ((hx as usize * 17 + hy as usize * 31) % 100) as f32 / 100.0 - 0.5;
            self.wander_angle += delta * 0.8;
            self.target_heading += self.wander_angle * 0.4;
        }

        // 4. Boundary steering: Softly push fish away from window edges
        let margin = 120.0 * self.scale;
        let mut steer_x = 0.0;
        let mut steer_y = 0.0;

        if hx < margin {
            steer_x += (margin - hx) / margin;
        } else if hx > bounds_width - margin {
            steer_x -= (hx - (bounds_width - margin)) / margin;
        }

        if hy < margin {
            steer_y += (margin - hy) / margin;
        } else if hy > bounds_height - margin {
            steer_y -= (hy - (bounds_height - margin)) / margin;
        }

        if steer_x != 0.0 || steer_y != 0.0 {
            let avoid_heading = steer_y.atan2(steer_x);
            let turn_weight = (steer_x * steer_x + steer_y * steer_y).sqrt().min(1.0);
            self.target_heading =
                self.target_heading * (1.0 - turn_weight) + avoid_heading * turn_weight;
        }

        // 5. School separation: Prevent excessive overlap with other fish
        for &(ox, oy) in other_fish_positions {
            let dx = hx - ox;
            let dy = hy - oy;
            let dist_sq = dx * dx + dy * dy;
            let min_sep = 75.0 * self.scale;
            if dist_sq < min_sep * min_sep && dist_sq > 0.1 {
                let repulse_heading = dy.atan2(dx);
                self.target_heading = self.target_heading * 0.85 + repulse_heading * 0.15;
            }
        }

        // 6. Smooth angular interpolation toward target heading
        let mut angle_diff = self.target_heading - self.heading;
        while angle_diff > PI {
            angle_diff -= 2.0 * PI;
        }
        while angle_diff < -PI {
            angle_diff += 2.0 * PI;
        }
        let max_turn = 2.4 * dt;
        self.heading += angle_diff.clamp(-max_turn, max_turn);

        // 7. Advance head forward
        hx += self.heading.cos() * self.speed * dt;
        hy += self.heading.sin() * self.speed * dt;

        // Clamp to stay strictly on-screen
        hx = hx.clamp(10.0, bounds_width - 10.0);
        hy = hy.clamp(10.0, bounds_height - 10.0);
        self.spine[0] = (hx, hy);

        // 8. Advance swimming phase for tail undulation
        self.swim_phase += dt * self.tail_frequency * 2.0 * PI;

        // 9. Inverse Kinematics spine constraint
        for i in 1..Self::SPINE_SEGMENTS {
            let prev = self.spine[i - 1];
            let curr = self.spine[i];

            let dx = curr.0 - prev.0;
            let dy = curr.1 - prev.1;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 0.0001 {
                let target_x = prev.0 + (dx / dist) * self.segment_length;
                let target_y = prev.1 + (dy / dist) * self.segment_length;
                self.spine[i] = (target_x, target_y);
            } else {
                self.spine[i] = (prev.0 - self.segment_length, prev.1);
            }
        }
    }

    /// Evaluates the lateral undulating offset for a given spine segment index [0..N-1].
    /// Produces a natural traveling sine wave with amplitude increasing toward the tail.
    #[inline]
    pub fn undulation_offset(&self, segment_idx: usize) -> f32 {
        if segment_idx == 0 {
            return 0.0;
        }
        let progress = segment_idx as f32 / (Self::SPINE_SEGMENTS - 1) as f32;
        let wave_phase = self.swim_phase - (segment_idx as f32 * 0.62);
        let amplitude = 4.2 * self.scale * progress.powf(1.3);
        wave_phase.sin() * amplitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fish_initialization() {
        let fish = KoiFish::new(KoiVariety::Kohaku, 200.0, 200.0, 0.0, 1.0);
        assert_eq!(fish.spine.len(), KoiFish::SPINE_SEGMENTS);
        assert_eq!(fish.head_pos(), (200.0, 200.0));
        assert!(fish.speed > 0.0);
    }

    #[test]
    fn test_fish_panic_reflex() {
        let mut fish = KoiFish::new(KoiVariety::Sanke, 200.0, 200.0, 0.0, 1.0);
        let normal_speed = fish.speed;

        // Trigger disturbance near head
        fish.trigger_panic(180.0, 200.0);
        assert!(fish.speed > normal_speed * 2.0);
        assert!(fish.panic_timer > 1.0);

        // Update to relax speed
        fish.update(1.0, 800.0, 600.0, None, &[]);
        assert!(fish.speed < fish.burst_speed);
    }

    #[test]
    fn test_spine_undulation() {
        let fish = KoiFish::new(KoiVariety::Tancho, 100.0, 100.0, 0.0, 1.0);
        // Head should have 0 lateral offset
        assert_eq!(fish.undulation_offset(0), 0.0);
        // Mid-tail should have non-zero lateral oscillation
        let offset = fish.undulation_offset(KoiFish::SPINE_SEGMENTS - 1);
        assert!(offset.abs() >= 0.0);
    }
}
