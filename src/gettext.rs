// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ffi::{c_int, CString};
use std::io::Result;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use glib::GStr;

pub fn bindtextdomain(domainname: &GStr, locale_dir: PathBuf) -> Result<()> {
    let locale_dir = CString::new(locale_dir.into_os_string().into_vec()).unwrap();
    let new_dir = unsafe { native::bindtextdomain(domainname.as_ptr(), locale_dir.as_ptr()) };
    if new_dir.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn textdomain(domainname: &GStr) -> Result<()> {
    let new_domain = unsafe { native::textdomain(domainname.as_ptr()) };
    if new_domain.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn bind_textdomain_codeset(domainname: &GStr, codeset: &GStr) -> Result<()> {
    let new_codeset =
        unsafe { native::bind_textdomain_codeset(domainname.as_ptr(), codeset.as_ptr()) };
    if new_codeset.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn setlocale(category: c_int, locale: &GStr) -> Result<()> {
    let current_locale = unsafe { native::setlocale(category, locale.as_ptr()) };
    if current_locale.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub const LC_ALL: c_int = 6;

mod native {
    use std::ffi::{c_char, c_int};

    extern "C" {
        pub fn bindtextdomain(domainname: *const c_char, dirname: *const c_char) -> *mut c_char;

        pub fn textdomain(domain_name: *const c_char) -> *mut c_char;

        pub fn bind_textdomain_codeset(
            domainname: *const c_char,
            codeset: *const c_char,
        ) -> *mut c_char;

        pub fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    }
}
