use crate::simulation::fish::{KoiFish, KoiVariety};
use crate::simulation::ripple::Ripple;

/// Represents the virtual koi pond simulation world containing the school of
/// koi fish, surface ripples, and interaction boundaries.
pub struct PondWorld {
    /// School of swimming koi fish.
    pub fish: Vec<KoiFish>,
    /// Active surface ripples.
    pub ripples: Vec<Ripple>,
    /// Pond surface width in pixels.
    pub width: f32,
    /// Pond surface height in pixels.
    pub height: f32,
    /// Current mouse cursor position in pond coordinates.
    pub cursor_pos: Option<(f32, f32)>,
}

impl PondWorld {
    /// Initializes a new pond world with a diverse school of Japanese Koi.
    pub fn new(width: f32, height: f32) -> Self {
        let center_x = width * 0.5;
        let center_y = height * 0.5;

        // Populate a school of 5 distinct, stylized Koi fish
        let fish = vec![
            KoiFish::new(
                KoiVariety::Kohaku,
                center_x - 120.0,
                center_y - 60.0,
                0.4,
                1.05,
            ),
            KoiFish::new(
                KoiVariety::Sanke,
                center_x + 90.0,
                center_y - 110.0,
                2.1,
                0.95,
            ),
            KoiFish::new(
                KoiVariety::YamabukiOgon,
                center_x - 70.0,
                center_y + 130.0,
                -1.2,
                1.12,
            ),
            KoiFish::new(
                KoiVariety::Showa,
                center_x + 140.0,
                center_y + 80.0,
                3.0,
                1.0,
            ),
            KoiFish::new(
                KoiVariety::Tancho,
                center_x + 20.0,
                center_y - 20.0,
                -0.6,
                0.92,
            ),
        ];

        Self {
            fish,
            ripples: Vec::with_capacity(16),
            width,
            height,
            cursor_pos: None,
        }
    }

    /// Updates the boundary dimensions when screen resolution changes.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    /// Updates current mouse cursor coordinates.
    pub fn set_cursor(&mut self, pos: Option<(f32, f32)>) {
        self.cursor_pos = pos;
    }

    /// Handles a mouse click event by creating a surface ripple and scattering
    /// fish within the disturbance shockwave radius.
    pub fn handle_click(&mut self, x: f32, y: f32) {
        // Spawn surface ripple
        self.ripples.push(Ripple::new(x, y));

        // Disturb fish in proximity
        let scatter_radius_sq = 280.0 * 280.0;
        for fish in &mut self.fish {
            let (hx, hy) = fish.head_pos();
            let dx = hx - x;
            let dy = hy - y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < scatter_radius_sq {
                fish.trigger_panic(x, y);
            }
        }
    }

    /// Advances the simulation by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        // 1. Update ripples and cull expired ones
        self.ripples.retain_mut(|r| r.update(dt));

        // 2. Ripple shockwave propagation: fish touching expanding wave crests get startled
        for ripple in &self.ripples {
            for fish in &mut self.fish {
                if fish.panic_timer <= 0.0 {
                    let (hx, hy) = fish.head_pos();
                    let dist = ((hx - ripple.x).powi(2) + (hy - ripple.y).powi(2)).sqrt();
                    // If near the advancing wave crest
                    if (dist - ripple.radius).abs() < 25.0 && ripple.alpha() > 0.35 {
                        fish.trigger_panic(ripple.x, ripple.y);
                    }
                }
            }
        }

        // 3. Collect head positions for flocking / school separation
        let positions: Vec<(f32, f32)> = self.fish.iter().map(|f| f.head_pos()).collect();

        // 4. Update each fish
        let num_fish = self.fish.len();
        for i in 0..num_fish {
            let other_positions: Vec<(f32, f32)> = positions
                .iter()
                .enumerate()
                .filter(|&(idx, _)| idx != i)
                .map(|(_, &pos)| pos)
                .collect();

            self.fish[i].update(
                dt,
                self.width,
                self.height,
                self.cursor_pos,
                &other_positions,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pond_world_lifecycle_and_interaction() {
        let mut world = PondWorld::new(1920.0, 1080.0);
        assert_eq!(world.fish.len(), 5);
        assert_eq!(world.ripples.len(), 0);

        // Click near the first fish
        let (fx, fy) = world.fish[0].head_pos();
        world.handle_click(fx + 20.0, fy + 20.0);

        assert_eq!(world.ripples.len(), 1);
        assert!(world.fish[0].panic_timer > 0.0);

        // Update world
        world.update(0.016);
        assert!(world.ripples[0].radius > 4.0);
    }
}
