use neuro_types::{NeuroRuntimeError, NeuroRuntimeErrorCode};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum NeuroAdtErrorCode {
    InvalidArgument,
    RuntimeInitError,
    Unknown,
}

#[derive(Debug, Clone)]
pub(crate) struct NeuroAdtError {
    pub(crate) code: NeuroAdtErrorCode,
    pub(crate) message: String,
    pub(crate) details: Option<Value>,
    runtime_code: Option<NeuroRuntimeErrorCode>,
}

impl NeuroAdtError {
    pub(crate) fn new(
        code: NeuroAdtErrorCode,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details,
            runtime_code: Some(code.to_runtime_code()),
        }
    }

    pub(crate) fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(NeuroAdtErrorCode::InvalidArgument, message, None)
    }

    pub(crate) fn runtime_init_error(message: impl Into<String>) -> Self {
        Self::new(NeuroAdtErrorCode::RuntimeInitError, message, None)
    }

    pub(crate) fn from_runtime_error(error: NeuroRuntimeError) -> Self {
        let NeuroRuntimeError {
            code,
            message,
            details,
        } = error;

        Self {
            code: NeuroAdtErrorCode::from_runtime_code(code),
            message,
            details,
            runtime_code: Some(code),
        }
    }

    pub(crate) fn runtime_code(&self) -> NeuroRuntimeErrorCode {
        self.runtime_code
            .unwrap_or_else(|| self.code.to_runtime_code())
    }
}

impl NeuroAdtErrorCode {
    pub(crate) fn from_runtime_code(code: NeuroRuntimeErrorCode) -> Self {
        match code {
            NeuroRuntimeErrorCode::InvalidArgument => Self::InvalidArgument,
            NeuroRuntimeErrorCode::RuntimeInitError => Self::RuntimeInitError,
            _ => Self::Unknown,
        }
    }

    pub(crate) fn to_runtime_code(self) -> NeuroRuntimeErrorCode {
        match self {
            Self::InvalidArgument => NeuroRuntimeErrorCode::InvalidArgument,
            Self::RuntimeInitError => NeuroRuntimeErrorCode::RuntimeInitError,
            Self::Unknown => NeuroRuntimeErrorCode::Unknown,
        }
    }
}
