//! A small XML tree for the OPI messages: parse a message into
//! elements with attributes and text, write attribute values safely.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// One element: local name (no namespace prefix), attributes in order,
/// its own text, children.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Elem {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<Elem>,
}

impl Elem {
    pub(crate) fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// The first direct child called `name`, ignoring case (the public
    /// samples write both `POSData` and `POSdata`).
    #[cfg(test)]
    pub(crate) fn child(&self, name: &str) -> Option<&Elem> {
        self.children
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// The first element called `name` in this subtree, this element
    /// included, depth first.
    pub(crate) fn find(&self, name: &str) -> Option<&Elem> {
        if self.name.eq_ignore_ascii_case(name) {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(name))
    }

    /// Every element called `name` in this subtree, document order.
    pub(crate) fn find_all<'a>(&'a self, name: &str, out: &mut Vec<&'a Elem>) {
        if self.name.eq_ignore_ascii_case(name) {
            out.push(self);
        }
        for c in &self.children {
            c.find_all(name, out);
        }
    }

    /// The trimmed text of this element.
    pub(crate) fn text(&self) -> &str {
        self.text.trim()
    }
}

fn start_elem(e: &BytesStart<'_>) -> Elem {
    let mut elem = Elem {
        name: e.local_name().as_ref().to_string(),
        ..Elem::default()
    };
    for a in e.attributes().flatten() {
        let key = a.key.local_name().as_ref().to_string();
        let value = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_or_else(|_| a.value.to_string(), std::borrow::Cow::into_owned);
        // Namespace declarations are not data.
        if key != "xmlns" && !a.key.as_ref().starts_with("xmlns:") {
            elem.attrs.push((key, value));
        }
    }
    elem
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

/// Deepest element nesting `parse` accepts. Real OPI messages are a few
/// levels deep; the cap keeps the recursive drop of the tree (and a frame of
/// nested tags from any LAN host) from overflowing the stack.
const MAX_DEPTH: usize = 64;

/// Parse one XML document into its root element.
pub(crate) fn parse(xml: &str) -> Result<Elem, String> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Elem> = Vec::new();
    let mut root: Option<Elem> = None;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                if stack.len() >= MAX_DEPTH {
                    return Err("xml too deep".into());
                }
                stack.push(start_elem(&e));
            }
            Event::Empty(e) => {
                if stack.len() >= MAX_DEPTH {
                    return Err("xml too deep".into());
                }
                let elem = start_elem(&e);
                match stack.last_mut() {
                    Some(parent) => parent.children.push(elem),
                    None => root = Some(elem),
                }
            }
            Event::End(_) => {
                let elem = stack.pop().ok_or("unbalanced end tag")?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(elem),
                    None => root = Some(elem),
                }
            }
            Event::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text
                        .push_str(&t.xml_content(quick_xml::XmlVersion::Implicit1_0));
                }
            }
            Event::CData(c) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&c);
                }
            }
            Event::GeneralRef(r) => {
                let resolved = match r.resolve_char_ref() {
                    Ok(Some(c)) => Some(c),
                    _ => entity(&r),
                };
                if let (Some(c), Some(top)) = (resolved, stack.last_mut()) {
                    top.text.push(c);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err("unclosed element".into());
    }
    root.ok_or_else(|| "no root element".to_string())
}

/// `text` as XML character data or an attribute value.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if u32::from(c) < 0x20 && !matches!(c, '\n' | '\t' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_is_read_into_a_tree() {
        let e = parse(
            "<?xml version=\"1.0\"?><A x=\"1 &amp; 2\" xmlns=\"urn:z\"><B y=\"2\"/>\
             <C>t &lt; u &#65;</C><C>second</C></A>",
        )
        .unwrap();
        assert_eq!(e.name, "A");
        assert_eq!(e.attr("x"), Some("1 & 2"));
        assert_eq!(e.attr("xmlns"), None);
        assert_eq!(e.child("b").unwrap().attr("y"), Some("2"));
        assert_eq!(e.child("C").unwrap().text(), "t < u A");
        let mut all = Vec::new();
        e.find_all("C", &mut all);
        assert_eq!(all.len(), 2);
        assert_eq!(e.find("C").unwrap().text(), "t < u A");
    }

    #[test]
    fn deeply_nested_xml_is_an_error_not_a_crash() {
        let depth = 300_000;
        let xml = format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth));
        assert_eq!(parse(&xml).unwrap_err(), "xml too deep");
        let ok = format!("{}{}", "<a>".repeat(60), "</a>".repeat(60));
        assert!(parse(&ok).is_ok());
    }

    #[test]
    fn broken_xml_is_an_error() {
        assert!(parse("<A><B></A>").is_err());
        assert!(parse("").is_err());
        assert!(parse("<A>").is_err());
    }

    #[test]
    fn escaping_covers_the_markup_characters() {
        assert_eq!(
            escape("a&b<c>\"d\"'e'"),
            "a&amp;b&lt;c&gt;&quot;d&quot;&apos;e&apos;"
        );
    }
}
