use core::fmt;
use std::rc::Rc;

use crate::{calx::CalxType, syntax::CalxSyntax, CalxLocalDecl, SourceSpan};

use super::instr::CalxInstr;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxFunc {
  pub name: Rc<str>,
  pub params_types: Rc<Vec<CalxType>>,
  pub ret_types: Rc<Vec<CalxType>>,
  /// Non-parameter local declarations. Parameters remain first in the local index space.
  pub locals: Rc<Vec<CalxLocalDecl>>,
  pub syntax: Rc<Vec<CalxSyntax>>,
  /// Source ranges parallel to `syntax`. Legacy AST-only parsing leaves this empty.
  pub source_spans: Rc<Vec<Option<SourceSpan>>>,
  pub instrs: Rc<Vec<CalxInstr>>,
  pub local_names: Rc<Vec<String>>,
}

impl CalxFunc {
  pub fn new(name: impl Into<Rc<str>>, params_types: Vec<CalxType>, ret_types: Vec<CalxType>, syntax: Vec<CalxSyntax>) -> Self {
    let local_names = (0..params_types.len()).map(|index| format!("${index}")).collect();
    Self {
      name: name.into(),
      params_types: Rc::new(params_types),
      ret_types: Rc::new(ret_types),
      locals: Rc::new(vec![]),
      syntax: Rc::new(syntax),
      source_spans: Rc::new(vec![]),
      instrs: Rc::new(vec![]),
      local_names: Rc::new(local_names),
    }
  }

  pub fn with_locals(mut self, locals: Vec<CalxLocalDecl>) -> Self {
    let names = Rc::make_mut(&mut self.local_names);
    names.truncate(self.params_types.len());
    names.extend(locals.iter().map(|local| local.name.to_string()));
    self.locals = Rc::new(locals);
    self
  }

  pub fn with_local_names(mut self, local_names: Vec<String>) -> Self {
    self.local_names = Rc::new(local_names);
    self
  }

  pub fn with_source_spans(mut self, source_spans: Vec<Option<SourceSpan>>) -> Self {
    self.source_spans = Rc::new(source_spans);
    self
  }

  pub fn with_instrs(mut self, instrs: Vec<CalxInstr>) -> Self {
    self.instrs = Rc::new(instrs);
    self
  }
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
