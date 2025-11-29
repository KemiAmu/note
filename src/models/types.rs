use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use redb::Database;
use std::sync::Arc;

use crate::config::CONFIG;

pub type AppState = State<Arc<Database>>;

pub type Result<T> = std::result::Result<T, Ex>;

#[derive(Debug)]
pub enum Ex {
    InvalidUsername,
    InvalidFilename,
    InvalidTimestamp,
    FileExists,
    UserExists,
    UserNotFound,
    InvalidCredentials,
    PageNotFound,
    PageAlreadyExists,
    PermissionDenied,
    InvalidInvite,
    CannotInviteSelf,
    DatabaseError,
    DatabaseTableError,
    DatabaseCommitError,
    DatabaseStorageError,
    DatabaseTransactionError,
    DataEncodingError,
    TemplateRenderingError,
    InternalServerError,
}

impl From<()> for Ex {
    fn from(_: ()) -> Self {
        Ex::InternalServerError
    }
}

impl From<redb::Error> for Ex {
    fn from(_: redb::Error) -> Self {
        Ex::DatabaseError
    }
}

impl From<redb::TableError> for Ex {
    fn from(_: redb::TableError) -> Self {
        Ex::DatabaseTableError
    }
}

impl From<redb::CommitError> for Ex {
    fn from(_: redb::CommitError) -> Self {
        Ex::DatabaseCommitError
    }
}

impl From<redb::StorageError> for Ex {
    fn from(_: redb::StorageError) -> Self {
        Ex::DatabaseStorageError
    }
}

impl From<redb::TransactionError> for Ex {
    fn from(_: redb::TransactionError) -> Self {
        Ex::DatabaseTransactionError
    }
}

impl From<askama::Error> for Ex {
    fn from(_: askama::Error) -> Self {
        Ex::TemplateRenderingError
    }
}

impl IntoResponse for Ex {
    fn into_response(self) -> Response {
        #[derive(Template)]
        #[template(path = "error.html")]
        struct Page<'a> {
            base_url: &'a str,
            site_title: &'a str,
            title: &'a str,
            message: &'a str,
        }

        let (status_code, title, message) = match self {
            Ex::InvalidUsername => (
                StatusCode::BAD_REQUEST,
                "Invalid Username",
                "The username you entered does not meet the required format. Usernames must follow specific character and length requirements.",
            ),
            Ex::InvalidFilename => (
                StatusCode::BAD_REQUEST,
                "Invalid Filename",
                "The filename you provided contains invalid characters or is too long. Please use a different filename that meets the system requirements.",
            ),
            Ex::InvalidTimestamp => (
                StatusCode::BAD_REQUEST,
                "Invalid Timestamp",
                "The timestamp format is incorrect. Please ensure it follows the expected format and represents a valid date/time.",
            ),
            Ex::FileExists => (
                StatusCode::CONFLICT,
                "File Exists",
                "A file with this name already exists in the system. Please choose a different filename or delete the existing file first.",
            ),
            Ex::UserExists => (
                StatusCode::CONFLICT,
                "User Exists",
                "This username is already registered in the system. Please choose a different username or try to recover your existing account.",
            ),
            Ex::UserNotFound => (
                StatusCode::NOT_FOUND,
                "User Not Found",
                "No user account was found with the provided credentials. Please check your username and try again, or contact support if you believe this is an error.",
            ),
            Ex::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "Invalid Credentials",
                "The username or password you entered is incorrect. Please verify your credentials and try again. If you've forgotten your password, please use the password recovery option.",
            ),
            Ex::PageNotFound => (
                StatusCode::NOT_FOUND,
                "Page Not Found",
                "The page you are looking for does not exist. Please check the URL for typos or navigate back to the homepage.",
            ),
            Ex::PageAlreadyExists => (
                StatusCode::CONFLICT,
                "Page Already Exists",
                "A page with this name already exists. Please choose a different name or edit the existing page.",
            ),
            Ex::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "Permission Denied",
                "You do not have the necessary permissions to access this resource. Please contact your administrator if you believe you should have access.",
            ),
            Ex::InvalidInvite => (
                StatusCode::UNAUTHORIZED,
                "Invalid Invite",
                "The invite code you provided is invalid or has expired. Please request a new invite code from the system administrator.",
            ),
            Ex::CannotInviteSelf => (
                StatusCode::BAD_REQUEST,
                "Cannot Invite Self",
                "You cannot send an invitation to yourself. Please provide a different email address or username to invite.",
            ),
            Ex::DatabaseError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database Error",
                "An unexpected database error occurred. Our technical team has been notified and is working to resolve the issue. Please try again later.",
            ),
            Ex::DatabaseTableError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database Table Error",
                "There was an error accessing a database table. This is likely a temporary issue. Please try again in a few moments.",
            ),
            Ex::DatabaseCommitError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database Commit Error",
                "The database was unable to commit your changes. This could be due to a temporary system issue. Please try your operation again.",
            ),
            Ex::DatabaseStorageError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database Storage Error",
                "A storage error occurred in the database system. This may be due to disk space issues or hardware problems. Our team has been alerted.",
            ),
            Ex::DatabaseTransactionError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database Transaction Error",
                "A database transaction failed to complete properly. This could be due to conflicting operations or system constraints. Please try again.",
            ),
            Ex::DataEncodingError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Data Encoding Error",
                "There was an error encoding or decoding data. This is a system issue that our technical team will investigate.",
            ),
            Ex::TemplateRenderingError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Template Error",
                "The system encountered an error while rendering the page template. This is likely a temporary issue. Please refresh the page or try again later.",
            ),
            Ex::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server Error",
                "An unexpected internal server error occurred. Our technical team has been notified and is working to resolve the issue. We apologize for the inconvenience.",
            ),
        };

        let page = Page {
            base_url: CONFIG.base_url,
            site_title: CONFIG.site_title,
            title: title,
            message,
        };
        (status_code, Html(page.render().unwrap())).into_response()
    }
}
