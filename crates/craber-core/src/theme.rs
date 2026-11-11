//! Colours — the single definition every CrabER surface draws from.
//!
//! The definition lives here and `web/src/generated/tokens.css` is written
//! from it by a test, the same way `web/src/types` is written from the IR by
//! ts-rs, so the stylesheet never states a colour of its own.
//!
//! Node sizes are next door in [`geometry`](crate::geometry); the code that
//! writes the stylesheet is in `codegen`, next door.

/// Declare a palette: each field paired with the custom property that carries
/// it into the browser.
///
/// The pairing used to be written twice — once as a struct field, once as a
/// line in a hand-maintained `tokens()` — and the doc comment on this module
/// had to warn people to update both. A token missing from the list is a token
/// the browser does not have, which is a colour that silently falls back, so
/// the list is generated from the fields instead of kept alongside them.
macro_rules! palette {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$field_meta:meta])* $field:ident => $token:literal ),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            $( $(#[$field_meta])* pub $field: &'static str, )*
        }

        impl $name {
            /// Each colour paired with the custom property that carries it into
            /// the web app.
            #[must_use]
            pub fn tokens(&self) -> Vec<(&'static str, &'static str)> {
                vec![ $( ($token, self.$field), )* ]
            }
        }
    };
}

palette! {
    /// The colours a diagram is made of.
    ///
    /// Interface tokens (hover states, button text) live in [`Chrome`] instead,
    /// so that this type stays an honest list of what a diagram is made of.
    Theme {
        background => "--craber-bg",
        grid => "--craber-grid",
        surface => "--craber-surface",
        surface_raised => "--craber-surface-raised",
        border => "--craber-border",
        text => "--craber-text",
        text_muted => "--craber-text-muted",
        accent => "--craber-accent",
        /// The marker on a foreign-key column. Distinct from [`Self::accent`],
        /// which marks the primary key.
        glyph_fk => "--craber-glyph-fk",
        edge => "--craber-edge",
        font => "--craber-font",
        font_mono => "--craber-font-mono",
    }
}

impl Theme {
    /// CrabER's dark theme — the default everywhere, per the plan's UI section.
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            background: "#16181d",
            grid: "#2a2e37",
            surface: "#1e2128",
            surface_raised: "#262a33",
            border: "#333844",
            text: "#e6e8ec",
            text_muted: "#8b93a3",
            accent: "#e8542f",
            glyph_fk: "#d99a5b",
            edge: "#5b6474",
            font: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
            font_mono: "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace",
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

palette! {
    /// Interface colours the diagram itself never uses: hover feedback, the
    /// text on an accent-filled button, and what a table's relationships light
    /// up in when it is pointed at or selected.
    ///
    /// Only the web app reads these. They are defined here anyway so that the
    /// stylesheet has exactly one source rather than one and a half.
    Chrome {
        accent_dim => "--craber-accent-dim",
        accent_text => "--craber-accent-text",
        surface_hover => "--craber-surface-hover",
        highlight => "--craber-highlight",
        /// A table a workflow step reads from.
        flow_read => "--craber-flow-read",
        /// A table a workflow step inserts into or updates.
        flow_write => "--craber-flow-write",
        /// A table a workflow step deletes from.
        flow_delete => "--craber-flow-delete",
    }
}

impl Chrome {
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            accent_dim: "#8a3520",
            accent_text: "#1a0d09",
            surface_hover: "#2d323d",
            // Redder than the accent: the accent already marks every primary
            // key and the brand, so a lit relationship has to be told apart
            // from those at a glance.
            highlight: "#f0434f",
            // Read, write, delete: cool to hot, and none of them the
            // highlight red, which a step's joins are lit in.
            flow_read: "#4f9cf0",
            flow_write: "#3fbf7f",
            flow_delete: "#e0a030",
        }
    }
}

impl Default for Chrome {
    fn default() -> Self {
        Self::dark()
    }
}
