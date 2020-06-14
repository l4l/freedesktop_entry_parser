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

use std::{
    collections::HashMap, fs::File, hash::Hash, io::Read,
    marker::PhantomPinned, mem::transmute, path::Path, pin::Pin, ptr::NonNull,
    rc::Rc,
};

type SectionMap = HashMap<(SP, Option<SP>), SP>;

struct Internal {
    /// Section, attribute, param, value
    map: Option<HashMap<SP, SectionMap>>,
    data: Vec<u8>,
    _pin: PhantomPinned,
}

impl Internal {
    fn get<'a>(
        self: &'a Pin<Box<Self>>,
        section: &str,
        name: &str,
        param: Option<&str>,
    ) -> Option<&'a str> {
        self.map
            .as_ref()
            .unwrap()
            .get(&SP::from(section))
            .map(|map| {
                map.get(&(SP::from(name), param.map(|v| SP::from(v))))
                    // SAFETY: This is safe because the string does live as long as the struct
                    .map(|v| unsafe { transmute(v.0.as_ptr()) })
            })
            .flatten()
    }
}

/// str pointer
#[derive(Eq)]
struct SP(NonNull<str>);

impl SP {
    fn from(s: &str) -> Self {
        SP(NonNull::from(s))
    }
}

impl PartialEq for SP {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: This is safe because both references are dropped at the end
        // of the fn
        unsafe { *self.0.as_ref() == *other.0.as_ref() }
    }
}

impl Hash for SP {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // SAFETY: This is safe because the reference is dropped at the end
        // of the fn
        unsafe { self.0.as_ref().hash(state) }
    }
}

pub struct Entry(Pin<Box<Internal>>);

impl Entry {
    pub fn parse(input: impl Into<Vec<u8>>) -> Result<Self, ParseError> {
        let this = Internal {
            map: None,
            data: input.into(),
            _pin: PhantomPinned,
        };
        let mut boxed = Box::pin(this);

        let entry_bytes =
            parse_entry(&boxed.data).collect::<Result<Vec<_>, _>>()?;

        let mut sections = HashMap::new();

        for section_bytes in entry_bytes {
            let section = parse_str(section_bytes.title)?;
            let mut map = HashMap::new();
            for attr_bytes in section_bytes.attrs {
                let value = parse_str(attr_bytes.value)?;

                match attr_bytes.param {
                    Some(param) => {
                        let name = parse_str(param.attr_name)?;
                        let param = parse_str(param.param)?;
                        map.insert(
                            (SP::from(name), Some(SP::from(param))),
                            SP::from(value),
                        );
                    }
                    None => {
                        let name = parse_str(attr_bytes.name)?;
                        map.insert((SP::from(name), None), SP::from(value));
                    }
                }
            }
            sections.insert(SP::from(section), map);
        }
        // SAFETY: we know this is safe because modifying a field doesn't move the whole struct
        unsafe {
            let mut_ref: Pin<&mut Internal> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).map = Some(sections);
        }
        Ok(Entry(boxed))
    }

    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        let mut file = File::open(path).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        Self::parse(buf)
    }

    pub fn section<'a, T: AsRef<str>>(&'a self, name: T) -> SectionSelector<T> {
        SectionSelector {
            name: name,
            entry: self,
        }
    }

    // pub fn sections(&self) -> SectionMap {
    //     self.0.
    // }
}

pub struct Section<'a>(&'a SectionMap);

#[derive(Debug)]
pub struct Attr<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

pub struct SectionSelector<'a, T> {
    name: T,
    entry: &'a Entry,
}

impl<'a, T: AsRef<str>> SectionSelector<'a, T> {
    pub fn attr(&self, name: impl AsRef<str>) -> Option<&'a str> {
        self.entry.0.get(self.name.as_ref(), name.as_ref(), None)
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

    pub fn attrs(&self) -> Option<Vec<Attr<'a>>> {
        self.entry
            .0
            .map
            .as_ref()
            .unwrap()
            .get(&SP::from(self.name.as_ref()))
            .map(|section| {
                section
                    .iter()
                    .map(|((name, _), value)| Attr {
                        key: unsafe { transmute(name.0.as_ptr()) },
                        value: unsafe { transmute(value.0.as_ptr()) },
                    })
                    .collect()
            })
    }
}

pub fn parse_str(input: &[u8]) -> Result<&str, ParseError> {
    std::str::from_utf8(input).map_err(|e| ParseError::Utf8Error {
        bytes: input.to_owned(),
        source: e,
    })
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
        // let desc = entry.get("Unit", "Description", None);
        let desc = entry.section("Unit").attr("Description");
        println!("{:?}", desc);
        println!("{:?}", desc);
        std::mem::drop(entry);
        // println!("{:?}", desc);
    }
}
