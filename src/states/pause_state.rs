use anyhow::{Result, anyhow};
use std::{cell::RefCell, rc::Rc};

use crate::{
    game::MenuButton,
    gfx::{Color, RenderDest, RenderOptions, Renderer, Texture},
    math::RectF,
    menu::LuaMenu,
    states::{StackableState, StackableStateResult},
};

pub struct PauseState {
    menu: LuaMenu,
    background: Texture,
    renderer: Rc<RefCell<Renderer>>,
    alpha: f32,
}

pub enum PauseReturn {
    Resume,
    EndRound,
    EndGame,
}

impl PauseState {
    pub fn new(renderer: Rc<RefCell<Renderer>>) -> Result<Self> {
        let size = renderer.borrow().size();
        let menu = LuaMenu::new("menus.pause", renderer.clone(), RectF::new(0.0, 0.0, size.0 as f32, size.1 as f32))?;

        let background = Texture::from_image(&renderer.borrow(), &renderer.borrow().screenshot()?)?;

        Ok(Self {
            renderer,
            menu,
            background,
            alpha: 1.0,
        })
    }

    pub fn render(&self) {
        let renderer = self.renderer.borrow();
        renderer.clear();
        self.background.render(&renderer, &RenderOptions{
            dest: RenderDest::Fill,
            color: Color::WHITE.with_alpha(self.alpha),
            ..Default::default()
        });
        self.menu.render();
        renderer.present();
    }
}

impl StackableState for PauseState {
    fn resize_screen(&mut self) {
        let size = self.renderer.borrow().size();
        self.menu.relayout(RectF::new(0.0, 0.0, size.0 as f32, size.1 as f32));
    }

    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        if matches!(button, MenuButton::Back) {
            return StackableStateResult::Return(Box::new(PauseReturn::Resume))
        }

        match self.menu.handle_button(button) {
            Ok(res) => match res.as_str() {
                "" => StackableStateResult::Continue,
                "resume" => StackableStateResult::Return(Box::new(PauseReturn::Resume)),
                "endround" => StackableStateResult::Return(Box::new(PauseReturn::EndRound)),
                "endgame" => StackableStateResult::Return(Box::new(PauseReturn::EndGame)),
                x => StackableStateResult::Error(anyhow!("Unhandled pause menu result: {}", x)),
            }
            Err(e) => StackableStateResult::Error(e),
        }
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        if let Err(e) = self.menu.step(timestep) {
            return StackableStateResult::Error(e.into());
        }

        if self.alpha > 0.3 {
            self.alpha -= timestep;
        }

        self.render();
        StackableStateResult::Continue
    }
}
