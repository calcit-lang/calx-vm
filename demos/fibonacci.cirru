
fn main ()
  call fibo
    const 34
  echo

fn fibo (($x i64) -> i64)
  block (->)
    local.get $x
    const 3
    i.lt
    br-if 0

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

  const 1
  return
