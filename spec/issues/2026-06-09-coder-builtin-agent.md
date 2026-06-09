# 创建 coder 内置 Agent 类型

**状态**：Open
**优先级**：中
**创建日期**：2026-06-09

## 问题描述

当前 4 个内置 Agent（explore、general-purpose、plan、verification）中，general-purpose 承担了大量代码实现类任务（实现/迁移/重构/修复），但它被设计为全工具、全能力的通用 Agent，导致两个问题：

1. **上下文浪费**：全工具集（含 WebSearch、WebFetch、Agent、AskUserQuestion）占用 system prompt 空间，但这些工具在代码实现场景中极少使用
2. **过度搜索**：典型案例中，一个 general-purpose SubAgent 对同一 pattern 执行了 562 次 Grep 搜索——上下文被挤出后 agent 忘记已有结果，陷入重复搜索循环

通过 `subagent_collab.ts` 的调研数据，general-purpose 的任务中约 92.3% 属于 coder 类（实现/搬运/迁移等建造任务），说明需要一个专门为代码实现优化的 Agent 类型。

## 症状详情

### 调研数据

| 指标 | 数值 | 说明 |
|------|------|------|
| coder 类任务占比 | 92.3% | 实现/搬运/迁移等建造任务占 general-purpose 总量 |
| 极端案例消息数 | 717 | 同一任务因上下文丢失导致循环搜索 |
| 极端案例 Grep 重复 | 562 次 | 同一 pattern 在同一文件中被反复搜索 |
| 成功案例 P50 | 52 条消息 | 任务完成的典型消息量 |
| 成功案例 P95 | 153 条消息 | 95% 任务在此范围内完成 |

### 现象：general-purpose 在 coder 任务中的典型退化路径

1. Agent 收到代码实现任务
2. 开始 Grep 搜索目标位置，Read 读取相关文件
3. 执行若干 Edit/Write 操作
4. 上下文窗口被工具输出占满，早期搜索结果被挤出
5. Agent 忘记已搜索过的内容，重新 Grep → 回到步骤 2
6. 循环直至达到迭代上限或侥幸完成

## 期望改进方向

创建一个新的内置 Agent 类型 `coder`，专门用于代码实现类任务：

- 缩减工具集，移除 coder 场景中不需要的工具，节省 system prompt 空间
- 设置合理的迭代上限，避免无限循环
- 在 system prompt 中强化"记住搜索结果、避免重复搜索"的行为指导

## coder Agent 规格

### 工具集

| 保留（7 个） | 移除（4 个） | 移除理由 |
|-------------|-------------|----------|
| Read | ~~WebSearch~~ | coder 不需要搜索网页 |
| Grep | ~~WebFetch~~ | coder 不需要抓取网页 |
| Glob | ~~Agent~~ | coder 不需要启动子 Agent |
| Bash | ~~AskUserQuestion~~ | coder 不需要向用户提问 |
| LineEdit | | |
| Write | | |
| TodoWrite | | |

### 迭代与上下文

| 参数 | 值 | 依据 |
|------|-----|------|
| 迭代上限 | 200 | P95 = 153，留约 30% 余量 |
| 上下文预算 | -30% vs general-purpose | 缩减工具集后 system prompt 更短 |

### System Prompt 关键差异

相比 general-purpose，coder 的 system prompt 应：
- 移除"搜索代码、分析架构"等探索类描述
- 强调"先搜索定位、再编辑修改"的工作流
- 加入反循环指导："搜索前先确认是否已有搜索结果在上下文中"
- 保留 general-purpose 中的编辑约束（不创建文档、优先编辑已有文件）

## 涉及文件

- `peri-middlewares/src/subagent/built-in/coder.md` —— 新建，coder Agent 定义文件
- `peri-middlewares/src/subagent/built_in_agents.rs` —— 第 29 行 `[BuiltInAgent; 4]` 改为 `[BuiltInAgent; 5]`，新增 coder 条目
- `peri-middlewares/src/subagent/built-in/general-purpose.md` —— 参考格式（不修改）

## 验证标准

1. `bun run src/metrics/subagent_collab.ts --since N` 的"内置 Agent 分类分析"中应出现 `coder` 类型
2. 用同一代码实现任务分别以 `general-purpose` 和 `coder` 执行，对比：
   - 消息总数
   - Grep 调用次数（coder 应显著减少）
   - 是否出现重复搜索同一 pattern 的退化行为
3. coder 应能独立完成实现/迁移/重构类任务，不需要回退到 general-purpose

## 状态变更记录

| 日期 | 从 | 到 | 操作人 | 说明 |
|------|-----|-----|--------|------|
| 2026-06-09 | — | Open | agent | 创建 |

## 修复记录

（由 fix-issue 或 issue-verify skill 追加，创建时留空）
