# Separate ABI edition from dependency versions

## 中文

修正 strict 迁移说明的英文连接词：保持不变的是 ABI edition，消费者依赖版本应在
发布后升级，不存在新的 dependency edition 概念。仅文档；复用刚通过的完整
runtime gates，再检查格式/diff 与 package verification。

## English

Clarify that the ABI edition remains unchanged while consumer dependency versions
are upgraded after publication; no dependency-edition concept is introduced.
Documentation only, with the just-passed runtime gates plus formatting/diff and
package verification.
