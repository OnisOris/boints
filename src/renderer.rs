use algorithm_playground::{api::Sim, physics::Shape};
use macroquad::prelude::*;

pub struct Renderer {
    sphere: Mesh,
    cube: Mesh,
    batch: Mesh,
}
impl Renderer {
    pub fn new() -> Self {
        let mut sphere = Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            texture: None,
        };
        let (rings, slices) = (6, 10);
        for ring in 0..=rings {
            let phi = std::f32::consts::PI * ring as f32 / rings as f32;
            for slice in 0..=slices {
                let theta = std::f32::consts::TAU * slice as f32 / slices as f32;
                let p = vec3(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                sphere.vertices.push(Vertex::new2(p, Vec2::ZERO, WHITE));
            }
        }
        for ring in 0..rings {
            for slice in 0..slices {
                let a = (ring * (slices + 1) + slice) as u16;
                let b = a + (slices + 1) as u16;
                sphere
                    .indices
                    .extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }
        let mut cube = Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            texture: None,
        };
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                for x in [-1.0, 1.0] {
                    cube.vertices
                        .push(Vertex::new2(vec3(x, y, z), Vec2::ZERO, WHITE));
                }
            }
        }
        cube.indices.extend_from_slice(&[
            0, 1, 3, 0, 3, 2, 4, 6, 7, 4, 7, 5, 0, 4, 5, 0, 5, 1, 2, 3, 7, 2, 7, 6, 0, 2, 6, 0, 6,
            4, 1, 5, 7, 1, 7, 3,
        ]);
        Self {
            sphere,
            cube,
            batch: Mesh {
                vertices: Vec::with_capacity(16000),
                indices: Vec::with_capacity(80000),
                texture: None,
            },
        }
    }
    pub fn draw(&mut self, sim: &Sim, grid: bool) {
        self.batch.vertices.clear();
        self.batch.indices.clear();
        for (visual, body) in sim.visuals() {
            let (template, scale) = match visual.shape {
                Shape::Ball(radius) => (&self.sphere, Vec3::splat(radius)),
                Shape::Box(size) => (&self.cube, size),
            };
            // Bound a batch for u16 indices and the configured Macroquad buffers.
            if self.batch.vertices.len() + template.vertices.len() > 24000
                || self.batch.indices.len() + template.indices.len() > 120000
            {
                draw_mesh(&self.batch);
                self.batch.vertices.clear();
                self.batch.indices.clear();
            }
            let base = self.batch.vertices.len() as u16;
            let p = body.translation();
            let q = body.rotation();
            let rotation = Quat::from_xyzw(q.x, q.y, q.z, q.w);
            for vertex in &template.vertices {
                let position = rotation * (vertex.position * scale) + vec3(p.x, p.y, p.z);
                // Cheap baked vertex shading gives low-poly shapes depth without shaders.
                let shade = 0.65
                    + 0.35
                        * (vertex
                            .position
                            .normalize()
                            .dot(vec3(-0.3, 0.8, 0.5).normalize()))
                        .max(0.0);
                let c = visual.color;
                self.batch.vertices.push(Vertex::new2(
                    position,
                    Vec2::ZERO,
                    Color::new(c.r * shade, c.g * shade, c.b * shade, c.a),
                ));
            }
            self.batch
                .indices
                .extend(template.indices.iter().map(|i| base + i));
        }
        draw_mesh(&self.batch);
        if grid {
            draw_grid_ex(
                24,
                1.0,
                Color::new(0.32, 0.42, 0.55, 1.0),
                Color::new(0.2, 0.27, 0.36, 1.0),
                vec3(0.0, 0.02, 0.0),
                Quat::IDENTITY,
            );
            draw_line_3d(Vec3::ZERO, vec3(2.0, 0.0, 0.0), RED);
            draw_line_3d(Vec3::ZERO, vec3(0.0, 2.0, 0.0), GREEN);
            draw_line_3d(Vec3::ZERO, vec3(0.0, 0.0, 2.0), BLUE);
        }
    }
}
