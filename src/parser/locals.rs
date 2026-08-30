/// a struct for gathering names of locals and use index
pub struct LocalsCollector {
  pub locals: Vec<String>,
}

impl LocalsCollector {
  pub fn new() -> Self {
    LocalsCollector { locals: vec![] }
  }
  pub fn declare(&mut self, name: &str) -> Result<usize, String> {
    if self.locals.iter().any(|existing| existing == name) {
      Err(format!("duplicate local declaration `{name}`"))
    } else {
      self.locals.push(name.to_string());
      Ok(self.locals.len() - 1)
    }
  }

  pub fn resolve(&self, name: &str) -> Option<usize> {
    self.locals.iter().position(|existing| existing == name)
  }

  pub fn len(&self) -> usize {
    self.locals.len()
  }
}
