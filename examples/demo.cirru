
fn f1 () (load 1) (echo)

fn main ()
  , (load 1) (echo) (load 4.) (load 2.) (add) (echo)
  block (load 1.) (load 2.) (neg) (add) (echo)
    block (load 1.) (load 2.) (neg) (add) (echo)
      block (load 1.) (load 2.) (neg) (add) (echo)
  load "|======"
  block (load 1.) (load 2.) (neg) (add) (echo)
  , (load "|demo of string") (echo)
  quit 0
