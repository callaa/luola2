use anyhow::Result;
use std::{cell::RefCell, rc::Rc};

use crate::{
    game::MenuButton,
    gfx::{Color, RenderTextOptions, Renderer, Text},
    math::Vec2,
    menu::{Menu, MenuItem},
    states::{StackableState, StackableStateResult},
};

pub struct PauseState {
    menu: ActionMenu,
    title: Text,
    renderer: Rc<RefCell<Renderer>>,
}

type ActionMenu = Menu<PauseReturn>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PauseReturn {
    Resume,
    EndRound,
    EndGame,
}

impl PauseState {
    pub fn new(renderer: Rc<RefCell<Renderer>>) -> Result<Self> {
        let mut menu = Menu::new(
            &renderer.borrow(),
            &[
                MenuItem::Escape("Resume", PauseReturn::Resume),
                MenuItem::Link("End round", PauseReturn::EndRound),
                MenuItem::Link("End game", PauseReturn::EndGame),
            ],
        )?;

        menu.appear();

        let title = renderer
            .borrow()
            .fontset()
            .menu_big
            .create_text(&renderer.borrow(), "Paused")?
            .with_color(Color::new(1.0, 0.3, 0.3));

        Ok(Self {
            renderer,
            title,
            menu,
        })
    }

    pub fn render(&self) {
        let renderer = self.renderer.borrow();
        renderer.clear();

        self.title.render(&RenderTextOptions {
            dest: crate::gfx::RenderTextDest::TopCenter(Vec2(renderer.width() as f32 / 2.0, 10.0)),
            ..Default::default()
        });
        self.menu.render(&renderer);
        renderer.present();
    }
}

impl StackableState for PauseState {
    fn resize_screen(&mut self) {
        let size = self.renderer.borrow().size();
        self.menu.update_window_size(size.0 as f32, size.1 as f32);
    }

    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        match self.menu.handle_button(button) {
            Some(ret) => StackableStateResult::Return(Box::new(ret)),
            None => StackableStateResult::Continue,
        }
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        self.menu.step(None, timestep);
        self.render();
        StackableStateResult::Continue
    }
}
