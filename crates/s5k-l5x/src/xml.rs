//! A thin owned-event cursor over `quick_xml::Reader`.
//!
//! The project reader is recursive descent over these events. Each element is
//! either consumed by a handler or skipped with [`Xml::skip`], so memory use
//! stays proportional to one element, not to the file.

use std::borrow::Cow;
use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;

use crate::Error;

/// An element start tag with owned attributes.
#[derive(Debug, Clone)]
pub(crate) struct El {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    /// Self-closing (`<X/>`): there is no content to consume.
    pub empty: bool,
    /// Byte offset of the start tag.
    pub offset: u64,
}

impl El {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// What a handler sees inside an element.
pub(crate) enum Node {
    Elem(El),
    /// Text or CDATA. Whitespace-only text between elements is dropped.
    Text(String),
}

pub(crate) struct Xml<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
}

enum Ev {
    Elem(El),
    End,
    Text(String),
    Eof,
}

impl<R: BufRead> Xml<R> {
    pub fn new(input: R) -> Self {
        let mut reader = Reader::from_reader(input);
        // End-tag names are checked by quick-xml (the default), which catches
        // truncated and hand-edited files early.
        reader.config_mut().check_end_names = true;
        Self {
            reader,
            buf: Vec::with_capacity(64 * 1024),
        }
    }

    pub fn position(&self) -> u64 {
        self.reader.buffer_position()
    }

    fn xml_err(&self, source: quick_xml::Error) -> Error {
        Error::Xml {
            offset: self.reader.buffer_position(),
            source,
        }
    }

    fn next(&mut self) -> Result<Ev, Error> {
        loop {
            self.buf.clear();
            let offset = self.reader.buffer_position();
            let event = match self.reader.read_event_into(&mut self.buf) {
                Ok(e) => e,
                Err(e) => return Err(self.xml_err(e)),
            };
            return Ok(match event {
                Event::Start(e) => Ev::Elem(el_from(&e, false, offset)),
                Event::Empty(e) => Ev::Elem(el_from(&e, true, offset)),
                Event::End(_) => Ev::End,
                Event::Text(t) => {
                    if t.iter().all(u8::is_ascii_whitespace) {
                        continue;
                    }
                    let s = match t.unescape() {
                        Ok(s) => s.into_owned(),
                        // Unknown entity (we never resolve DTD entities): keep raw text.
                        Err(_) => lossy(t.into_inner()),
                    };
                    Ev::Text(s)
                }
                Event::CData(c) => Ev::Text(lossy(c.into_inner())),
                Event::Eof => Ev::Eof,
                // Declarations, comments, processing instructions, doctype.
                _ => continue,
            });
        }
    }

    /// Next element at the top of the document (before the root is entered).
    pub fn next_top_level(&mut self) -> Result<Option<El>, Error> {
        loop {
            match self.next()? {
                Ev::Elem(el) => return Ok(Some(el)),
                Ev::Eof => return Ok(None),
                Ev::End | Ev::Text(_) => {}
            }
        }
    }

    /// Next child of `parent`, or `None` once its end tag is reached.
    ///
    /// Contract: call it in a loop until `None`, and consume every returned
    /// non-empty `Node::Elem` completely (via `next_child`, `text`, `skip` ...).
    pub fn next_child(&mut self, parent: &El) -> Result<Option<Node>, Error> {
        if parent.empty {
            return Ok(None);
        }
        match self.next()? {
            Ev::Elem(el) => Ok(Some(Node::Elem(el))),
            Ev::Text(t) => Ok(Some(Node::Text(t))),
            Ev::End => Ok(None),
            Ev::Eof => Err(Error::UnexpectedEof {
                element: parent.name.clone(),
            }),
        }
    }

    /// Next child element of `parent`, skipping text. `None` at its end tag.
    pub fn next_elem(&mut self, parent: &El) -> Result<Option<El>, Error> {
        while let Some(node) = self.next_child(parent)? {
            if let Node::Elem(el) = node {
                return Ok(Some(el));
            }
        }
        Ok(None)
    }

