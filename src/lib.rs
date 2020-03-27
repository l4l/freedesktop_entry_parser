/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! A library for parsing FreeDesktop entry files.
//! These files are used in the
//! [Desktop Entry](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html)
//! and [Icon Theme](https://specifications.freedesktop.org/icon-theme-spec/icon-theme-spec-latest.html).
//! They are similar to ini files but are distinct enough that
//! an ini parse would not work.
//!
//! Example:
//! ```
//! use freedesktop_entry_parser::*;
//!
//! let file = b"[Desktop Entry]
//! Name=Firefox
//! Exec=firefox %u
//! Icon=firefox";
//!
//! assert_eq!(parse_entry(file).next().unwrap()?, SectionBytes {
//!     title: b"Desktop Entry",
//!     attrs: vec![
//!         AttrBytes { name: b"Name", value: b"Firefox"},
//!         AttrBytes { name: b"Exec", value: b"firefox %u"},
//!         AttrBytes { name: b"Icon", value: b"firefox"},
//!     ]
//! });
//! # Ok::<(), Error>(())
//! ```

mod debug;

pub use anyhow::Error;
use anyhow::{anyhow, Result};
use nom::{
    bytes::complete::{tag, take_till, take_till1},
    error::ErrorKind,
    multi::many1,
    sequence::{delimited, terminated},
    IResult,
};
use std::iter::Iterator;
use std::str::from_utf8;

/// A name and value pair from a [`SectionBytes`](struct.SectionBytes.html)
#[derive(PartialEq, Eq)]
pub struct AttrBytes<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
}

/// One section on a entry file
#[derive(PartialEq, Eq)]
pub struct SectionBytes<'a> {
    /// Section title
    pub title: &'a [u8],
    /// List of attributes
    pub attrs: Vec<AttrBytes<'a>>,
}

fn not_whitespace(c: u8) -> bool {
    c != '\n' as u8 && c != '\t' as u8 && c != '\r' as u8 && c != ' ' as u8
}

fn header(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(tag(b"["), take_till1(|c| c == ']' as u8), tag(b"]"))(input)
}

fn next_line(input: &[u8]) -> Result<&[u8], nom::Err<(&[u8], ErrorKind)>> {
    if input.is_empty() {
        return Ok(b"");
    }
    let (rem, _) = take_till(not_whitespace)(input)?;
    if rem.get(0) == Some(&('#' as u8)) {
        let (rem, _) = take_till(|c| c == '\n' as u8)(rem)?;
        return next_line(rem);
    }
    Ok(rem)
}

fn find_start(input: &[u8]) -> Result<&[u8], nom::Err<(&[u8], ErrorKind)>> {
    let (rem, _): (&[u8], &[u8]) = take_till(|c| c == '[' as u8)(input)?;
    Ok(rem)
}

fn attr(input: &[u8]) -> IResult<&[u8], AttrBytes> {
    if input.get(0) == Some(&('[' as u8)) {
        return Err(nom::Err::Error((input, ErrorKind::Complete)));
    }
    let (rem, name) =
        terminated(take_till(|c| c == '=' as u8), tag(b"="))(input)?;
    let (rem, value) = take_till(|c| c == '\n' as u8)(rem)?;
    Ok((next_line(rem)?, AttrBytes { name, value }))
}

fn section(input: &[u8]) -> IResult<&[u8], SectionBytes> {
    let (rem, title) = header(input)?;
    let rem = next_line(rem)?;
    let (rem, attrs) = many1(attr)(rem)?;
    Ok((rem, SectionBytes { title, attrs }))
}

/// An iterator over the sections in a entry file.
/// Returns [`SectionBytes`](struct.SectionBytes.html)
pub struct EntryIter<'a> {
    rem: &'a [u8],
    found_start: bool,
}

impl<'a> EntryIter<'a> {
    fn next_section(&mut self) -> Result<SectionBytes<'a>> {
        if !self.found_start {
            self.rem = find_start(self.rem).map_err(format_nom_err)?;
            self.found_start = true;
        }
        let (rem, _): (&[u8], &[u8]) =
            take_till(|c| c == '[' as u8)(self.rem).map_err(format_nom_err)?;
        let (rem, section_bytes) = section(rem).map_err(format_nom_err)?;
        self.rem = rem;
        Ok(section_bytes)
    }
}

