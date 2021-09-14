use crate::primes::{Calx, CalxFrame, CalxFunc, CalxInstr};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxVM {
  pub stack: Vec<Calx>,
  pub globals: Vec<Calx>,
  pub funcs: Vec<CalxFunc>,
  pub frames: Vec<CalxFrame>,
}

impl CalxVM {
  pub fn new(fns: Vec<CalxFunc>, globals: Vec<Calx>) -> Self {
    CalxVM {
      stack: vec![],
      globals,
      funcs: fns,
      frames: vec![],
    }
  }

  /// parses
  /// ```cirru
  /// fn <f-name> (i64 f64)
  ///   load 1
  ///   echo
  /// ```
  pub fn eval(idx: usize, params: Vec<Calx>) -> Calx {
    // TODO
    Calx::Bool(true)
  }
}
