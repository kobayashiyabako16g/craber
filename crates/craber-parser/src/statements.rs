//! A script, read one statement at a time.
//!
//! `Parser::parse_sql` gives up on the whole script at the first statement it
//! cannot read, and a real dump always has one: `pg_dump` writes psql
//! meta-commands (`\restrict`), `ALTER ... OWNER TO` for every object, and
//! `CREATE SEQUENCE` options sqlparser-rs does not take. None of those draw
//! anything, so here a statement that does not parse is skipped and the rest of
//! the script is still read.
//!
//! The tokenizer runs over the whole script first, so a `;` inside a string or a
//! `$$` function body never ends a statement early.

use craber_core::{Diagnostic, Dialect, Outcome};
use sqlparser::ast::Statement;
use sqlparser::keywords::Keyword;
use sqlparser::parser::{Parser, ParserError};
use sqlparser::tokenizer::{Token, TokenWithSpan, Tokenizer};

use crate::dialect::sqlparser_dialect;

/// The statements of `sql` that parsed, in order.
///
/// A skipped statement that could have changed the diagram is a warning on
/// `outcome`; the rest are skipped quietly. Only a script that does not
/// tokenize, or of which not one statement parsed, is an error.
pub(crate) fn read<T>(sql: &str, dialect: Dialect, outcome: &mut Outcome<T>) -> Vec<Statement> {
    let parser_dialect = sqlparser_dialect(dialect);
    let fail = |outcome: &mut Outcome<T>, error: ParserError| {
        outcome.push(Diagnostic::error(format!(
            "could not parse SQL as {dialect}: {error}"
        )));
    };

    let tokens = match Tokenizer::new(parser_dialect.as_ref(), sql).tokenize_with_location() {
        Ok(tokens) => tokens,
        Err(error) => {
            fail(outcome, error.into());
            return Vec::new();
        }
    };
    let mut parser =
        Parser::new(parser_dialect.as_ref()).with_tokens_with_locations(tokens.clone());

    let mut statements = Vec::new();
    let mut first_error = None;
    loop {
        while parser.consume_token(&Token::SemiColon) {}
        if parser.peek_token_ref().token == Token::EOF {
            break;
        }
        let start = parser.index();
        let parsed = parser.parse_statement().and_then(|statement| {
            match parser.peek_token_ref().token {
                Token::SemiColon | Token::EOF => Ok(statement),
                // It parsed, but not up to its `;`: what follows is an option
                // sqlparser-rs does not know, and the part it did read is not
                // the whole statement.
                _ => parser.expected_ref("end of statement", parser.peek_token_ref()),
            }
        });
        let error = match parsed {
            Ok(statement) => {
                statements.push(statement);
                continue;
            }
            Err(error) => error,
        };

        let head = head(&tokens, start);
        if may_draw(&tokens[head..]) {
            let line = tokens[head].span.start.line;
            let text = usize::try_from(line)
                .ok()
                .and_then(|line| sql.lines().nth(line.checked_sub(1)?))
                .unwrap_or_default()
                .trim();
            outcome.push(
                Diagnostic::warning("statement could not be parsed and was skipped")
                    .with_context(format!("line {line}: {text}")),
            );
        }
        first_error.get_or_insert(error);
        seek(&mut parser, skip(&tokens, head));
    }

    if statements.is_empty()
        && let Some(error) = first_error
    {
        fail(outcome, error);
    }
    statements
}

/// The first token from `start` that is not whitespace or a comment.
fn head(tokens: &[TokenWithSpan], start: usize) -> usize {
    (start..tokens.len())
        .find(|&i| !matches!(tokens[i].token, Token::Whitespace(_)))
        .unwrap_or(tokens.len())
}

/// Where reading resumes after the statement starting at `head` failed.
fn skip(tokens: &[TokenWithSpan], head: usize) -> usize {
    match tokens.get(head) {
        // A psql meta-command such as `\restrict <key>` ends at its line break,
        // not at a `;`; running on to the next `;` would swallow a statement.
        Some(TokenWithSpan {
            token: Token::Backslash,
            span,
        }) => (head..tokens.len())
            .find(|&i| tokens[i].span.start.line > span.start.line)
            .unwrap_or(tokens.len()),
        _ => (head..tokens.len())
            .find(|&i| tokens[i].token == Token::SemiColon)
            .map_or(tokens.len(), |i| i + 1),
    }
}

/// Move `parser` to token `target`. A failed parse leaves it anywhere in the
/// statement, possibly past the `;`, so this goes either way.
fn seek(parser: &mut Parser, target: usize) {
    while parser.index() > target {
        parser.prev_token();
    }
    while parser.index() < target {
        parser.next_token_no_skip();
    }
}

/// Whether a statement starting with `tokens` is one `translate` would have
/// turned into part of the diagram: a table, an enum, an index, a comment, or a
/// change to a table. Keep in step with `translate::define` and
/// `translate::amend`; losing any other statement loses nothing visible, so it
/// is not worth a warning.
fn may_draw(tokens: &[TokenWithSpan]) -> bool {
    let mut words = tokens.iter().filter_map(|t| match &t.token {
        Token::Word(word) => Some(word.keyword),
        Token::Whitespace(_) => None,
        _ => Some(Keyword::NoKeyword),
    });
    match words.next() {
        Some(Keyword::CREATE) => words
            .take(4)
            .find(|k| {
                !matches!(
                    k,
                    Keyword::OR
                        | Keyword::REPLACE
                        | Keyword::TEMP
                        | Keyword::TEMPORARY
                        | Keyword::UNLOGGED
                        | Keyword::GLOBAL
                        | Keyword::LOCAL
                        | Keyword::UNIQUE
                )
            })
            .is_some_and(|k| matches!(k, Keyword::TABLE | Keyword::TYPE | Keyword::INDEX)),
        Some(Keyword::ALTER) => words.next() == Some(Keyword::TABLE),
        Some(Keyword::COMMENT) => words.next() == Some(Keyword::ON),
        _ => false,
    }
}
