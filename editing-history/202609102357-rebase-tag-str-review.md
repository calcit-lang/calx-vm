# Rebase Tag/String review follow-up / Tag/String rebase 与 review 收尾

## English

- Rebase the open work onto main after the core Tag/String implementation landed independently in PR #77.
- Keep the upstream implementation as the source of truth instead of combining duplicate core changes.
- Clarify the unchanged Cirru token format versus the changed `:name` parsed-value semantics.
- Extend the instruction-set version header to the unreleased 0.6.0 candidate and fix the roadmap Markdown issue reference.
- Retain focused incremental coverage for equal-text CLI display, host signature mismatch, and builder initializer mismatch.

## 中文

- PR #77 独立合入 Tag/String 核心实现后，将当前工作 rebase 到最新 main。
- 以上游实现为 source of truth，不机械拼接两套重复的核心改动。
- 澄清 Cirru token 格式不变，但 `:name` 的 parsed-value 语义发生变化。
- 将 instruction-set 版本范围扩展到未发布的 0.6.0 candidate，并修复 roadmap 的 Markdown issue reference。
- 保留 equal-text CLI display、host signature mismatch 与 builder initializer mismatch 的增量回归。
