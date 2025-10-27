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

use std::{error::Error, ffi::CStr, fmt, result};

use log::error;
use sdl3_sys::error::SDL_GetError;

#[derive(Debug)]
pub struct SdlError {
    pub message: String,
}

impl SdlError {
    pub fn log(context: &str) {
        let error = unsafe { CStr::from_ptr(SDL_GetError()) };
        error!(
            "{}: {}",
            context,
            error.to_str().unwrap_or("(unknown SDL error)")
        );
    }

    /**
     * Get the current SDL error.
     *
     */
    pub fn get_error(context: &str) -> SdlError {
        let error = unsafe { CStr::from_ptr(SDL_GetError()) };
        SdlError {
            message: format!(
                "{}: {}",
                context,
                error.to_str().unwrap_or("(unknown SDL error)")
            ),
        }
    }
}

impl fmt::Display for SdlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for SdlError {}

pub type SdlResult<T> = result::Result<T, SdlError>;
