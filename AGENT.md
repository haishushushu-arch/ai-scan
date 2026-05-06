# ai-scan 产品与工程执行计划

## 1. 项目定位

`ai-scan` 是面向 msutools AI 中转站用户的跨平台桌面客户端。它不是单纯的环境检测脚本，也不是只有网页登录入口的壳，而是一个完整的“账号客户端 + API 管理 + 系统体检 + 一键修复 + 环境安装器”。

目标参考：

- CCSwitch：现代、清晰、轻量的桌面工具体验。
- 早期 360safe：普通用户能直接理解“体检、修复、完成”的路径。
- 3DMGame 运行库安装工具：缺少运行环境时，用户能一键安装或获得明确安装路径。

核心服务：

- 用户站点：https://www.msutools.cn
- 中转 API：基于 OpenAI-compatible `/v1` 接口，并兼容 Sub2API 现有能力。
- 桌面端必须支持现有用户登录、余额显示、充值入口、API Key 管理、模型/API 连通性检测。

## 2. 技术栈

首选技术栈：

- Desktop Shell：Tauri 2
- Frontend：React + TypeScript + Vite
- UI：自研系统工具风格组件，必要时引入 Radix/shadcn 思路但不照搬营销页风格
- Core：Rust
- Storage：本地加密/脱敏存储优先，扫描历史可用 SQLite 或 JSONL，敏感令牌不得明文写入普通日志
- Packaging：Tauri bundle，后续接入自动更新、代码签名和平台安装包

架构原则：

- React 只负责交互、展示、确认和进度。
- Rust 负责真实扫描、系统能力、网络诊断、文件配置读写、修复计划执行。
- Tauri command 必须是结构化 API，禁止暴露任意 shell 执行能力。
- 可复用扫描核心要尽量独立于 UI，后续可以抽成 CLI，方便客服远程排障。

## 3. 用户分层

普通用户：

- 年龄跨度按 8-80 岁设计。
- 首屏只需要理解三个状态：可以使用、需要修复、需要人工帮助。
- 主路径必须是“一键体检 -> 一键修复 -> 自动复检 -> 开始使用”。
- 默认不展示大段日志、命令、JSON、端口细节。

专业用户：

- 可以开启专业模式。
- 能查看每个结论的真实证据：命令输出、HTTP 状态码、DNS/TLS 结果、配置路径、请求耗时、错误体。
- 能导出脱敏报告、复制诊断命令、查看修复前后 diff。
- 能对单个检测项重新检测。

## 4. 信息架构

第一版主导航：

1. 首页体检
   - 一键体检
   - 当前账号状态
   - API 服务状态
   - 最近一次问题摘要

2. 我的账户
   - 登录/退出
   - 用户信息
   - 余额
   - 套餐或额度
   - 充值入口
   - 账单/消费入口，若服务端接口可用则内嵌展示，否则打开官网对应页面

3. API Key 管理
   - 查看当前可用 Key 列表
   - 新建 Key
   - 删除/禁用 Key
   - 复制 Key，复制动作必须有明确提示
   - Key 默认只展示脱敏值
   - 对选中的 Key 执行 `/v1/models`、chat、stream 测试

4. 环境体检
   - API Base URL 检测
   - DNS/TCP/TLS/HTTP 分层检测
   - 系统代理检测
   - 环境变量检测
   - Node/Python/Git/curl/Docker 检测
   - 系统时间和证书检测

5. 客户端配置
   - Cursor
   - VS Code
   - Cline
   - Continue
   - Cherry Studio
   - Open WebUI
   - Codex/Claude Code 等 CLI 工具
   - 默认只读发现，写入必须预览和备份

6. 修复中心
   - 可自动修复
   - 需要确认
   - 只能手动处理
   - 修复动作必须支持复检

7. 环境安装
   - 推荐安装
   - 自定义安装
   - Node.js LTS
   - Git
   - Python
   - Docker Desktop
   - WebView2
   - VC++ Runtime
   - 证书/代理相关工具
   - 第一阶段优先做检测、官方入口、安装计划，自动静默安装必须逐项验证

