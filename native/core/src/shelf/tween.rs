//! A value eased toward a target over a fixed time (the current UI's
//! `animate(node, prop, value, { dur, easing: "out" })`).

#[derive(Clone, Copy, Debug)]
pub struct Tween {
    from: f32,
    to: f32,
    /// When the current move started, µs.
    start: u64,
    duration_us: u64,
}

/// Cubic ease-out: fast at first, settling at the end.
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

impl Tween {
    pub fn new(value: f32, duration_ms: u32) -> Tween {
        Tween { from: value, to: value, start: 0, duration_us: duration_ms as u64 * 1000 }
    }

    /// The value at `now`.
    pub fn at(&self, now: u64) -> f32 {
        let t = (now.saturating_sub(self.start) as f32 / self.duration_us as f32).min(1.0);
        self.from + (self.to - self.from) * ease_out(t)
    }

    pub fn target(&self) -> f32 {
        self.to
    }

    /// Move toward `to` from where the value is at `now`.
    pub fn set(&mut self, to: f32, now: u64) {
        if to != self.to {
            self.from = self.at(now);
            self.to = to;
            self.start = now;
        }
    }

    /// Stand at `value` with no movement.
    pub fn jump(&mut self, value: f32) {
        self.from = value;
        self.to = value;
    }

    pub fn moving(&self, now: u64) -> bool {
        now.saturating_sub(self.start) < self.duration_us && self.from != self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eases_to_the_target_and_restarts_from_where_it_is() {
        let mut tween = Tween::new(0.0, 200);
        tween.set(100.0, 1_000);
        assert_eq!(tween.at(1_000), 0.0);
        assert!(tween.at(101_000) > 50.0);
        assert_eq!(tween.at(201_000), 100.0);
        tween.set(0.0, 101_000);
        let middle = tween.at(101_000);
        assert!(middle > 50.0 && middle < 100.0);
        assert_eq!(tween.at(301_000), 0.0);
    }
}
