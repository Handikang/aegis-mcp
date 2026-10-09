# Aegis-MCP 核心引擎实现计划

> **Goal:** 构建轻量级、高并发的 Rust 单二进制 MCP 安全网关，实现零延迟透明反向代理、凭证防泄漏(DLP)、危险命令拦截与 1-Click Auto-Wrap。
> **Architecture:** Stdio 双向透明代理管道 + 内存零拷贝 JSON-RPC 2.0 评估引擎 + 正则/Aho-Corasick 威胁规则库 + CLI 自动接管器。
> **Tech Stack:** Rust 2024, Tokio, Serde/Serde_json, Clap, Regex, Colored, Dirs.

---

### Task 1: JSON-RPC 2.0 与 MCP 协议模型 (`src/protocol.rs`)
- **Files:**
  - Create: `src/protocol.rs`
- **Interfaces:**
  - `JsonRpcRequest`: 解析客户端发出的 JSON-RPC 请求（包含 `method`, `params`, `id`）。
  - `ToolCallParams`: 解析 MCP 的 `tools/call` 请求（提取 `name` 与 `arguments`）。
  - `JsonRpcError`: 构造包含规范指导提示词的 `PermissionDenied` 错误响应。
- [ ] **Step 1:** 编写协议解析与错误构造的单元测试
- [ ] **Step 2:** 实现协议结构体与序列化/反序列化逻辑
- [ ] **Step 3:** 运行 `cargo test protocol` 确保测试通过

---

### Task 2: DLP 与命令安全防火墙引擎 (`src/firewall.rs`)
- **Files:**
  - Create: `src/firewall.rs`
- **Interfaces:**
  - `SecurityRule`: 包含规则名称、风险等级（Critical/High/Medium）、检测正则与阻断说明。
  - `FirewallEngine`: 管理默认规则集（SSH私钥、.env凭证、破坏性删除、管道数据外发）。
  - `evaluate(tool_name, arguments) -> EvaluationResult`: 毫秒级返回阻断或放行判决。
- [ ] **Step 1:** 编写覆盖各种真实 Prompt Injection 与泄密 Payload 的测试用例
- [ ] **Step 2:** 实现高性能多规则评估引擎
- [ ] **Step 3:** 运行 `cargo test firewall` 确保测试通过

---

### Task 3: Stdio 双向透明拦截代理 (`src/proxy.rs`)
- **Files:**
  - Create: `src/proxy.rs`
- **Interfaces:**
  - `run_proxy(upstream_cmd: &str, upstream_args: &[String]) -> Result<()>`:
    - 异步拉起下游真实的 MCP 子进程
    - 截获宿主客户端标准输入：若是 `tools/call` 且命中高危规则，向终端输出彩色告警，并直接向客户端返回阻断响应，不透传给下游
    - 正常请求与响应双向无感知转发
- [ ] **Step 1:** 编写 Stdio 模拟转发与阻断测试
- [ ] **Step 2:** 实现 Tokio 双向流式转发主循环
- [ ] **Step 3:** 运行 `cargo test proxy` 验证转发与阻断行为

---

### Task 4: 一键无感接管与无损回滚引擎 (`src/wrapper.rs`)
- **Files:**
  - Create: `src/wrapper.rs`
- **Interfaces:**
  - `wrap_configs() -> Result<WrapSummary>`: 自动扫描 Claude Desktop / Cursor 配置文件，安全备份后注入 `aegis proxy --`。
  - `unwrap_configs() -> Result<UnwrapSummary>`: 从 `.aegis_bak` 备份文件 100% 还原原始配置。
- [ ] **Step 1:** 编写配置文件备份与替换的单元测试
- [ ] **Step 2:** 实现跨平台路径嗅探与 JSON 包装器
- [ ] **Step 3:** 运行 `cargo test wrapper` 验证备份与恢复正确性

---

### Task 5: CLI 命令集成与攻击模拟器 (`src/cli.rs`, `src/main.rs`)
- **Files:**
  - Create: `src/cli.rs`
  - Modify: `src/main.rs`
- **Interfaces:**
  - `aegis proxy`: 启动透明代理
  - `aegis wrap`: 一键接管所有客户端
  - `aegis unwrap`: 一键恢复配置
  - `aegis simulate-attack`: 15 秒终结演示，输出震撼的极客彩色终端告警
- [ ] **Step 1:** 实现 Clap 命令行分发与美化日志
- [ ] **Step 2:** 实现 `simulate-attack` 演示子命令
- [ ] **Step 3:** 编译完整项目并运行 `cargo run -- simulate-attack` 验证体验

---

### Task 6: 爆款 GitHub README 与开源文档 (`README.md`)
- **Files:**
  - Create: `README.md`
- **Interfaces:**
  - 极具视觉冲击力的 ASCII Banner、Badges、架构图与 15 秒攻击演示场景。
- [ ] **Step 1:** 编写专业高质感英文 README，打造极客范
