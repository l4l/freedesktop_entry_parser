//! Internal entry map
//!
//! The map uses unsafe code and this module provides a safe but
//! unergonomic API for use by the nicer API.
use crate::{parser::parse_entry, ParseError};
use std::{
    collections::{hash_map::Keys, HashMap},
    fmt::{Debug, Formatter},
    hash::Hash,
    intrinsics::transmute,
    marker::PhantomPinned,
    pin::Pin,
    ptr::NonNull,
};

pub struct AttrValue {
    value: Option<SP>,
    param_map: Option<ParamMap>,
}

/// <section, <attribute, {value, <param, param_vale>}>>
type InternalMap = HashMap<SP, AttrMap>;

pub(crate) struct AttrMap(HashMap<SP, AttrValue>);
pub(crate) struct ParamMap(HashMap<SP, SP>);

pub(crate) struct Internal {
    map: Option<InternalMap>,
    data: Vec<u8>,
    _pin: PhantomPinned,
}

impl Internal {
    pub(crate) fn new(data: Vec<u8>) -> Result<Pin<Box<Self>>, ParseError> {
        let this = Self {
            map: None,
            data,
            _pin: PhantomPinned,
        };
        let mut boxed = Box::pin(this);

        let entry_bytes =
            parse_entry(&boxed.data).collect::<Result<Vec<_>, _>>()?;

        let mut sections: InternalMap = HashMap::new();

        for section_bytes in entry_bytes {
            let section = parse_str(section_bytes.title)?;
            let mut map: HashMap<SP, AttrValue> = HashMap::new();
            for attr_bytes in section_bytes.attrs {
                let value = parse_str(attr_bytes.value)?;

                match attr_bytes.param {
                    Some(param) => {
                        let name = parse_str(param.attr_name)?;
                        let param = parse_str(param.param)?;
                        map.entry(SP::from(name))
                            .and_modify(|attr| {
                                attr.param_map
                                    .get_or_insert_with(ParamMap::new)
                                    .0
                                    .insert(SP::from(param), SP::from(value));
                            })
                            .or_insert(AttrValue {
                                value: None,
                                param_map: {
                                    let mut map = HashMap::new();
                                    map.insert(
                                        SP::from(param),
                                        SP::from(value),
                                    );
                                    Some(ParamMap(map))
                                },
                            });
                    }
                    None => {
                        let name = parse_str(attr_bytes.name)?;
                        map.entry(SP::from(name))
                            .and_modify(|attr| {
                                attr.value = Some(SP::from(value))
                            })
                            .or_insert(AttrValue {
                                value: Some(SP::from(value)),
                                param_map: None,
                            });
                    }
                }
            }
            sections.insert(SP::from(section), AttrMap(map));
        }
        // SAFETY: we know this is safe because modifying a field doesn't move the whole struct
        unsafe {
            let mut_ref: Pin<&mut Internal> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).map = Some(sections);
        }
        Ok(boxed)
    }

    fn get_section<'a>(
        self: &'a Pin<Box<Self>>,
        section_name: &str,
    ) -> Option<&'a AttrMap> {
        self.map.as_ref().unwrap().get(&SP::from(section_name))
    }

    pub(crate) fn get<'a>(
        self: &'a Pin<Box<Self>>,
        section_name: &str,
        attr_name: &str,
        param_name: Option<&str>,
    ) -> Option<&'a str> {
        let section_map = self.get_section(section_name)?;
        let attr_val = section_map.get_attr(attr_name)?;
        match param_name {
            Some(param_name) => {
                let param_map = attr_val.param_map.as_ref()?;
                param_map.get_param(param_name)
            }
            None => attr_val.get_value(),
        }
    }

    pub(crate) fn get_attr<'a>(
        self: &'a Pin<Box<Self>>,
        section_name: &str,
        attr_name: &str,
    ) -> Option<&'a AttrValue> {
        let section_map = self.get_section(section_name)?;
        section_map.get_attr(attr_name)
    }

    pub(crate) fn has_section(
        self: &Pin<Box<Self>>,
        section_name: &str,
    ) -> bool {
        self.get_section(section_name).is_some()
    }

    pub(crate) fn section_names_iter<'a>(
        self: &'a Pin<Box<Self>>,
    ) -> SectionNamesIter<'a> {
        KeysIter(self.map.as_ref().unwrap().keys())
    }

    pub(crate) fn attr_names_iter<'a>(
        self: &'a Pin<Box<Self>>,
        section_name: &str,
    ) -> Option<AttrNamesIter<'a>> {
        Some(KeysIter(self.get_section(section_name)?.0.keys()))
    }

    pub(crate) fn param_names_iter<'a>(
        self: &'a Pin<Box<Self>>,
        section_name: &str,
        attr_name: &str,
    ) -> Option<ParamNamesIter<'a>> {
        let section_map = self.get_section(section_name)?;
        let attr_val = section_map.get_attr(attr_name)?;
        let param_map = attr_val.param_map.as_ref()?;
        Some(KeysIter(param_map.0.keys()))
    }
}

impl AttrMap {
    pub(crate) fn get_attr(&self, attr_name: &str) -> Option<&AttrValue> {
        self.0.get(&SP::from(attr_name))
    }
}

impl AttrValue {
    pub(crate) fn get_value<'a>(&'a self) -> Option<&'a str> {
        // SAFETY: This is safe because the string has the same lifetime as Entry
        self.value
            .as_ref()
            .map(|s| unsafe { transmute(s.0.as_ptr()) })
    }

    pub(crate) fn get_params(&self) -> Option<&ParamMap> {
        self.param_map.as_ref()
    }
}

impl ParamMap {
    fn new() -> ParamMap {
        ParamMap(HashMap::new())
    }

    pub(crate) fn get_param<'a>(&'a self, param_val: &str) -> Option<&'a str> {
        self.0
            .get(&SP::from(param_val))
            // SAFETY: This is safe because the string has the same lifetime as Entry
            .map(|s| unsafe { transmute(s.0.as_ptr()) })
    }
}

pub(crate) struct KeysIter<'a, T>(Keys<'a, SP, T>);

impl<'a, T> Iterator for KeysIter<'a, T> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|sp| unsafe { transmute(sp.0.as_ptr()) })
    }
}

pub(crate) type SectionNamesIter<'a> = KeysIter<'a, AttrMap>;
pub(crate) type AttrNamesIter<'a> = KeysIter<'a, AttrValue>;
pub(crate) type ParamNamesIter<'a> = KeysIter<'a, SP>;

/// str pointer
#[derive(Eq)]
pub(crate) struct SP(NonNull<str>);

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

impl Debug for SP {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // SAFETY: This is safe because the reference is dropped at the end of
        // the fn
        unsafe { self.0.as_ref().fmt(f) }
    }
}

#[inline]
fn parse_str(input: &[u8]) -> Result<&str, ParseError> {
    std::str::from_utf8(input).map_err(|e| ParseError::Utf8Error {
        bytes: input.to_owned(),
        source: e,
    })
}
