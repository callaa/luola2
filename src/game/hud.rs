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

use super::objects::Ship;
use crate::{gfx::Color, gfx::Renderer, math::RectF};

pub fn draw_hud(renderer: &Renderer, ship: &Ship) {
    let bar_height = 3.0 + 4.0 * 2.0;
    let bar_width = renderer.width() as f32 - bar_height * 2.0;

    let bar_rect = RectF::new(
        ((renderer.width() as f32 - bar_width) / 2.0).floor(),
        renderer.height() as f32 - bar_height * 2.0,
        bar_width,
        bar_height,
    );
    renderer.draw_filled_rectangle(bar_rect, &Color::new(0.1, 0.1, 0.1));

    let health = ship.health();
    let health_rect = RectF::new(
        bar_rect.x() + 1.0,
        bar_rect.y() + 1.0,
        (bar_rect.w() - 2.0) * health,
        4.0,
    );

    let health_color = if health > 0.5 {
        Color::new(0.31, 0.38, 0.72)
    } else if health > 0.2 {
        Color::new(0.78, 0.78, 0.0)
    } else {
        Color::new(0.78, 0.0, 0.0)
    };

    renderer.draw_filled_rectangle(health_rect, &health_color);

    let ammo_rect = RectF::new(
        bar_rect.x() + 1.0,
        bar_rect.y() + 2.0 + 4.0,
        (bar_rect.w() - 2.0) * ship.ammo(),
        4.0,
    );

    renderer.draw_filled_rectangle(ammo_rect, &Color::new(0.72, 0.76, 0.76));
}
