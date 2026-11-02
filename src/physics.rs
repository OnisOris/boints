use crate::api::{Color, Vec3};
use rapier3d::prelude::*;

pub const DT: f32 = 1.0 / 60.0;

/// A generational Rapier handle plus scene generation: stale IDs fail after reset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectId {
    handle: RigidBodyHandle,
    scene: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct ObjectState {
    pub id: ObjectId,
    pub position: Vec3,
    pub velocity: Vec3,
}

#[derive(Clone, Copy)]
pub enum Shape {
    Ball(f32),
    Box(Vec3),
}
#[derive(Clone, Copy)]
pub struct Visual {
    pub id: ObjectId,
    pub shape: Shape,
    pub color: Color,
}

pub struct Sim {
    /// Advanced escape hatch. Use Sim's spawn/remove methods to keep visuals in sync.
    pub world: PhysicsWorld,
    pub agents: usize,
    pub time: f64,
    scene: u64,
    visuals: Vec<Visual>,
}

// Rapier and Macroquad use different glam versions; convert only at this boundary.
fn physics(v: Vec3) -> Vector {
    Vector::new(v.x, v.y, v.z)
}
fn graphics(v: Vector) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

impl Sim {
    pub fn new(agents: usize) -> Self {
        Self {
            world: PhysicsWorld::default(),
            agents,
            time: 0.0,
            scene: 0,
            visuals: Vec::new(),
        }
    }
    pub fn clear(&mut self) {
        self.world = PhysicsWorld::default();
        self.visuals.clear();
        self.time = 0.0;
        self.scene = self
            .scene
            .checked_add(1)
            .expect("scene generation overflow");
    }
    fn insert(
        &mut self,
        body: RigidBodyBuilder,
        collider: ColliderBuilder,
        shape: Shape,
    ) -> ObjectId {
        let (handle, _) = self
            .world
            .insert(body, collider.friction(0.45).restitution(0.75));
        let id = ObjectId {
            handle,
            scene: self.scene,
        };
        self.visuals.push(Visual {
            id,
            shape,
            color: Color::new(0.2, 0.8, 0.95, 1.0),
        });
        id
    }
    pub fn spawn_ball(&mut self, position: Vec3, radius: f32) -> ObjectId {
        assert!(
            position.is_finite() && radius.is_finite() && radius > 0.0,
            "invalid ball dimensions"
        );
        self.insert(
            RigidBodyBuilder::dynamic()
                .translation(physics(position))
                .ccd_enabled(true),
            ColliderBuilder::ball(radius),
            Shape::Ball(radius),
        )
    }
    /// `half_extents` are half the full size, matching Rapier's cuboid convention.
    pub fn spawn_box(&mut self, position: Vec3, half_extents: Vec3) -> ObjectId {
        assert!(
            position.is_finite() && half_extents.is_finite() && half_extents.min_element() > 0.0,
            "invalid box dimensions"
        );
        self.insert(
            RigidBodyBuilder::fixed().translation(physics(position)),
            ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z),
            Shape::Box(half_extents),
        )
    }
    fn body(&self, id: ObjectId) -> Option<&RigidBody> {
        (id.scene == self.scene)
            .then(|| self.world.bodies.get(id.handle))
            .flatten()
    }
    fn body_mut(&mut self, id: ObjectId) -> Option<&mut RigidBody> {
        (id.scene == self.scene)
            .then(|| self.world.bodies.get_mut(id.handle))
            .flatten()
    }
    pub fn state(&self, id: ObjectId) -> Option<ObjectState> {
        let body = self.body(id)?;
        Some(ObjectState {
            id,
            position: graphics(body.translation()),
            velocity: graphics(body.linvel()),
        })
    }
    pub fn states(&self) -> impl Iterator<Item = ObjectState> + '_ {
        self.visuals.iter().filter_map(|v| self.state(v.id))
    }
    pub fn ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.states().map(|s| s.id)
    }
    pub fn len(&self) -> usize {
        self.states().count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn set_velocity(&mut self, id: ObjectId, velocity: Vec3) -> bool {
        if !velocity.is_finite() {
            return false;
        }
        if let Some(body) = self.body_mut(id) {
            body.set_linvel(physics(velocity), true);
            true
        } else {
            false
        }
    }
    pub fn set_position(&mut self, id: ObjectId, position: Vec3) -> bool {
        if !position.is_finite() {
            return false;
        }
        if let Some(body) = self.body_mut(id) {
            body.set_translation(physics(position), true);
            true
        } else {
            false
        }
    }
    /// Force acts for the next fixed step; the shell clears it after that step.
    pub fn apply_force(&mut self, id: ObjectId, force: Vec3) -> bool {
        if !force.is_finite() {
            return false;
        }
        if let Some(body) = self.body_mut(id) {
            body.add_force(physics(force), true);
            true
        } else {
            false
        }
    }
    pub fn set_gravity(&mut self, gravity: Vec3) {
        if gravity.is_finite() {
            self.world.gravity = physics(gravity);
        }
    }
    pub fn set_color(&mut self, id: ObjectId, color: Color) -> bool {
        if let Some(v) = self.visuals.iter_mut().find(|v| v.id == id) {
            v.color = color;
            true
        } else {
            false
        }
    }
    pub fn set_material(&mut self, id: ObjectId, restitution: f32, friction: f32) -> bool {
        if !restitution.is_finite() || !friction.is_finite() {
            return false;
        }
        let Some(body) = self.body(id) else {
            return false;
        };
        let Some(handle) = body.colliders().first().copied() else {
            return false;
        };
        let Some(collider) = self.world.colliders.get_mut(handle) else {
            return false;
        };
        collider.set_restitution(restitution.clamp(0.0, 1.0));
        collider.set_friction(friction.max(0.0));
        true
    }
    pub fn remove(&mut self, id: ObjectId) -> bool {
        if id.scene != self.scene || self.world.remove_body(id.handle).is_none() {
            return false;
        }
        self.visuals.retain(|v| v.id != id);
        true
    }
    pub fn step(&mut self) {
        self.world.integration_parameters.dt = DT;
        self.world.step();
        for (_, body) in self.world.bodies.iter_mut() {
            body.reset_forces(false);
        }
        self.time += DT as f64;
    }
    pub fn visuals(&self) -> impl Iterator<Item = (&Visual, &RigidBody)> {
        self.visuals
            .iter()
            .filter_map(|v| Some((v, self.body(v.id)?)))
    }
}
