use anyhow::{Result, anyhow};
use core::ops::Deref;
use mlua::{FromLua, Function, Lua, LuaSerdeExt, String as LuaString, Table, UserData, Value};
use sdl3_sys::keyboard::SDL_GetKeyName;
use sdl3_sys::keycode::SDL_Keycode;
use std::{cell::RefCell, ffi::CStr, rc::Rc, sync::Arc};

use crate::{
    configfile::{GAME_CONFIG, UserConfig, save_user_config},
    events,
    fs::find_datafile_path,
    game::{GameControllerSet, MenuButton},
    gfx::{
        Color, RenderDest, RenderOptions, RenderTextDest, RenderTextOptions, Renderer, Text,
        TextOutline, TextureId,
    },
    math::{RectF, Vec2},
};

enum MenuAction {
    None,
    Push(MenuScreen),
    Pop,
    Return(String),
    KeyGrab, // activate keygrab mode and assign KeyGrab value to current selection when done
}

type CachedText = RefCell<Option<Text>>;

enum MenuItemValue {
    None,
    Toggle(bool, CachedText),
    KeyGrab(SDL_Keycode, CachedText),
}

enum MenuItemContent {
    Text(Text),
    Image(TextureId),
    Blank,
}

struct MenuItem {
    content: MenuItemContent,
    center: bool,
    rect: RectF,
    value: MenuItemValue,
    action: Option<Function>,
}

#[derive(Copy, Clone)]
enum MenuScreenState {
    Appearing(f32),
    Normal,
    Hiding(f32),
    Hidden, // Note: hiding the topmost menu screen will close it
}

impl MenuScreenState {
    fn is_visible(&self) -> bool {
        !matches!(self, MenuScreenState::Hidden)
    }
}

struct MenuScreen {
    items: Vec<MenuItem>,
    current: usize,
    state: MenuScreenState,
    animated_offset: Vec2,
    cursorpos: Vec2,

    on_exit: Option<Function>,
}

pub struct LuaMenu {
    lua: Lua,
    renderer: Rc<RefCell<Renderer>>,
    window: Rc<RefCell<RectF>>, // where to draw the menu
    menu_stack: Vec<MenuScreen>,
    cursor: Text,
    cursor_bounce: f32, // cursor bounce animation state
    key_grabbing: bool, // key grab in progress
    key_grab_text: Text,
}

impl UserData for MenuAction {}
impl UserData for MenuScreen {}

impl UserData for MenuItem {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("toggle", |_, this, _: ()| {
            if let MenuItemValue::Toggle(value, _) = &this.value {
                let toggled = !value;
                this.value = MenuItemValue::Toggle(toggled, RefCell::new(None));
                Ok(toggled)
            } else {
                Err(anyhow!("toggle called on non-toggleable menu item!").into())
            }
        });
    }
}
impl UserData for MenuItemValue {}

impl FromLua for MenuScreen {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        if let Value::UserData(ud) = value {
            ud.take::<Self>()
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "MenuScreen".to_owned(),
                message: Some("expected MenuScreen".to_string()),
            })
        }
    }
}

impl FromLua for MenuItem {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        if let Value::UserData(ud) = value {
            ud.take::<Self>()
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "MenuItem".to_owned(),
                message: Some("expected MenuItem".to_string()),
            })
        }
    }
}

impl FromLua for MenuAction {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        if let Value::UserData(ud) = value {
            ud.take::<Self>()
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "MenuAction".to_owned(),
                message: Some("expected MenuAction".to_string()),
            })
        }
    }
}

impl FromLua for MenuItemValue {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        if let Value::UserData(ud) = value {
            ud.take()
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "MenuItemValue".to_owned(),
                message: Some("expected MenuItemValue".to_string()),
            })
        }
    }
}

