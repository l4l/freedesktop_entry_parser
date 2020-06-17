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
mod internal;
mod parser;

pub use crate::parser::parse_entry;
pub use crate::parser::AttrBytes;
pub use crate::parser::SectionBytes;
pub use errors::ParseError;
use internal::{
    AttrNamesIter, AttrValue, Internal, ParamMap, ParamNamesIter,
    SectionNamesIter,
};

use std::{fs::File, io::Read, path::Path, pin::Pin};

pub struct Entry(Pin<Box<Internal>>);

impl Entry {
    pub fn parse(input: impl Into<Vec<u8>>) -> Result<Self, ParseError> {
        Ok(Entry(Internal::new(input.into())?))
    }

    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        let mut file = File::open(path).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        Self::parse(buf)
    }

    pub fn has_section(&self, name: impl AsRef<str>) -> bool {
        self.0.has_section(name.as_ref())
    }

    pub fn section<'a, T: AsRef<str>>(&'a self, name: T) -> AttrSelector<T> {
        AttrSelector {
            name: name,
            entry: self,
        }
    }

    pub fn sections(&self) -> SectionIter {
        SectionIter {
            iter: self.0.section_names_iter(),
            entry: self,
        }
    }
}

pub struct SectionIter<'a> {
    iter: SectionNamesIter<'a>,
    entry: &'a Entry,
}

impl<'a> Iterator for SectionIter<'a> {
    type Item = AttrSelector<'a, &'a str>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|name| AttrSelector {
            name,
            entry: self.entry,
        })
    }
}

pub struct AttrSelector<'a, T: AsRef<str>> {
    name: T,
    entry: &'a Entry,
}

impl<'a, T: AsRef<str>> AttrSelector<'a, T> {
    pub fn attr(&self, name: impl AsRef<str>) -> Option<&'a str> {
        self.entry.0.get(self.name.as_ref(), name.as_ref(), None)
    }

    pub fn has_attr(&self, name: impl AsRef<str>) -> bool {
        self.entry
            .0
            .get_attr(self.name.as_ref(), name.as_ref())
            .is_some()
    }

    pub fn attr_with_param(
        &self,
        name: impl AsRef<str>,
        param_val: impl AsRef<str>,
    ) -> Option<&str> {
        let section = self.name.as_ref();
        self.entry
            .0
            .get(section, name.as_ref(), Some(param_val.as_ref()))
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn attrs(&'a self) -> Option<AttrIter<'a>> {
        Some(AttrIter {
            section_name: self.name.as_ref(),
            iter: self.entry.0.attr_names_iter(self.name.as_ref())?,
            entry: &self.entry,
        })
    }
}

pub struct Attr<'a> {
    pub section_name: &'a str,
    pub name: &'a str,
    pub value: Option<&'a str>,
    attr: &'a AttrValue,
    entry: &'a Entry,
}

impl<'a> Attr<'a> {
    pub fn has_value(&self) -> bool {
        self.attr.get_value().is_some()
    }

    pub fn has_params(&self) -> bool {
        self.attr.get_params().is_some()
    }

    pub fn params(&self) -> Option<ParamIter<'a>> {
        Some(ParamIter {
            section_name: self.section_name,
            attr_name: self.name,
            iter: self
                .entry
                .0
                .param_names_iter(self.section_name, self.name)?,
            params: self.attr.get_params()?,
        })
    }
}

pub struct AttrIter<'a> {
    section_name: &'a str,
    iter: AttrNamesIter<'a>,
    entry: &'a Entry,
}

impl<'a> Iterator for AttrIter<'a> {
    type Item = Attr<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let attr_name = self.iter.next()?;
        let attr = self.entry.0.get_attr(self.section_name, attr_name)?;
        Some(Attr {
            attr,
            name: attr_name,
            section_name: self.section_name,
            entry: self.entry,
            value: attr.get_value(),
        })
    }
}

pub struct AttrParam<'a> {
    pub section_name: &'a str,
    pub attr_name: &'a str,
    pub param_val: &'a str,
    pub value: &'a str,
}

pub struct ParamIter<'a> {
    section_name: &'a str,
    attr_name: &'a str,
    iter: ParamNamesIter<'a>,
    params: &'a ParamMap,
}

impl<'a> Iterator for ParamIter<'a> {
    type Item = AttrParam<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let param_val = self.iter.next()?;
        let value = self.params.get_param(param_val)?;
        Some(AttrParam {
            section_name: self.section_name,
            attr_name: self.attr_name,
            param_val,
            value,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lookup() {
        let entry = Entry::parse_file("./test_data/sshd.service").unwrap();
        assert_eq!(
            entry.section("Unit").attr("Description"),
            Some("OpenSSH Daemon"),
        );
    }

    #[test]
    fn drop() {
        let entry = Entry::parse_file("./test_data/sshd.service").unwrap();
        // let mut iter = entry.sections();
        // let first = iter.next().unwrap();
        // let name = first.name();
        std::mem::drop(entry);
        // println!("{}", name);
        // let desc = entry.get("Unit", "Description", None);
        // let desc = entry.section("Unit").attr("Description");
        // println!("{:?}", desc);
        // println!("{:?}", desc);
        // std::mem::drop(entry);
        // println!("{:?}", desc);
    }
}
