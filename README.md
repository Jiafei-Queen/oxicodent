# Oxicodent — 基于 Markdown 代码块的轻量级 Agent 架构方案

[![许可证：MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)  [![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

## ⚠️ WARNING / 警告

```
本项目正在开发中，该应用现阶段 并不支持实质性的开发工作
```

## 技术细节
### 1. 核心交互协议：隐式工具调用 (Implicit Tool Use)
放弃严苛的 JSON Schema，利用模型对 Markdown 代码块的天然生成能力。
- **```exec**: 拦截并执行 Shell 命令（ls, cargo build, npm test 等）。
- **```write:path/to/file**: 直接拦截块内内容并写入指定物理路径。
- **```read --lines 100-200**: 按需读取文件片段，避免大文件撑爆上下文。
- **```diff**: 用于代码修改
- **```remem <ID>**: 调取历史存档中的详细执行结果或代码 Diff。

### 2. 上下文管理策略：三层过滤机制
- **动态状态注入 (Heartbeat)**：每轮对话开头自动注入当前 CWD（工作目录）、文件树简报和环境变量。
- **结果标记**：让用户将 **代码错误** 和 **执行结果错误** 进行 **标记**，帮助模型 **快速回顾** 并 **避免错误**。
- **执行结果折叠 (Rolling Feedback)**：仅保留最近 1-2 次 `exec` 的完整输出。旧输出自动压缩为单行摘要：`[ID 5] exec: 'cargo test' (Unpassed: 因为代码并未通过检查) - hidden`。
- **行号增强 (Line-Numbered Context)**：所有读取的代码片段强制带上行号，大幅提升模型编写 `patch` 或 `diff` 的精准度。

### 3. 记忆系统：二级缓存与“考古”能力
- **Memory Bank (内存/硬盘索引)**：维护一个结构化存储，记录所有操作。
- **ID 引用机制**：模型输出 `remem 5` 时，从持久化存储中提取 [ID 5] 的完整日志并重新喂给模型。
- **Token 杠杆**：用极短的索引摘要（几十 Token）锚定海量的历史数据（几千 Token）。

### 4. 持久化与人为干预 (State-as-a-File)
- **Session YAML**：将整个对话状态、记忆索引、项目目标持久化为可读的 `.agent_session.yaml`。
- **断点续传**：重启程序后，模型通过读取 YAML 恢复“工程记忆”。
- **上帝视角 (Human-in-the-loop)**：用户可直接手动修改 YAML 文件中的“总结”或“目标”，纠正模型偏离的逻辑。
- **主动索取**：模型会在 用户需求模糊，项目意图不明确，API 接口不明确 等情况下，向用户索取信息，让 模型 拥有一定主导权

### 5. Rust 拦截器实现路径
- **正则/流式解析**：使用 `regex` 或状态机实时扫描模型输出流中的代码块标识。
- **交互式确认**：在执行 `exec` 前暂停流输出，等待用户 `y/n` 授权。
- **反馈回填**：将执行后的 Stdout/Stderr 封装为特定的 `--- Execution Result ---` 标记位，作为下一轮 User Prompt 发送。

---
### 架构优势
1. **模型兼容性**：Qwen2.5-Coder 等 7B/14B 模型即可流畅运行。
2. **Token 效率**：通过主动压缩和按需调取，极大延长了有效上下文寿命。
3. **开发成本**：无需处理复杂的 Tool-calling API，只需字符串处理和进程控制。