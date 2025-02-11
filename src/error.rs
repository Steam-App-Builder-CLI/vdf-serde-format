use std;
use std::fmt::{self, Display};

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Debug, PartialEq)]
pub enum Error {
    // One or more variants that can be created by data structures through the
    // `ser::Error` and `de::Error` traits. For example the Serialize impl for
    // Mutex<T> might return an error because the mutex is poisoned, or the
    // Deserialize impl for a struct may return an error because a required
    // field is missing.
    Message(String),

    // Zero or more variants that can be created directly by the Serializer and
    // Deserializer without going through `ser::Error` and `de::Error`. These
    // are specific to the format, in this case JSON.
    Eof,
    Syntax,
    ExpectedBoolean,
    ExpectedInteger,
    ExpectedString,
    ExpectedNull,
    ExpectedArray,
    ExpectedMap,
    ExpectedMapKey,
    ExpectedMapEnd,
    ExpectedEnum,
    CannotSupportedTupleType,
    TrailingCharacters,

    ExpectedStringOrBlock,
    UnsupportedEnums,
    UnsupportedSelfDiscribing,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::Eof => formatter.write_str("unexpected end of input"),
            Error::Syntax => formatter.write_str("syntax error"),
            Error::ExpectedBoolean => formatter.write_str("expected boolean"),
            Error::ExpectedInteger => formatter.write_str("expected integer"),
            Error::ExpectedString => formatter.write_str("expected string"),
            Error::ExpectedNull => formatter.write_str("expected null"),
            Error::ExpectedArray => formatter.write_str("expected array"),
            Error::ExpectedMap => formatter.write_str("expected map"),
            Error::ExpectedMapEnd => formatter.write_str("expected map end"),
            Error::ExpectedEnum => formatter.write_str("expected enum"),
            Error::CannotSupportedTupleType => formatter.write_str("cannot supported tuple type, make a Tuple Struct instead, i.e. enum { Example { x: i32, y: i32 } } or "),
            Error::TrailingCharacters => formatter.write_str("trailing characters"),
            Error::ExpectedStringOrBlock => formatter.write_str("expected string or block"),
            Error::UnsupportedEnums => formatter.write_str("unsupported enums"),
            Error::UnsupportedSelfDiscribing => formatter.write_str("unsupported self-discribing [ANY]"),
            /* and so forth */
            #[allow(unreachable_patterns)]
            _ => formatter.write_str("unknown parse error"),
        }
    }
}

impl std::error::Error for Error {}
