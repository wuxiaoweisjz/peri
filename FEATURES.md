# Perihelion 功能列表

## 核心智能体框架 (rust-create-agent)

| Feature | Description |
|---------|-------------|
| ReAct Loop | 推理-行动循环执行器，最大 50 轮迭代，支持取消中断 |
| LLM Multi-Provider | OpenAI / Anthropic 双适配器，`BaseModel` trait 可扩展 |
| RetryableLLM | 指数退避重试装饰器，自动恢复瞬态错误 |
| Streaming Response | LLM 流式响应处理，实时输出 TextChunk |
| Message System | 4 种消息类型（Human/Ai/System/Tool）+ 7 种 ContentBlock |
| Event System | 11 种核心事件（AiReasoning/TextChunk/ToolStart/ToolEnd 等） |
| Tool System | `BaseTool` trait + 动态 ToolProvider，支持参数 JSON Schema |
| Middleware Chain | 6 钩子（before/after_agent, before/after_tool, collect_tools, on_error） |
| Thread Persistence | SQLite + Filesystem 双后端，消息序列化/反序列化/快照恢复 |
| Context Compression | Micro-compact（零 API）+ Full Compact（LLM 摘要），85% 阈值自动触发 |
| Telemetry | OpenTelemetry OTLP + Langfuse 双链路追踪 |
| Thinking/Reasoning | Anthropic thinking mode + OpenAI reasoning_effort 支持 |
| CancellationToken | 优雅中断 Agent 执行，恢复已输入文本 |
| AskUser Tool | 批量提问工具，支持单选/多选/自定义输入 |

## 中间件实现 (rust-agent-middlewares)

| Feature | Description |
|---------|-------------|
| FilesystemMiddleware | Read/Write/Edit/Glob/Grep/folder_operations 6 个文件系统工具 |
| TerminalMiddleware | Bash 工具，子进程执行，超时处理，stdout/stderr 捕获 |
| AgentDefineMiddleware | 解析 `.claude/agents/` YAML frontmatter，提取 role/maxTurns/tools |
| AgentsMdMiddleware | 读取 CLAUDE.md/AGENTS.md，注入项目上下文到系统提示词 |
| SkillsMiddleware | 三级搜索（用户/全局/项目），SKILL.md 解析，摘要注入 |
| SkillPreloadMiddleware | `#skill-name` 全文注入，fake tool 序列实现 |
| TodoMiddleware | TodoWrite 工具，任务状态管理（pending/in_progress/completed） |
| CronMiddleware | Cron 注册/列表/删除，5 字段表达式，内存调度（最多 20 任务） |
| HITL Middleware | 5 级权限模式（Yolo/Ask/Delegate/Auto/Approved），批量审批，工具分类 |
| SubAgentMiddleware | Fork 模式（继承完整上下文），后台执行（最大 3 并发），工具过滤 |
| MCP Middleware | stdio/StreamableHTTP 传输，OAuth 2.0，连接池，资源读取，工具桥接 |

## TUI 终端应用 (rust-agent-tui)

| Feature | Description |
|---------|-------------|
| Interactive TUI | ratatui 终端 UI，异步事件驱动，独立渲染线程 |
| Multi-Session Split | 多 session 并列分屏，彩色边框指示焦点 |
| Slash Commands | 15 个内置命令（/login /model /history /agents /compact /clear /config /cost /context /memory /help /loop /cron /mcp /split） |
| Model Panel | 交互式模型选择，别名切换（Alt+M），Thinking 模式配置 |
| Provider Config | Anthropic/OpenAI/Custom Provider 管理，API Key 存储 |
| Thread Browser | 历史对话浏览，线程加载/删除（含确认） |
| Memory Panel | 跨 session 持久化记忆，CRUD 操作 |
| MCP Panel | 服务器列表/状态显示，启用/禁用切换，连接管理 |
| Agent Panel | Agent 定义列表，工具过滤可视化 |
| Cron Panel | 定时任务管理，注册/删除/启停 |
| HITL Approval UI | 批量审批弹窗，工具分组，内联编辑参数，逐条/全部批准 |
| AskUser UI | 单选/多选问题，自定义输入，批量提问 |
| OAuth Flow UI | 本地回调服务器（端口 39367），PKCE 流程，Token 交换 |
| Setup Wizard | 首次运行引导，Provider 设置，API Key 录入 |
| Markdown Rendering | pulldown-cmark 解析，syntect 语法高亮，主题感知 |
| Context Compression | `/compact` 手动触发 + 自动阈值触发，9 段结构化摘要 |
| Cost Tracking | 按 Model 统计 Token 用量与成本 |
| Status Bar | 双行状态栏，上下文感知快捷键提示 |
| Input Handling | 多行输入，Ctrl+C 中断恢复，剪贴板粘贴，Unicode 宽字符支持 |
| Headless Testing | TestBackend 集成，事件注入，帧断言，配置隔离 |
| Keyboard Shortcuts | Shift+Tab 权限切换，Alt+M 模型切换，统一面板导航 |

