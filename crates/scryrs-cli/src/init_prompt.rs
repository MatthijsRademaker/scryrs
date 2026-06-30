use std::io::IsTerminal;

use dialoguer::{Confirm, Input, theme::ColorfulTheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalState {
    pub(crate) stdin_is_terminal: bool,
    pub(crate) stdout_is_terminal: bool,
}

impl TerminalState {
    pub(crate) fn detect() -> Self {
        Self {
            stdin_is_terminal: std::io::stdin().is_terminal(),
            stdout_is_terminal: std::io::stdout().is_terminal(),
        }
    }

    pub(crate) fn allows_prompting(self) -> bool {
        self.stdin_is_terminal && self.stdout_is_terminal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PromptSpec {
    pub(crate) field_name: &'static str,
    pub(crate) prompt: &'static str,
}

#[derive(Debug)]
pub(crate) enum InitPromptError {
    Cancelled,
    Io(std::io::Error),
}

impl std::fmt::Display for InitPromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitPromptError::Cancelled => write!(f, "cancelled"),
            InitPromptError::Io(error) => error.fmt(f),
        }
    }
}

impl From<std::io::Error> for InitPromptError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::Interrupted {
            Self::Cancelled
        } else {
            Self::Io(error)
        }
    }
}

impl From<dialoguer::Error> for InitPromptError {
    fn from(error: dialoguer::Error) -> Self {
        match error {
            dialoguer::Error::IO(io_error) => io_error.into(),
        }
    }
}

pub(crate) trait InitPrompt {
    fn prompt_text(&mut self, spec: &PromptSpec) -> Result<String, InitPromptError>;

    fn confirm(&mut self, prompt: &str, default: bool) -> Result<bool, InitPromptError>;
}

#[derive(Default)]
pub(crate) struct DialoguerInitPrompt {
    theme: ColorfulTheme,
}

impl InitPrompt for DialoguerInitPrompt {
    fn prompt_text(&mut self, spec: &PromptSpec) -> Result<String, InitPromptError> {
        Input::with_theme(&self.theme)
            .with_prompt(spec.prompt)
            .allow_empty(true)
            .interact_text()
            .map_err(Into::into)
    }

    fn confirm(&mut self, prompt: &str, default: bool) -> Result<bool, InitPromptError> {
        Confirm::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(default)
            .interact()
            .map_err(Into::into)
    }
}
