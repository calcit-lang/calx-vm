fn main (-> i64)
  local $answer i64
  call identity (const 3.)
  local.set $answer
  local.get $answer
  return

fn identity (($n i64) -> i64)
  local.get $n
  return
