/// Represents an expanding, fading water ripple on the pond surface.
#[derive(Debug, Clone)]
pub struct Ripple {
    /// Center X coordinate in pixels.
    pub x: f32,
    /// Center Y coordinate in pixels.
    pub y: f32,
    /// Current outer radius in pixels.
    pub radius: f32,
    /// Maximum outer radius before the ripple disappears.
    pub max_radius: f32,
    /// Expansion speed in pixels per second.
    pub speed: f32,
    /// Elapsed time since creation in seconds.
    pub age: f32,
    /// Total duration of the ripple effect in seconds.
    pub duration: f32,
}

impl Ripple {
    /// Creates a new surface ripple at the specified coordinates.
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            radius: 4.0,
            max_radius: 130.0,
            speed: 110.0,
            age: 0.0,
            duration: 1.3,
        }
    }

    /// Advances the ripple animation by `dt` seconds.
    /// Returns `true` while the ripple is still active, or `false` once expired.
    pub fn update(&mut self, dt: f32) -> bool {
        self.age += dt;
        self.radius += self.speed * dt;
        self.age < self.duration && self.radius < self.max_radius
    }

    /// Normalized progress from 0.0 (just spawned) to 1.0 (fully faded).
    pub fn progress(&self) -> f32 {
        (self.age / self.duration).clamp(0.0, 1.0)
    }

    /// Current opacity factor [0.0, 1.0] with smooth ease-out fading.
    pub fn alpha(&self) -> f32 {
        let p = self.progress();
        (1.0 - p).powf(1.4)
    }

    /// Evaluates disturbance strength at a given point for fish reaction.
    #[allow(dead_code)]
    pub fn disturbance_at(&self, px: f32, py: f32) -> f32 {
        let dx = px - self.x;
        let dy = py - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let wave_dist = (dist - self.radius).abs();
        if wave_dist < 40.0 {
            let falloff = 1.0 - (wave_dist / 40.0);
            falloff * self.alpha()
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripple_progression_and_expiration() {
        let mut ripple = Ripple::new(100.0, 100.0);
        assert_eq!(ripple.x, 100.0);
        assert_eq!(ripple.y, 100.0);
        assert!(ripple.alpha() > 0.9);

        // Advance halfway
        let active = ripple.update(0.65);
        assert!(active);
        assert!(ripple.radius > 4.0);
        assert!(ripple.alpha() < 0.9 && ripple.alpha() > 0.0);

        // Advance past duration
        let active_expired = ripple.update(1.0);
        assert!(!active_expired);
        assert_eq!(ripple.alpha(), 0.0);
    }

    #[test]
    fn test_disturbance_calculation() {
        let ripple = Ripple::new(100.0, 100.0);
        let direct_hit = ripple.disturbance_at(100.0, 104.0);
        assert!(direct_hit > 0.0);

        let far_away = ripple.disturbance_at(500.0, 500.0);
        assert_eq!(far_away, 0.0);
    }
}
