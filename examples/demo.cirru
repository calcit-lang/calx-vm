
fn f1 (-> i64) (const "|data in f1") (echo) (const 10) (return)

fn f2 (i64 -> i64) (local.get 0) (echo) (const 10) (return)

fn demo (-> i64)
  , (const 1) (echo) (const 4.) (const 2.) (add) (echo)
  block (->) (br 0) (const 1.) (const 2.) (neg) (add) (echo)
    block (->) (const 1.) (const 2.) (neg) (add) (echo)
      block (->) (const 1.) (const 2.) (neg) (add) (echo)
  const "|======"
  echo
  block (->) (br 0) (const 1.) (const 2.) (neg) (add) (echo)
  , (const "|demo of string") (echo)
  block (-> i64)
    const 0
    local.set 0
    const 0
    loop (i64 -> i64)
      const 1
      i.add
      dup
      local.get 0
      i.add
      local.set 0
      dup
      const 100000
      i.ge
      br-if 1
      br 0
  const "|check sum"
  echo
  local.get 0
  echo

fn main (-> i64)
  const "|loading program"
  echo

  call demo

  const 2
  const 3
  call-import log2
  echo

  return
