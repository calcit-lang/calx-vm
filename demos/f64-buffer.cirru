fn main (f64-buffer f64 -> f64)
  local $held f64-buffer
  local.get 0
  local.set $held
  local.get $held
  local.get 1
  f64.to-i64-index
  f64-buffer.get
  return
