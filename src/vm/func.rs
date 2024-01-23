use bincode::{Decode, Encode};
use core::fmt;
use std::rc::Rc;

use crate::{calx::CalxType, syntax::CalxSyntax};

use super::instr::CalxInstr;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxFunc {
  pub name: Rc<str>,
  pub params_types: Rc<Vec<CalxType>>,
  pub ret_types: Rc<Vec<CalxType>>,
  pub syntax: Rc<Vec<CalxSyntax>>,
  pub instrs: Option<Rc<Vec<CalxInstr>>>,
  pub local_names: Rc<Vec<String>>,
}

impl fmt::Display for CalxFunc {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "CalxFunc {} (", self.name)?;
    for p in &*self.params_types {
      write!(f, "{:?} ", p)?;
    }
    f.write_str("-> ")?;
    for p in &*self.ret_types {
      write!(f, "{:?} ", p)?;
    }
    f.write_str(")")?;
    if !self.local_names.is_empty() {
      f.write_str("\n  local_names:")?;
      for (idx, l) in self.local_names.iter().enumerate() {
        write!(f, " {}_{}", idx, l)?;
      }
      f.write_str(" .")?;
    }
    match &self.instrs {
      Some(instrs) => {
        for (idx, instr) in instrs.iter().enumerate() {
          write!(f, "\n  {:02} {:?}", idx, instr)?;
        }
      }
      None => {
        write!(f, "\n  <none>")?;
      }
    }
    f.write_str("\n")?;
    Ok(())
  }
}
