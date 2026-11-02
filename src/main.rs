mod camera;
mod renderer;
mod ui;

use algorithm_playground::simulation::Simulation;
use macroquad::prelude::*;

fn config() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Rust Algorithm Playground".into(),
            window_width: 1100,
            window_height: 760,
            high_dpi: false,
            sample_count: 1,
            ..Default::default()
        },
        draw_call_vertex_capacity: 32768,
        draw_call_index_capacity: 131072,
        ..Default::default()
    }
}
#[macroquad::main(config)]
async fn main() {
    simulate_mouse_with_touch(false);
    let mut session = Simulation::new(100);
    let mut renderer = renderer::Renderer::new();
    let mut camera = camera::OrbitCamera::default();
    let mut ui = ui::Ui::default();
    loop {
        let touches = touches(); // Macroquad returns an owned list; share it with UI/camera.
        clear_background(Color::new(0.035, 0.05, 0.08, 1.0));
        camera.update(&touches, ui::over_ui);
        session.frame(get_frame_time());
        set_camera(&camera.view());
        renderer.draw(&session.sim, ui.grid);
        set_default_camera();
        if ui.draw(&mut session, &touches) {
            camera = camera::OrbitCamera::default();
        }
        next_frame().await;
    }
}