fn make_text(renderer: &Renderer, text: &LuaString, font: Option<LuaString>) -> mlua::Result<Text> {
    let font = if let Some(fontname) = font {
        match fontname.as_bytes().deref() {
            b"big" => &renderer.fontset().menu_big,
            b"caption" => &renderer.fontset().menu_caption,
            b"normal" => &renderer.fontset().menu,
            x => {
                return mlua::Result::Err(mlua::Error::WithContext {
                    context: format!("Unknown font: {:?}", x),
                    cause: Arc::new(anyhow!("Font not found").into()),
                });
            }
        }
    } else {
        &renderer.fontset().menu
    };

    match text.to_str() {
        Ok(txt) => match font.create_text(renderer, &txt) {
            Ok(t) => Ok(t.with_outline_color(Color::new(0.2, 0.2, 0.4))),
            Err(e) => mlua::Result::Err(mlua::Error::WithContext {
                context: format!("Couldn't render text \"{}\"", txt),
                cause: Arc::new(e.into()),
            }),
        },
        Err(e) => mlua::Result::Err(mlua::Error::ExternalError(Arc::new(e))),
    }
}

fn make_menu(table: Table, window: RectF) -> mlua::Result<MenuScreen> {
    let mut items = table
        .sequence_values::<MenuItem>()
        .collect::<mlua::Result<Vec<_>>>()?;

    let current = items.iter().position(|i| i.is_selectable()).unwrap_or(0);
    MenuScreen::layout_items(&mut items, window);
    let cursorpos = items[0].rect.topleft() - Vec2(MenuScreen::SPACING, 0.0);
    Ok(MenuScreen {
        items,
        current,
        state: MenuScreenState::Appearing(0.0),
        animated_offset: Vec2::ZERO,
        cursorpos,
        on_exit: table.get("on_exit")?,
    })
}

fn make_heading(table: Table, renderer: &Renderer) -> mlua::Result<MenuItem> {
    let label = table.get::<LuaString>("label")?;
    let text = make_text(renderer, &label, table.get::<Option<LuaString>>("font")?)?;
    let rect = RectF::new(0.0, 0.0, text.width(), text.height());

    Ok(MenuItem {
        content: MenuItemContent::Text(text),
        center: table.get::<Option<bool>>("center")?.unwrap_or(false),
        rect,
        value: MenuItemValue::None,
        action: None,
    })
}

fn make_link(table: Table, renderer: &Renderer) -> mlua::Result<MenuItem> {
    let label = table.get::<LuaString>("label")?;
    let action = Some(table.get::<Function>("action")?);
    let text = make_text(renderer, &label, None)?;
    let rect = RectF::new(0.0, 0.0, text.width(), text.height());
    let value = table
        .get::<Option<MenuItemValue>>("value")?
        .unwrap_or(MenuItemValue::None);

    Ok(MenuItem {
        content: MenuItemContent::Text(text),
        center: false,
        value,
        rect,
        action,
    })
}

fn make_image(table: Table, renderer: &Renderer) -> mlua::Result<MenuItem> {
    let texture_name = table.get::<LuaString>("texture")?;
    let texid = match renderer
        .texture_store()
        .find_texture(&texture_name.as_bytes())
    {
        Ok(id) => id,
        Err(e) => {
            return mlua::Result::Err(mlua::Error::WithContext {
                context: "Couldn't find texture for menu item".to_owned(),
                cause: Arc::new(e.into()),
            });
        }
    };

    let tex = renderer.texture_store().get_texture(texid);

    Ok(MenuItem {
        content: MenuItemContent::Image(texid),
        center: table.get::<Option<bool>>("center")?.unwrap_or(false),
        rect: RectF::new(0.0, 0.0, tex.width(), tex.height()),
        value: MenuItemValue::None,
        action: None,
    })
}

fn make_spacer(height: f32) -> MenuItem {
    MenuItem {
        content: MenuItemContent::Blank,
        center: false,
        rect: RectF::new(0.0, 0.0, 1.0, height),
        value: MenuItemValue::None,
        action: None,
    }
}

