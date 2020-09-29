use std::io;

#[derive(Debug)]
pub struct FactoryErrors(pub Vec<String>);

#[derive(Debug)]
pub struct ContextInitError(pub Vec<String>);

#[derive(Debug)]
pub enum OttersInitError {
    IOError(io::Error),
    SerdeError(serde_json::Error),
    UnitConfigError(FactoryErrors),
    ContextError(Vec<String>),
}

impl From<io::Error> for OttersInitError {
    fn from(e: io::Error) -> OttersInitError {
        OttersInitError::IOError(e)
    }
}

impl From<serde_json::Error> for OttersInitError {
    fn from(e: serde_json::Error) -> OttersInitError {
        OttersInitError::SerdeError(e)
    }
}

impl From<FactoryErrors> for OttersInitError {
    fn from(e: FactoryErrors) -> OttersInitError {
        OttersInitError::UnitConfigError(e)
    }
}

impl From<ContextInitError> for OttersInitError {
    fn from(e: ContextInitError) -> OttersInitError {
        OttersInitError::ContextError(e.0)
    }
}
