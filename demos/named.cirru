
fn f-add (($a i64) ($b i64) -> i64)
  local.new $c
  const 100
  local.set $c
  i.add
    i.add
      local.get $a
      local.get $b
    local.get $c
  dup
  echo
  return

fn main ()
  const 1
  const 2
  call f-add
  drop
