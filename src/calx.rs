use core::fmt;

use bincode::{Decode, Encode};

/// Simplied from Calcit, but trying to be basic and mutable
#[derive(Debug, Clone, PartialEq, PartialOrd, Decode, Encode)]
pub enum Calx {
  Nil,
  Bool(bool),
  I64(i64),
  F64(f64),
  Str(String),
  List(Vec<Calx>),
  // to simultate linked structures
  // Link(Box<Calx>, Box<Calx>, Box<Calx>),
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Decode, Encode)]
pub enum CalxType {
  Nil,
  Bool,
  I64,
  F64,
  Str,
  List,
  Link,
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
