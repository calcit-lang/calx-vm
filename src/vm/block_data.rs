use std::rc::Rc;

use crate::calx::CalxType;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct BlockData {
  pub looped: bool,
  pub params_types: Rc<Vec<CalxType>>,
  pub ret_types: Rc<Vec<CalxType>>,
  pub from: usize,
  pub to: usize,
  pub initial_stack_size: usize,
}

impl BlockData {
  // size of stack after block finished or breaked
  pub fn expected_finish_size(&self) -> usize {
    self.initial_stack_size - self.params_types.len() + self.ret_types.len()
  }
}
