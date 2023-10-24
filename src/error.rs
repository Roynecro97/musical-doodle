use std::fmt::Display;
use std::sync::mpsc::RecvError;

use crate::common::WSMsg;

#[derive(Debug, thiserror::Error)]
pub enum DoodleError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    MpscRecvError(RecvError),
    NoOpen(WSMsg),
    SocketError(ws::Error),
    UnexpectedResponse(WSMsg),
    UrlError(url::ParseError),
    // FailureResponse(common::Error),  // TODO: add error
    // FaultStatus,
    // FailedStatus,
    Generic(String),
}

impl Display for DoodleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl From<std::io::Error> for DoodleError {
    fn from(v: std::io::Error) -> Self {
        Self::IoError(v)
    }
}

impl From<serde_json::Error> for DoodleError {
    fn from(v: serde_json::Error) -> Self {
        Self::JsonError(v)
    }
}

impl From<RecvError> for DoodleError {
    fn from(v: RecvError) -> Self {
        Self::MpscRecvError(v)
    }
}

impl From<ws::Error> for DoodleError {
    fn from(v: ws::Error) -> Self {
        Self::SocketError(v)
    }
}

impl From<url::ParseError> for DoodleError {
    fn from(v: url::ParseError) -> Self {
        Self::UrlError(v)
    }
}

pub trait AsDoodleErrorResult {
    type OkType;

    fn as_doodle_result(self) -> core::result::Result<Self::OkType, DoodleError>;
}

pub trait AsEyreErrorResult {
    type OkType;

    fn as_eyre_result(self) -> color_eyre::eyre::Result<Self::OkType>;
}

impl<T, E: Into<DoodleError>> AsDoodleErrorResult for Result<T, E> {
    type OkType = T;

    fn as_doodle_result(self) -> core::result::Result<Self::OkType, DoodleError> {
        self.map_err(Into::into)
    }
}

impl<T, E: Into<DoodleError>> AsEyreErrorResult for Result<T, E> {
    type OkType = T;

    fn as_eyre_result(self) -> color_eyre::eyre::Result<Self::OkType> {
        Ok(self.as_doodle_result()?)
    }
}
