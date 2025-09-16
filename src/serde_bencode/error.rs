#[derive(Debug)]
pub(crate) enum Error {
    Message(String),
    Eof,
    Syntax,
    ExpectedByteString,
    ExpectedInteger,
    ExpectedList,
    ExpectedMap,
    /// Byte strings have colon char after number
    ExpectedColon,
    /// Deserialisation hasn't processed all bytes in input data
    LeftoverData,
    ExpectedListEnd,
    ExpectedMapEnd,
}

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Message(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Message(msg.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Message(msg) => f.write_str(msg),
            Error::Eof => f.write_str("Encountered unexpected EOF"),
            Error::Syntax => f.write_str("Invalid syntax"),
            Error::ExpectedMap => f.write_str("Expected map value"),
            Error::ExpectedList => f.write_str("Expected list value"),
            Error::ExpectedInteger => f.write_str("Expected integer value"),
            Error::ExpectedByteString => f.write_str("Expected byte string value"),
            Error::ExpectedColon => f.write_str("Expected colon character"),
            Error::LeftoverData => f.write_str("Expected no data leftover"),
            Error::ExpectedListEnd => f.write_str("Expected end of list"),
            Error::ExpectedMapEnd => f.write_str("Expected end of map"),
        }
    }
}

impl std::error::Error for Error {}
