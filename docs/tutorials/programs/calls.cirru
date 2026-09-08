fn main (-> i64)
  local $answer i64
  call countdown (const 3)
  local.set $answer
  local.get $answer
  return

fn countdown (($n i64) -> i64)
  i.le (local.get $n) (const 0)
  if (-> i64)
    do
      const 42
    do
      i.add (local.get $n) (const -1)
      return-call countdown
  return
