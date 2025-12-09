use crate::{
    call_state_method,
    game::{
        PlayerId,
        level::{Level, TerrainLineHit, terrain::Terrain},
        objects::GameObject,
    },
    get_state_method,
    math::{LineF, Vec2},
};

/**
 * A special projectile that lives for only one tick.
 */
#[derive(Clone, Debug)]
pub struct HitscanProjectile {
    start: Vec2,
    stop: Vec2,
    owner: PlayerId,

    /// If true, hitscan is limited by terrain (and terrain type is set)
    hit_terrain: bool,

    /// If false, on_hit_object is called only for the closest object
    hit_multiple: bool,

    /// Nearest distance (squared) set when hit_multiple is false
    nearest_dist: f32,

    /// Type of terrain that was hit
    terrain: Terrain,

    /// Scripting state
    state: Option<mlua::Table>,
}

impl mlua::UserData for HitscanProjectile {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("is_hitscan", true);
        fields.add_field_method_get("start", |_, this| Ok(this.start));
        fields.add_field_method_get("stop", |_, this| Ok(this.stop));
        fields.add_field_method_get("owner", |_, this| Ok(this.owner));
        fields.add_field_method_get("terrain", |_, this| Ok(this.terrain));
        fields.add_field_method_get("state", |_, this| Ok(this.state.clone()));

        // compatibility with Projectiles
        fields.add_field_method_get("vel", |_, this| Ok(this.stop - this.start));
    }
}

impl mlua::FromLua for HitscanProjectile {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            let start: Vec2 = table.get("start")?;
            let stop: Vec2 = table.get("stop")?;
            Ok(Self {
                start,
                stop,
                owner: table.get::<Option<PlayerId>>("owner")?.unwrap_or(0),
                terrain: 0,
                hit_terrain: table.get::<Option<bool>>("hit_terrain")?.unwrap_or(true),
                hit_multiple: table.get::<Option<bool>>("hit_multiple")?.unwrap_or(false),
                nearest_dist: f32::MAX,
                state: table.get("state")?,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "HitscanProjectile".to_owned(),
                message: Some("expected a table describing a hitscan projectile".to_string()),
            })
        }
    }
}

impl HitscanProjectile {
    pub fn start(&self) -> Vec2 {
        self.start
    }

    pub fn stop(&self) -> Vec2 {
        self.stop
    }

    pub fn left(&self) -> f32 {
        self.start.0.min(self.stop.0)
    }

    pub fn right(&self) -> f32 {
        self.start.0.max(self.stop.0)
    }

    pub fn owner(&self) -> PlayerId {
        self.owner
    }

    pub fn hit_level(&mut self, level: &Level) {
        if self.hit_terrain {
            match level.terrain_line(LineF(self.start, self.stop)) {
                TerrainLineHit::Hit(t, pos) => {
                    self.stop = pos;
                    self.terrain = t;
                }
                TerrainLineHit::Miss(t) => self.terrain = t,
            }
        }
    }

    fn circle_linesegment(a: Vec2, b: Vec2, c: Vec2, r: f32) -> Option<Vec2> {
        let ac = c - a;
        let ab = b - a;

        let ad = ac.project(ab);
        let d = ad + a;

        let dist = d.dist_squared(c);
        if dist.is_nan() {
            return None;
        }

        let k = if ad.0 > ad.1 {
            ad.0 / ab.0
        } else {
            ad.1 / ab.1
        };

        if (0.0..=1.0).contains(&k) && dist <= (r * r) {
            Some(d)
        } else {
            None
        }
    }

    pub fn check_hit<T>(&self, obj: &mut T) -> Option<Vec2>
    where
        T: GameObject,
    {
        // Note: normal object collisions involve two radiuses, whereas the hitscan
        // beam has a radius of zero. To compensate for the difference in feel, we buff up the target radius.
        Self::circle_linesegment(self.start, self.stop, obj.pos(), obj.radius() * 2.0)
    }

    /// Check for a hit against this object and execute callback if hit_multiple is true
    /// Returns true if this was the nearest object hit so far (only if hit_multiple is false)
    pub fn do_hit_object<T>(&mut self, lua: &mlua::Lua, obj: &mut T) -> bool
    where
        T: GameObject + mlua::UserData + 'static,
    {
        if let Some(hitpos) = self.check_hit(obj) {
            if self.hit_multiple {
                self.on_hit_object(lua, obj);
            } else {
                let d = hitpos.dist_squared(self.start);
                if d < self.nearest_dist {
                    self.stop = hitpos;
                    self.nearest_dist = d;
                    return true;
                }
            }
        }

        false
    }

    pub fn on_hit_object<T>(&self, lua: &mlua::Lua, obj: &mut T) -> bool
    where
        T: mlua::UserData + 'static,
    {
        get_state_method!(self, lua, "on_hit_object", (f, scope) => {
            f.call::<Option<bool>>((
                scope.create_userdata_ref(self)?,
                scope.create_userdata_ref_mut(obj)?,
            ))
        })
        .unwrap_or(true)
    }

    pub fn on_done(&mut self, lua: &mlua::Lua) {
        call_state_method!(*self, lua, "on_done");
    }
}