    /// Skip an element and everything inside it.
    pub fn skip(&mut self, el: &El) -> Result<(), Error> {
        if el.empty {
            return Ok(());
        }
        self.buf.clear();
        match self
            .reader
            .read_to_end_into(QName(el.name.as_bytes()), &mut self.buf)
        {
            Ok(_) => Ok(()),
            Err(e) => Err(self.xml_err(e)),
        }
    }

    /// All text and CDATA inside `el`, including nested elements, concatenated.
    pub fn text(&mut self, el: &El) -> Result<String, Error> {
        let mut out = String::new();
        self.collect_text(el, &mut out)?;
        Ok(out)
    }

    fn collect_text(&mut self, el: &El, out: &mut String) -> Result<(), Error> {
        while let Some(node) = self.next_child(el)? {
            match node {
                Node::Text(t) => out.push_str(&t),
                Node::Elem(child) => self.collect_text(&child, out)?,
            }
        }
        Ok(())
    }

    /// Text of a `Description`/`Comment` element. Handles both the plain form
    /// and the multi-language form (`LocalizedDescription Lang="en-US"`),
    /// preferring English. Result is trimmed; `None` when empty.
    pub fn localized_text(&mut self, el: &El) -> Result<Option<String>, Error> {
        let mut direct = String::new();
        let mut english: Option<String> = None;
        let mut first: Option<String> = None;
        while let Some(node) = self.next_child(el)? {
            match node {
                Node::Text(t) => direct.push_str(&t),
                Node::Elem(child) => {
                    let is_en = child
                        .get("Lang")
                        .is_some_and(|l| l.to_ascii_lowercase().starts_with("en"));
                    let t = self.text(&child)?;
                    if is_en && english.is_none() {
                        english = Some(t);
                    } else if first.is_none() {
                        first = Some(t);
                    }
                }
            }
        }
        let chosen = if direct.trim().is_empty() {
            english.or(first).unwrap_or_default()
        } else {
            direct
        };
        let trimmed = chosen.trim();
        Ok((!trimmed.is_empty()).then(|| trimmed.to_owned()))
    }

    /// Consume `el` and return the first value of attribute `attr` found on an
    /// element named `elem` anywhere inside it (e.g. `Structure/@DataType`).
    pub fn find_attr(&mut self, el: &El, elem: &str, attr: &str) -> Result<Option<String>, Error> {
        let mut found = None;
        self.find_attr_into(el, elem, attr, &mut found)?;
        Ok(found)
    }

    fn find_attr_into(
        &mut self,
        el: &El,
        elem: &str,
        attr: &str,
        found: &mut Option<String>,
    ) -> Result<(), Error> {
        while let Some(child) = self.next_elem(el)? {
            if found.is_none() && child.name == elem {
                *found = child.get(attr).map(str::to_owned);
            }
            if found.is_some() {
                self.skip(&child)?;
            } else {
                self.find_attr_into(&child, elem, attr, found)?;
            }
        }
        Ok(())
    }
}

fn el_from(e: &BytesStart<'_>, empty: bool, offset: u64) -> El {
    let name = lossy(Cow::Borrowed(e.name().as_ref()));
    let attrs = e
        .attributes()
        .flatten()
        .map(|a| {
            let key = lossy(Cow::Borrowed(a.key.as_ref()));
            let value = match a.unescape_value() {
                Ok(v) => v.into_owned(),
                Err(_) => lossy(a.value),
            };
            (key, value)
        })
        .collect();
    El {
        name,
        attrs,
        empty,
        offset,
    }
}

fn lossy(bytes: Cow<'_, [u8]>) -> String {
    match bytes {
        Cow::Borrowed(b) => String::from_utf8_lossy(b).into_owned(),
        Cow::Owned(v) => String::from_utf8(v)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
    }
}
