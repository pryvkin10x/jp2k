#[derive(Debug)]
pub enum Error {
    NulError(std::ffi::NulError),
    Io(std::io::Error),
    Boxed(Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    pub fn boxed<E: Into<Box<dyn std::error::Error + 'static + Send + Sync>>>(e: E) -> Self {
        Error::Boxed(e.into())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            NulError(ref e) => {
                write!(f, "{}", e)?;
            }
            Io(ref e) => {
                write!(f, "{}", e)?;
            }
            Boxed(ref e) => {
                write!(f, "{}", e)?;
            }
        }

        Ok(())
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(t: std::ffi::NulError) -> Self {
        Error::NulError(t)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
