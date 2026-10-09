use eros::{ErrorUnion, TypeSet};
use std::fmt;

/// An application error whose message is intended to be shown to users.
#[derive(Debug)]
pub struct UserError(String);

impl UserError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for UserError {}

pub fn user_message<E: TypeSet>(error: &ErrorUnion<E>) -> String {
    let mut messages = Vec::new();
    if let Some(error) = error.inner().as_any().downcast_ref::<UserError>() {
        messages.push(error.to_string());
    }
    messages.extend(
        error
            .contexts()
            .filter(|frame| frame.is_user_facing())
            .map(ToString::to_string),
    );
    if messages.is_empty() {
        "An unexpected error occurred.".to_owned()
    } else {
        messages.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_errors_and_internal_context_are_hidden() {
        let error = eros::error!("internal diagnostic")
            .context("internal operation")
            .user_context("Could not load the template.")
            .user_context("Check the template configuration.");
        assert_eq!(
            user_message(&error),
            "Could not load the template.\nCheck the template configuration."
        );
        assert_eq!(
            user_message(&eros::error!("internal diagnostic").context("internal operation")),
            "An unexpected error occurred."
        );
    }

    #[test]
    fn recognized_errors_include_only_user_context() {
        let error = eros::error!(UserError::new("Template not found."))
            .context("internal operation")
            .user_context("Run `stamp list` to see available templates.");
        assert_eq!(
            user_message(&error),
            "Template not found.\nRun `stamp list` to see available templates."
        );
    }
}
