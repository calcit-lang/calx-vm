
fn f1 (-> i64) (const "|data in f1") (echo) (const 10) (return)

fn f2 (i64 -> i64) (local.get 0) (echo) (const 10) (return)

fn blocks (-> i64)
  , (const 1) (echo) (const 4.) (const 2.) (add) (echo)
  block (->) (br 0) (const 1.) (const 2.) (neg) (add) (echo)
    block (->) (const 1.) (const 2.) (neg) (add) (echo)
      block (->) (const 1.) (const 2.) (neg) (add) (echo)
  const "|======"
  echo
  block (->) (br 0) (const 1.) (const 2.) (neg) (add) (echo)
  , (const "|demo of string") (echo)
  const 0

fn sum (-> i64)
  local.new
  block (-> i64)
    const 0
    local.set 0
    const 0
    loop (i64)
      ;; "echo inspect i"
      ;; const |inspect
      ;; echo
      ;; dup
      ;; echo i

      ;; "i += 1"
      const 1
      i.add

      ;; "acc += i"
      dup
      local.get 0
      i.add
      local.set 0

      ;; "if >= 1M"
      dup
      const 1000000
      i.ge
      br-if 1

      br 0
  drop
  const "|check sum"
  echo
  local.get 0
  dup
  echo

fn echos (-> i64)
  const "|loading program"
  echo

  call blocks

  const 2
  const 3
  call-import log2
  echo

  return


fn main (-> i64)
  call sum
  return
