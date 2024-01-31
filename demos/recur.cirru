
fn main ()
  const 0
  const 20000000
  call sum
  echo

fn sum (($acc i64) ($x i64) -> i64)
  local.get $acc
  local.get $x
  add
  ;; dup
  ;; echo
  block (->)
    block (->)
      local.get $x
      const 1
      i.le
      br-if 1
    local.get $x
    const -1
    add
    return-call sum
  return
