# Calx 核心教程 / Core tutorials

## 中文

从仓库根目录运行教程里的命令，需要 Rust/Cargo。`cargo run --quiet -- ...` 构建并执行当前 checkout 的 CLI；已安装匹配版本的 `calx` 时可以替换这个前缀。示例源码在 [programs/](programs/)，可直接编辑并重新检查。

按顺序阅读：

1. [整数算术](integer-arithmetic.md)：编写 folded Cirru、检查类型、运行结果。
2. [条件](conditionals.md)：Bool 条件与两个分支的结果契约。
3. [循环](loops.md)：block/loop label、branch depth 与 trace 上限。
4. [函数](functions.md)：普通调用与尾调用。
5. [Trap](traps.md)：区分验证失败和运行期失败。
6. [端到端](end-to-end.md)：同一源码的 syntax、validation、lowering 与真实运行事件。

代码都使用 typed local 声明以选择 CLI 的 strict module 路径，不使用 Nil、Dynamic、无类型 storage 或隐式数值转换。命令下面是关键输出片段，不是完整输出；成功示例摘录 stdout，失败示例摘录 stderr，并以非零状态退出。耗时不作断言。

CI 的 `cargo test` 直接读取六篇教程中的命令、预期片段和成功/失败标记，执行当前构建的 CLI。运行 `cargo test --test tutorial_tests` 可单独验证；无需维护一份与文档分离的命令副本。教程不是性能 benchmark。

## English

Run commands from the repository root with Rust/Cargo installed. The `cargo run --quiet -- ...` prefix builds and runs the checked-out CLI; a matching installed `calx` may replace it. Edit the linked programs, then check and run again.

Follow arithmetic → conditions → loops → functions → traps → end-to-end. Every program selects strict typed validation through an explicit typed local, without Nil, Dynamic, untyped storage or implicit numeric conversions. Output blocks are key excerpts: stdout for success, stderr with nonzero exit status for failure. Timings are intentionally not asserted.

The CI `cargo test` suite reads the commands and expectations directly from these six documents and executes the built CLI. Run `cargo test --test tutorial_tests` for just this check. There is no duplicated command manifest and these examples are not performance benchmarks.

## 权威契约 / Authoritative contracts

- [Source spans and diagnostics](../diagnostics.md)
- [Bounded runtime trace](runtime-trace.md)
- [Wasm mapping and intentional differences](../wasm-mapping.md)
- [Validation and traps RFC](../../RFCs/0001-validation-and-traps.md)
- [Calcit producer boundary](https://github.com/calcit-lang/calcit/blob/main/docs/run/calx-target.md) — owned by Calcit, not duplicated here.

Tracked by [#35](https://github.com/calcit-lang/calx-vm/issues/35). This completes a teaching surface, not the application-adoption acceptance in Calcit #887.
