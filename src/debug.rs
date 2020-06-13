/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::parser::{AttrBytes, ParamBytes, SectionBytes};
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
            .field("param", &self.param)
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

impl<'a> Debug for ParamBytes<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let attr_name = match from_utf8(self.attr_name) {
            Ok(s) => s.to_owned(),
            Err(_) => format!("{:?}", self.attr_name).to_owned(),
        };
        let param = match from_utf8(self.param) {
            Ok(s) => s.to_owned(),
            Err(_) => format!("{:?}", self.param).to_owned(),
        };
        f.debug_struct("AttrBytes")
            .field("attr_name", &attr_name)
            .field("param", &param)
            .finish()
    }
}
