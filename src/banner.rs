//! The BadgeHub wordmark, shown once at the top of `bh new`.
//!
//! The logo beside it is deliberately not drawn. It is a name badge with a
//! lanyard hole, a nested screen, a shield, two circuit nodes and a download
//! arrow — four ideas stacked on top of each other. In the dozen rows a
//! terminal can spare it collapses into a blob resembling none of them, and a
//! logo the store would not recognise is worse than no logo at all.

use std::io::{IsTerminal, Write, stderr};

use figleter::FIGfont;
use terminal_size::{Width, terminal_size};

/// Block letters kept alongside the number of columns they need, so that
/// whether they can be shown is settled before anything reaches the terminal.
pub struct Wordmark {
    lines: String,
    columns: usize,
}

impl Wordmark {
    /// A font that will not parse gives a wordmark that shows nothing: a
    /// banner is decoration, not something to fail a scaffold over.
    pub fn of(word: &str) -> Self {
        let lines = rendered(word).unwrap_or_default();
        Self {
            columns: widest_line(&lines),
            lines,
        }
    }

    /// Written to stderr, so that whatever is done with the stdout of `bh new`
    /// gets the scaffolding report and nothing else.
    pub fn greet(&self) {
        if !self.showable() {
            return;
        }
        let _ = writeln!(stderr(), "{}", self.lines.trim_end());
    }

    /// Nothing to show, nobody watching this stream, or too little room to
    /// show it in without wrapping into nonsense.
    fn showable(&self) -> bool {
        !self.lines.is_empty()
            && stderr().is_terminal()
            && TerminalWidth::measured().accommodates(self.columns)
    }
}

/// The columns there are to draw into. A terminal that will not say how wide
/// it is — a pipe, a dumb TERM — is taken to be the conventional eighty.
struct TerminalWidth(usize);

impl TerminalWidth {
    const ASSUMED: usize = 80;

    fn measured() -> Self {
        let Some((Width(columns), _)) = terminal_size() else {
            return Self(Self::ASSUMED);
        };
        Self(usize::from(columns))
    }

    fn accommodates(&self, columns: usize) -> bool {
        columns <= self.0
    }
}

fn rendered(word: &str) -> Option<String> {
    Some(FIGfont::standard().ok()?.convert(word)?.to_string())
}

fn widest_line(lines: &str) -> usize {
    lines
        .lines()
        .map(|line| line.trim_end().chars().count())
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wordmark_is_drawn_in_block_letters() {
        let wordmark = Wordmark::of("BadgeHub");
        assert!(wordmark.lines.lines().count() > 3);
        assert!(!wordmark.lines.contains("BadgeHub"));
    }

    #[test]
    fn the_wordmark_stays_inside_eighty_columns() {
        assert!(
            TerminalWidth(TerminalWidth::ASSUMED).accommodates(Wordmark::of("BadgeHub").columns)
        );
    }

    #[test]
    fn a_wordmark_wider_than_the_terminal_is_refused() {
        assert!(!TerminalWidth(20).accommodates(Wordmark::of("BadgeHub").columns));
    }

    #[test]
    fn an_empty_word_shows_nothing() {
        assert!(!Wordmark::of("").showable());
    }
}