8. 专业模式
   - 系统
   - 网络
   - 代理
   - API
   - 客户端
   - 日志
   - 原始请求/响应，默认脱敏

9. 用户手册与文档
   - 普通用户手册
   - 首次使用流程
   - 常见错误排障
   - 专业模式说明
   - 客服报告解读
   - 每次功能变更必须同步更新相关文档

10. 官方网站
   - 产品介绍
   - 下载入口
   - 功能状态
   - 使用手册入口
   - 更新日志入口
   - 不得宣传尚未真实完成的能力

## 5. msutools/Sub2API 集成计划

所有接口必须通过真实探测、服务端文档或现有源码确认，不允许猜测后硬编码。

第一阶段需要确认：

- 登录方式：账号密码、邮箱验证码、OAuth、Token、Cookie 或现有 Sub2API 前端机制。
- 当前用户接口：用户 id、用户名/邮箱、角色、状态。
- 余额接口：余额、额度、到期时间、用量统计。
- 充值入口：是否有桌面可调用 API，若没有则打开官网充值页。
- API Key 管理：列表、新建、删除、更新、复制、额度/权限信息。
- 模型列表：`GET /v1/models` 或服务端自定义模型接口。
- API 测试：`POST /v1/chat/completions`，最小真实请求。
- 流式测试：SSE chunk 和 `[DONE]` 检测。

若接口需要登录态：

- 桌面端应使用系统 WebView 登录或结构化登录 API。
- Token/Cookie 必须进入系统安全存储或 Tauri 安全存储方案，不得写入普通日志。
- 诊断报告必须默认脱敏 Authorization、Cookie、API Key、refresh token、邮箱等敏感信息。

## 6. 扫描等级

Quick Scan：

- Base URL 格式规范化
- API Key 格式和脱敏
- DNS
- TCP
- TLS
- `/v1/models`
- 最小 chat completions

Stream Scan：

- `stream: true`
- SSE chunk
- `[DONE]`
- 代理缓冲和超时

Local Env Scan：

- OS/架构/shell
- PATH
- 系统代理
- WinHTTP/WinINET/macOS networksetup/Linux desktop proxy
- `HTTP_PROXY`、`HTTPS_PROXY`、`NO_PROXY`
- `OPENAI_API_KEY`、`OPENAI_BASE_URL`、`OPENAI_API_BASE`
- Node/npm/pnpm/yarn
- Python/pip
- Git
- curl/curl.exe
- Docker CLI/daemon
- 系统时间和证书

Client Scan：

- Cursor
- VS Code settings
- Cline
- Continue
- Cherry Studio
- Open WebUI
- 常见 `.env`、`settings.json`、`config.yaml`
- 只读检测优先，写入必须备份、diff 和确认

## 7. 修复动作原则

不允许边扫边改。流程必须是：

1. 扫描
2. 生成 Finding
3. 生成 Repair Plan
4. 展示影响、风险、动作、备份位置和是否需要管理员权限
5. 用户确认
6. 执行
7. 记录日志
8. 自动复检
9. 给出通过/失败证据

低风险可自动修复：

- 清理用户输入中的 `Bearer ` 前缀、空格、换行
- 规范化 Base URL，避免 `/v1/v1`
- 生成正确的环境变量命令
- 写入用户级环境变量，需确认
- 修复 Git/NPM 代理配置，需确认
- 生成 Continue/Cline/Open WebUI 配置片段，默认不直接覆盖

高风险或需要管理员权限：

- 修改系统代理
- 安装 Docker/Node/Git/Python
- 修改系统证书
- 停止进程或释放端口
- 写入客户端私有配置库
- 修改 Docker compose 或服务配置

这些动作必须逐项设计权限、回滚、备份和失败处理。

## 8. 真实检测红线

以下功能必须真实有效，不允许假 UI、假状态、假进度、假成功：

