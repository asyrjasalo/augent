//! Git error handling
//!
//! This module handles:
//! - Interpreting git2 errors into user-friendly messages
//! - Categorizing errors by type (not found, auth, network, etc.)

use git2::{Error, ErrorClass};

/// Internal enum for error type classification
#[derive(Clone, Copy)]
enum ErrorClassOrMessage {
    RepositoryNotFound,
    AuthenticationFailed,
    PermissionDenied,
    NetworkError,
    HttpCertificate,
    HttpSsl,
    Other(ErrorClass),
}

fn classify_error_type(msg: &str, class: ErrorClass) -> ErrorClassOrMessage {
    for (check, result) in ERROR_CLASSIFICATIONS {
        if check(msg, class) {
            return *result;
        }
    }
    ErrorClassOrMessage::Other(class)
}

type ErrorCheck = fn(&str, ErrorClass) -> bool;

const ERROR_CLASSIFICATIONS: &[(ErrorCheck, ErrorClassOrMessage)] = &[
    (
        |msg, _| {
            msg.contains("not found")
                || msg.contains("404")
                || msg.contains("too many redirects")
                || msg.contains("authentication replays")
        },
        ErrorClassOrMessage::RepositoryNotFound,
    ),
    (
        |msg, _| msg.contains("authentication") || msg.contains("credentials"),
        ErrorClassOrMessage::AuthenticationFailed,
    ),
    (
        |msg, _| msg.contains("permission denied") || msg.contains("access denied"),
        ErrorClassOrMessage::PermissionDenied,
    ),
    (
        |msg, _| {
            msg.contains("connection")
                || msg.contains("network")
                || msg.contains("timeout")
                || msg.contains("timed out")
        },
        ErrorClassOrMessage::NetworkError,
    ),
    (
        |msg, class| class == ErrorClass::Http && msg.contains("certificate"),
        ErrorClassOrMessage::HttpCertificate,
    ),
    (
        |msg, class| class == ErrorClass::Http && msg.contains("ssl"),
        ErrorClassOrMessage::HttpSsl,
    ),
];

/// Interpret a git2 error and provide a more user-friendly message
pub fn interpret_git_error(err: &Error) -> String {
    let message = err.message().to_lowercase();

    // Classify the error
    let error_type = classify_error_type(message.as_str(), err.class());

    match error_type {
        ErrorClassOrMessage::RepositoryNotFound => "Repository not found".to_string(),
        ErrorClassOrMessage::AuthenticationFailed => "Authentication failed".to_string(),
        ErrorClassOrMessage::PermissionDenied => "Permission denied".to_string(),
        ErrorClassOrMessage::NetworkError => "Network error".to_string(),
        ErrorClassOrMessage::HttpCertificate => "Certificate error".to_string(),
        ErrorClassOrMessage::HttpSsl => "SSL error".to_string(),
        ErrorClassOrMessage::Other(class) => {
            // For HTTP and SSH errors, provide the original message with class name
            // For all other errors, use the original message
            match class {
                ErrorClass::Http | ErrorClass::Ssh => {
                    format!("{} error: {}", error_class_name(class), err.message())
                }
                _ => err.message().to_string(),
            }
        }
    }
}

/// Get display name for error class
fn error_class_name(class: ErrorClass) -> &'static str {
    match class {
        ErrorClass::Http => "HTTP",
        ErrorClass::Ssh => "SSH",
        _ => "Unknown",
    }
}
