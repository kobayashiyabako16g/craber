//! The bundle's `index.html`, adapted to the schema being published.

/// Adapts the bundle's `index.html` to the schema being published.
///
/// The browser tab and any bookmark of it are all a reader has to tell two
/// published diagrams apart, so the schema name goes in the title. The bundle
/// itself cannot do this: it is built once and serves every schema.
pub(crate) fn index_html(html: &str, schema_name: &str, base_path: Option<&str>) -> String {
    let titled = retitle(html, schema_name);
    match base_path {
        Some(base) => with_base_path(&titled, base),
        None => titled,
    }
}

/// Replaces whatever is between `<title>` and `</title>`.
///
/// This used to match the literal `<title>CrabER</title>`, which meant editing
/// `web/index.html` silently turned the feature off — no error, just every
/// published diagram sharing one tab name. Scanning for the tags survives any
/// title the frontend cares to set.
fn retitle(html: &str, schema_name: &str) -> String {
    let Some(open) = html.find("<title>") else {
        return html.to_owned();
    };
    let start = open + "<title>".len();
    let Some(length) = html[start..].find("</title>") else {
        return html.to_owned();
    };
    let mut out = String::with_capacity(html.len() + schema_name.len());
    out.push_str(&html[..start]);
    out.push_str(&escape_html(schema_name));
    out.push_str(" — CrabER");
    out.push_str(&html[start + length..]);
    out
}

/// Points the bundle's relative asset references at an absolute prefix.
///
/// Only asset URLs move. `schema.json` is fetched relative to the document, so
/// it follows `index.html` wherever it is served from without being rewritten.
fn with_base_path(html: &str, base_path: &str) -> String {
    let trimmed = base_path.trim().trim_matches('/');
    let base = if trimmed.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}/", escape_html(trimmed))
    };
    // Every root-relative reference the bundle emits, not just the hashed
    // assets: the favicon is copied to the site root as-is.
    html.replace("./assets/", &format!("{base}assets/"))
        .replace("./favicon.svg", &format!("{base}favicon.svg"))
}

/// Text made safe to put in an HTML element or a quoted attribute.
///
/// The schema name is a file stem or a database name, and the base path is a
/// flag: neither is markup, and a published page must not run one as such.
fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"<title>CrabER</title>
<link rel="icon" href="./favicon.svg" />
<script type="module" crossorigin src="./assets/index-abc.js"></script>
<link rel="stylesheet" crossorigin href="./assets/index-abc.css">"#;
    #[test]
    fn the_schema_name_is_text_not_markup() {
        let html = index_html(INDEX, "a</title><script>x()</script>", None);
        assert!(!html.contains("<script>x()"), "{html}");
        assert!(html.contains("a&lt;/title&gt;&lt;script&gt;"), "{html}");
    }

    #[test]
    fn the_base_path_cannot_close_its_attribute() {
        let html = with_base_path(INDEX, r#"x" onload="y"#);
        assert!(!html.contains(r#"" onload=""#), "{html}");
    }

    #[test]
    fn base_path_is_normalized_to_one_leading_and_trailing_slash() {
        for base in ["/myrepo/", "myrepo", "/myrepo", "  myrepo/  "] {
            let html = with_base_path(INDEX, base);
            assert!(
                html.contains(r#"src="/myrepo/assets/index-abc.js""#),
                "base {base:?} produced {html}"
            );
            assert!(html.contains(r#"href="/myrepo/assets/index-abc.css""#));
        }
    }

    #[test]
    fn the_favicon_follows_the_base_path() {
        assert!(with_base_path(INDEX, "myrepo").contains(r#"href="/myrepo/favicon.svg""#));
    }

    #[test]
    fn the_title_names_the_schema_so_two_published_diagrams_differ() {
        let html = index_html(INDEX, "billing", None);
        assert!(html.contains("<title>billing — CrabER</title>"), "{html}");
        assert!(
            html.contains(r#"src="./assets/index-abc.js""#),
            "assets stay relative"
        );
    }

    /// The title rewrite must not depend on `web/index.html` saying exactly
    /// "CrabER" — that coupling used to break the feature silently.
    #[test]
    fn the_title_is_rewritten_whatever_it_said_before() {
        for before in [
            "<title>CrabER</title>",
            "<title>Something Else</title>",
            "<title></title>",
            "<title>CrabER — diagrams</title>",
        ] {
            let html = index_html(before, "billing", None);
            assert!(
                html.contains("<title>billing — CrabER</title>"),
                "{before} produced {html}"
            );
        }
    }

    /// An `index.html` with no title at all is passed through rather than
    /// mangled: better a missing tab name than a corrupted document.
    #[test]
    fn html_without_a_title_is_left_alone() {
        let html = index_html("<p>no title here</p>", "billing", None);
        assert_eq!(html, "<p>no title here</p>");
    }

    #[test]
    fn an_empty_base_path_serves_from_the_root() {
        assert!(with_base_path(INDEX, "/").contains(r#"src="/assets/index-abc.js""#));
    }
}