impl LuaMenu {
    pub fn new(script_file: &str, renderer: Rc<RefCell<Renderer>>, window: RectF) -> Result<Self> {
        let script_path = find_datafile_path("script")?;
        let window = Rc::new(RefCell::new(window));

        let lua = Lua::new();
        // Load modules from the script path only
        lua.globals()
            .get::<Table>("package")?
            .set("path", format!("{}/?.lua", script_path.to_str().unwrap()))?;

        // Register constructors
        {
            let window = window.clone();
            lua.globals().set(
                "Menu",
                lua.create_function(move |_lua, props: Table| make_menu(props, *window.borrow()))?,
            )?;
        }

        {
            let renderer = renderer.clone();
            lua.globals().set(
                "Link",
                lua.create_function(move |_lua, props: Table| {
                    make_link(props, &renderer.borrow())
                })?,
            )?;
        }

        {
            let renderer = renderer.clone();
            lua.globals().set(
                "Image",
                lua.create_function(move |_lua, props: Table| {
                    make_image(props, &renderer.borrow())
                })?,
            )?;
        }

        {
            let renderer = renderer.clone();
            lua.globals().set(
                "Heading",
                lua.create_function(move |_lua, props: Table| {
                    make_heading(props, &renderer.borrow())
                })?,
            )?;
        }

        lua.globals().set(
            "Spacer",
            lua.create_function(move |_lua, height: f32| Ok(make_spacer(height)))?,
        )?;

        let menuactions = lua.create_table()?;
        menuactions.set(
            "Push",
            lua.create_function(|_lua, menu: MenuScreen| Ok(MenuAction::Push(menu)))?,
        )?;

        menuactions.set(
            "Pop",
            lua.create_function(|_lua, _: ()| Ok(MenuAction::Pop))?,
        )?;

        menuactions.set(
            "Return",
            lua.create_function(|_lua, val: String| Ok(MenuAction::Return(val)))?,
        )?;

        menuactions.set(
            "KeyGrab",
            lua.create_function(|_lua, _: ()| Ok(MenuAction::KeyGrab))?,
        )?;

        lua.globals().set("Action", menuactions)?;

        let itemvalues = lua.create_table()?;
        itemvalues.set(
            "Toggle",
            lua.create_function(|_lua, val: bool| {
                Ok(MenuItemValue::Toggle(val, RefCell::new(None)))
            })?,
        )?;
        itemvalues.set(
            "KeyGrab",
            lua.create_function(|_lua, key: u32| {
                Ok(MenuItemValue::KeyGrab(SDL_Keycode(key), RefCell::new(None)))
            })?,
        )?;

        lua.globals().set("Value", itemvalues)?;

        lua.globals().set(
            "load_settings",
            lua.create_function(|lua, _: ()| {
                let config = GAME_CONFIG.read().unwrap();
                lua.to_value_with(
                    &config.deref(),
                    mlua::SerializeOptions::new().serialize_none_to_null(false),
                )
            })?,
        )?;

        lua.globals().set(
            "save_settings",
            lua.create_function(|lua, config: Value| {
                let config = lua.from_value::<UserConfig>(config)?;
                save_user_config(config);
                Ok(())
            })?,
        )?;

        lua.globals().set(
            "get_default_keymap",
            lua.create_function(|lua, id: usize| {
                if id < 1 || id > GameControllerSet::DEFAULT_KEYMAP.len() {
                    Err(anyhow!("invalid keymap ID {id}").into())
                } else {
                    lua.to_value(&GameControllerSet::DEFAULT_KEYMAP[id - 1])
                }
            })?,
        )?;

        // Load menu script file and get main menu by running entrypoint function
        lua.load(format!(r#"require "{}""#, script_file)).exec()?;

        let main_menu = lua
            .globals()
            .get::<Function>("main_menu")?
            .call::<MenuScreen>(())?;

        let cursor = renderer
            .borrow()
            .fontset()
            .menu
            .create_text(&renderer.borrow(), ">")?;

        let key_grab_text = renderer
            .borrow()
            .fontset()
            .menu
            .create_text(&renderer.borrow(), "Press a key")?;
        Ok(Self {
            lua,
            renderer,
            cursor,
            menu_stack: vec![main_menu],
            cursor_bounce: 0.0,
            window,
            key_grabbing: false,
            key_grab_text,
        })
    }

    pub fn relayout(&mut self, window: RectF) {
        self.window.replace(window);
        for menu in self.menu_stack.iter_mut() {
            MenuScreen::layout_items(&mut menu.items, window);
        }
    }

    pub fn reload(&mut self) -> mlua::Result<()> {
        let main_menu = self
            .lua
            .globals()
            .get::<Function>("main_menu")?
            .call::<MenuScreen>(())?;
        self.menu_stack = vec![main_menu];
        Ok(())
    }

    /// Animate menus.
    /// Returns false if menu stack is empty
    pub fn step(&mut self, timestep: f32) -> mlua::Result<bool> {
        // Animate all visible menus. During transition animations, multiple
        // menus can be visibly atop each other
        let window = *self.window.borrow();
        let stacklen = self.menu_stack.len();
        for (idx, menu) in self
            .menu_stack
            .iter_mut()
            .enumerate()
            .filter(|(_, m)| m.state.is_visible())
        {
            menu.step(window, timestep);
            if !menu.state.is_visible() && idx + 1 == stacklen {
                // Execute callback when menu is closed
                if let Some(on_exit) = menu.on_exit.as_ref() {
                    let values = self.lua.create_table()?;

                    for item in &menu.items {
                        values.push(match item.value {
                            MenuItemValue::None => Value::NULL,
                            MenuItemValue::Toggle(v, _) => Value::Boolean(v),
                            MenuItemValue::KeyGrab(v, _) => Value::Integer(v.0 as _),
                        })?;
                    }
                    on_exit.call::<()>(values)?;
                }
            }
        }

        // Menus are always removed from the top of the stack
        if let Some(top) = self.menu_stack.last()
            && !top.state.is_visible()
        {
            self.menu_stack.pop();
        }

        // Cursor animation
        self.cursor_bounce += 4.0 * timestep;
        if self.cursor_bounce > std::f32::consts::PI {
            self.cursor_bounce = 0.0;
        }

        Ok(!self.menu_stack.is_empty())
    }

    pub fn render(&self) {
        let renderer = self.renderer.borrow();
        let cursor_animation_offset =
            Vec2(-self.cursor_bounce.sin() * self.cursor.width() / 2.0, 0.0);
        for menu in self.menu_stack.iter().filter(|m| m.state.is_visible()) {
            menu.render(&renderer);

            if matches!(
                menu.state,
                MenuScreenState::Normal | MenuScreenState::Appearing(_)
            ) {
                self.cursor.render(&RenderTextOptions {
                    dest: RenderTextDest::TopRight(
                        menu.cursorpos + cursor_animation_offset + menu.animated_offset,
                    ),
                    ..Default::default()
                });
            }
        }

        // Render keybrab overlay
        if self.key_grabbing {
            let window = self.window.borrow();
            let rect = RectF::new(
                window.x() + (window.w() - self.key_grab_text.width()) / 2.0 - 8.0,
                window.y() + (window.h() - self.key_grab_text.height()) / 2.0 - 8.0,
                self.key_grab_text.width() + 16.0,
                self.key_grab_text.height() + 16.0,
            );
            let color = Color::new(0.8, 0.0, 0.0);
            renderer.draw_filled_rectangle(rect, &color);
            renderer.draw_filled_rectangle(
                RectF::new(
                    rect.x() + 1.0,
                    rect.y() + 1.0,
                    rect.w() - 2.0,
                    rect.h() - 2.0,
                ),
                &Color::new(0.0, 0.0, 0.0),
            );
            self.key_grab_text.render(&RenderTextOptions {
                dest: RenderTextDest::Centered(rect.center()),
                color: Some(color),
                ..Default::default()
            });
        }
    }

    pub fn handle_button(&mut self, button: MenuButton) -> Result<String> {
        if matches!(button, MenuButton::GrabbedKey(_)) {
            self.key_grabbing = false;
        }

        if self.menu_stack.is_empty() {
            return Ok(String::new());
        }

        if matches!(button, MenuButton::Back) {
            let stacksize = self.menu_stack.len();
            if stacksize > 1 {
                self.menu_stack[stacksize - 1].hide();
                self.menu_stack[stacksize - 2].appear();
            }
            return Ok(String::new());
        }

        let active_menu = self.menu_stack.last_mut().filter(|m| {
            matches!(
                m.state,
                MenuScreenState::Normal | MenuScreenState::Appearing(_)
            )
        });

        if let Some(active_menu) = active_menu {
            match active_menu.handle_button(&self.lua, button)? {
                MenuAction::None => {}
                MenuAction::Push(m) => {
                    if let Some(top) = self.menu_stack.last_mut() {
                        top.hide();
                    }
                    debug_assert!(matches!(m.state, MenuScreenState::Appearing(_)));
                    self.menu_stack.push(m);
                }
                MenuAction::Pop => {
                    if let Some(top) = self.menu_stack.last_mut() {
                        top.hide();
                    }
                    let stacklen = self.menu_stack.len();
                    if stacklen > 1 {
                        self.menu_stack[stacklen - 2].appear();
                    }
                }
                MenuAction::Return(val) => {
                    active_menu.hide();
                    return Ok(val);
                }
                MenuAction::KeyGrab => {
                    self.key_grabbing = true;
                    events::push_grabkey_event();
                }
            };
        }
        Ok(String::new())
    }
}

impl MenuItem {
    fn is_selectable(&self) -> bool {
        self.action.is_some()
    }
}

impl MenuScreen {
    const SPACING: f32 = 3.0;

    fn appear(&mut self) {
        self.state = match self.state {
            MenuScreenState::Hiding(a) | MenuScreenState::Appearing(a) => {
                MenuScreenState::Appearing(a)
            }
            MenuScreenState::Normal => MenuScreenState::Normal,
            MenuScreenState::Hidden => MenuScreenState::Appearing(0.0),
        };
    }

    fn hide(&mut self) {
        self.state = match self.state {
            MenuScreenState::Hiding(a) | MenuScreenState::Appearing(a) => {
                MenuScreenState::Hiding(a)
            }
            MenuScreenState::Normal => MenuScreenState::Hiding(1.0),
            MenuScreenState::Hidden => MenuScreenState::Hidden,
        };
    }

    fn step(&mut self, window: RectF, timestep: f32) {
        match self.state {
            MenuScreenState::Appearing(a) => {
                let a2 = a + timestep * 2.0;
                if a2 >= 1.0 {
                    self.state = MenuScreenState::Normal;
                    self.animated_offset = Vec2::ZERO;
                } else {
                    self.state = MenuScreenState::Appearing(a2);
                    self.animated_offset = Vec2((1.0 - a2).powf(2.0) * window.w(), 0.0);
                }
            }
            MenuScreenState::Hiding(a) => {
                let a2 = a - timestep * 2.0;
                if a2 <= 0.0 {
                    self.state = MenuScreenState::Hidden;
                } else {
                    self.state = MenuScreenState::Hiding(a2);
                }
                self.animated_offset = Vec2(-(1.0 - a2).powf(2.0) * window.w(), 0.0);
            }
            _ => {}
        };

        let cursor_target_pos = self.items[self.current].rect.topleft() - Vec2(Self::SPACING, 0.0);
        self.cursorpos = Vec2(
            cursor_target_pos.0,
            self.cursorpos.1 + (cursor_target_pos.1 - self.cursorpos.1) * 10.0 * timestep,
        );
    }

    fn layout_items(items: &mut [MenuItem], window: RectF) {
        let (width, height) = items
            .iter()
            .map(|i| {
                if i.center {
                    (1.0, i.rect.h())
                } else {
                    i.rect.size()
                }
            })
            .reduce(|acc, (w, h)| (acc.0.max(w), acc.1 + h + Self::SPACING))
            .unwrap();

        let x = window.x() + (window.w() - width) / 2.0;
        let mut y = window.y() + (window.h() - height) / 2.0;

        for item in items.iter_mut() {
            if item.center {
                item.rect = RectF::new(
                    window.x() + (window.w() - item.rect.w()) / 2.0,
                    y,
                    item.rect.w(),
                    item.rect.h(),
                );
            } else {
                item.rect = RectF::new(x, y, item.rect.w(), item.rect.h());
            }
            y += item.rect.h() + Self::SPACING;
        }
    }

    fn render(&self, renderer: &Renderer) {
        let alpha = match self.state {
            MenuScreenState::Normal => 1.0,
            MenuScreenState::Appearing(a) | MenuScreenState::Hiding(a) => a,
            MenuScreenState::Hidden => 0.0,
        };
        for item in &self.items {
            match &item.content {
                MenuItemContent::Text(text) => {
                    text.render(&RenderTextOptions {
                        dest: RenderTextDest::TopLeft(item.rect.topleft() + self.animated_offset),
                        outline: TextOutline::Shadow,
                        alpha,
                        ..Default::default()
                    });

                    match &item.value {
                        MenuItemValue::None => {}
                        MenuItemValue::Toggle(value, text) => {
                            if text.borrow().is_none() {
                                text.replace(Some(
                                    renderer
                                        .fontset()
                                        .menu
                                        .create_text(renderer, if *value { "yes" } else { "no" })
                                        .unwrap()
                                        .with_color(if *value {
                                            Color::new(0.0, 0.8, 0.0)
                                        } else {
                                            Color::new(0.8, 0.0, 0.0)
                                        }),
                                ));
                            }

                            text.borrow().as_ref().unwrap().render(&RenderTextOptions {
                                dest: RenderTextDest::TopLeft(
                                    item.rect.topright() + Vec2(10.0, 0.0) + self.animated_offset,
                                ),
                                alpha,
                                ..Default::default()
                            });
                        }
                        MenuItemValue::KeyGrab(key, text) => {
                            if text.borrow().is_none() {
                                let keystr = unsafe { CStr::from_ptr(SDL_GetKeyName(*key)) };
                                text.replace(Some(
                                    renderer
                                        .fontset()
                                        .menu
                                        .create_text(renderer, keystr.to_str().unwrap())
                                        .unwrap()
                                        .with_color(Color::new(0.6, 0.6, 0.8)),
                                ));
                            }

                            text.borrow().as_ref().unwrap().render(&RenderTextOptions {
                                dest: RenderTextDest::TopLeft(
                                    item.rect.topright() + Vec2(10.0, 0.0) + self.animated_offset,
                                ),
                                alpha,
                                ..Default::default()
                            });
                        }
                    };
                }
                MenuItemContent::Image(texture) => {
                    renderer.texture_store().get_texture(*texture).render(
                        renderer,
                        &RenderOptions {
                            dest: RenderDest::Rect(item.rect + self.animated_offset),
                            color: Color::WHITE.with_alpha(alpha),
                            ..Default::default()
                        },
                    );
                }
                MenuItemContent::Blank => {}
            }
        }
    }

    fn handle_button(&mut self, lua: &Lua, button: MenuButton) -> mlua::Result<MenuAction> {
        match button {
            MenuButton::Up(_) => loop {
                if self.current == 0 {
                    self.current = self.items.len() - 1;
                } else {
                    self.current -= 1;
                }
                if self.items[self.current].is_selectable() {
                    break;
                }
            },
            MenuButton::Down(_) => loop {
                if self.current == self.items.len() - 1 {
                    self.current = 0;
                } else {
                    self.current += 1;
                }
                if self.items[self.current].is_selectable() {
                    break;
                }
            },
            MenuButton::Start | MenuButton::Select(_) => {
                let item = &mut self.items[self.current];
                if let Some(action) = item.action.clone() {
                    let result = lua
                        .scope(|scope| action.call::<Value>(scope.create_userdata_ref_mut(item)))?;

                    if !result.is_nil() {
                        return MenuAction::from_lua(result, lua);
                    }
                }
            }
            MenuButton::GrabbedKey(key) => {
                self.items[self.current].value = MenuItemValue::KeyGrab(key, RefCell::new(None));
            }
            _ => {}
        }
        Ok(MenuAction::None)
    }
}
