#[macro_export]
macro_rules! call_state_method {
    ($obj:expr, $lua:ident, $name:literal $(, $param:expr)*) => {
       if let Some(state) = $obj.state.as_mut() {
            match state.get::<Option<mlua::Function>>($name) {
                Ok(Some(f)) => {
                    if let Err(err) = $lua.scope(|scope| {
                        f.call::<()>((
                            scope.create_userdata_ref_mut(&mut $obj)?,
                            $($param,)*
                        ))
                    }) {
                        log::error!("{}: {}", $name, err);
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    log::error!("Couldn't get state function {}: {}", $name, err);
                }
            }
        }
   };
}

#[macro_export]
macro_rules! get_state_method {
    ($obj:ident, $lua:ident, $name:literal, ($f:ident, $scope:ident) => $block:block) => {
        if let Some(state) = $obj.state.as_ref() {
            match state.get::<Option<mlua::Function>>($name) {
                Ok(Some($f)) => match $lua.scope(|$scope| $block) {
                    Ok(o) => o,
                    Err(err) => {
                        log::error!(concat!($name, ": {}"), err);
                        None
                    }
                },
                Ok(None) => None,
                Err(err) => {
                    log::error!(concat!("Couldn't get state function ", $name, ": {}"), err);
                    None
                }
            }
        } else {
            None
        }
    };
}

#[macro_export]
macro_rules! gameobject_timer {
    ($obj:expr, $lua:ident, $timestep:ident) => {
        if let Some(timer) = &mut $obj.timer {
            *timer -= $timestep;
            $obj.timer_accumulator += $timestep;
            if *timer <= 0.0 {
                let acc = $obj.timer_accumulator;
                $obj.timer_accumulator = 0.0;
                match $lua.scope(|scope| {
                    $lua.globals()
                        .get::<mlua::Function>("luola_on_object_timer")?
                        .call::<Option<f32>>((scope.create_userdata_ref_mut(&mut $obj)?, acc))
                }) {
                    Ok(rerun) => {
                        $obj.timer = rerun;
                    }
                    Err(err) => {
                        log::error!("Object timer error: {err}");
                        $obj.timer = None;
                    }
                };
            }
        }
    };
}
