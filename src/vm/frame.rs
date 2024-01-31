use core::fmt;
use std::rc::Rc;

use crate::calx::{Calx, CalxType};

use super::instr::CalxInstr;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxFrame {
  pub name: Rc<str>,
  pub locals: Vec<Calx>, // params + added locals
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
      write!(f, "{:?} ", p)?;
    }
    write!(f, ") @{}", self.pointer)?;
    for (idx, instr) in self.instrs.iter().enumerate() {
      write!(f, "\n  {:02} {:?}", idx, instr)?;
    }
    f.write_str("\n")?;
    Ok(())
  }
}
