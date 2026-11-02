use macroquad::prelude::*;

pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    mouse: Option<Vec2>,
    previous: Vec<(u64, Vec2)>,
    blocked: Vec<u64>,
}
impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.65,
            pitch: 0.5,
            distance: 28.0,
            mouse: None,
            previous: Vec::with_capacity(10),
            blocked: Vec::with_capacity(10),
        }
    }
}
impl OrbitCamera {
    pub fn update(&mut self, touches: &[Touch], over_ui: impl Fn(Vec2) -> bool) {
        for touch in touches {
            if touch.phase == TouchPhase::Started && over_ui(touch.position) {
                self.blocked.push(touch.id);
            }
        }
        let mut active = touches.iter().filter(|t| {
            !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled)
                && !self.blocked.contains(&t.id)
        });
        let first = active.next();
        let second = active.next();
        if let (Some(a), Some(b)) = (first, second) {
            // Match by ID, since Macroquad's HashMap iteration order can change.
            let old_a = self.previous.iter().find(|(id, _)| *id == a.id);
            let old_b = self.previous.iter().find(|(id, _)| *id == b.id);
            if let (Some((_, pa)), Some((_, pb))) = (old_a, old_b) {
                let old_distance = pa.distance(*pb);
                let new_distance = a.position.distance(b.position);
                if old_distance > 4.0 && new_distance > 4.0 {
                    self.distance *= old_distance / new_distance;
                }
            }
        } else if let Some(touch) = first {
            // Do not rotate on the transition from two fingers to one.
            if self.previous.len() == 1
                && !over_ui(touch.position)
                && let Some((_, old)) = self.previous.iter().find(|(id, _)| *id == touch.id)
            {
                let delta = touch.position - *old;
                self.yaw -= delta.x * 0.007;
                self.pitch += delta.y * 0.007;
            }
        }
        self.previous.clear();
        for touch in touches {
            if !matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled)
                && !self.blocked.contains(&touch.id)
            {
                self.previous.push((touch.id, touch.position));
            }
        }
        self.blocked.retain(|id| {
            touches.iter().any(|t| {
                t.id == *id && !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled)
            })
        });
        let pointer = Vec2::from(mouse_position());
        if touches.is_empty() {
            if is_mouse_button_pressed(MouseButton::Left) && !over_ui(pointer) {
                self.mouse = Some(pointer);
            }
            if !is_mouse_button_down(MouseButton::Left) {
                self.mouse = None;
            }
            if let Some(old) = self.mouse {
                if !over_ui(pointer) {
                    let delta = pointer - old;
                    self.yaw -= delta.x * 0.007;
                    self.pitch += delta.y * 0.007;
                }
                self.mouse = Some(pointer);
            }
            if !over_ui(pointer) {
                self.distance *= (-mouse_wheel().1 * 0.1).exp();
            }
        } else {
            self.mouse = None;
        }
        self.pitch = self.pitch.clamp(0.08, 1.4);
        self.distance = self.distance.clamp(5.0, 70.0);
        self.yaw = self.yaw.rem_euclid(std::f32::consts::TAU);
    }
    pub fn view(&self) -> Camera3D {
        let target = vec3(0.0, 3.0, 0.0);
        Camera3D {
            position: target
                + self.distance
                    * vec3(
                        self.yaw.sin() * self.pitch.cos(),
                        self.pitch.sin(),
                        self.yaw.cos() * self.pitch.cos(),
                    ),
            target,
            up: Vec3::Y,
            z_near: 0.1,
            z_far: 200.0,
            aspect: Some(screen_width().max(1.0) / screen_height().max(1.0)),
            ..Default::default()
        }
    }
}
