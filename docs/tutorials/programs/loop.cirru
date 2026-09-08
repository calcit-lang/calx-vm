fn main (-> i64)
  local $count i64
  const 0
  block (i64 -> i64)
    loop (i64 -> i64)
      i.add (const 1)
      dup
      i.ge (const 3)
      br-if 1
      br 0
  local.set $count
  local.get $count
  return