- 登录状态
- 余额
- 充值入口
- API Key 列表和管理动作
- `/v1/models`
- chat completions
- stream SSE
- DNS/TCP/TLS/HTTP
- 系统代理
- 环境变量
- 客户端配置文件
- Node/Python/Git/curl/Docker
- 端口占用和进程
- 修复动作结果
- 修复后的复检
- 导出的诊断报告

`/health` 只能证明健康接口可访问，不能单独证明 DB、Redis、账号池、上游模型或用户额度都正常。

网页 HTML 返回 200 不能证明应用可用，必须检查关键 JS/CSS 资源、登录路由、API 路由和实际接口。

## 9. UI/UX 验收标准

普通模式：

- 首页必须有唯一主按钮：开始体检。
- 扫描结果必须按影响程度排序。
- 错误说明必须使用普通用户能理解的语言。
- 每个问题必须告诉用户“影响什么”和“下一步做什么”。
- 不得默认展示日志墙。
- 所有按钮文案必须是动作，不用含糊词。

专业模式：

- 每个结论可展开证据。
- 支持复制命令、复制诊断摘要、打开日志目录。
- 支持查看原始请求/响应，敏感信息默认脱敏。
- 支持单项复检。
- 支持导出 JSON/Markdown/HTML 诊断报告。

视觉：

- 工具型界面，信息密度适中，不做营销页。
- 左侧导航 + 顶部账号区域 + 主工作区。
- 状态颜色保持克制：成功、警告、错误、信息四类。
- 卡片圆角不超过 8px。
- 不使用大面积单色渐变、装饰性光斑或无意义插画。
- 关键动作按钮必须易点，适合不熟悉电脑的用户。

## 10. 数据与安全

- API Key、Token、Cookie、refresh token、Authorization 默认脱敏。
- 账号凭据不得出现在普通日志和诊断报告中。
- 本地持久化敏感数据必须使用系统安全存储或加密存储。
- 导出报告默认不包含敏感原文。
- 任何“包含敏感信息”的导出必须显式确认。
- Tauri capability 必须最小化。
- 前端不能任意读写文件，路径必须由 Rust 白名单控制。
- 禁止任意 shell command API。

## 11. 推荐仓库结构

```text
ai-scan/
  AGENT.md
  package.json
  vite.config.ts
  tsconfig.json
  src/
    app/
    components/
    features/
      account/
      api-keys/
      dashboard/
      diagnostics/
      installers/
      professional/
      repair/
      settings/
    lib/
    styles/
  src-tauri/
    Cargo.toml
    tauri.conf.json
    capabilities/
    src/
      main.rs
      lib.rs
      commands/
      core/
      scanners/
      repairs/
      platform/
      storage/
      telemetry/
```

## 12. 第一阶段里程碑

M0：项目骨架

- Tauri 2 + React + TypeScript + Rust 可启动。
- 前端可调用 Rust command。
- 基础布局完成：左侧导航、顶部账号区、首页体检。
- 本地日志和脱敏工具存在。

M1：msutools 账号与 API

- 实测并确认登录/用户信息/余额/API Key 管理接口。
- 登录态安全存储。
- 展示余额和充值入口。
- API Key 列表、新建、删除或跳转管理页。
- 对选中 Key 执行真实模型/API 测试。

M2：一键体检

- DNS/TCP/TLS/HTTP 分层检测。
- `/v1/models` 检测。
- 最小 chat completions 检测。
- stream SSE 检测。
- 结果按影响程度排序。
- 每项有 status、severity、message、evidence、fix_suggestion。

M3：本机环境

- 系统信息。
- 代理。
- 环境变量。
- Node/Python/Git/curl/Docker。
- 系统时间和证书。
- 客户端配置只读发现。

M4：修复中心

- 低风险修复动作闭环。
- dry-run。
- 用户确认。
- 备份或回滚说明。
- 执行日志。
- 自动复检。

M5：环境安装

- 推荐安装清单。
- 缺失组件检测。
- 官方下载/安装入口。
- 后续逐项加入真实自动安装。

