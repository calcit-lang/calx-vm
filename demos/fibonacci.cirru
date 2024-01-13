
fn main ()
  call fibo
    const 35
  echo

fn fibo (($x i64) -> i64)
  local.get $x
  block (i64 -> i64)
    const 3
    i.lt
    dup
    br-if 0
    drop

    local.get $x
    const -1
    i.add
    call fibo

    local.get $x
    const -2
    i.add
    call fibo

    i.add

    return

  drop
  const 1
  return
