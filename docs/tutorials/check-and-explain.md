# 使用 `calx check` 与 `calx explain`

Calx 的教学工具把程序处理过程拆成可观察的阶段：

```text
folded Cirru
  -> expanded CalxSyntax
  -> typed validation
  -> lowered CalxInstr
  -> execution
```

`check` 停在 typed validation，`explain` 继续 lowering 但不执行，`run` 才执行 guest 程序。三条命令复用同一套 parser、validator 和 lowering，不各自近似实现语义。

## 只检查程序

```bash
calx check demos/if.cirru
```

成功时输出函数和展开指令数量：

```text
[calx check] ok: 2 function(s), 14 syntax instruction(s)
```

`check` 不执行 `echo`、import、`quit` 或其他 guest 指令，因此适合编辑器保存检查和 CI。类型错误会在执行前失败：

```text
validation error in main at syntax[2]: expected I64, found F64
operand stack: []
```

当前位置是函数内扁平 `CalxSyntax` index；Cirru source span 和稳定诊断码仍是后续工作。

## 解释验证与 lowering

```bash
calx explain demos/if.cirru --function demo
```

每个函数先显示 normalized folded Cirru，再按实际验证顺序列出：

```text
syntax[001] If { ret_types: [], else_at: 5, to: 8 }
  operand: [I64] -> []
  control: [Function(...)] -> [Function(...) > If(...)]
  lowered: JmpIf(5)
```

- `operand` 是指令前后的验证类型栈，右端为栈顶；
- `control` 从外到内列出 function/block/loop/if frame；
- `height` 是控制结构入口处移除参数后的操作数栈高度；
- `label` 是 `br` 或 `br-if` 到该 frame 时需要保留的类型；
- `reachable/unreachable` 展示不可达栈多态状态；
- `lowered` 是解释器实际执行的内部指令。

Calx parser 目前为绝对跳转 lowering 把 `if` 的 else 分支放在扁平 syntax 前半段，所以 `explain` 展示的验证顺序可能是 else 后 then；原始 folded Cirru 仍保留源代码结构。这一差异也记录在 [`RFC 0001`](../../RFCs/0001-validation-and-traps.md)。

## 理解 `Dynamic`

`Dynamic` 表示 validator 缺少静态信息，不表示该值已经满足任何类型。它目前可能来自无类型 local/global 或宿主 import：

```text
operand: [Dynamic, I64] -> [Dynamic]
```

此类操作仍保留 interpreter 的运行时类型检查。函数签名、常量和有类型的 block label 则显示为 `I64`、`F64`、`Bool` 等已知类型。

## 运行兼容性

显式运行命令是：

```bash
calx run demos/hello.cirru
```

原有调用仍可使用，并等价于 `run`：

```bash
calx demos/hello.cirru
calx -s demos/hello.cirru
```

`-s/--show-code` 和 `-v/--verbose` 属于 `run`；`--function` 属于 `explain`。
