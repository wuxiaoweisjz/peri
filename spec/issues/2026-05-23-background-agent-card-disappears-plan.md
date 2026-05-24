# Implementation Plan: Background Agent 卡片消失 + 无数据回传

**Issue**: `spec/issues/2026-05-23-background-agent-card-disappears-no-result.md`
**Date**: 2026-05-23

## 问题回顾

后台 agent 完成事件通过 `event_handler.on_event(BackgroundTaskCompleted)` 发送到 executor 的 `event_tx` channel。主 agent `execute()` 返回后 `close_channel()` 将 channel sender 设为 None，导致事件被静默丢弃。TUI 永远收不到完成事件 → 卡片不更新 → 无自动 continuation。

## 方案概述

**统一通过 `ReActAgent.notification_rx` 单通道交付 BackgroundTaskCompleted，删除 spawn 闭包中的 `event_handler.on_event(BackgroundTaskCompleted)` 调用。**

理由：
- `notification_rx` 不会被 `close_channel()` 影响 — 它是 `ReActAgent` 的字段，仅随 `drop(agent_output.executor)` 释放
- `drain_notifications()` 已在执行期间消费此通道（路径 A），只需补齐：(1) drain 时同步 emit 事件，(2) 执行结束后兜底 drain
- 消除 event_handler + notification_rx 双路径，彻底避免历史消息重复问题

## 三步改动

### Step 1: `drain_notifications()` 同步 emit `BackgroundTaskCompleted`

**文件**: `peri-agent/src/agent/executor/final_answer.rs:25-49`

在 `drain_notifications()` 的 while 循环内，`state.add_message(msg)` 之后增加：

```rust
agent.emit(AgentEvent::BackgroundTaskCompleted(result));
```

这使执行期间到达的后台通知同时更新 TUI 卡片（路径 A + 路径 B 语义合并）。

### Step 2: `execute()` 返回后、`drop` 前兜底 drain `notification_rx`

**文件**: `peri-acp/src/session/executor.rs:349-355`

在 `execute()` 返回之后、`drop(agent_output.executor)` 之前插入：

```rust
// Drain remaining background task notifications that arrived after
// the final answer but before the executor is dropped.
if let Some(ref rx) = agent_output.executor.notification_rx {
    let mut rx_lock = rx.lock().await;
    while let Ok(bg_result) = rx_lock.try_recv() {
        if let Some(tx) = event_tx.lock().unwrap().as_ref() {
            let _ = tx.send(ExecutorEvent::BackgroundTaskCompleted(bg_result));
        }
    }
}
```

**注意**: drain 必须在 `drop(agent_output.executor)` **之前**，因为 `notification_rx` 生命周期绑定在 `ReActAgent` 上。

### Step 3: 删除 spawn 闭包中的 `event_handler.on_event(BackgroundTaskCompleted)`

**文件**: `peri-middlewares/src/subagent/tool/define.rs`

两处删除：

1. `invoke_background()` spawn 闭包，第 463-465 行：
   ```rust
   // 删除以下三行
   if let Some(ref handler) = event_handler {
       handler.on_event(AgentEvent::BackgroundTaskCompleted(result));
   }
   ```

2. `invoke_background_fork()` spawn 闭包，第 584-586 行：同上。

## 数据流验证

### 场景 A：后台任务在 agent 执行期间完成

```
spawn 闭包:
  registry.complete(result)
    → notification_tx → notification_rx (buffer)
  
drain_notifications():  ← handle_final_answer / emit_snapshot_and_drain_notifications
  try_recv() → result ✓
  state.add_message(notification)
  agent.emit(BackgroundTaskCompleted)         ← Step 1 新增
    → event_tx → event pump → TUI
      → handle_background_task_completed
      → agent_done_pending_bg = false
      → 缓冲到 pre_done_bg_completions
  agent.emit(StateSnapshot(notification_msg))
    → TUI extend agent_state_messages

agent Done:
  handle_done():
    → 消费 pre_done_bg_completions
    → pending_bg_continuation = combined

poll_agent():
  → submit_message(continuation)
  → 新 agent 轮次启动 ✓
```

### 场景 B：后台任务在 `execute()` 返回后完成

```
spawn 闭包:
  registry.complete(result)
    → notification_tx → notification_rx (buffer)
  ~~event_handler.on_event() 已删除~~       ← Step 3

execute() returns:
  
post-execute drain:  ← Step 2 新增
  try_recv() → result ✓
  send BackgroundTaskCompleted → event_tx (仍然 open)
    → event pump → TUI
      → handle_background_task_completed
      → agent_done_pending_bg = true
      → push Human msg to agent_state_messages
      → update SubAgentGroup card (is_running = false)
      → background_task_count → 0
      → pending_bg_continuation = display_notification

close_channel()
wait_for_pump()

poll_agent():
  → submit_message(continuation)
  → 新 agent 轮次启动 ✓
```

### 场景 C：无后台任务时

`notification_rx.try_recv()` 返回 `Err(Empty)` → 无额外事件 → 行为不变 ✓

## 不改动的文件

| 文件 | 原因 |
|------|------|
| `agent_events_bg.rs` | 现有 handler 逻辑正确：Path B guard + card update + continuation |
| `agent_ops/lifecycle.rs` | handle_done/handle_error 的 `agent_done_pending_bg` 逻辑不变 |
| `agent_ops/polling.rs` | poll_agent 的 `pending_bg_continuation` 消费逻辑不变 |
| `agent.rs` (TUI map) | `map_executor_event` 已映射 `BackgroundTaskCompleted`，无需改动 |
| `builder.rs` | 不新增通道，`notification_rx` 已有 |
| `events.rs` (agent) | `BackgroundTaskCompleted` 变体已存在 |
| `background.rs` | `registry.complete()` 逻辑不变 |

## 风险评估

| 风险 | 级别 | 缓解 |
|------|------|------|
| `drain_notifications()` 增加 emit 调用导致事件量翻倍 | 低 | emit 是 unbounded channel send，非阻塞 |
| post-execute drain 与 `registry.complete()` 竞态 | 低 | 两者都是 tokio 单线程内串行；spawn 闭包写 `notification_tx` 后才返回，不会丢数据 |
| `notification_rx` 在 drop 前被 move | 低 | `agent_output.executor.notification_rx` 是共享引用，drain 后 drop 无影响 |

## 验证计划

1. **单元测试**：`peri-agent/src/agent/executor/` 下新增/更新 drain_notifications 测试，验证 emit 调用
2. **集成测试**：`peri-tui/src/ui/headless_test.rs:3651` 已有 `fork+run_in_background` 诊断测试，验证卡片不消失 + continuation 触发
3. **手动验证**：
   - TUI 中让 LLM 调用 `Agent(run_in_background: true)` 工具
   - 观察后台任务完成后 (a) SubAgent 卡片更新为 completed 状态 (b) 父 agent 自动收到结果并回复
   - 执行多轮对话后 `/exit`，重新进入 `-c` 继续，验证消息不重复
