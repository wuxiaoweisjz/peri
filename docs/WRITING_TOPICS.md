# Peri 博客写作素材库

已有博客覆盖的主题不再列入：compact-mechanism、edit-tool、multi-agent-patterns、perf-optimization、web-search、prompt-cache-optimization、acp-separation、introducing-peri、streaming-render。

## Prompt Cache

- [x] Prompt Cache 命中率从 20% 爬到 98.5%——为什么 cache_control 的位置比内容更重要 → `prompt-cache-optimization`
- [x] 重启 Agent 后 Prompt Cache 全部失效？根因是 HashMap 迭代顺序在进程间不稳定 → `prompt-cache-optimization`
- [x] 一个中间件让所有对话首轮缓存全部失效——头部插入消息会悄悄移动缓存标记 → `prompt-cache-optimization`
- [x] MCP 工具列表一变，缓存就全失效——用边界标记把静态段（约 8K tokens）和动态段隔离开 → `prompt-cache-optimization`

## 内存管理

- [x] 内存分配器换了三次才找到正确答案——jemalloc、mimalloc 都没用，真正的问题是大对象每轮重建 → `perf-optimization`
- [x] Agent 每跑一轮多 40MB 内存——追踪到每轮 68 万次瞬态分配，根源是 HTTP 客户端每轮重建 → `perf-optimization`
- [x] session 级缓存复用如何让内存增长停下来——不是调分配器，是减少不必要的重建 → `perf-optimization`

## 工具调度

- [x] 并发工具调用的完整设计——批量审批、并发执行、延迟错误收集、统一写入；一个失败其他结果不丢，取消路径也不产生孤儿工具调用；连续 5 次同错误自动注入纠正消息防止 LLM 原地打转 → `concurrent-tool-dispatch`
- [ ] LLM 调工具时的名称和参数问题——三层工具名匹配（精确→大小写无关→语义别名 task/shell/reading）+ 参数名归一化（path→file_path），来自 1344 次调用的真实错误数据

## LLM 适配

- [x] 统一适配层兼容 10 家模型——Anthropic/OpenAI 之外的国产模型各有哪些不兼容之处 → `domestic-models-adaptation`
- [x] 推理内容如何在多轮对话中正确回传——不同模型用不同字段名，同时兼容两套格式 → `domestic-models-adaptation`
- [x] DeepSeek 每条 assistant 消息都要带回思考块，但注入的伪消息没有——400 的根因和修复 → `domestic-models-adaptation`
- [x] Kimi 的推理参数和推理强度参数不能共存——运行时检测模型名并移除冲突字段 → `domestic-models-adaptation`
- [x] 流式输出的 token 用量统计：Qwen 需要额外请求参数才会返回，其他家不需要 → `domestic-models-adaptation`
- [x] GLM 用两个不同的字段名返回同一个东西——解析端同时检查两个字段兼容历史版本 → `domestic-models-adaptation`

## 中间件与 ReAct 循环

- [ ] 17 个中间件链式组合的设计约束——为什么工具钩子不能在执行过程中读取对话历史
- [ ] Peri 的 ReAct 循环全貌——从用户输入到工具执行再到最终回答的完整流程
- [ ] Agent 执行危险操作前如何询问用户——HITL 审批机制、权限模式动态切换和 LLM 自动分类器
- [ ] 通过 MCP 协议把任意工具接入 Agent——连接池管理、OAuth 回调、断线重连的实现

## 错误处理与踩坑

- [ ] Ctrl+C 取消后 Agent 失忆——中断时无条件截断历史导致已完成的工作被丢弃
- [ ] 中断和完成事件的竞争条件——两个事件都想修改同一个状态，谁先到谁说了算
- [ ] 子 Agent 的输出为什么会在界面上越叠越多——状态没有在正确的时机清空
- [ ] 自定义 Slash 命令的隐蔽陷阱——绕过 Agent 循环的命令必须自己发完成信号，否则前端永远等待
- [ ] 恢复历史对话时，系统内部消息出现在界面上——过滤逻辑漏掉了持久化进数据库的内部消息

## TUI 与渲染

- [ ] 中文鼠标点击偏移：修了三次才真正修好——鼠标坐标是显示列宽，光标位置是字符索引，两者不是同一回事
- [x] 独立渲染线程解析 Markdown + 计算行包装，UI 线程只负责从缓存读取可见行重绘 → `streaming-render`
- [x] 流式输出自适应帧率：短消息 30fps，长消息降到 5fps，减少 CPU 空转 → `streaming-render`
- [x] 鼠标滚动事件合并：连续滚动只保留最后一个，避免单次滚动触发多次重绘 → `streaming-render`
- [ ] Markdown 表格里的中文列被压扁了——从等比缩放改为最小宽度优先的修复过程
- [ ] 双击 ESC 回滚对话：300ms 计时器如何区分"退出输入"和"触发回滚"两种意图

## 命令系统

- [ ] Slash 命令的三种执行模式——立即执行、透传给 LLM、参数转换后执行，各适合什么场景
- [ ] /rewind 如何回滚文件变更——截断对话历史的同时，逆向还原磁盘上所有被修改过的文件
- [ ] /bg 命令如何在后台跑独立任务——为什么故意不给后台 Agent 配置 MCP 工具

## 文件与安全

- [ ] 防止路径穿越攻击的三层校验——绝对路径拒绝、目录深度检测、解析后前缀验证，缺一不可
- [ ] Windows 上的路径分隔符问题——一个在 macOS 上完全正常的路径函数，在 Windows 上会悄悄产生反斜杠
- [x] Git 提交自动署名：追踪 Agent 的文件修改，commit 时附上 Co-Authored-By，支持 9 家模型的邮箱映射 → `domestic-models-work`

## 模式与运维

- [x] -p 非交互模式：不启动界面，执行完直接输出结果，支持 text/json/stream-json 三种格式 → `acp-separation`
- [x] 推理增强模式：Anthropic 和 OpenAI 的推理参数不同，如何统一控制思考深度 → `domestic-models-adaptation`
- [x] 一键安装脚本：自动检测平台和架构，绕过 GitHub API 限速，支持代理下载 → `riscv-peri`
- [ ] 对话历史如何持久化——SQLite 管理多会话、子线程和取消操作，断点续跑的实现
- [ ] 插件的三种作用域：用户级、项目级、本地级——安装、卸载和工具资源聚合的设计
- [ ] 给 Agent 加一个定时器——用 Cron 表达式注册定时任务，到点自动触发新一轮对话
- [ ] 把 LSP 语言服务接入 Agent——让 Agent 能调用代码补全、定义跳转、实时诊断

## Side Project

- [ ] git-graph：在终端里可视化 Git 分支历史——拓扑排序布局、分支着色、三栏视图展示 stash/remote/status
