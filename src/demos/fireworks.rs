use crate::{
    gfx::{Color, ColorDiff, RenderDest, RenderOptions, Renderer},
    math::Vec2,
};

pub struct Fireworks {
    sparklers: Vec<Sparkler>,
    streamers: Vec<Sparkler>,
}

struct Sparkler {
    pos: Vec2,
    vel: Vec2,
    a: Vec2,
    color: Color,
    streamer_color: Color,
    dcolor: ColorDiff,
    lifetime: f32,
}

impl Fireworks {
    pub fn new() -> Self {
        Self {
            sparklers: Vec::new(),
            streamers: Vec::new(),
        }
    }

    const G: Vec2 = Vec2(0.0, 40.0);

    pub fn add_firework(&mut self, pos: Vec2, color: Color) {
        let lifetime = 2.0;
        let dcolor = (Color::WHITE.with_alpha(0.0) - Color::WHITE) / lifetime;

        for _ in 0..30 {
            // Fake 3D fireworks in orthogonal perspective
            let a1 = fastrand::i32(0..360) as f32;
            let a2 = fastrand::i32(0..360) as f32;

            let v1 = Vec2::for_angle(a1, 800.0);
            let v2 = Vec2::for_angle(a2, 800.0);
            let vel = v1.project(v2);

            self.sparklers.push(Sparkler {
                pos,
                vel,
                a: Self::G - vel * 0.3,
                lifetime,
                color: Color::WHITE,
                streamer_color: color,
                dcolor,
            });
        }
    }

    pub fn step(&mut self, timestep: f32) {
        for p in self.sparklers.iter_mut() {
            p.vel = p.vel
                + (Self::G - (p.vel.normalized() * (0.02 * p.vel.magnitude_squared()))) * timestep;
            p.pos = p.pos + p.vel * timestep;
            p.lifetime -= timestep;
            p.color = p.color + p.dcolor * timestep;

            let lifetime = p.lifetime / 2.0;
            self.streamers.push(Sparkler {
                pos: p.pos,
                vel: Vec2::ZERO,
                a: Vec2::ZERO,
                lifetime,
                color: p.streamer_color,
                streamer_color: p.streamer_color,
                dcolor: (p.streamer_color.with_alpha(0.0) - p.streamer_color) / lifetime,
            });

            // Random sparklies at the end
            if lifetime <= 0.1 && fastrand::f32() < 0.1 {
                let lifetime = fastrand::f32() * 1.2;
                self.streamers.push(Sparkler {
                    pos: p.pos + Vec2(fastrand::f32() * 60.0 - 30.0, fastrand::f32() * 60.0 - 30.0),
                    vel: Vec2::ZERO,
                    a: Vec2::ZERO,
                    lifetime,
                    color: Color::WHITE,
                    streamer_color: Color::WHITE,
                    dcolor: (Color::WHITE.with_alpha(0.0) - Color::WHITE) / lifetime,
                });
            }
        }

        for p in self.streamers.iter_mut() {
            p.vel = p.vel + p.a * timestep;
            p.pos = p.pos + p.vel * timestep;
            p.lifetime -= timestep;
            p.color = p.color + p.dcolor * timestep;
        }
        self.sparklers.retain(|p| p.lifetime > 0.0);
        self.streamers.retain(|p| p.lifetime > 0.0);
    }

    pub fn render(&self, renderer: &Renderer) {
        let texture = renderer.texture_store().get_texture(
            renderer
                .texture_store()
                .find_texture(b"sparkle")
                .expect("sparkle texture needed"),
        );

        let mut opts = RenderOptions::default();
        for particle in self.streamers.iter() {
            opts.dest = RenderDest::Centered(particle.pos);
            opts.color = particle.color;
            texture.render(renderer, &opts);
        }

        for particle in self.sparklers.iter() {
            opts.dest = RenderDest::Centered(particle.pos);
            opts.color = particle.color;
            texture.render(renderer, &opts);
        }
    }
}
