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
use sdl3_sys::filesystem::{
    SDL_Folder, SDL_GetBasePath, SDL_GetPrefPath, SDL_GetUserFolder, SDL_GlobDirectory,
};
use sdl3_sys::stdinc::SDL_free;
use std::collections::HashSet;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::gfx::SdlError;

static BASEPATH: LazyLock<String> = LazyLock::new(|| {
    let bp = unsafe { SDL_GetBasePath() };
    if bp.is_null() {
        // shouldn't happen
        panic!("Couldn't find application base path!");
    }

    let bp = unsafe { CStr::from_ptr(bp) };
    let bp = bp.to_str().expect("basepath not utf-8 encoded!");
    bp.into()
});

static USERPATH: LazyLock<String> = LazyLock::new(|| {
    let bp = unsafe { SDL_GetPrefPath(c"io.github.callaa.luola2".as_ptr(), c"luola2".as_ptr()) };
    if bp.is_null() {
        // shouldn't happen
        panic!("Couldn't find preferences base path!");
    }

    let prefpath = unsafe { CStr::from_ptr(bp) };
    let prefpath = prefpath
        .to_str()
        .expect("preferences path not utf-8 encoded!");
    let prefpath = prefpath.into();

    unsafe {
        SDL_free(bp as *mut c_void);
    }

    prefpath
});

static SHARE_PATH: &str = "/usr/share/luola2";
static LOCAL_SHARE_PATH: &str = "/usr/local/share/luola2";

fn if_exists(p: PathBuf) -> Option<PathBuf> {
    if p.exists() { Some(p) } else { None }
}

/**
 * Find the named file or directory.
 *
 * Search order is:
 *
 *   1. User data/data (XDG_DATA_HOME)
 *   2. EXE/data
 *   3. /usr/local/share/luola2/ (Linux only)
 *   4. /usr/share/luola2/ (Linux only)
 *
 * Returns the full path to the first file or directory found.
 */
pub fn find_datafile_path(path: &str) -> Result<PathBuf> {
    let p = if_exists([&USERPATH, "data", path].iter().collect())
        .or_else(|| if_exists([&BASEPATH, "data", path].iter().collect()));

    let p = if cfg!(unix) {
        p.or_else(|| if_exists([LOCAL_SHARE_PATH, path].iter().collect()))
            .or_else(|| if_exists([SHARE_PATH, path].iter().collect()))
    } else {
        p
    };

    p.ok_or_else(|| anyhow!("File not found: {:?}", path))
}

fn do_glob(
    basepath: &PathBuf,
    pattern: &CStr,
    filenames: &mut HashSet<String>,
    paths: &mut Vec<PathBuf>,
) -> Result<()> {
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
        unsafe {
            SDL_free(files as *mut c_void);
        }

        // This is typically just a "folder not found" error, which is expected
        SdlError::log(&format!("{:?}/{:?}", basepath, pattern));
        return Ok(());
    }

    let count = count as usize;
    for i in 0..count {
        let f = unsafe { CStr::from_ptr(*files.add(i) as *const c_char) };
        if let Ok(f) = f.to_str()
            && !filenames.contains(f)
        {
            filenames.insert(f.to_owned());
            paths.push([basepath, Path::new(f)].iter().collect());
        }
    }

    unsafe {
        SDL_free(files as *mut c_void);
    }
    Ok(())
}

/**
 * Return a list of files or directories in the given directory that match the glob pattern.
 *
 * Files from all available data directories (see find_datafile_path) are searched.
 * Files from higher priority directories will hide files from low priority ones,
 * so that if "settings.toml" is found in both user data and /usr/share, only the user data
 * one will be returned.
 */
pub fn glob_datafiles(path: &str, pattern: &str) -> Result<Vec<PathBuf>> {
    let pattern = CString::new(pattern)?;
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut filenames: HashSet<String> = HashSet::new();

    do_glob(
        &[&USERPATH, "data", path].iter().collect(),
        &pattern,
        &mut filenames,
        &mut paths,
    )?;

    do_glob(
        &[&BASEPATH, "data", path].iter().collect(),
        &pattern,
        &mut filenames,
        &mut paths,
    )?;

    if cfg!(unix) {
        do_glob(
            &[LOCAL_SHARE_PATH, path].iter().collect(),
            &pattern,
            &mut filenames,
            &mut paths,
        )?;

        do_glob(
            &[SHARE_PATH, path].iter().collect(),
            &pattern,
            &mut filenames,
            &mut paths,
        )?;
    }
    Ok(paths)
}

/**
 * Get the full path to a saveable file (such as a configuration file.)
 */
pub fn get_savefile_path(path: &str) -> PathBuf {
    [&USERPATH, path].iter().collect()
}

/**
 * Find the root path for screenshot folder
 */
pub fn get_screenshot_path() -> Result<PathBuf> {
    let mut root = unsafe { SDL_GetUserFolder(SDL_Folder::SCREENSHOTS) };

    if root.is_null() {
        root = unsafe { SDL_GetUserFolder(SDL_Folder::PICTURES) };
    }

    if root.is_null() {
        return Err(anyhow!("Could not find suitable path for screenshots!"));
    }

    let p = unsafe { CStr::from_ptr(root) };

    Ok(p.to_str()?.into())
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
