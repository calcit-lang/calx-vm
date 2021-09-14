mod cirru;
mod primes;
mod vm;

pub use cirru::parse_function;
pub use primes::{Calx, CalxFrame, CalxFunc};
pub use vm::CalxVM;

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
    assert_eq!(2 + 2, 4);
  }
}
