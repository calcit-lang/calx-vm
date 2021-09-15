
fn f1 () (load 1) (echo)

fn main ()
  , (load 1) (echo) (load 4.) (load 2.) (add) (echo)
  block (br 0) (load 1.) (load 2.) (neg) (add) (echo)
    block (load 1.) (load 2.) (neg) (add) (echo)
      block (load 1.) (load 2.) (neg) (add) (echo)
  load "|======"
  block (br 0) (load 1.) (load 2.) (neg) (add) (echo)
  , (load "|demo of string") (echo)
  block
    load 0
    local.set 0
    load 0
    loop
      load 1
      i.add
      dup
      local.get 0
      i.add
      local.set 0
      dup
      load 100000
      i.ge
      br-if 1
      br 0
  load "|check sum"
  echo
  local.get 0
  echo
  quit 0
