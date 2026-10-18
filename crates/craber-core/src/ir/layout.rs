//! Where a table sits, and how much of it is showing.
//!
//! Part of the wire format like everything else in [`ir`](crate::ir), but a
//! different fact about a schema: [`schema`](super::schema) is what the database
//! said, and this is what CrabER decided about drawing it. The three types here
//! interlock — a [`Detail`] level picks one of a [`Positions`]' three
//! [`Position`]s, because a node's height depends on how many rows it shows and
//! so each level packs differently.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Declare the detail levels once: the variant, and the `Positions` field that
/// carries that level's coordinates.
///
/// The pairing used to be written three times — `Detail::level`,
/// `Positions::at` and `Positions::set` were the same three-arm match — while
/// `sugiyama/coords.rs` and `sugiyama/pack.rs` indexed `[f32; LEVELS]` arrays by
/// the integer those arms produced. Adding a level touched six places and the
/// compiler could not say which. Generating all of it from one list means the
/// order, the field names and the index are one statement.
///
/// Declaration order is load-bearing twice over: it is `ALL`'s order (shortest
/// node first), and it is what `level()` returns, because a fieldless enum's
/// discriminants are its positions.
macro_rules! detail_levels {
    (
        $( $(#[$meta:meta])* $variant:ident => $field:ident ),* $(,)?
    ) => {
        /// How much of a table the node shows, and so how tall it is.
        ///
        /// The frontend's `show` control switches between these. They are part of the
        /// wire format because the node's height decides the layout, and layout runs in
        /// Rust — the browser picks a level, it does not compute one.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
        #[serde(rename_all = "camelCase")]
        #[ts(export)]
        pub enum Detail {
            $( $(#[$meta])* $variant, )*
        }

        impl Detail {
            /// Every level, shortest node first.
            // The length is counted off the same list rather than written as a
            // number, so a new level cannot outgrow the array.
            pub const ALL: [Self; [$(stringify!($variant)),*].len()] =
                [$(Self::$variant),*];

            /// Index into [`Self::ALL`], for callers holding one array per level.
            ///
            /// The discriminant, which is the position in the list above — not a
            /// second mapping that could disagree with it.
            pub fn level(self) -> usize {
                self as usize
            }
        }

        /// Where a table sits, at each detail level.
        ///
        /// A node's width never changes, so the three positions share an `x` and differ
        /// only in `y`. They are written out in full anyway: a reader picks the one for
        /// the level it is drawing and needs to know nothing about how they relate.
        #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
        #[serde(rename_all = "camelCase")]
        #[ts(export)]
        pub struct Positions {
            $( pub $field: Position, )*
        }

        impl Positions {
            /// The same spot at every level — what a layout that ignores node
            /// height produces.
            pub fn uniform(position: Position) -> Self {
                Self { $( $field: position, )* }
            }

            pub fn at(&self, detail: Detail) -> Position {
                match detail {
                    $( Detail::$variant => self.$field, )*
                }
            }

            pub fn set(&mut self, detail: Detail, position: Position) {
                match detail {
                    $( Detail::$variant => self.$field = position, )*
                }
            }
        }
    };
}

detail_levels! {
    /// The name alone.
    TableName => table_name,
    /// The columns an edge can attach to: primary and foreign keys.
    KeyOnly => key_only,
    /// Every column, with its type.
    AllFields => all_fields,
}

impl Detail {
    /// The level the CLI's own renderers draw, and the tallest node.
    pub const FULL: Self = Self::AllFields;
}

/// Top-left corner of a table node on the canvas, assigned by `craber-layout`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use super::{Detail, Position, Positions};

    /// `level()` is the index into `ALL`, and three places rely on that:
    /// `coords.rs`, `pack.rs` and `mod.rs` all hold one array per level.
    #[test]
    fn level_is_the_index_into_all() {
        for (index, detail) in Detail::ALL.into_iter().enumerate() {
            assert_eq!(detail.level(), index, "{detail:?}");
        }
    }

    /// `at` and `set` are generated from one list, so this checks the list
    /// pairs each level with its own field — the failure it cannot rule out by
    /// construction is two levels pointing at the same one.
    #[test]
    fn each_level_keeps_its_own_position() {
        // Exactly representable and distinct, so this is a wiring check rather
        // than a question about float arithmetic.
        const MARKS: [f32; Detail::ALL.len()] = [16.0, 32.0, 64.0];

        let mut positions = Positions::uniform(Position::new(0.0, 0.0));
        for (index, detail) in Detail::ALL.into_iter().enumerate() {
            positions.set(detail, Position::new(1.0, MARKS[index]));
        }
        for (index, detail) in Detail::ALL.into_iter().enumerate() {
            assert_eq!(
                positions.at(detail),
                Position::new(1.0, MARKS[index]),
                "{detail:?} does not round trip through its own field"
            );
        }
    }
}
