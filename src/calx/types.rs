use bincode::{Decode, Encode};
use std::str::FromStr;

/// syntax like `(i64 -> i64)` can be used to types of functions and blocks
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Decode, Encode)]
pub enum CalxType {
  /// TODO
  Nil,
  /// TODO
  Bool,
  /// i64 value
  I64,
  /// f64 value
  F64,
  /// TODO
  Str,
  /// TODO
  List,
  /// TODO
  Link,
}

impl FromStr for CalxType {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "nil" => Ok(CalxType::Nil),
      "bool" => Ok(CalxType::Bool),
      "i64" => Ok(CalxType::I64),
      "f64" => Ok(CalxType::F64),
      "str" => Ok(CalxType::Str),
      "list" => Ok(CalxType::List),
      "link" => Ok(CalxType::Link),
      _ => Err(format!("unknown type: {s}")),
    }
  }
}
