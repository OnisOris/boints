use algorithm_playground::{algorithms::ALGORITHMS, simulation::Simulation};
use macroquad::prelude::*;
use std::fmt::Write;

pub struct Ui {
    pub grid: bool,
    text: String,
}
impl Default for Ui {
    fn default() -> Self {
        Self {
            grid: true,
            text: String::with_capacity(200),
        }
    }
}
fn panels() -> (Rect, Rect) {
    let width = screen_width();
    let height = if width < 600.0 { 160.0 } else { 112.0 };
    (
        Rect::new(0.0, 0.0, width, 84.0),
        Rect::new(0.0, screen_height() - height, width, height),
    )
}
pub fn over_ui(point: Vec2) -> bool {
    let (a, b) = panels();
    a.contains(point) || b.contains(point)
}
fn button(rect: Rect, label: &str, touches: &[Touch], accent: bool) -> bool {
    let pointer = Vec2::from(mouse_position());
    let hover = rect.contains(pointer);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if accent {
            Color::new(0.13, 0.39, 0.46, 1.0)
        } else if hover {
            Color::new(0.22, 0.28, 0.38, 1.0)
        } else {
            Color::new(0.14, 0.19, 0.27, 1.0)
        },
    );
    let size = if rect.w < 100.0 { 16.0 } else { 19.0 };
    let text = measure_text(label, None, size as u16, 1.0);
    draw_text(
        label,
        rect.x + (rect.w - text.width) * 0.5,
        rect.y + rect.h * 0.5 + size * 0.33,
        size,
        WHITE,
    );
    (hover && is_mouse_button_pressed(MouseButton::Left))
        || touches
            .iter()
            .any(|t| t.phase == TouchPhase::Started && rect.contains(t.position))
}
impl Ui {
    /// Returns camera reset; all other commands apply to the simulation here.
    pub fn draw(&mut self, session: &mut Simulation, touches: &[Touch]) -> bool {
        let (top, bottom) = panels();
        let panel_color = Color::new(0.055, 0.075, 0.115, 0.97);
        for rect in [top, bottom] {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, panel_color);
        }
        draw_text(
            "ALGORITHM / 3D",
            14.0,
            27.0,
            23.0,
            Color::new(0.55, 0.88, 0.92, 1.0),
        );
        self.text.clear();
        let _ = write!(
            self.text,
            "{}  |  {} FPS  |  {} objects  |  {:.1}s",
            session.algorithm.name(),
            get_fps(),
            session.sim.len(),
            session.sim.time
        );
        draw_text(
            &self.text,
            14.0,
            51.0,
            if screen_width() < 420.0 { 15.0 } else { 18.0 },
            LIGHTGRAY,
        );
        draw_text(
            "Drag to orbit / scroll or pinch to zoom",
            14.0,
            72.0,
            15.0,
            GRAY,
        );
        let mut y = bottom.y + 8.0;
        self.text.clear();
        let _ = write!(self.text, "Algorithm: {}  >", session.algorithm.name());
        if button(
            Rect::new(8.0, y, screen_width() - 16.0, 40.0),
            &self.text,
            touches,
            false,
        ) {
            session.select((session.selected + 1) % ALGORITHMS.len());
        }
        y += 48.0;
        let cols = if screen_width() < 600.0 { 4 } else { 8 };
        let width = (screen_width() - 16.0 - (cols - 1) as f32 * 6.0) / cols as f32;
        let speed_label = match session.speed {
            s if s < 0.5 => "0.25x",
            s if s > 1.5 => "2x",
            _ => "1x",
        };
        let labels = [
            if session.playing { "Pause" } else { "Play" },
            "Step",
            "Reset",
            speed_label,
            "Agents -",
            "Agents +",
            "Camera",
            if self.grid { "Grid on" } else { "Grid off" },
        ];
        let mut camera_reset = false;
        for (i, label) in labels.iter().enumerate() {
            let rect = Rect::new(
                8.0 + (i % cols) as f32 * (width + 6.0),
                y + (i / cols) as f32 * 48.0,
                width,
                42.0,
            );
            if button(rect, label, touches, i == 0) {
                match i {
                    0 => session.set_playing(!session.playing),
                    1 => session.single_step(),
                    2 => session.reset(),
                    3 => {
                        session.speed = if session.speed < 0.5 {
                            1.0
                        } else if session.speed < 1.5 {
                            2.0
                        } else {
                            0.25
                        }
                    }
                    4 | 5 => {
                        session.sim.agents = if i == 4 {
                            session.sim.agents.saturating_sub(25).max(25)
                        } else {
                            (session.sim.agents + 25).min(200)
                        };
                        session.reset();
                    }
                    6 => camera_reset = true,
                    7 => self.grid = !self.grid,
                    _ => {}
                }
            }
        }
        camera_reset
    }
}
