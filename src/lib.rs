/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! A library for parsing FreeDesktop entry files.
//! These files are used in the
//! [Desktop Entry files](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html),
//! [Icon Theme index files](https://specifications.freedesktop.org/icon-theme-spec/icon-theme-spec-latest.html),
//! and [Systemd unit files](https://www.freedesktop.org/software/systemd/man/systemd.unit.html).
//! They are similar to ini files but are distinct enough that
//! an ini parse would not work.
//!
//! Example:
//! ```
//! use freedesktop_entry_parser::{parse_entry, SectionBytes, AttrBytes};
//!
//! let file = b"[Desktop Entry]
//! Name=Firefox
//! Exec=firefox %u
//! Icon=firefox";
//!
//! assert_eq!(parse_entry(file).next().unwrap()?, SectionBytes {
//!     title: b"Desktop Entry",
//!     attrs: vec![
//!         AttrBytes { name: b"Name", value: b"Firefox", param: None},
//!         AttrBytes { name: b"Exec", value: b"firefox %u", param: None},
//!         AttrBytes { name: b"Icon", value: b"firefox", param: None},
//!     ]
//! });
//! # Ok::<(), freedesktop_entry_parser::ParseError>(())
//! ```

mod debug;
pub mod errors;
mod parser;

pub use crate::parser::parse_entry;
pub use crate::parser::AttrBytes;
pub use crate::parser::SectionBytes;
pub use errors::ParseError;

use std::path::Path;

pub struct Entry {}

impl Entry {
    pub fn parse(input: impl AsRef<[u8]>) -> Result<Self, ParseError> {
        todo!()
    }

    pub fn parse_file(file_name: impl AsRef<Path>) -> Result<Self, ParseError> {
        todo!()
    }

    pub fn section(&self, name: impl AsRef<str>) -> &Section {
        todo!()
    }

    pub fn sections(&self) -> &[Section] {
        todo!()
    }
}

pub struct Section {}

impl Section {
    pub fn attr(&self, name: impl AsRef<str>) -> Option<&str> {
        todo!()
    }

    pub fn attr_with_param(
        &self,
        name: impl AsRef<str>,
        param_val: impl AsRef<str>,
    ) -> Option<&str> {
        todo!()
    }

    pub fn attrs(&self) -> &[Attr] {
        todo!()
    }
}

pub struct Attr {}
