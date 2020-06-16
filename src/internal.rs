use crate::{parse_entry, ParseError};
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
    param_map: Option<HashMap<SP, SP>>,
}

/// <section, <attribute, {value, <param, param_vale>}>>
type InternalMap = HashMap<SP, HashMap<SP, AttrValue>>;

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
                                    .get_or_insert_with(HashMap::new)
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
                                    Some(map)
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
            sections.insert(SP::from(section), map);
        }
        // SAFETY: we know this is safe because modifying a field doesn't move the whole struct
        unsafe {
            let mut_ref: Pin<&mut Internal> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).map = Some(sections);
        }
        Ok(boxed)
    }

    pub(crate) fn get<'a>(
        self: &'a Pin<Box<Self>>,
        section_name: &str,
        attr_name: &str,
        param_name: Option<&str>,
    ) -> Option<&'a str> {
        self.map
            .as_ref()
            .unwrap()
            .get(&SP::from(section_name))
            .map(|map| {
                map.get(&SP::from(attr_name)).map(|attr| match param_name {
                    Some(param_name) => match &attr.param_map {
                        Some(map) => map.get(&SP::from(param_name)),
                        None => None,
                    },
                    None => attr.value.as_ref(),
                })
            })
            .flatten()
            .flatten()
            // SAFETY: This is safe because the string does live as long as the struct
            .map(|s| unsafe { transmute(s.0.as_ptr()) })
    }

    pub(crate) fn section_names_iter<'a>(&'a self) -> SectionNamesIter<'a> {
        KeysIter(self.map.as_ref().unwrap().keys())
    }

    pub(crate) fn attr_names_iter<'a>(
        &'a self,
        section_name: &str,
    ) -> Option<AttrNamesIter<'a>> {
        self.map
            .as_ref()
            .unwrap()
            .get(&SP::from(section_name))
            .map(|map| KeysIter(map.keys()))
    }

    pub(crate) fn param_names_iter<'a>(
        &'a self,
        section_name: &str,
        attr_name: &str,
    ) -> Option<ParamNamesIter<'a>> {
        self.map
            .as_ref()
            .unwrap()
            .get(&SP::from(section_name))
            .map(|map| {
                map.get(&SP::from(attr_name)).map(|attr| {
                    match &attr.param_map {
                        Some(map) => Some(KeysIter(map.keys())),
                        None => None,
                    }
                })
            })
            .flatten()
            .flatten()
    }
}

pub(crate) struct KeysIter<'a, T>(Keys<'a, SP, T>);

impl<'a, T> Iterator for KeysIter<'a, T> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|sp| unsafe { transmute(sp.0.as_ptr()) })
    }
}

pub(crate) type SectionNamesIter<'a> = KeysIter<'a, HashMap<SP, AttrValue>>;
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
