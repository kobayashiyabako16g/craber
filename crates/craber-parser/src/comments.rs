//! The comments of a workflow script, read as its descriptions.
//!
//! sqlparser drops comments, so they are read here from the token stream on
//! their own and matched to statements by position. A comment written directly
//! above a statement — no blank line between — describes that step. At the top
//! of the file, the comment that stands apart from the first statement describes
//! the whole workflow:
//!
//! ```sql
//! -- A checkout: the order is created and paid.   ← the workflow
//!
//! -- Who is buying.                                ← step 1
//! SELECT * FROM users;
//! ```
//!
//! A comment on the same line as the `;` before it is a note on that statement,
//! not the next one, and is dropped. Comments inside a statement are dropped too.

use sqlparser::dialect::Dialect;
use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};

/// The descriptions found in one script.
#[derive(Debug, Default)]
pub(crate) struct Comments {
    pub workflow: Option<String>,
    /// One per statement, in order. Empty when the statements could not be
    /// matched up, so no step gets another's description.
    steps: Vec<Option<String>>,
}

impl Comments {
    /// Read `sql`, which parsed into `statements` statements. A script that
    /// does not tokenize, or whose statements do not line up with the parse,
    /// has no descriptions rather than wrong ones.
    pub fn read(sql: &str, dialect: &dyn Dialect, statements: usize) -> Self {
        let Ok(tokens) = Tokenizer::new(dialect, sql).tokenize() else {
            return Self::default();
        };
        let mut comments = Self::scan(&tokens);
        if comments.steps.len() != statements {
            comments.steps.clear();
        }
        comments
    }

    /// The description of the statement at `index`.
    pub fn step(&self, index: usize) -> Option<String> {
        self.steps.get(index).cloned().flatten()
    }

    fn scan(tokens: &[Token]) -> Self {
        let mut comments = Self::default();
        // Comment blocks seen since the last statement ended, split at blank lines.
        let mut blocks: Vec<Vec<String>> = Vec::new();
        let mut block: Vec<String> = Vec::new();
        // Line breaks since the last comment; two make a blank line.
        let mut newlines = 0;
        // Whether nothing but whitespace precedes this point on its line.
        let mut line_start = true;
        let mut in_statement = false;
        let mut first = true;

        for token in tokens {
            match token {
                Token::Whitespace(Whitespace::Newline) => {
                    newlines += 1;
                    line_start = true;
                }
                Token::Whitespace(Whitespace::Space | Whitespace::Tab) => {}
                Token::Whitespace(_) if in_statement || !line_start => {}
                Token::Whitespace(comment) => {
                    if newlines > 1 && !block.is_empty() {
                        blocks.push(std::mem::take(&mut block));
                    }
                    block.extend(lines(comment));
                    newlines = 0;
                    line_start = false;
                }
                Token::SemiColon => {
                    if in_statement {
                        in_statement = false;
                        blocks.clear();
                        block.clear();
                        newlines = 0;
                    }
                    line_start = false;
                }
                _ if in_statement => {}
                _ => {
                    // A statement starts. The block right above it is its own;
                    // one a blank line away is not.
                    let attached = !block.is_empty() && newlines <= 1;
                    if !attached && !block.is_empty() {
                        blocks.push(std::mem::take(&mut block));
                    }
                    if first {
                        let paragraphs: Vec<String> =
                            blocks.iter().filter_map(|b| text(b)).collect();
                        if !paragraphs.is_empty() {
                            comments.workflow = Some(paragraphs.join("\n\n"));
                        }
                        first = false;
                    }
                    comments
                        .steps
                        .push(if attached { text(&block) } else { None });
                    in_statement = true;
                }
            }
        }
        comments
    }
}

/// The lines of one comment, without its markers.
fn lines(comment: &Whitespace) -> Vec<String> {
    match comment {
        Whitespace::SingleLineComment { comment, .. } => {
            let line = comment.strip_prefix(' ').unwrap_or(comment);
            vec![line.trim_end().to_owned()]
        }
        Whitespace::MultiLineComment(body) => body
            .lines()
            .map(|line| {
                let line = line.trim();
                // The ` * ` that starts each line of a boxed block comment.
                let line = line.strip_prefix('*').unwrap_or(line);
                line.strip_prefix(' ').unwrap_or(line).to_owned()
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A block's lines as one description, or `None` when they are all blank.
fn text(lines: &[String]) -> Option<String> {
    let text = lines.join("\n");
    let text = text.trim_matches('\n');
    (!text.trim().is_empty()).then(|| text.to_owned())
}
