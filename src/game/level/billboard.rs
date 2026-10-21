use crate::{
    game::level::{BillboardToml, LEVEL_SCALE},
    gfx::{Texture, TextureStore},
    math::RectF,
};

/**
 * A non-parallax background image drawn behind the level artwork.
 */
pub struct Billboard {
    pub bounds: RectF,
    pub texture: Texture,
}

impl Billboard {
    pub fn from_toml(
        config: &[BillboardToml],
        ts: &TextureStore,
    ) -> (Vec<Billboard>, Vec<Billboard>) {
        let mut bgs = Vec::new();
        let mut fgs = Vec::new();

        for c in config {
            let tex_id = match ts.find_texture(&c.texture.as_bytes()) {
                Ok(t) => t,
                Err(err) => {
                    log::error!("Couldn't make background billboard: {}", err);
                    continue;
                }
            };

            // Reminder: texture clones are COW and thus cheap, and this way we don't need
            // to pass TextureStore to the level render function.
            let texture = ts.get_texture(tex_id).clone();

            let bb = Billboard {
                bounds: RectF::new(
                    c.pos.0 as f32 * LEVEL_SCALE,
                    c.pos.1 as f32 * LEVEL_SCALE,
                    texture.width() * LEVEL_SCALE,
                    texture.height() * LEVEL_SCALE,
                ),
                texture,
            };

            if c.fg {
                fgs.push(bb);
            } else {
                bgs.push(bb);
            }
        }
        (bgs, fgs)
    }
}
