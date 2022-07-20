use crate::primes::{Calx, CalxError};

pub fn log_calx_value(xs: Vec<Calx>) -> Result<Calx, CalxError> {
  println!("log: {:?}", xs);
  Ok(Calx::Nil)
}
