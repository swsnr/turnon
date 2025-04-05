// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ffi::{CString, c_int};
use std::io::Result;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use glib::{GStr, gstr};

fn bindtextdomain(domainname: &GStr, locale_dir: PathBuf) -> Result<()> {
    let locale_dir = CString::new(locale_dir.into_os_string().into_vec()).unwrap();
    // SAFETY: domainname, being a GStr, is nul-terminated, and we explicitly convert locale_dir to a nul-terminated.
    // string. bindtextdomain does not take ownership of these pointers so we need not copy.  We ignore the returned
    // pointer, other than checking for NULL.
    let new_dir = unsafe { native::bindtextdomain(domainname.as_ptr(), locale_dir.as_ptr()) };
    if new_dir.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn textdomain(domainname: &GStr) -> Result<()> {
    // SAFETY: domainname, being a GStr, is nul-terminated. textdomain does not take ownership of this pointer so we
    // need not copy.  We ignore the returned pointer, other than checking for NULL.
    let new_domain = unsafe { native::textdomain(domainname.as_ptr()) };
    if new_domain.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn bind_textdomain_codeset(domainname: &GStr, codeset: &GStr) -> Result<()> {
    // SAFETY: domainname and codeset, being GStrs, are nul-terminated already. bind_textdomain_codeset does not take
    // ownership of these pointers so we need not copy.  We ignore the returned pointer, other than checking for NULL.
    let new_codeset =
        unsafe { native::bind_textdomain_codeset(domainname.as_ptr(), codeset.as_ptr()) };
    if new_codeset.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn setlocale(category: c_int, locale: &GStr) {
    // SAFETY: locale, being a GStr, is nul-terminated already.  setlocale does not take ownership of this pointer so
    // we need not copy.  We just ignore the return value as we don't need the old locale value and there's nothing we
    // can do about errors anyway.
    unsafe { libc::setlocale(category, locale.as_ptr()) };
}

/// Initialize gettext.
///
/// Set locale and text domain, and bind the text domain to the given `locale_dir`.
///
/// See <https://www.gnu.org/software/gettext/manual/gettext.html#Triggering-gettext-Operations>.
pub fn init_gettext(text_domain: &GStr, locale_dir: PathBuf) -> Result<()> {
    setlocale(libc::LC_ALL, gstr!(""));
    bindtextdomain(text_domain, locale_dir)?;
    bind_textdomain_codeset(text_domain, gstr!("UTF-8"))?;
    textdomain(text_domain)?;
    Ok(())
}

mod native {
    use std::ffi::c_char;

    unsafe extern "C" {
        pub fn bindtextdomain(domainname: *const c_char, dirname: *const c_char) -> *mut c_char;

        pub fn textdomain(domain_name: *const c_char) -> *mut c_char;

        pub fn bind_textdomain_codeset(
            domainname: *const c_char,
            codeset: *const c_char,
        ) -> *mut c_char;
    }
}
