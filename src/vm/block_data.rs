use std::rc::Rc;

use crate::calx::CalxType;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum BlockData {
  Block {
    params_types: Rc<Vec<CalxType>>,
    ret_types: Rc<Vec<CalxType>>,
    to: usize,
    initial_stack_size: usize,
  },
  Loop {
    params_types: Rc<Vec<CalxType>>,
    ret_types: Rc<Vec<CalxType>>,
    from: usize,
    to: usize,
    initial_stack_size: usize,
  },
  If {
    ret_types: Rc<Vec<CalxType>>,
    else_to: usize,
    to: usize,
    initial_stack_size: usize,
  },
}

impl BlockData {
  // size of stack after block finished or breaked
  pub fn expected_finish_size(&self) -> usize {
    match self {
      BlockData::Block {
        initial_stack_size,
        params_types,
        ret_types,
        ..
      } => *initial_stack_size - params_types.len() + ret_types.len(),
      BlockData::Loop {
        initial_stack_size,
        params_types,
        ret_types,
        ..
      } => *initial_stack_size - params_types.len() + ret_types.len(),
      BlockData::If {
        initial_stack_size,
        ret_types,
        ..
      } => *initial_stack_size - 1 + ret_types.len(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct BlockStack {
  pub stack: Vec<BlockData>,
}

impl BlockStack {
  pub fn new() -> Self {
    BlockStack { stack: vec![] }
  }

  pub fn push(&mut self, block: BlockData) {
    self.stack.push(block);
  }

  pub fn is_empty(&self) -> bool {
    self.stack.is_empty()
  }

  pub fn len(&self) -> usize {
    self.stack.len()
  }

  /// expected a `If` result, return error otherwise
  pub fn pop_if(&mut self) -> Result<BlockData, String> {
    match self.stack.pop() {
      Some(a @ BlockData::If { .. }) => Ok(a),
      None => Err("BlockStack::pop_if: stack is empty".to_string()),
      block => Err(format!("BlockStack::pop_if: expected If, got {block:?}")),
    }
  }

  pub fn peek_if(&self) -> Result<&BlockData, String> {
    match self.stack.last() {
      Some(a @ BlockData::If { .. }) => Ok(a),
      None => Err("BlockStack::peek_if: stack is empty".to_string()),
      block => Err(format!("BlockStack::peek_if: expected If, got {block:?}")),
    }
  }

  /// pops `block` or `loop`, if `if` block occurs, just remove, return error is empty
  pub fn pop_block(&mut self) -> Result<BlockData, String> {
    loop {
      let b = self.stack.pop();
      match b {
        Some(v @ BlockData::Block { .. }) => return Ok(v),
        Some(v @ BlockData::Loop { .. }) => return Ok(v),
        Some(BlockData::If { .. }) => continue,
        None => return Err("BlockStack::pop_block: stack is empty".to_string()),
      }
    }
  }

  pub fn peek_block(&self) -> Result<&BlockData, String> {
    loop {
      let b = self.stack.last();
      match b {
        Some(v @ BlockData::Block { .. }) => return Ok(v),
        Some(v @ BlockData::Loop { .. }) => return Ok(v),
        Some(BlockData::If { .. }) => continue,
        None => return Err("BlockStack::peek_block: stack is empty".to_string()),
      }
    }
  }

  pub fn peek_block_level(&self, level: usize) -> Result<&BlockData, String> {
    if level == 0 {
      return self.peek_block();
    }

    let mut count = 0;
    loop {
      let b = self.stack.get(self.stack.len() - 1 - count);
      match b {
        Some(v @ BlockData::Block { .. }) => {
          if count == level {
            return Ok(v);
          } else {
            count += 1;
          }
        }
        Some(v @ BlockData::Loop { .. }) => {
          if count == level {
            return Ok(v);
          } else {
            count += 1;
          }
        }
        Some(BlockData::If { .. }) => continue,
        None => return Err("BlockStack::peek_block: stack is empty".to_string()),
      }
    }
  }
}
