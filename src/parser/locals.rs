/// a struct for gathering names of locals and use index
pub struct LocalsCollector {
  pub locals: Vec<String>,
}

impl LocalsCollector {
  pub fn new() -> Self {
    LocalsCollector { locals: vec![] }
  }
  pub fn track(&mut self, name: &str) -> usize {
    match self.locals.iter().position(|n| n == name) {
      Some(i) => i,
      None => {
        self.locals.push(name.to_string());
        self.locals.len() - 1
      }
    }
  }
}
