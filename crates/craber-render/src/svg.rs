//! Emitting SVG by hand.
//!
//! No XML library: the output is a fixed shape CrabER controls end to end, and
//! a dependency-light renderer is what keeps the single binary single.
//!
//! [`Xml`] carries every element the renderer draws, so a tag's *shape* — which
//! coordinate goes in which attribute, at what precision — is written once here
//! instead of in each caller's `format!`. What stays a string is the attribute
//! tail, because that is what makes two rects different; what does not is the
//! geometry, because that is what a caller can get in the wrong order.
//!
//! Two elements are still written as literals, both because they carry another
//! element inside them rather than being a shape: the grid `<pattern>` in
//! [`crate::to_svg_with`], and the `<tspan>` a namespaced node title is built
//! from. A method for either would put a fact about what CrabER draws into the
//! thing that only knows how to write it down.

/// An indented line buffer. Indentation is cosmetic, but an SVG people paste
/// into a README and open in an editor is worth keeping readable.
pub(crate) struct Xml {
    buffer: String,
    depth: usize,
}

impl Xml {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            depth: 0,
        }
    }

    /// A line of the document, as written. Prefer the element methods below;
    /// this is for the one-off literal that would not be clearer as a method.
    pub fn line(&mut self, content: &str) {
        for _ in 0..self.depth {
            self.buffer.push_str("  ");
        }
        self.buffer.push_str(content);
        self.buffer.push('\n');
    }

    pub fn indent(&mut self) {
        self.depth += 1;
    }

    pub fn dedent(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    pub fn finish(self) -> String {
        self.buffer
    }

    /// Open an element and indent what follows. `attributes` may be empty.
    pub fn open(&mut self, tag: &str, attributes: &str) {
        if attributes.is_empty() {
            self.line(&format!("<{tag}>"));
        } else {
            self.line(&format!("<{tag} {attributes}>"));
        }
        self.indent();
    }

    /// Close the element [`open`](Self::open) opened, and dedent.
    pub fn close(&mut self, tag: &str) {
        self.dedent();
        self.line(&format!("</{tag}>"));
    }

    /// A `<rect>` at a point, with a size.
    ///
    /// Four coordinates in a fixed order, which is exactly what a caller
    /// passing them positionally can get wrong — the diagram background used to
    /// pass `bounds.x` and `bounds.width` twice each in one `format!`.
    pub fn rect(&mut self, at: (f32, f32), size: (f32, f32), attributes: &str) {
        self.line(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" {attributes}/>"#,
            at.0, at.1, size.0, size.1
        ));
    }

    /// A `<circle>` at a point.
    pub fn circle(&mut self, at: (f32, f32), radius: f32, attributes: &str) {
        self.line(&format!(
            r#"<circle cx="{:.1}" cy="{:.1}" r="{radius}" {attributes}/>"#,
            at.0, at.1
        ));
    }

    /// A `<path>` with the given `d`.
    ///
    /// `d` is built by the caller because its shape is the drawing's business —
    /// a rounded header cap and a bezier between two tables have nothing in
    /// common but being paths.
    pub fn path(&mut self, d: &str, attributes: &str) {
        self.line(&format!(r#"<path d="{d}" {attributes}/>"#));
    }

    /// A `<line>` element between two points.
    ///
    /// Three places draw one — the rule under a node's header, the crow's foot,
    /// the "one" tick — and each had written out the same four coordinate
    /// attributes. `attributes` is the rest: what makes each of them different.
    pub fn draw_line(&mut self, from: (f32, f32), to: (f32, f32), attributes: &str) {
        self.line(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" {attributes}/>"#,
            from.0, from.1, to.0, to.1
        ));
    }

    /// A `<text>` element whose `y` is its middle rather than its baseline.
    ///
    /// `dominant-baseline="central"` is on every piece of text a node draws —
    /// each one is centred on a row — so it belongs here rather than in four
    /// call sites that have to remember it.
    ///
    /// `markup` goes in as written: a caller passing text from a schema escapes
    /// it with [`escape`], and the node title passes a `<tspan>` it has built
    /// out of escaped parts.
    pub fn draw_text(&mut self, at: (f32, f32), attributes: &str, markup: &str) {
        self.line(&format!(
            r#"<text x="{:.1}" y="{:.1}" dominant-baseline="central" {attributes}>{markup}</text>"#,
            at.0, at.1
        ));
    }
}

/// Escape text for an XML text node or a double-quoted attribute.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(character),
        }
    }
    out
}

/// How wide a character is as a fraction of the font size, for the monospace
/// face column names and types are set in.
///
/// [`text_width`] and [`fits`] are inverses of each other, so a ratio written
/// out twice is two numbers that have to agree and nothing to make them.
const CHARACTER_WIDTH: f32 = 0.6;

/// Approximate width of monospace text. SVG has no text overflow of its own, so
/// the renderer has to decide where to cut — and for that an estimate is enough.
pub(crate) fn text_width(text: &str, font_size: f32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "identifier lengths are far below f32 precision limits"
    )]
    let characters = text.chars().count() as f32;
    characters * font_size * CHARACTER_WIDTH
}

/// How many characters fit in `width` at `font_size`.
pub(crate) fn fits(width: f32, font_size: f32) -> usize {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to a non-negative range that fits in usize"
    )]
    let characters = (width / (font_size * CHARACTER_WIDTH)).floor().max(0.0) as usize;
    characters
}

/// Cut `text` to `max` characters, marking the cut so a truncated identifier is
/// never mistaken for a real one.
pub(crate) fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    text.chars().take(max - 1).chain(['…']).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_the_characters_that_would_break_the_document() {
        assert_eq!(escape(r#"a<b>&"c'"#), "a&lt;b&gt;&amp;&quot;c&apos;");
    }

    #[test]
    fn truncation_marks_the_cut_and_respects_character_boundaries() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 5), "abcd…");
        // Counted in characters, not bytes, so multi-byte names survive.
        assert_eq!(truncate("ユーザー識別子", 4), "ユーザ…");
    }
}
