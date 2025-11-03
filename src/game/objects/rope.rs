use crate::{
    game::objects::PhysicalObject,
    gfx::{Color, Renderer},
    math::{LineF, Vec2},
};

/**
 * Rope physics
 */
#[derive(Clone, Debug)]
pub struct Rope {
    endpoint: Vec2,
    length: f32,
}

impl Rope {
    pub fn new(startpoint: Vec2, endpoint: Vec2) -> Self {
        Self {
            endpoint,
            length: startpoint.dist(endpoint),
        }
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn adjust(&mut self, dl: f32) {
        self.length = (self.length + dl).max(1.0);
    }

    pub fn render(&self, other_end: Vec2, renderer: &Renderer, camera_pos: Vec2) {
        renderer.draw_line(
            Color::new(1.0, 1.0, 1.0),
            LineF(other_end - camera_pos, self.endpoint - camera_pos),
        );
    }

    /**
     * Perform a physics step.
     */
    pub fn physics_step(&self, phys: &mut PhysicalObject) {
        // See Michael Schmidt Nissen's stable springs paper.
        // Currently, the other end of the spring is fixed,
        // and the coefficients are all 1, so the spring has infinite
        // stiffness.

        let dist = phys.pos - self.endpoint;
        let unit = dist.normalized();

        let d_err = unit.dot(dist) - self.length;
        let v_err = unit.dot(phys.vel);

        let dist_i = d_err * 60.0; // FPS is currently fixed
        let vel_i = v_err;

        let impulse = -(dist_i + vel_i) / phys.imass;

        phys.add_impulse(unit * impulse);
    }
}
