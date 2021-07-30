pub use crate::errors::ParseError;
use nom::{
    bytes::complete::{tag, take_till, take_till1},
    error::ErrorKind,
    multi::many1,
    sequence::{delimited, terminated},
    IResult,
};
use std::iter::Iterator;

/// A name and value pair from a [`SectionBytes`](struct.SectionBytes.html)
#[derive(PartialEq, Eq)]
pub struct AttrBytes<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
    /// Some attributes have a parameter like `GenericName[es]=Navegador web`
    /// If it does this field will be present
    pub param: Option<ParamBytes<'a>>,
}

/// A param value and attribute name
#[derive(PartialEq, Eq)]
pub struct ParamBytes<'a> {
    /// Value of the the param, ex. `es`
    pub param: &'a [u8],
    /// Name of the attribute without the param ex `GenericName`
    pub attr_name: &'a [u8],
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
    c != b'\n' && c != b'\t' && c != b'\r' && c != b' '
}

/// Parse a header line.  Return the header name
fn header(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(tag(b"["), take_till1(|c| c == b']'), tag(b"]"))(input)
}

/// Find the next line, ignoring comments
fn next_line(
    input: &[u8],
) -> Result<&[u8], nom::Err<nom::error::Error<&[u8]>>> {
    if input.is_empty() {
        return Ok(b"");
    }
    let (rem, _) = take_till(not_whitespace)(input)?;
    if rem.get(0) == Some(&(b'#')) {
        let (rem, _) = take_till(|c| c == b'\n')(rem)?;
        return next_line(rem);
    }
    Ok(rem)
}

fn find_start(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_till(|c| c == b'[')(input)
}

/// Parse attr params
fn params(input: &[u8]) -> IResult<&[u8], ParamBytes> {
    let (rem, attr_name) =
        terminated(take_till(|c| c == b'['), tag(b"["))(input)?;
    let (rem, param) = take_till(|c| c == b']')(rem)?;
    Ok((rem, ParamBytes { param, attr_name }))
}

fn attr(input: &[u8]) -> IResult<&[u8], AttrBytes> {
    if input.get(0) == Some(&(b'[')) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            ErrorKind::Complete,
        )));
    }
    let (rem, name) = terminated(take_till(|c| c == b'='), tag(b"="))(input)?;
    let (rem, value) = take_till(|c| c == b'\n')(rem)?;

    Ok((
        next_line(rem)?,
        AttrBytes {
            name,
            value,
            param: params(name).ok().map(|(_, param)| param),
        },
    ))
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
    fn next_section(&mut self) -> Result<SectionBytes<'a>, ParseError> {
        if !self.found_start {
            self.rem = find_start(self.rem)?.0;
            self.found_start = true;
        }
        let (rem, _) = find_start(self.rem)?;
        let (rem, section_bytes) = section(rem)?;
        self.rem = rem;
        Ok(section_bytes)
    }
}

impl<'a> Iterator for EntryIter<'a> {
    type Item = Result<SectionBytes<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rem.is_empty() {
            return None;
        }
        Some(self.next_section())
    }
}

/// Parse a FreeDesktop entry file.
/// Returns and iterator over the sections in the file.
pub fn parse_entry(input: &[u8]) -> EntryIter<'_> {
    EntryIter {
        rem: input,
        found_start: false,
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
                header(b"hello").unwrap_err(),
                nom::Err::Error(nom::error::make_error(
                    &b"hello"[..],
                    ErrorKind::Tag
                ))
            );
        }

        #[test]
        fn no_end() {
            assert_eq!(
                header(b"[hello").unwrap_err(),
                nom::Err::Error(nom::error::make_error(
                    &b""[..],
                    ErrorKind::Tag
                ))
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
                        value: &b"world"[..],
                        param: None,
                    }
                ))
            );
        }

        #[test]
        fn with_param() {
            assert_eq!(
                attr(b"hello[en]=world"),
                Ok((
                    &b""[..],
                    AttrBytes {
                        name: &b"hello[en]"[..],
                        value: &b"world"[..],
                        param: Some(ParamBytes {
                            attr_name: &b"hello"[..],
                            param: &b"en"[..]
                        }),
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
                        value: &b"world today"[..],
                        param: None,
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
                        value: &b""[..],
                        param: None,
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
                        value: &b"world"[..],
                        param: None,
                    }
                ))
            );
        }

        #[test]
        fn no_eq() {
            assert_eq!(
                attr(b"hello"),
                Err(nom::Err::Error(nom::error::Error {
                    input: &b""[..],
                    code: ErrorKind::Tag
                }))
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
                                value: &b"48"[..],
                                param: None,
                            },
                            AttrBytes {
                                name: &b"Scale"[..],
                                value: &b"1"[..],
                                param: None,
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
                Err(nom::Err::Error(nom::error::Error {
                    input: &b""[..],
                    code: ErrorKind::Tag
                }))
            );
        }

        #[test]
        fn no_header() {
            assert_eq!(
                section(b"Size=48\nScale=1"),
                Err(nom::Err::Error(nom::error::Error {
                    input: &b"Size=48\nScale=1"[..],
                    code: ErrorKind::Tag
                }))
            );
        }
    }

    #[test]
    fn parse_icon_index() {
        let input = include_bytes!("./../test_data/gnome-index.theme");
        let sections = parse_entry(input)
            .collect::<Result<Vec<_>, _>>()
            .expect("Error parsing input");
        assert_eq!(sections.len(), 68);
    }

    #[test]
    fn parse_firefox_desktop_entry() {
        let input = include_bytes!("./../test_data/firefox.desktop");
        let sections = parse_entry(input)
            .collect::<Result<Vec<_>, _>>()
            .expect("Error parsing input");
        assert_eq!(sections.len(), 3);
        assert_eq!(
            sections[0].attrs[1],
            AttrBytes {
                name: &b"Name"[..],
                value: &b"Firefox"[..],
                param: None,
            }
        );
        assert_eq!(
            sections[0].attrs[4],
            AttrBytes {
                name: &b"GenericName[ast]"[..],
                value: &b"Restolador Web"[..],
                param: Some(ParamBytes {
                    attr_name: &b"GenericName"[..],
                    param: &b"ast"[..]
                }),
            }
        );
    }

    #[test]
    fn parse_sshd_systemd_unit() {
        let input = include_bytes!("./../test_data/sshd.service");
        let sections = parse_entry(input)
            .collect::<Result<Vec<_>, _>>()
            .expect("Error parsing input");
        assert_eq!(sections.len(), 3);
    }
}
