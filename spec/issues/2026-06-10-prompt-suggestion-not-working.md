# Prompt Suggestion 功能实现完成但端到端无效果

**状态**：Open
**优先级**：中
**创建日期**：2026-06-10

## 问题描述

实现了 Prompt Suggestion 功能（agent 执行完成后，异步调用 LLM 生成下一个 prompt 建议，经过滤管道后通过 TUI placeholder 展示），17 个单元测试全部通过，但在 TUI 中实际运行 agent 完成后**看不到任何 suggestion 提示**。功能完全不可用，最终撤回全部代码。

## 症状详情

### 预期行为

1. 用户在 TUI 中执行 agent（发送 prompt 并等待 agent 完成）
2. Agent 完成后，TUI 输入区上方应出现**灰色斜体的 placeholder 文本**，展示 AI 建议的下一个 prompt
3. 用户按 Tab 键可接受建议（placeholder 文本变为输入框内容）

### 实际行为

Agent 完成后，TUI 输入区**没有任何 suggestion 提示出现**，如同功能未实现一样。

### 单元测试结果

17/17 测试通过，覆盖：
- 4 个过滤器逻辑（重复检测、长度检查、substring 匹配、空白检测）
- 5 个服务层 mock 单测（正常生成、empty response 抑制、网络错误抑制、取消后不 emit、重复 prompt 过滤）

### 已排除的因素

- **compact 耦合**：原实现中 suggestion 的 LLM 实例取自 `cached_llm.compact_model`，当 `DISABLE_COMPACT` 设置时 `cached_llm` 为 None 导致 suggestion 永不触发。已改为直接从 provider 构建 LLM，修复后问题依然存在

### 未验证的数据流环节

suggestion 的完整数据流路径为：

```
executor.rs spawn fire-and-forget
  → suggestion::spawn() 调用 LLM generate
  → filter pipeline 过滤
  → emit(SuggestionReady { text })
  → peri-acp event/mapper.rs → MappedEvent::tui_only()
  → TransportEventSink → peri/agent_event 通知
  → TUI acp_client → map_executor_event()
  → agent_ops 设置 ui.prompt_suggestion
  → main_ui 渲染 placeholder
```

以下环节**未逐段验证**（因用户决定停止排查并撤回代码）：

| 环节 | 是否验证 | 说明 |
|------|---------|------|
| executor spawn 是否被调用 | ❌ | fire-and-forget task 可能未 spawn 或提前被 drop |
| LLM generate 是否返回结果 | ❌ | 仅 mock 测试通过，未验证真实 LLM 调用 |
| filter pipeline 是否过滤掉了结果 | ❌ | 单测通过但未验证真实 LLM 输出的过滤行为 |
| SuggestionReady 事件是否 emit | ❌ | 未添加日志确认 |
| mapper.rs 是否正确映射 | ❌ | 代码审查确认逻辑正确，但未运行时验证 |
| TransportEventSink 是否发送通知 | ❌ | 未添加日志确认 |
| TUI map_executor_event 是否处理 | ❌ | 如果编译通过说明映射存在，但未运行时验证 |
| agent_ops 是否正确设置 ui 状态 | ❌ | 同上 |
| main_ui placeholder 渲染是否正确 | ❌ | 同上 |

## 涉及文件

以下文件在实现中曾被修改或新建（已全部撤回）：

**新建（已删除）：**
- `peri-acp/src/suggestion/mod.rs` — 服务层：`spawn()` + `generate()` + suppress 守卫
- `peri-acp/src/suggestion/prompt.rs` — SUGGESTION_PROMPT 模板常量
- `peri-acp/src/suggestion/filter.rs` — 4 个过滤器 + `should_filter()`
- `peri-acp/src/suggestion/filter_test.rs` — 12 个过滤器单测
- `peri-acp/src/suggestion/mod_test.rs` — 5 个服务层 mock 单测

**修改（已还原）：**
- `peri-agent/src/agent/events.rs` — AgentEvent 新增 `SuggestionReady { text: String }` 变体
- `peri-acp/src/session/executor.rs` — agent 完成处 spawn suggestion task（L610-613 附近）
- `peri-acp/src/event/mapper.rs` — `SuggestionReady` → `MappedEvent::tui_only()`
- `peri-tui/src/app/agent.rs` — `map_executor_event` 新增 `SuggestionReady` 映射
- `peri-tui/src/app/events.rs` — TUI AgentEvent 新增 `SuggestionReady` 变体
- `peri-tui/src/app/agent_ops/mod.rs` — 处理 `SuggestionReady` → 设置 `ui.prompt_suggestion`
- `peri-tui/src/ui/main_ui/mod.rs` — placeholder 渲染（灰色斜体，覆盖在 textarea 上方）
- `peri-tui/src/event/keyboard/normal_keys.rs` — Tab 键接受建议

## 修复方向

按数据流顺序逐段验证，定位首个断裂点后针对性修复：

1. 在 `executor.rs` spawn 处添加 `tracing::info!` 日志，确认 fire-and-forget task 被创建
2. 在 `suggestion::spawn()` 中添加日志，确认 LLM 调用是否发起、是否返回结果
3. 在 filter pipeline 后添加日志，确认 LLM 输出是否被过滤
4. 在 `emit(SuggestionReady)` 处添加日志
5. 在 `TransportEventSink::push_event` 中确认 `peri/agent_event` 是否发送
6. 在 TUI 端确认是否收到通知并正确映射

## 状态变更记录

| 日期 | 从 | 到 | 操作人 | 说明 |
|------|-----|-----|--------|------|
| 2026-06-10 | — | Open | agent | 创建 |

## 修复记录

（待修复时追加）
