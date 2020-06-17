/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! A library for parsing FreeDesktop entry files. These files are used in the
//! [Desktop Entry
//! files](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html),
//! [Icon Theme index
//! files](https://specifications.freedesktop.org/icon-theme-spec/icon-theme-spec-latest.html),
//! and [Systemd unit
//! files](https://www.freedesktop.org/software/systemd/man/systemd.unit.html).
//! They are similar to ini files but are distinct enough that an ini parse
//! would not work.
//!
//! # Struct of Freedesktop Entry Files
//!
//! Freedesktop entry files are split up into section, each with a header in the
//! form `[NAME]`. Each section has attributes, which are key value pairs,
//! separated by and `=`.  Some attributes have parameters.  These are values
//! between `[]` and the end of the attribute name.  These are often use for
//! localization.
//!
//! Here is a snippet from `firefox.desktop`
//!
//! ```
//! [Desktop Entry]
//! Version=1.0
//! Name=Firefox
//! GenericName=Web Browser
//! GenericName[ar]=متصفح ويب
//! GenericName[ast]=Restolador Web
//! GenericName[bn]=ওয়েব ব্রাউজার
//! GenericName[ca]=Navegador web
//! Exec=/usr/lib/firefox/firefox %u
//! Icon=firefox
//!
//! [Desktop Action new-window]
//! Name=New Window
//! Name[ach]=Dirica manyen
//! Name[af]=Nuwe venster
//! Name[an]=Nueva finestra
//! Exec=/usr/lib/firefox/firefox --new-window %u
//! ```
//!
//! The first section is called `Desktop Entry`.  Is has many attributes
//! including `Name` which is `Firefox`.  The `GenericName` attributes has a
//! param. The default value is in English but there are also values with a
//! parameter for different locales.
//!
//! # APIs
//!
//! This library has two APIs, a high level api and a lower level byte oriented
//! api. The main entry point for the high level API is
//! [`Entry`](struct.Entry.html) and the entry point for the lower level API is
//! the [`parse_entry`](fn.parse_entry.html) function.
//!
//! # High Level API
//!
//! As example input lets use the contents of `sshd.service`
//! ```
//! [Unit]
//! Description=OpenSSH Daemon
//! Wants=sshdgenkeys.service
//! After=sshdgenkeys.service
//! After=network.target
//!
//! [Service]
//! ExecStart=/usr/bin/sshd -D
//! ExecReload=/bin/kill -HUP $MAINPID
//! KillMode=process
//! Restart=always
//!
//! [Install]
//! WantedBy=multi-user.target
//! ```
//!
//! For example, to print the start command we could do this:
//! ```
//! use freedesktop_entry_parser::Entry;
//!
//! let entry = Entry::parse_file("./test_data/sshd.service")?;
//! let start_cmd = entry
//!     .section("Service")
//!     .attr("ExecStart")
//!     .expect("Attribute doesn't exist");
//! println!("{}", start_cmd);
//!
//! # Ok::<(), freedesktop_entry_parser::ParseError>(())
//! ```
//! There are more examples in the [`examples`]() directory.
//!
//! # Lower Level API
//!
//! The lower level api is byte oriented and simply provides an iterator over
//! the sections in the file as they appear. This API is faster and may be more
//! suitable in certain circumstances.
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
pub use crate::parser::EntryIter;
pub use crate::parser::SectionBytes;
pub use errors::ParseError;
use internal::{
    AttrNamesIter, AttrValue, Internal, ParamMap, ParamNamesIter,
    SectionNamesIter,
};

use std::{fs::File, io::Read, path::Path, pin::Pin};

/// Parse a Freedesktop entry
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

/// Iterate over the sections in an entry.
///
/// Created from [`Entry::sections`](struct.Entry.html#method.sections)
/// Outputs [`AttrSelector`](struct.AttrSelector.html)
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

/// Get attributes and their values from a given section.
///
/// Created from [`Entry::section`](struct.Entry.html#method.section) or
/// [`SectionIter`](struct.SectionIter.html)
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

/// A single attribute and it's value. Can also get attribute params is they
/// exist.
///
/// The value param is an `Option` because this attribute without a param may
/// not have a value.
pub struct Attr<'a> {
    /// Name of the section the attribute is from
    pub section_name: &'a str,
    /// Name of the attribute
    pub name: &'a str,
    /// Value of the attribute if it exists.
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
