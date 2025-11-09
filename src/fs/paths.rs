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

use anyhow::{Result, anyhow};
use sdl3_sys::filesystem::{SDL_GetBasePath, SDL_GetPrefPath, SDL_GlobDirectory};
use sdl3_sys::stdinc::SDL_free;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use crate::gfx::SdlError;

fn get_basepath() -> PathBuf {
    let bp = unsafe { SDL_GetBasePath() };
    if bp.is_null() {
        // shouldn't happen
        panic!("Couldn't find application base path!");
    }

    let bp = unsafe { CStr::from_ptr(bp) };
    let bp = bp.to_str().expect("basepath not utf-8 encoded!");

    bp.to_owned().into()
}

/**
 * Find the named datafile or directory.
 *
 * Search order is:
 *
 *   1. EXE/data
 *   2. User data (XDG_DATA_HOME, TODO)
 *   3. /usr/share/luola2/ (Linux only, TODO)
 *
 * Returns the full path to the file or directory if it exists
 */
pub fn find_datafile_path(path: &[&str]) -> Result<PathBuf> {
    // Try relative to binary

    let mut p = get_basepath();
    p.push("data");
    for pc in path {
        p.push(pc);
    }

    if p.exists() {
        return Ok(p);
    }

    // TODO other paths
    Err(anyhow!("File not found: {:?}", path))
}

/**
 * Return a list of files or directories in the given directory that match the glob pattern.
 *
 * Files from all available data directories (see find_datafile_path) are searched.
 * Files from higher priority directories will hide files from low priority ones,
 * so that if "settings.toml" is found in both user data and /usr/share, only the user data
 * one will be returned.
 */
pub fn glob_datafiles(path: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let mut basepath = get_basepath();
    let pattern = CString::new(pattern)?;

    basepath.push("data");
    basepath.push(path);

    let basepath = basepath;

    let mut count: c_int = 0;
    let files: *mut *mut c_char = unsafe {
        SDL_GlobDirectory(
            pathbuf_to_cstring(basepath.clone())?.as_ptr(),
            pattern.as_ptr(),
            0,
            &mut count,
        )
    };

    if files.is_null() {
        return Err(SdlError::get_error(&format!("{:?} / {:?}", path, pattern)).into());
    }

    let count = count as usize;
    let mut paths: Vec<PathBuf> = Vec::with_capacity(count);

    for i in 0..count {
        let f = unsafe { CStr::from_ptr(*files.add(i) as *const c_char) };
        if let Ok(f) = f.to_str() {
            paths.push(basepath.join(f));
        }
    }

    unsafe {
        SDL_free(files as *mut c_void);
    }

    Ok(paths)
}

/**
 * Get the full path to a saveable file (such as a configuration file.)
 */
pub fn get_savefile_path(path: &Path) -> PathBuf {
    let bp = unsafe { SDL_GetPrefPath(c"io.github.callaa.luola2".as_ptr(), c"luola2".as_ptr()) };
    if bp.is_null() {
        // shouldn't happen
        panic!("Couldn't find preferences base path!");
    }

    let prefpath = unsafe { CStr::from_ptr(bp) };
    let prefpath = prefpath
        .to_str()
        .expect("preferences path not utf-8 encoded!");
    let prefpath = PathBuf::from(prefpath);

    unsafe {
        SDL_free(bp as *mut c_void);
    }

    prefpath.join(path)
}

/**
 * SDL interop helper
 */
pub fn pathbuf_to_cstring(path: PathBuf) -> Result<CString> {
    let osstr = path.into_os_string();
    if let Ok(string) = osstr.into_string() {
        Ok(CString::new(string)?)
    } else {
        Err(anyhow!("Couldn't convert path to string"))
    }
}
