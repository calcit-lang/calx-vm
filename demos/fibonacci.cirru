
fn main ()
  call fibo
    const 34
  echo

fn fibo (($x i64) -> i64)
  block (-> i64)
    local.get $x
    dup
    const 3
    i.lt
    br-if 0

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
