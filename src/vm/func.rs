use core::fmt;
use std::rc::Rc;

use crate::{calx::CalxType, syntax::CalxSyntax, SourceSpan};

use super::instr::CalxInstr;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxFunc {
  pub name: Rc<str>,
  pub params_types: Rc<Vec<CalxType>>,
  pub ret_types: Rc<Vec<CalxType>>,
  pub syntax: Rc<Vec<CalxSyntax>>,
  /// Source ranges parallel to `syntax`. Legacy AST-only parsing leaves this empty.
  pub source_spans: Rc<Vec<Option<SourceSpan>>>,
  pub instrs: Rc<Vec<CalxInstr>>,
  pub local_names: Rc<Vec<String>>,
}

impl fmt::Display for CalxFunc {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "CalxFunc {} (", self.name)?;
    for p in &*self.params_types {
      write!(f, "{p:?} ")?;
    }
    f.write_str("-> ")?;
    for p in &*self.ret_types {
      write!(f, "{p:?} ")?;
    }
    f.write_str(")")?;
    if !self.local_names.is_empty() {
      f.write_str("\n  local_names:")?;
      for (idx, l) in self.local_names.iter().enumerate() {
        write!(f, " {idx}_{l}")?;
      }
      f.write_str(" .")?;
    }
    for (idx, instr) in self.instrs.iter().enumerate() {
      if let Some(Some(span)) = self.source_spans.get(idx) {
        write!(f, "\n  {idx:02} {instr:?} @ {span}")?;
      } else {
        write!(f, "\n  {idx:02} {instr:?}")?;
      }
    }
    f.write_str("\n")?;
    Ok(())
  }
}
