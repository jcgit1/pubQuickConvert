use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvertError {
    EmptyInput,
    InvalidNumber,
    InputTooLarge,
    InputTooSmall,
    Overflow,
    Underflow,
    IncompatibleUnits,
    DivisionByZero,
    UnknownUnit(String),
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "Enter a value to convert"),
            Self::InvalidNumber => write!(f, "Invalid number"),
            Self::InputTooLarge => write!(f, "Input too large"),
            Self::InputTooSmall => write!(f, "Input too small"),
            Self::Overflow => write!(f, "Result overflows"),
            Self::Underflow => write!(f, "Result underflows"),
            Self::IncompatibleUnits => write!(f, "These units cannot be converted"),
            Self::DivisionByZero => write!(f, "Cannot convert with zero offset"),
            Self::UnknownUnit(u) => write!(f, "Unknown unit: {u}"),
        }
    }
}

impl std::error::Error for ConvertError {}

pub type ConvertResult<T> = Result<T, ConvertError>;
