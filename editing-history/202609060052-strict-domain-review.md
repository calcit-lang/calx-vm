# Strict domain review / strict 值域评审

## 中文

- PR #62：补充 validator 与 buffer 测试作为契约来源；区分 CalxProgram 的类型准入、global 限制与后续 validator 的 F64Buffer constant 限制。
- README 明确拒绝所有 List；roadmap 对齐 implemented/unreleased 状态并修复行首 issue 引用格式。
- 仅文档修正，代码保持 59c31d9 的已验证状态；执行 fmt/diff 检查与 package 验证。
- 隔离 Calcit dd8138aa2f41d213544453fbbf5ca618ea1996c7 临时 path override 已通过 7 项 codegen 和 11 项 source-backed Calx consumer 回归；未修改正式依赖。

## English

- Clarify source-of-truth files and the distinction between type admission, global restrictions, and validator-side F64Buffer constant rejection.
- State that all Lists are excluded; align implemented/unreleased roadmap status and fix Markdown issue-reference formatting.
- Documentation-only review fixes; retain the tested implementation from 59c31d9 and verify formatting/diff/package.
- The isolated Calcit revision above passed seven codegen and eleven source-backed consumer tests through a temporary VM path override without changing formal dependencies.
