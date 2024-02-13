mod types;

// use bincode::{Decode, Encode};
use core::fmt;
use lazy_static::lazy_static;
use regex::Regex;
use std::{rc::Rc, str::FromStr};

pub use types::CalxType;

/// Simplied from Calcit, but trying to be basic and mutable
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Calx {
  /// TODO
  Nil,
  /// TODO
  Bool(bool),
  /// `i64`
  I64(i64),
  /// `f64`
  F64(f64),
  // TODO
  Str(Rc<str>),
  /// TODO
  List(Vec<Calx>),
  // to simultate linked structures
  // Link(Box<Calx>, Box<Calx>, Box<Calx>),
}

impl FromStr for Calx {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "nil" => Ok(Calx::Nil),
      "true" => Ok(Calx::Bool(true)),
      "false" => Ok(Calx::Bool(false)),
      "" => Err(String::from("unknown empty string")),
      _ => {
        let s0 = s.chars().next().unwrap();
        if s0 == '|' || s0 == ':' {
          Ok(Calx::Str(Rc::from(&s[1..s.len()])))
        } else if FLOAT_PATTERN.is_match(s) {
          match s.parse::<f64>() {
            Ok(u) => Ok(Calx::F64(u)),
            Err(e) => Err(format!("failed to parse: {}", e)),
          }
        } else if INT_PATTERN.is_match(s) {
          match s.parse::<i64>() {
            Ok(u) => Ok(Calx::I64(u)),
            Err(e) => Err(format!("failed to parse: {}", e)),
          }
        } else {
          Err(format!("unknown value: {}", s))
        }
      }
    }
  }
}

impl Calx {
  // for runtime type checking
  pub fn typed_as(&self, t: CalxType) -> bool {
    match self {
      Calx::Nil => t == CalxType::Nil,
      Calx::Bool(_) => t == CalxType::Bool,
      Calx::I64(_) => t == CalxType::I64,
      Calx::F64(_) => t == CalxType::F64,
      Calx::Str(_) => t == CalxType::Str,
      Calx::List(_) => t == CalxType::List,
      // Calx::Link(_, _, _) => t == CalxType::Link,
    }
  }

  pub fn truthy(&self) -> bool {
    match self {
      Calx::Nil => false,
      Calx::Bool(b) => *b,
      Calx::I64(n) => *n != 0,
      Calx::F64(n) => *n != 0.0,
      Calx::Str(_) => false,
      Calx::List(_) => false,
      // Calx::Link(_, _, _) => true,
    }
  }
}

impl fmt::Display for Calx {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Calx::Nil => f.write_str("nil"),
      Calx::Bool(b) => f.write_str(&b.to_string()),
      Calx::I64(n) => f.write_str(&n.to_string()),
      Calx::F64(n) => f.write_str(&n.to_string()),
      Calx::Str(s) => f.write_str(s),
      Calx::List(xs) => {
        f.write_str("(")?;
        let mut at_head = true;
        for x in xs {
          if at_head {
            at_head = false
          } else {
            f.write_str(" ")?;
          }
          x.fmt(f)?;
        }
        f.write_str(")")?;
        Ok(())
      } // Calx::Link(..) => f.write_str("TODO LINK"),
    }
  }
}

lazy_static! {
  static ref FLOAT_PATTERN: Regex = Regex::new("^-?\\d+\\.(\\d+)?$").unwrap();
  static ref INT_PATTERN: Regex = Regex::new("^-?\\d+$").unwrap();
  static ref USIZE_PATTERN: Regex = Regex::new("^\\d+$").unwrap();
}
