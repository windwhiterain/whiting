//! Writing the XML.
//!
//! # Why a hand-written writer
//!
//! The target is an XML file, and the obvious move is to depend on an XML crate. Two things
//! argue against it. The document is flat — one root, a dozen element names, attributes
//! only, no text content, no namespaces, no mixed content — so a serialiser would carry
//! machinery for none of it. And the one genuinely awkward part is not the XML at all: the
//! arrays are space-separated floats in a single attribute, which is a string-building
//! problem rather than a tree problem.
//!
//! Escaping is handled rather than ignored. The values written are numbers, which cannot
//! need escaping, but element and attribute *names* come from scene labels, which are
//! author-supplied strings, and those are escaped on the way out.

use std::fmt::Write as _;

/// Builds one Cycles XML document.
///
/// Tracks indentation so the emitted file is readable, which matters more here than it
/// might: the format has no schema and no diagnostics beyond its log, so when a scene does
/// not render as expected the file is the first thing anyone reads.
pub struct Writer {
    buffer: String,
    depth: usize,
}

impl Writer {
    /// A writer with the `<cycles>` root already open.
    pub fn new() -> Self {
        let mut buffer = String::new();
        buffer.push_str("<cycles>\n");
        Self { buffer, depth: 1 }
    }

    /// The document written so far, with the root closed.
    pub fn finish(mut self) -> String {
        self.indent();
        self.buffer.push_str("</cycles>\n");
        self.buffer
    }

    fn indent(&mut self) {
        for _ in 0..self.depth {
            self.buffer.push('\t');
        }
    }

    /// Opens a container element, writing its attributes.
    ///
    /// The caller closes it with [`Writer::close`]. Nesting is explicit rather than inferred
    /// so that the transform and state scopes, which is most of this format's structure, read
    /// the way they mean.
    ///
    /// Every container in this format behaves the same way: the loader reads the attributes
    /// into a copy of the read state and then recurses into the children with that copy. That
    /// is what `xml_read_scene` does for `<transform>`, `<state>` and `<object>` alike, and it
    /// is why a scoped element and the geometry it applies to are parent and child here rather
    /// than siblings. A scope written as a self-closing element pushes state and then ends it,
    /// which leaves whatever follows outside the scope without ever saying so.
    pub fn open(&mut self, name: &str, attributes: &[(&str, String)]) {
        self.indent();
        self.buffer.push('<');
        self.buffer.push_str(name);
        for (key, value) in attributes {
            let _ = write!(self.buffer, " {}=\"{}\"", escape(key), escape(value));
        }
        self.buffer.push_str(">\n");
        self.depth += 1;
    }

    /// Opens an element with no attributes.
    pub fn open_bare(&mut self, name: &str) {
        self.open(name, &[]);
    }

    /// Closes the innermost open element.
    pub fn close(&mut self, name: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.indent();
        let _ = writeln!(self.buffer, "</{name}>");
    }

    /// Writes a self-closing element.
    pub fn leaf(&mut self, name: &str, attributes: &[(&str, String)]) {
        self.indent();
        self.buffer.push('<');
        self.buffer.push_str(name);
        for (key, value) in attributes {
            let _ = write!(self.buffer, " {}=\"{}\"", escape(key), escape(value));
        }
        self.buffer.push_str(" />\n");
    }

    /// Writes a comment. The examples in the repository are commented, and a scene a person
    /// has to debug by reading is a scene that benefits from saying what each block is.
    pub fn comment(&mut self, text: &str) {
        self.indent();
        let _ = writeln!(self.buffer, "<!-- {text} -->");
    }

    /// Writes one array attribute in the form the loader parses.
    ///
    /// The loader splits on whitespace with `string_split`, so a single space between
    /// entries is correct and anything else is not. Floating point is written at full
    /// precision: these values are positions and normals, and rounding them here would put
    /// error into the geometry that no later stage could attribute.
    pub fn float_array(values: &[f64]) -> String {
        let mut text = String::with_capacity(values.len() * 12);
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                text.push(' ');
            }
            let _ = write!(text, "{value}");
        }
        text
    }

    /// Writes one integer array in the form the loader parses.
    pub fn int_array(values: &[i64]) -> String {
        let mut text = String::with_capacity(values.len() * 4);
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                text.push(' ');
            }
            let _ = write!(text, "{value}");
        }
        text
    }

    /// Writes a colour triple the way the loader reads it.
    ///
    /// Comma separated, not space separated. This differs from the vertex arrays and is the
    /// kind of detail that produces a silently black render if it is wrong.
    pub fn color(r: f64, g: f64, b: f64) -> String {
        format!("{r}, {g}, {b}")
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

/// Escapes the five characters XML reserves.
///
/// Element and attribute names are author-supplied scene labels, and a label containing an
/// ampersand would otherwise produce a document the loader's parser rejects, with the
/// failure pointing at the parser rather than at the label.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}