impl<'a> Iterator for EntryIter<'a> {
    type Item = Result<SectionBytes<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rem.is_empty() {
            return None;
        }
        Some(self.next_section())
    }
}

/// Parse a FreeDesktop entry file.
/// Returns and iterator over the sections in the file.
pub fn parse_entry<'a>(input: &'a [u8]) -> EntryIter<'a> {
    EntryIter {
        rem: input,
        found_start: false,
    }
}

fn format_nom_err(
    e: nom::Err<(&[u8], nom::error::ErrorKind)>,
) -> anyhow::Error {
    match e {
        nom::Err::Error((bytes, kind)) | nom::Err::Failure((bytes, kind)) => {
            match from_utf8(bytes) {
                Ok(s) => anyhow!("{} at `{}`", kind.description(), s),
                Err(e) => e.into(),
            }
        }
        nom::Err::Incomplete(_) => anyhow!("Incomplete Input"),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod fn_header {
        use super::*;

        #[test]
        fn ok() {
            assert_eq!(header(b"[hello]"), Ok((&b""[..], &b"hello"[..])));
        }

        #[test]
        fn no_start() {
            assert_eq!(
                header(b"hello"),
                Err(nom::Err::Error((&b"hello"[..], ErrorKind::Tag)))
            );
        }

        #[test]
        fn no_end() {
            assert_eq!(
                header(b"[hello"),
                Err(nom::Err::Error((&b""[..], ErrorKind::Tag)))
            );
        }
    }

    mod fn_next_line {
        use super::*;
        #[test]
        fn empty() {
            assert_eq!(next_line(b""), Ok(&b""[..]));
        }

        #[test]
        fn only_whitespace() {
            assert_eq!(next_line(b" \t \t\n\r\nhello"), Ok(&b"hello"[..]));
        }

        #[test]
        fn comment() {
            assert_eq!(
                next_line(b"   \t\n# Comment\nhello"),
                Ok(&b"hello"[..])
            );
        }

        #[test]
        fn no_change() {
            assert_eq!(next_line(b"hello\n"), Ok(&b"hello\n"[..]));
        }
    }

    mod fn_attr {
        use super::*;

        #[test]
        fn ok() {
            assert_eq!(
                attr(b"hello=world"),
                Ok((
                    &b""[..],
                    AttrBytes {
                        name: &b"hello"[..],
                        value: &b"world"[..]
                    }
                ))
            );
        }

        #[test]
        fn space_in_value() {
            assert_eq!(
                attr(b"hello=world today"),
                Ok((
                    &b""[..],
                    AttrBytes {
                        name: &b"hello"[..],
                        value: &b"world today"[..]
                    }
                ))
            );
        }

        #[test]
        fn no_value() {
            assert_eq!(
                attr(b"hello="),
                Ok((
                    &b""[..],
                    AttrBytes {
                        name: &b"hello"[..],
                        value: &b""[..]
                    }
                ))
            );
        }

        #[test]
        fn no_name() {
            assert_eq!(
                attr(b"=world"),
                Ok((
                    &b""[..],
                    AttrBytes {
                        name: &b""[..],
                        value: &b"world"[..]
                    }
                ))
            );
        }

        #[test]
        fn no_eq() {
            assert_eq!(
                attr(b"hello"),
                Err(nom::Err::Error((&b""[..], ErrorKind::Tag)))
            );
        }
    }

    mod fn_section {
        use super::*;

        #[test]
        fn ok() {
            assert_eq!(
                section(b"[apps]\nSize=48\nScale=1"),
                Ok((
                    &b""[..],
                    SectionBytes {
                        title: &b"apps"[..],
                        attrs: vec![
                            AttrBytes {
                                name: &b"Size"[..],
                                value: &b"48"[..]
                            },
                            AttrBytes {
                                name: &b"Scale"[..],
                                value: &b"1"[..]
                            }
                        ]
                    }
                ))
            );
        }

        #[test]
        fn no_attrs() {
            assert_eq!(
                section(b"[apps]\n"),
                Err(nom::Err::Error((&b""[..], ErrorKind::Tag)))
            );
        }

        #[test]
        fn no_header() {
            assert_eq!(
                section(b"Size=48\nScale=1"),
                Err(nom::Err::Error((
                    &b"Size=48\nScale=1"[..],
                    ErrorKind::Tag
                )))
            );
        }
    }
}
