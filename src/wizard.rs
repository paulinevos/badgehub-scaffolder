use std::fmt::Display;
use std::io::{IsTerminal, stdin};

use anyhow::{Context, Result, bail};
use inquire::{Confirm, MultiSelect, Password, Select, Text as TextPrompt};

/// Asks only for what the flags left unanswered. When there is no terminal —
/// a CI run, a pipe — an unanswered field is an error naming the flag that
/// would have supplied it, rather than a prompt nobody can see.
pub struct Wizard {
    interactive: bool,
}

impl Wizard {
    pub fn on_this_terminal() -> Self {
        Self {
            interactive: stdin().is_terminal(),
        }
    }

    pub fn text(&self, label: &str, flag: &str, given: Option<String>) -> Result<String> {
        if let Some(answer) = given {
            return Ok(answer);
        }
        self.refuse_when_silent(flag)?;
        TextPrompt::new(label)
            .prompt()
            .with_context(|| format!("asking for {label}"))
    }

    pub fn optional_text(&self, label: &str, given: Option<String>) -> Result<Option<String>> {
        if given.is_some() || !self.interactive {
            return Ok(given);
        }
        let answer = TextPrompt::new(label)
            .with_help_message("optional — leave blank to skip")
            .prompt()
            .with_context(|| format!("asking for {label}"))?;
        Ok(Some(answer).filter(|value| !value.trim().is_empty()))
    }

    /// Prefilled with what is already known, so pressing enter keeps it. The
    /// plain `text` cannot do this: a value given there is taken as the answer.
    pub fn text_defaulting_to(&self, label: &str, default: &str) -> Result<String> {
        if !self.interactive {
            return Ok(default.to_owned());
        }
        TextPrompt::new(label)
            .with_default(default)
            .prompt()
            .with_context(|| format!("asking for {label}"))
    }

    pub fn optional_text_defaulting_to(
        &self,
        label: &str,
        current: Option<String>,
    ) -> Result<Option<String>> {
        if !self.interactive {
            return Ok(current);
        }
        let answer = TextPrompt::new(label)
            .with_default(current.as_deref().unwrap_or(""))
            .with_help_message("optional — leave blank to skip")
            .prompt()
            .with_context(|| format!("asking for {label}"))?;
        Ok(Some(answer).filter(|value| !value.trim().is_empty()))
    }

    /// Never echoed, and never carries a default: a secret shown back as a
    /// prefilled answer is a secret on someone's screen.
    pub fn secret(&self, label: &str) -> Result<Option<String>> {
        if !self.interactive {
            return Ok(None);
        }
        let answer = Password::new(label)
            .without_confirmation()
            .with_help_message("leave blank to keep what is already there")
            .prompt()
            .with_context(|| format!("asking for {label}"))?;
        Ok(Some(answer).filter(|value| !value.trim().is_empty()))
    }

    /// For a secret there is no sensible default for and no way to continue
    /// without: a blank answer is refused rather than quietly meaning nothing.
    pub fn secret_or_refuse(&self, label: &str, complaint: &str) -> Result<String> {
        let Some(answer) = self.secret(label)? else {
            bail!("{complaint}");
        };
        Ok(answer)
    }

    pub fn confirm(&self, label: &str) -> Result<bool> {
        if !self.interactive {
            return Ok(false);
        }
        Confirm::new(label)
            .with_default(false)
            .prompt()
            .with_context(|| format!("asking for {label}"))
    }

    pub fn choose<T: Display>(
        &self,
        label: &str,
        flag: &str,
        options: Vec<T>,
        given: Option<T>,
    ) -> Result<T> {
        if let Some(answer) = given {
            return Ok(answer);
        }
        self.refuse_when_silent(flag)?;
        Select::new(label, options)
            .prompt()
            .with_context(|| format!("asking for {label}"))
    }

    /// For a question that is fair to skip entirely when nobody is watching.
    pub fn choose_optional<T: Display>(&self, label: &str, options: Vec<T>) -> Result<Option<T>> {
        if !self.interactive {
            return Ok(None);
        }
        Select::new(label, options)
            .prompt()
            .map(Some)
            .with_context(|| format!("asking for {label}"))
    }

    pub fn choose_several(
        &self,
        label: &str,
        flag: &str,
        options: Vec<String>,
        given: Vec<String>,
    ) -> Result<Vec<String>> {
        if !given.is_empty() {
            return Ok(given);
        }
        self.refuse_when_silent(flag)?;
        MultiSelect::new(label, options)
            .prompt()
            .with_context(|| format!("asking for {label}"))
    }

    pub fn refuse_when_silent(&self, flag: &str) -> Result<()> {
        if !self.interactive {
            bail!("no terminal to ask on: pass {flag}");
        }
        Ok(())
    }
}
