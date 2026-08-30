use core::fmt;
use std::rc::Rc;

use crate::calx::{Calx, CalxType};

use super::instr::CalxInstr;

/// Storage state for locals and globals.
///
/// `Uninitialized` is control state, not the explicit language value
/// [`Calx::Nil`]. Both strict declarations and legacy allocation instructions
/// use this state until the first write.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CalxSlot {
  Uninitialized,
  Value(Calx),
}

impl CalxSlot {
  pub fn value(value: Calx) -> Self {
    Self::Value(value)
  }

  pub fn as_value(&self) -> Option<&Calx> {
    match self {
      Self::Uninitialized => None,
      Self::Value(value) => Some(value),
    }
  }

  pub fn set(&mut self, value: Calx) {
    *self = Self::Value(value);
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxFrame {
  pub name: Rc<str>,
  pub locals: Vec<CalxSlot>, // params + declared or legacy-allocated locals
  /** store return values */
  pub instrs: Rc<Vec<CalxInstr>>,
  pub pointer: usize,
  pub initial_stack_size: usize,
  pub ret_types: Rc<Vec<CalxType>>,
}

impl Default for CalxFrame {
  fn default() -> Self {
    CalxFrame {
      name: String::from("<zero>").into(),
      locals: vec![],
      instrs: Rc::new(vec![]),
      pointer: 0,
      initial_stack_size: 0,
      ret_types: Rc::new(vec![]),
    }
  }
}

impl fmt::Display for CalxFrame {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("CalxFrame ")?;
    write!(f, "_{} (", self.initial_stack_size)?;
    for p in &*self.ret_types {
      write!(f, "{p:?} ")?;
    }
    write!(f, ") @{}", self.pointer)?;
    for (idx, instr) in self.instrs.iter().enumerate() {
      write!(f, "\n  {idx:02} {instr:?}")?;
    }
    f.write_str("\n")?;
    Ok(())
  }
}
