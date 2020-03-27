use crate::{AttrBytes, SectionBytes};
use std::fmt::{Debug, Formatter, Result};
use std::str::from_utf8;

impl<'a> Debug for AttrBytes<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let name = match from_utf8(self.name) {
            Ok(s) => s.to_owned(),
            Err(_) => format!("{:?}", self.name).to_owned(),
        };
        let value = match from_utf8(self.value) {
            Ok(s) => s.to_owned(),
            Err(_) => format!("{:?}", self.value).to_owned(),
        };
        f.debug_struct("AttrBytes")
            .field("name", &name)
            .field("value", &value)
            .finish()
    }
}

impl<'a> Debug for SectionBytes<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let title = match from_utf8(self.title) {
            Ok(s) => s.to_owned(),
            Err(_) => format!("{:?}", self.title).to_owned(),
        };
        f.debug_struct("SectionBytes")
            .field("title", &title)
            .field("attrs", &self.attrs)
            .finish()
    }
}