## 组件库 (perihelion-widgets)

| Feature | Description |
|---------|-------------|
| BorderedPanel | 带边框装饰的面板容器 |
| ScrollableArea | 可滚动内容区域 |
| SelectableList | 可选中/高亮的列表组件 |
| InputField | 文本输入框，支持校验 |
| FormState | 多字段表单状态管理 |
| CheckboxGroup | 多选复选框组 |
| RadioGroup | 单选单选按钮组 |
| TabBar | Tab 导航栏 |
| SpinnerWidget | 加载旋转指示器 |
| MessageBlockWidget | 消息展示块 |
| ToolCallWidget | 工具调用渲染组件 |
| Markdown Rendering | Markdown 渲染 + 语法高亮，零内部依赖 |

## DAG 工作流引擎 (acpx-g)

| Feature | Description |
|---------|-------------|
| YAML Workflow | YAML 定义 DAG 工作流，节点/依赖/参数声明 |
| Node Types | Shell 命令 / HTTP 请求 / 子工作流引用 |
| Execution Engine | 拓扑排序，并行执行独立节点，依赖解析，错误传播 |
| Retry & Timeout | 节点级别重试配置与超时控制 |
| SQLite Persistence | sqlx 异步 SQLite，运行历史/节点日志/模板存储 |
| REST API | Axum Web API，模板列表/提交/状态/日志端点 |
| File Watcher | 目录监控，工作流文件变更自动重载 |
| CLI | 端口/工作流目录/数据库 URL 配置 |

## Langfuse 遥测客户端 (langfuse-client)

| Feature | Description |
|---------|-------------|
| Event Batching | 批量发送事件，提高网络效率 |
| Trace/Span/Generation | 完整追踪链路：Trace → Span → Generation |
| Token Reporting | Token 用量上报（input/output/total） |
| Metadata | 自定义元数据附加，用户/会话标识 |
| Auto Flush | Drop 时自动刷出缓冲区 |
| Env Config | LANGFUSE_HOST / PUBLIC_KEY / SECRET_KEY 环境变量配置 |

## 协议与兼容性

| Feature | Description |
|---------|-------------|
| Claude Code Compatible | 复用 `.claude/agents/` `.claude/skills/` CLAUDE.md，零迁移 |
| MCP Protocol | stdio + StreamableHTTP 传输，OAuth 2.0，工具发现/执行/资源读取 |
| OpenAI API | Chat Completion 兼容，流式响应，reasoning_effort |
| Anthropic API | Messages API，流式响应，thinking mode，图片支持 |
| rmcp Patch | 本地补丁修复 `UnexpectedContentType(None)` 错误 |

## 开发者体验

| Feature | Description |
|---------|-------------|
| Workspace Architecture | 6 crate workspace，resolver v2，依赖单向 |
| Error Handling | 库 crate 用 thiserror，应用层用 anyhow |
| Structured Logging | tracing 宏，JSON 格式，文件输出 |
| Git Hooks | lefthook pre-commit（fmt + check + clippy） |
| Headless Testing | TUI 无真实终端测试，配置路径隔离 |
| Release Profile | strip + LTO + opt-level=z 优化体积 |
