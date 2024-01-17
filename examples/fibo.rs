//! fibonacci for comparisom
//! still orders in magnitude faster than Calx

fn fibo(n: i64) -> i64 {
  if n < 3 {
    1
  } else {
    fibo(n - 1) + fibo(n - 2)
  }
}

fn main() {
  println!("Result {}", fibo(40))
}
