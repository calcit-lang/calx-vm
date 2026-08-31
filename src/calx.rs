mod types;

// use bincode::{Decode, Encode};
use core::fmt;
use regex::Regex;
use std::{rc::Rc, str::FromStr, sync::LazyLock};

pub use types::CalxType;

/// Simplied from Calcit, but trying to be basic and mutable
#[derive(Clone, PartialEq, PartialOrd)]
pub enum Calx {
  /// TODO
  Nil,
  /// TODO
  Bool(bool),
  /// `i64`
  I64(i64),
  /// `f64`
  F64(f64),
  /// Immutable, shared, homogeneous f64 storage.
  F64Buffer(Rc<[f64]>),
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
        let Some(s0) = s.chars().next() else {
          return Err(String::from("unknown empty string"));
        };
        if s0 == '|' || s0 == ':' {
          Ok(Calx::Str(Rc::from(&s[1..s.len()])))
        } else if FLOAT_PATTERN.is_match(s) {
          match s.parse::<f64>() {
            Ok(u) => Ok(Calx::F64(u)),
            Err(e) => Err(format!("failed to parse: {e}")),
          }
        } else if INT_PATTERN.is_match(s) {
          match s.parse::<i64>() {
            Ok(u) => Ok(Calx::I64(u)),
            Err(e) => Err(format!("failed to parse: {e}")),
          }
        } else {
          Err(format!("unknown value: {s}"))
        }
      }
    }
  }
}

impl Calx {
  /// Share existing immutable backing without copying its elements.
  pub fn f64_buffer_share(values: Rc<[f64]>) -> Self {
    Self::F64Buffer(values)
  }

  /// Adopt host-owned elements into immutable shared backing.
  ///
  /// This consumes the vector but deliberately does not promise zero-copy.
  pub fn f64_buffer_adopt(values: Vec<f64>) -> Self {
    Self::F64Buffer(Rc::from(values.into_boxed_slice()))
  }

  /// Copy borrowed elements into immutable shared backing.
  pub fn f64_buffer_copy_from_slice(values: &[f64]) -> Self {
    Self::F64Buffer(Rc::from(values))
  }

  /// Borrow the unboxed element sequence when this is an F64Buffer.
  pub fn as_f64_buffer(&self) -> Option<&[f64]> {
    match self {
      Self::F64Buffer(values) => Some(values),
      _ => None,
    }
  }

  pub fn value_type(&self) -> CalxType {
    match self {
      Calx::Nil => CalxType::Nil,
      Calx::Bool(_) => CalxType::Bool,
      Calx::I64(_) => CalxType::I64,
      Calx::F64(_) => CalxType::F64,
      Calx::F64Buffer(_) => CalxType::F64Buffer,
      Calx::Str(_) => CalxType::Str,
      Calx::List(_) => CalxType::List,
    }
  }

  // for runtime type checking
  pub fn typed_as(&self, t: CalxType) -> bool {
    self.value_type() == t
  }

  pub fn truthy(&self) -> bool {
    match self {
      Calx::Nil => false,
      Calx::Bool(b) => *b,
      Calx::I64(n) => *n != 0,
      Calx::F64(n) => *n != 0.0,
      Calx::F64Buffer(_) => true,
      Calx::Str(_) => true,
      Calx::List(_) => true,
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
      Calx::F64Buffer(values) => write!(f, "#<f64-buffer len={}>", values.len()),
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

impl fmt::Debug for Calx {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Nil => f.write_str("Nil"),
      Self::Bool(value) => f.debug_tuple("Bool").field(value).finish(),
      Self::I64(value) => f.debug_tuple("I64").field(value).finish(),
      Self::F64(value) => f.debug_tuple("F64").field(value).finish(),
      Self::F64Buffer(values) => f.debug_struct("F64Buffer").field("len", &values.len()).finish(),
      Self::Str(value) => f.debug_tuple("Str").field(value).finish(),
      Self::List(values) => f.debug_tuple("List").field(values).finish(),
    }
  }
}

static FLOAT_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new("^-?\\d+\\.(\\d+)?$").unwrap());
static INT_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new("^-?\\d+$").unwrap());
// static USIZE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new("^\\d+$").unwrap());