M6：报告与发布

- 脱敏诊断报告。
- 安装包。
- 平台构建验证。
- 自动更新和代码签名预留。

M7：文档、官网与发布资产

- `docs/` 维护普通用户手册、专业用户指南、排障文档和开发接口文档。
- `website/` 维护可直接打开或部署的现代化官网。
- `CHANGELOG.md` 记录用户可感知的变更。
- `RELEASE_TEMPLATE.md` 作为每次发布报告模板。
- `RELEASES/` 保存每次迭代的发布说明。
- 所有文档必须区分已实现、需要接口适配、规划中。

## 13. 子 agent 协作规则

主进程职责：

- 保持产品边界和架构一致。
- 决定接口契约。
- 统筹 worker 写入范围。
- 审核所有合并结果。
- 做最终构建、运行和验收。

Explorer：

- 只读研究产品、接口、代码或平台差异。
- 输出证据和建议，不改文件。

Worker：

- 负责明确文件范围内的实现。
- 不得改动其他 worker 的所有权文件。
- 不得回滚他人改动。
- 必须列出改动文件和验证结果。

任何 agent 都必须遵守：

- 不凭空假设接口。
- 不实现假数据冒充真实功能。
- 不将敏感信息写入日志。
- 不跳过错误处理。
- 不把失败包装成成功。
- 不发布与实际实现不一致的文档、官网文案或更新说明。
- 每次用户可见功能变化都要同步文档和发布记录。

## 13.1 Release discipline / capability honesty check

任何 agent 在修改官网、文档、发布记录或用户可见功能时，必须执行以下检查：

1. `docs/status.md` 是唯一完整功能状态页。状态变化先改这里，再同步 README、用户手册、官网、CHANGELOG 和 RELEASES。
2. 只有同时具备可见 UI 路径、真实命令或接口、验证过的响应形态，才能写为用户可用。
3. 账号、余额、充值和 API Key 管理必须经过 live 登录验证，才能从“接入中/部分可用”升级为“可用”。
4. 后端命令存在、页面入口存在、mock 数据、HTML 200、`/health` 正常，都不能单独证明功能可用。
5. 发布说明必须列出验证证据和不能承诺的内容；不能只写营销式摘要。
6. 没有实际构建和烟测过安装包时，官网必须写“暂无正式安装包”或等价表述。

文档发布红线：

- 不得宣传尚未真实完成的账号、余额、充值、Key 管理、一键修复、环境安装、chat completions 或 stream SSE 能力。
- 不得把“规划中”写成“即将自动可用”，也不得让普通用户以为重装或反复登录能解决接入中的功能。
- 不得在示例、截图、日志、报告或 release note 中出现未脱敏 API Key、Cookie、Token、Authorization、refresh token、账号密码或代理密码。
- 不得在多个文档维护互相独立的完整状态表；状态只链接 `docs/status.md`。
- 如果文档、官网、CHANGELOG、RELEASE_TEMPLATE 或 RELEASES 之间状态不一致，不允许发布。

## 14. MVP 完成定义

MVP 必须做到：

- 桌面应用可运行。
- 用户可以登录或进入明确的登录流程。
- 能显示真实账号/余额，若接口尚未确认则必须明确显示“需要接口适配”，不能伪造余额。
- 能进入充值入口。
- 能管理或跳转管理 API Key。
- 能用真实 API Key 检测 `/v1/models`。
- 能执行最小 chat completions。
- 能执行本机环境扫描。
- 能导出脱敏诊断报告。
- 至少有 3 个真实可执行或可生成命令的修复动作。
- 所有检测结果来自真实系统或真实 HTTP 请求。

## 15. 当前执行策略

第一步先建立可运行骨架和真实接口探测工具。

第二步确认 `https://www.msutools.cn` 当前前端和接口形态，形成 typed client。

第三步实现账号、余额、充值/API Key 管理入口。

第四步实现扫描核心和 UI。

第五步实现低风险修复闭环。

第六步打包验证。
