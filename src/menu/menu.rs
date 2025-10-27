// This file is part of Luola2
// Copyright (C) 2025 Calle Laakkonen
//
// Luola2 is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Luola2 is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Luola2.  If not, see <https://www.gnu.org/licenses/>.

use std::{cell::RefCell, ffi::CStr, fmt::Debug};

use anyhow::Result;
use sdl3_sys::keyboard::SDL_GetKeyName;

use crate::{
    events,
    game::{GameControllerSet, MenuButton},
    gfx::{Color, Renderer, Text},
    math::{RectF, Vec2},
};

pub enum MenuItem<'a, T> {
    Heading(&'a str, Color),
    Link(&'a str, T),
    Escape(&'a str, T),
    Value(&'a str, T),
    Spacer(f32),
}

#[derive(Clone)]
pub enum MenuValue {
    None,
    Return,
    Toggle(bool),
    KeyGrab(u32),
}

struct MenuLine<T> {
    text: Option<Text>,
    value_text: RefCell<Option<Text>>, // cached value
    rect: RectF,
    target_pos: Vec2,
    id: Option<T>,
    value: MenuValue,
}

#[derive(PartialEq, Clone, Copy)]
pub enum MenuState {
    Appearing,
    Normal,
    Disappearing,
    Disappeared,
}

pub struct Menu<T> {
    items: Vec<MenuLine<T>>,
    cursor: Text,
    cursorpos: Vec2,
    current: usize,

    state: MenuState,
    key_grabbing: bool,
    key_grab_text: Text,
    escape: Option<T>,

    sine: f32, // animation aid

    window_size: (f32, f32),
}

impl<T> MenuLine<T> {
    fn is_selectable(&self) -> bool {
        self.id.is_some()
    }

    fn set_value(&mut self, value: MenuValue) {
        self.value = value;
        self.value_text.take();
    }
}

impl<T: Copy + PartialEq + Debug> Menu<T> {
    const SPACING: f32 = 3.0;

    pub fn new(renderer: &Renderer, items: &[MenuItem<T>]) -> Result<Self> {
        let font = &renderer.fontset().menu;

        let window_size = (renderer.width() as f32, renderer.height() as f32);

        let cursor = font.create_text(renderer, ">")?;
        let escape = items
            .iter()
            .flat_map(|i| {
                if let MenuItem::Escape(_, r) = i {
                    Some(r)
                } else {
                    None
                }
            })
            .cloned()
            .next();

        let items = items
            .iter()
            .map(|item| {
                Ok(match item {
                    MenuItem::Heading(s, color) => {
                        let text = font.create_text(renderer, s)?.with_color(*color);
                        let (w, h) = text.size();
                        MenuLine {
                            text: Some(text),
                            value_text: RefCell::new(None),
                            rect: RectF::new(0.0, 0.0, w, h),
                            target_pos: Default::default(),
                            value: MenuValue::None,
                            id: None,
                        }
                    }
                    MenuItem::Link(s, id) | MenuItem::Escape(s, id) => {
                        let text = font.create_text(renderer, s)?;
                        let (w, h) = text.size();
                        MenuLine {
                            text: Some(text),
                            value_text: RefCell::new(None),
                            rect: RectF::new(0.0, 0.0, w, h),
                            target_pos: Default::default(),
                            value: MenuValue::Return,
                            id: Some(*id),
                        }
                    }
                    MenuItem::Value(s, id) => {
                        let text = font.create_text(renderer, s)?;
                        let (w, h) = text.size();
                        MenuLine {
                            text: Some(text),
                            value_text: RefCell::new(None),
                            rect: RectF::new(0.0, 0.0, w, h),
                            target_pos: Default::default(),
                            value: MenuValue::None, // to be assigned
                            id: Some(*id),
                        }
                    }
                    MenuItem::Spacer(h) => MenuLine {
                        text: None,
                        value_text: RefCell::new(None),
                        rect: RectF::new(0.0, 0.0, 1.0, *h),
                        target_pos: Default::default(),
                        value: MenuValue::None,
                        id: None,
                    },
                })
            })
            .collect::<Result<Vec<MenuLine<T>>>>()?;

        let key_grab_text = font
            .create_text(renderer, "Press a key")?
            .with_color(Color::new(1.0, 0.0, 0.0));

        let mut menu = Self {
            items,
            current: 0,
            cursorpos: Default::default(),
            cursor,
            state: MenuState::Disappeared,
            escape,
            key_grabbing: false,
            key_grab_text,
            sine: 0.0,
            window_size,
        };

        menu.center_items();

        Ok(menu)
    }

    pub fn height(&self) -> f32 {
        self.items.iter().fold(0.0, |acc, i| acc + i.rect.h())
    }

    pub fn update_window_size(&mut self, width: f32, height: f32) {
        self.window_size = (width, height);
        self.center_items();
    }

    fn center_items(&mut self) {
        if self.items.is_empty() || self.state == MenuState::Disappearing {
            return;
        }

        let (width, height) = self
            .items
            .iter()
            .map(|i| i.rect.size())
            .reduce(|acc, wh| (acc.0.max(wh.0), acc.1 + wh.1 + Self::SPACING))
            .unwrap();

        let x = (self.window_size.0 - width) / 2.0;
        let mut y = (self.window_size.1 - height) / 2.0;

        for item in self.items.iter_mut() {
            item.target_pos = Vec2(x, y);
            y += item.rect.h() + Self::SPACING;
        }

        self.cursorpos = self.get_cursorpos(self.current);
    }

    fn get_cursorpos(&self, selection: usize) -> Vec2 {
        self.items[selection]
            .rect
            .offset(-self.cursor.width() - Self::SPACING, 0.0)
            .topleft()
    }

    pub fn state(&self) -> MenuState {
        self.state
    }

    pub fn set_value(&mut self, id: T, value: MenuValue) {
        for item in self.items.iter_mut() {
            if let Some(i) = item.id
                && i == id
            {
                item.set_value(value);
                return;
            }
        }
    }

    fn get_value(&self, id: T) -> MenuValue {
        for item in self.items.iter() {
            if let Some(i) = item.id
                && i == id
            {
                return item.value.clone();
            }
        }
        panic!("Menu item ID {:?} does not exist", id);
    }

    pub fn get_toggle_value(&self, id: T) -> bool {
        let v = self.get_value(id);
        if let MenuValue::Toggle(v) = v {
            v
        } else {
            panic!("Not a Toggle item!")
        }
    }

    pub fn get_keygrab_value(&self, id: T) -> u32 {
        let v = self.get_value(id);
        if let MenuValue::KeyGrab(v) = v {
            v
        } else {
            panic!("Not a Toggle item!")
        }
    }

    pub fn appear(&mut self) {
        self.state = MenuState::Appearing;
        self.current = self
            .items
            .iter()
            .enumerate()
            .find(|(_, i)| i.is_selectable())
            .expect("menu should have at least one selectable item")
            .0;

        self.center_items();
        for item in self.items.iter_mut() {
            item.rect = RectF::new(
                self.window_size.0,
                item.target_pos.1,
                item.rect.w(),
                item.rect.h(),
            )
        }
    }

    pub fn disappear(&mut self) {
        self.state = MenuState::Disappearing;
        for item in self.items.iter_mut() {
            item.target_pos = Vec2(-100.0 - item.rect.w(), item.target_pos.1);
        }
    }

    pub fn handle_button(&mut self, button: MenuButton) -> Option<T> {
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
                match item.value {
                    MenuValue::None => {}
                    MenuValue::Return => {
                        return item.id;
                    }
                    MenuValue::Toggle(val) => {
                        item.set_value(MenuValue::Toggle(!val));
                    }
                    MenuValue::KeyGrab(_) => {
                        self.key_grabbing = true;
                        events::push_grabkey_event();
                    }
                }
            }
            MenuButton::Back => return self.escape,
            _ => {}
        }
        None
    }

    pub fn step(&mut self, controllers: &GameControllerSet, timestep: f32) {
        if self.key_grabbing {
            if controllers.last_grabbed_key != 0 {
                self.key_grabbing = false;
                self.items[self.current]
                    .set_value(MenuValue::KeyGrab(controllers.last_grabbed_key));
            }
        }

        // Menu item animation
        let mut all_in_position = true;
        for item in self.items.iter_mut() {
            let newrect = RectF::new(
                item.rect.x() + (item.target_pos.0 - item.rect.x()) * 5.0 * timestep,
                item.rect.y() + (item.target_pos.1 - item.rect.y()) * 5.0 * timestep,
                item.rect.w(),
                item.rect.h(),
            );

            all_in_position &= newrect.topleft().manhattan_dist(item.rect.topleft()) < 15.0;
            item.rect = newrect;
        }

        if all_in_position {
            self.state = match self.state {
                MenuState::Appearing => MenuState::Normal,
                MenuState::Disappearing => MenuState::Disappeared,
                x => x,
            }
        }

        // Cursor animation
        let targetpos = self.get_cursorpos(self.current);

        self.cursorpos = Vec2(
            self.items[self.current].rect.x() - self.cursor.width() - Self::SPACING,
            self.cursorpos.1 + (targetpos.1 - self.cursorpos.1) * 10.0 * timestep,
        );

        self.sine += 4.0 * timestep;
        if self.sine > 3.14 {
            self.sine = 0.0;
        }
    }

    pub fn render(&self, renderer: &Renderer) {
        for item in &self.items {
            if let Some(text) = &item.text {
                text.render(item.rect.topleft());
            }

            if item.value_text.borrow().is_none() {
                match item.value {
                    MenuValue::Toggle(v) => {
                        item.value_text.borrow_mut().replace(
                            renderer
                                .fontset()
                                .menu
                                .create_text(renderer, if v { "yes" } else { "no" })
                                .unwrap()
                                .with_color(if v {
                                    Color::new(0.0, 0.8, 0.0)
                                } else {
                                    Color::new(0.8, 0.0, 0.0)
                                }),
                        );
                    }
                    MenuValue::KeyGrab(key) => {
                        let keystr = unsafe { CStr::from_ptr(SDL_GetKeyName(key)) };
                        item.value_text.borrow_mut().replace(
                            renderer
                                .fontset()
                                .menu
                                .create_text(renderer, keystr.to_str().unwrap())
                                .unwrap()
                                .with_color(Color::new(0.6, 0.6, 0.8)),
                        );
                    }
                    _ => {}
                }
            }

            if let Some(value_text) = item.value_text.borrow().as_ref() {
                value_text.render(item.rect.topright() + Vec2(10.0, 0.0));
            }
        }

        if let MenuState::Normal = self.state {
            self.cursor
                .render(self.cursorpos - Vec2(self.sine.sin() * self.cursor.width() / 2.0, 0.0));
        }

        if self.key_grabbing {
            let rect = RectF::new(
                (self.window_size.0 - self.key_grab_text.width()) / 2.0 - 8.0,
                (self.window_size.1 - self.key_grab_text.height()) / 2.0 - 8.0,
                self.key_grab_text.width() + 16.0,
                self.key_grab_text.height() + 16.0,
            );
            renderer.draw_filled_rectangle(rect, &Color::new(0.8, 0.0, 0.0));
            renderer.draw_filled_rectangle(
                RectF::new(
                    rect.x() + 1.0,
                    rect.y() + 1.0,
                    rect.w() - 2.0,
                    rect.h() - 2.0,
                ),
                &Color::new(0.0, 0.0, 0.0),
            );
            self.key_grab_text.render(rect.topleft() + Vec2(8.0, 8.0));
        }
    }
}
