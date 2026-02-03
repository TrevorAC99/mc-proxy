pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    Io(std::io::Error),
    Other(&'static str),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<&'static str> for Error {
    fn from(msg: &'static str) -> Self {
        Error::Other(msg)
    }
}
