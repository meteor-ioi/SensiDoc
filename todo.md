# SensiDoc 开发任务清单与进度跟踪 (todo.md)

> 本文档实时记录 SensiDoc 项目各模块的开发实施进度，严格按检查点验证推进。  
> 标记说明：`[ ]` 待办 | `[-]` 进行中 | `[x]` 已完成并通过测试  
> 💬 **上下文会话导航**：[🔗 点击跳转到本次聊天对话记录 (Conversation ID: 28ffd366-73fb-426f-bf1e-99dd72430155)](conversation://28ffd366-73fb-426f-bf1e-99dd72430155)

---

## 阶段一：M1 - Rust 基座与 anydoc 解析引擎
- [x] 初始化 Rust Cargo 工程（Axum + Tokio + Tower-HTTP + AnyDoc 本地依赖）
- [x] 创建 `bin/` 目录并将本地 `llama-server` 软链接/复制至内置目录，验证执行权限
- [x] 封装 `converter.rs`：集成 `anydoc` 实现多格式文档（Word/PDF/PPTX/Excel/TXT）字节流转 Markdown
- [x] 编写并跑通 `converter` 单元测试与集成测试
- [x] 搭建 Axum 基础 HTTP 服务与静态资源静态托管路由

## 阶段二：M2 - llama-server 进程守护与 ModelScope 下载器
- [x] 封装 `model_manager.rs`：实现 `llama-server` 子进程拉起与停止
- [x] 核心健壮性：实现 `Drop` trait、全局信号捕获（SIGINT/SIGTERM）与端口健康探测，彻底杜绝僵尸进程
- [x] 实现魔搭（ModelScope）3 款预设离线模型下载器（支持 HTTP Range 断点续传与 SSE 实时进度）
- [x] 实现本地现有 `.gguf` 模型扫描与载入功能
- [x] 编写测试脚本验证子进程异常销毁机制

## 阶段三：M3 - 敏感信息提取引擎、Chunking 与快照落盘
- [x] 封装 `extractor.rs`：构建长文本分块机制 (Chunking，避免超 4096 tokens)
- [x] 实现 OpenAI 兼容协议向 `llama-server` 发送抽取请求与动态 System Prompt 组装
- [x] 实现正则表达式兜底提取（手机号、身份证、邮箱等常见 PII）
- [x] 实现位置冲突消解算法（消解正则与 LLM 抽取区间的重叠冲突）
- [x] 封装 `session.rs`：实现文档历史提取快照（Snapshot）数据结构与本地文件落盘（持久化）

## 阶段四：M4 - Geist-Neutral 网页端界面构建
- [x] 编写 `web/style.css`：严格落地 `DESIGN_SYSTEM-v1.md`（Geist-Neutral 极简、1px 边框、脉冲动效）
- [x] 编写 `web/index.html`：三栏式布局（左侧文件与快照树、中间可折叠规则与预览区、右侧敏感信息清单）
- [x] 实现设置模态框（包含预设模型一键下载进度条、本地模型切换）
- [x] 实现文件拖放多选投放区与历史快照展开树
- [x] 编写 `web/app.js`：Markdown 渲染与基于 `TreeWalker` 的纯文本节点 `<mark>` 安全高亮插桩
- [x] 实现双向联动：右侧敏感词卡片点击循环轮转跳转 + 背景两次脉冲 Flash 闪烁

## 阶段五：M5 - 一键导出与整机综合联调
- [x] 封装 `exporter.rs`：支持敏感词清单导出（CSV / JSON）
- [x] 实现脱敏文档导出（一键马赛克打码原文档并下载为 Markdown / TXT）
- [x] 全链路压测与长文档/多文件批量并发测试
- [x] 编写 `run.sh` 一键启动脚本与使用文档

---

## 阶段六：用户体验与模板管理细节优化 (已完成)
- [x] 6.1 顶部导航与模型状态联动：按钮更名为“设置”，支持 `/api/models/active` 实时同步当前运行的模型名称
- [x] 6.2 本地外部 GGUF 模型选取与导入：设置模态框支持选取本地 `.gguf` 模型文件导入并一键启动
- [x] 6.3 敏感词选中持久高亮：右侧选中敏感词卡片时，中间预览区目标项保持持久高亮聚焦状态，不随动画结束消失
- [x] 6.4 场景模板与字段标签库持久化：
  - 支持将当前规则保存为自定义场景模板
  - 支持将字段保存为“标签库”，新增字段时支持从标签库快速点选填充
  - 模板与标签库统一落盘持久化到 `.sensidoc_workspace.json`
- [x] 6.5 综合测试与验证回归：已通过 API 与落盘 JSON 全链路验证通过

---

## 阶段七：界面布局与标签呈现细节优化 (已完成)
- [x] 7.1 **常用字段标签库下拉菜单化**：
  - 将原先平铺的标签栏重构为标准下拉选择框 (`#fieldTagsSelect`)，包含占位符 `-- 选择并添加标签 --` 及包含中文风险等级的选项（如 `绝密密钥 [高危]`）。
  - 用户点选后即刻自动将标签追加至规则表格，并自动复位下拉菜单。
- [x] 7.2 **重构“新增空白字段”按钮布局**：
  - 移除了原先右上角易产生视觉混淆的按钮。
  - 在规则表格正下方统一部署整行虚线边框的「`+ 新增空白字段`」操作按钮，契合从上至下按需填表的操作心智。
- [x] 7.3 **全界面去英文字样与中文规范化**：
  - 规则表格与标签库中的风险等级彻底去除中英混用，统一为纯中文：`高危`、`中危`、`低危`。
- [x] 7.4 **代码整洁度与回归验证**：
  - 清理 `main.rs`、`session.rs`、`exporter.rs` 中的未使用导入与警告，`cargo check` 与前端联动测试全部通过。

---

## 阶段八：Prompt 调优、变量注入与输出规范支持 (已完成)
- [x] 8.1 **吸收小模型最佳实测经验重塑提示词**：
  - 参考 [`最佳系统提示词 - qwen2.5-coder-1.5b.md`](file:///Users/icychick/Desktop/测试提取/最佳系统提示词%20-%20qwen2.5-coder-1.5b.md) 落地**三步结构化思维协议**（第一步：识别与精准截取；第二步：过滤与排重；第三步：规范聚合输出）。
  - 参考 [`最佳系统提示词 - lfm2.5-vl-450m.md`](file:///Users/icychick/Desktop/测试提取/最佳系统提示词%20-%20lfm2.5-vl-450m.md) 落地**纯正向规则与变量插槽机制**，杜绝大段具象示例引发的小模型复读与失焦。
- [x] 8.2 **后端支持自定义 Prompt 与 `{FIELDS_DEFINITION}` 插槽注入**：
  - 在 [`src/extractor.rs`](file:///Users/icychick/Projects/SensiDoc/src/extractor.rs) 中实现 `build_system_prompt(fields, custom_template)` 与 `format_fields_definition`。
  - 用户自定义 Prompt 支持通过 `{FIELDS_DEFINITION}` 动态注入当前启用的字段名与描述。
  - 在 [`src/main.rs`](file:///Users/icychick/Projects/SensiDoc/src/main.rs) 中新增 `/api/rules/prompt/preview` 实时预览接口，并在 `/api/extract` 中接收 `custom_prompt`。
- [x] 8.3 **输出格式规范与超参数加固**：
  - 明确定义并展示底层 JSON Schema 契约（`[{"text":"...","category":"...","risk_level":"..."}]`）。
  - 本地推理调用强制锁定 `temperature: 0.1` 极低随机性，确保提取准确率与 JSON 合法性。
- [x] 8.4 **网页端「Prompt 调优」专属面板构建**：
  - 规则栏标题右侧新增「**Prompt 调优**」按钮。
  - 弹出专属模态框 (`#promptModal`)：包含可自由编辑修改的提示词文本域、完整 System Prompt 实时预览区、底层输出格式 Schema 展示、恢复默认模板按钮以及「应用此提示词并测试提取」按钮。
- [x] 8.5 **全链路编译与端到端回归验证**：
  - 跑通 9 项 Cargo 单元与集成测试（`cargo test -- --nocapture` 100% 通过）。
  - 完成 Release 版本编译构建（`cargo build --release`），并通过 API 验证了 Prompt 预览与动态提取功能。

---

## 阶段九：界面布局与快照/原始数据交互升级 (已完成)
- [x] 9.1 **模型状态迁移至中间预览区底部状态栏**：
  - 从顶部右上角移除 `activeModelStatus`，统一部署在中间 Markdown 预览面板底部信息栏（`.preview-footer-bar`）。
  - 搭配动态健康状态指示灯（`.status-dot`），运行中呈现绿色常亮，未启动时呈现灰色，界面更加平衡协调。
- [x] 9.2 **敏感词清单增加“原始 JSON 数据”查看器**：
  - 在右侧「敏感词清单」头部新增「`原始 JSON`」操作按钮。
  - 弹出专属代码查看模态框 (`#rawJsonModal`)，支持格式化查看当前快照的全部原始数据（包含精确偏移量、坐标、置信来源、风险等级等），并支持「一键复制 JSON」。
- [x] 9.3 **左侧文件项历史提取版本下拉选择化**：
  - 彻底解决版本切换无效的问题：将每个文档的提取版本重构为轻量原生下拉框（`<select class="snapshot-select">`）。
  - 下拉框实时展示 `#1版 17:34` 等直观信息，用户切换选项即可直接回放任意历史版本的正文高亮与右侧审计清单，并同步落盘记录当前活跃快照。
- [x] 9.4 **全链路端到端功能验证与回归**：
  - 前端 3 项交互与后端 Rust 服务的持久化全链路验证完毕，新版本已完成构建并在后台保持热运行。

---

## 阶段十：左侧文件列表两层卡片布局与历史折叠面板重构 (已完成)
- [x] 10.1 **文件列表项重构为双层卡片结构**：
  - 参考右侧敏感词卡片的开阔疏朗风格（两层卡片布局，去除压抑感）：
    - 第一层：加粗优雅展示完整文件名（带全称悬浮与截断省略）；
    - 第二层：展示文件字符数字符串（`376 字符`）、默认激活的最新版本胶囊标签（如 `#3版最新`），以及右侧直观的向下展开箭头（`▶ / ▼`）。
- [x] 10.2 **实现展开折叠面板展示历史提取记录（弃用下拉框）**：
  - 点击卡片右侧的展开箭头，平滑向下展开嵌入式历史面板；
  - 面板内逐项清晰展示历史提取快照：快照序号（`#1版`）、提取模板名、提取耗时、命中项数与时间戳；
  - 点击任意历史快照项，即刻将该快照切换为当前活跃状态（加粗+背景微高亮），并在中间 Markdown 预览区与右侧敏感词清单同步回放该快照的高亮与审计结果。
- [x] 10.3 **Geist-Neutral 样式精细化与全链路回归校验**：
  - 彻底去除下拉框的局促突兀感，左侧侧边栏视觉层次与呼吸感显著提升；
  - 9 项单元测试与集成测试全部通过，Release 产物成功部署。
- [x] 10.4 **折叠指示图标细化优化**：
  - 将实心播放三角形图标替换为精致的 SVG 细线 Chevron 箭头（`>`）；
  - 折叠展开时通过 CSS `transform: rotate(90deg)` 平滑转向正下方（`v`），符合主流桌面应用操作直觉。

---

## 阶段十一：Markdown 网页排版与精美表格样式渲染 (已完成)
- [x] 11.1 **Marked 编译器完全本地离线化**：
  - 将外部 jsdelivr CDN 资源替换为本地内置的 `web/marked.min.js`，彻底杜绝外网弱网或无网环境下 CDN 脚本加载失败导致退化为纯文本的问题。
- [x] 11.2 **Geist-Neutral 现代文档排版系统**：
  - 针对 `.markdown-body` 注入标题分级下划线（h1/h2）、段落行距优化、引用块（blockquote）、等宽代码行（inline code）与代码块（pre/code）样式。
- [x] 11.3 **精美现代网页表格样式（Table）落地**：
  - 实现现代化卡片感表格：带边框与圆角、沉降面表头（`th`）、浅色隔行条纹（zebra stripe）、行悬浮微高亮（hover）、等宽数值对齐，彻底替代生硬的纯文本表格展示。
- [x] 11.4 **TreeWalker 安全敏感词高亮全兼容**：
  - 经回归验证，基于 TreeWalker 的纯文本节点高亮插桩能够完美兼容表格 `<td>` 内的文本拆分，不破坏 `<table>`、`<tr>`、`<td>` 的任何 DOM 结构。

---

## 阶段十二：空模板默认载入机制与端侧小模型输出格式深度评估 (已完成)
- [x] 12.1 **导入新文件默认载入空模板**：
  - 前端改造 `loadRulePresets` 与 `handleFilesUpload`：新文件导入后默认不载入任何预设场景模板，下拉框自动选中 `-- 自定义空模板 (自行添加) --` 并呈现空规则表格，留待用户自主按需配置字段。
- [x] 12.2 **小模型输出格式与遵循度深度调研与盲测归纳**：
  - 启动专业研究子代理，深入分析 `/Users/icychick/Desktop/测试提取/` 目录下的实测盲测数据与《小模型训练策略建议.md》；
  - 明确瓶颈：强制要求 450M~1.5B 小模型输出三元组 `[{"text":"...", "category":"...", "risk_level":"..."}]` 会分散其在正文中搜寻实体的注意力，容易诱发格式破损或漏抽。
- [x] 12.3 **提出架构解耦方案并完成后端弹性双模兼容落地**：
  - 推荐最优解构：**模型端输出极简扁平纯字符串数组 `["词1", "词2"]` + 后端规则引擎自动分类与风险映射**；
  - 在 [`src/extractor.rs`](file:///Users/icychick/Projects/SensiDoc/src/extractor.rs) 中完成双模动态解析兼容：无缝支持对象数组 `[{"text":...}]` 与纯字符串数组 `["..."]`，确保无论选用哪种提示词风格均可 100% 稳健解析。
- [x] 12.4 **归档沉淀技术报告**：
  - 调研评估成果与架构解耦方案已正式保存为项目 Markdown 文档：[`SMALL_MODEL_BENCHMARK_REPORT.md`](file:///Users/icychick/Projects/SensiDoc/SMALL_MODEL_BENCHMARK_REPORT.md)。


---

## 阶段十七：方案C - 右侧一体化智能审计工作台布局改版 (已完成)
- [x] 17.1 **拆分任务计划与明确验收标准**：
  - 中间主舞台彻底去除规则折叠表格，实现 100% 纯净 Markdown 正文阅读视界；
  - 右侧栏重构为一体化智能审计工作台（Segmented Tabs: 提取规则 / 敏感清单）；
  - 输入输出全流程在右侧单手闭环，点击「立即提取」自动平滑切至结果清单；
  - 正文与审计清单双向联动无缝跳转并支持自动切换选项卡。
- [x] 17.2 **DOM 结构重构 (`web/index.html`)**：
  - 中间 `.main-stage` 剔除 `#rulesAccordion`，仅保留纯净正文阅读工具条与 Markdown 画布；
  - 右侧 `.audit-panel` 重塑为 `.inspector-panel`，集成 Segmented Control（`#tabRulesBtn`、`#tabAuditBtn`）与双 Tab 容器（`#rulesPane`、`#auditPane`）；
  - 保留并优雅对齐模态框（设置、Prompt 调优、原始 JSON）。
- [x] 17.3 **现代工作台视觉系统 (`web/style.css`)**：
  - 调整三栏网格比例（`240px minmax(420px, 1fr) 350px`），保证右侧从容排布；
  - 编写分段控制器、紧凑型规则卡片、常用标签推荐池、主动作触发按钮与状态切换动效；
  - 完整适配深浅双色主题（Geist-Neutral）。
- [x] 17.4 **交互与状态流转驱动 (`web/app.js`)**：
  - 接入 Segmented Tabs 切换逻辑；
  - 升级 `renderRulesTable` 为现代卡片流式渲染（支持开关、字段名、风险评级、描述、存标签与删除）；
  - 快捷标签池点击快速注入（1ms 极速追加）；
  - 点击「立即提取」并在完成后自动无缝切到「敏感清单」并高亮首项；
  - 正文中的敏感词点击反向联动触发自动切至「敏感清单」并居中定位。
- [x] 17.5 **全链路端到端功能验证与回归测试**：
  - 验证规则新增、删除、开关、模板保存、标签复用；
  - 验证 AI 提取、正文高亮联动、历史快照回放、CSV/脱敏导出；
  - 无头浏览器截图核验视觉质感与双色表现，Cargo 9 项测试 100% 通过。
- [x] 17.6 **界面视觉精简与前端基础 Bug 修复**：
  - 顶部导航栏：移除冗余徽标「Offline Local AI」，界面更纯粹紧凑；
  - 中间工具条：新增「预览 / 源码」极简左右拨杆开关，支持在 Markdown 渲染高亮与纯源码文本之间无缝切换；
  - 底部操作栏：规范操作按钮为纯中文居中显示「立即执行提取」，去除冗余英文与多余修饰；
  - 修复 `web/app.js` 中 `promptSaveCustomTemplate` 与 `saveRuleAsTag` 未定义导致的初始化阻断异常。
- [x] 17.7 **面板文案、提示词弹窗精炼对齐与卡片流空白消除**：
  - 文案统一：右侧底部按钮及配置弹窗标题由「Prompt 调优」统一重命名为「提取规则设定」；
  - 弹窗精炼：精简提示词模态框顶部冗长描述（单行聚焦于加固离线小模型与 `{FIELDS_DEFINITION}` 插槽传递）；
  - 弹窗对齐：修复 textarea 宽度未满问题（`width: 100%; box-sizing: border-box;`），实现上下区块完全等宽对齐；
  - 空白消除：移除 `.rule-card-list` 硬编码 `max-height: 380px`，通过 `flex: 1; min-height: 0;` 自适应充满右侧栏，彻底消除卡片列表与底部工具栏之间的 80px+ 冗余空隙。
- [x] 17.8 **场景模板与常用标签全生命周期删除支持**：
  - 后端接口：新增 `DELETE /api/rules/templates/{id}` 与 `DELETE /api/rules/tags/{name}` 路由，支持从 `.sensidoc_workspace.json` 中物理持久化删除；
  - 标签删除：常用标签库中每个标签芯片（tag-chip）右侧追加微型独立叉号（`✕`），悬停淡红提示，点击二次确认一键删除；
  - 模板删除：场景模板下拉选择框选中自定义模板时，标题栏右上角动态呈现红色「删除此模板」按钮，方便随时清理无用模板。
- [x] 17.9 **正文工具栏标题绝对居中与冗余提示清理**：
  - 工具栏重构：将正文工具栏重塑为 `position: relative` + `left: 50%; transform: translateX(-50%)`，确保文档主标题处于整个工作舞台严格的几何中心（实测几何偏差为 0px）；
  - 信息分流：左侧自适应显示完整历史快照信息（含模板名、时间与耗时），右侧为预览/源码拨杆；
  - 视觉提纯：移除「提示：点击正文敏感词可反向联动右侧定位」冗余文字，阅读视界更加干净开阔。
- [x] 17.10 **彻底移除早期内置硬编码场景模板**：
  - 清空后端 `Extractor::get_rule_presets()` 中写死的两个旧模板（`个人隐私信息保护 (PII)`、`商业机密与商务合规`）；
  - 场景预设模板完全清爽交由用户自行根据业务需要另存与管理，不再有无法删除的系统冗余模板干扰。
- [x] 17.11 **设置面板多板块重构与离线模型快捷控制条**：
  - 设置面板板块化：将系统设置拆分为「🤖 离线模型」与「⚙️ 提取规则与提示词」双板块选项卡；
  - 交互迁移降噪：将原本挤占在右侧栏底部的「提取规则设定」和「清空当前规则」收纳至设置面板，释放核心审计工作区纵向空间；
  - 提取按钮上方新增离线模型状态条（`.model-control-strip`）：
    - 状态指示灯三态表现：🟢 绿色亮点（模型运行中已就绪，带柔和光晕）/ ⚪ 灰色亮点（模型未启动）/ 🔴 红色亮点（离线进程故障或服务通信异常）；
    - 集成极简 Toggle 硬件质感拨杆开关，支持一键拉起或停止离线模型进程，状态与开关双向同步响应。
- [x] 17.12 **静默启动体验优化与中间预览底部信息栏重构**：
  - 移除模型启动成功的阻断式 `alert` 弹窗，实现无感平滑切换，完全由状态条绿灯与开关呈现；
  - 中间面板底部信息栏职责解耦：彻底剥离模型相关信息，全面赋能正文内容预览；
  - 实时预览统计条：精准展示当前文档名、总字数、命中字段/总处数、快照版本与执行耗时；
  - 文本字号缩放控制组：新增「A- / 100% / A+ / 重置」微型控制组件，支持 70% ~ 180% 动态字号缩放，在渲染预览与 Markdown 源码视图中实时自适应联动。
- [x] 17.13 **模型启停 Null 报错彻底修复与标题栏/底部信息降噪去重**：
  - 根因修复：在之前移除预览底栏旧模型文字节点后，`syncActiveModelStatus` 与 `startLlamaModel` 中残留直接对 `el.activeModelStatus.innerText` 赋值导致 `Cannot set properties of null`；已全面引入安全守卫判空逻辑，并在底部控制条正常无误同步；
  - 顶部标题栏纯净化：彻底移除顶部左侧的快照信息文案（如 `正在查看历史快照...`），顶部标题栏仅保留水平几何绝对居中的文档大标题与右侧拨杆；
  - 底部信息栏去重：移除重复的文件名显示，仅呈现字符数、命中数、快照版本与字体缩放控制，视觉更加紧凑专业。
- [x] 17.14 **重启服务与 Markdown 渲染预览失效彻底排查修复**：
  - 终止所有历史僵尸进程（包括旧 sensidoc 主服务与残留的 llama-server），释放端口并全新拉起；
  - 排查并修复前端 JS 阻断错误：在 `applySnapshotView` 中因模板提取优化时遗留了 `ReferenceError: tName is not defined` 异常，导致后续的 `renderMarkdownWithHighlights` 和 TreeWalker 无法执行；
  - 补齐 `tName` 作用域定义，经自动化 CDP 验证，Markdown 语法与 10 处敏感词 `<mark>` 高亮插桩已 100% 完整恢复。
- [x] 17.15 **标题栏与底部信息栏布局重构与冗余去重**：
  - 标题名纯净化：移除顶部标题栏中文件名后面的 `(951 字符)` 后缀，杜绝与底栏字符统计重复；
  - 工具栏左右重组：
    - 「预览 / 源码」拨杆移动至标题栏左侧（`.stage-toolbar-left`）；
    - 正文绝对居中文档标题保持严格居中（`.stage-toolbar-center`）；
    - 字体缩放控制组「A- / 100% / A+ / 重置」移动至标题栏右侧（`.stage-toolbar-right`），常驻顶部触手可及；
  - 底部信息栏极致精简：移除底部信息栏右侧缩放容器，仅纯粹呈现正文审计统计指标（字符数 · 字段/命中数 · 快照版本与耗时）。

---

## 阶段十八：左侧栏全栏拖拽响应与投放区极简化重构 (已完成)
- [x] 18.1 **移除常驻投放区块降低视觉噪点**：
  - 彻底移除 HTML 中常驻占位的 `#dropzone` 静态提示框（释放顶部约 80px 宝贵纵向高度）；
  - 将空间 100% 归还给核心业务（文档卡片与历史提取快照树）。
- [x] 18.2 **实现全侧边栏拖拽响应与动效反馈**：
  - 监听整个 `.sidebar` 的拖拽事件，支持盲操拖放（符合费茨法则）；
  - 引入 `sidebarDragCounter` 深度计数器与 `pointer-events: none` 浮层，彻底解决拖拽经过子元素时的高频闪烁问题；
  - 拖入时触发内边框高亮（`2px dashed var(--accent)`）与柔和全栏磨砂遮罩提示（“松开鼠标导入文档”）。
- [x] 18.3 **优雅补齐列表空状态占位 (Empty State)**：
  - 当无任何文档时，列表区域呈现轻量居中的虚线占位引导（支持点击导入与拖拽提示）；
  - 一旦导入文档，空状态自动隐入后台，列表纯粹清爽展示卡片流。

---

## 阶段十九：方案一 - 文档排序、关键词检索与格式筛选一体化工具条 (已完成)
- [x] 19.1 **后端数据结构升级与默认时间正序**：
  - 在 `src/session.rs` 的 `DocumentItem` 结构中新增 `created_at: DateTime<Utc>` 时间戳字段，并在落盘/加载中保持向后兼容；
  - `upsert_document` 自动记录创建时间，`list_documents` 接口默认切换为按 `created_at` 正序排序（最早添加在最前，契合直觉）。
- [x] 19.2 **方案一常驻一体化工具条前端构建**：
  - HTML & CSS：在侧边栏头部下方紧凑部署 `.sidebar-toolbar`，高度契合 Geist-Neutral 极简 1px 细线美学；
  - 集成搜索输入框（`.sidebar-search-input`，带搜索图标与一键清空 `✕` 按钮）；
  - 集成极简原生排序选择器（`docSortSelect`，支持时间最早/最新、文件名升降序、字数多至少/少至多）；
  - 集成横向平滑扩展名快捷筛选胶囊组（`.filter-pill`，涵盖 `全部`、`DOCX`、`PDF`、`XLSX`、`PPTX`、`TXT`、`CSV` 等）。
- [x] 19.3 **实时检索、过滤与排序逻辑响应**：
  - 键入搜索词实时过滤文件名（不区分大小写，毫秒级即时响应）；
  - 点击扩展名胶囊快速过滤特定类型文档；
  - 切换排序方式即刻无缝重排卡片树；
  - 搜索/筛选无结果时展示微型居中友好提示（“未找到匹配文档”）。



## 阶段二十：左右面板按需黄金宽度与自由拖拽调宽 (已完成)
- [x] 20.1 **左右侧面板默认黄金宽度对齐**：
  - 左侧「文件列表」面板初始宽度设定为紧凑适中的 `250px`（杜绝过宽占位，视觉清爽舒适）；
  - 右侧「提取规则与审计工作台」面板默认保持 `350px` 充裕宽度（确保规则表单与敏感词列表从容排布）；
  - 采用 CSS 变量 `--sidebar-width: 250px` 与 `--inspector-width: 350px` 弹性驱动 Grid 网格轨道。
- [x] 20.2 **双向拖拽调整条（Panel Resizer）落地**：
  - 在 HTML 中为左侧与中间舞台、中间舞台与右侧面板之间分别部署微型拖拽手柄（`#resizerLeft`、`#resizerRight`）；
  - CSS 落地极简 1px 细线、悬浮时加粗呈现 `var(--accent)` 高亮，鼠标悬浮呈现 `col-resize` 光标，拖动中在 `body` 级别施加防选中文本与强制光标。
- [x] 20.3 **防变形阈值保护与记忆持久化**：
  - 最小/最大安全限制：
    - 左侧面板宽度限制：`min: 220px, max: 480px`；
    - 右侧面板宽度限制：`min: 280px, max: 550px`；
    - 中间主舞台保留弹性视界底线：`min: 380px`，杜绝挤压变形；
  - 本地记忆持久化：拖拽松手后自动将双方宽度落盘至 `localStorage`，刷新页面自动保持自定义布局偏好。

---

## 阶段二十一：历史快照版本控制中心重构 (底栏时光机与左侧解耦) (已完成)
- [x] 21.1 **左侧文件卡片轻量化与折叠树剥离**：
  - 彻底移除左侧卡片内的嵌入式展开箭头（`.file-expand-btn`）与折叠历史快照树面板（`.snapshot-panel`）；
  - 卡片第二行仅保留轻量状态标签：已提取时展示如 `3个快照` 胶囊，未提取时展示 `未提取`；
  - 释放左侧 250px 纵向高度，恢复左侧栏纯粹的文件导航心智。
- [x] 21.2 **中间底栏升级为可交互的快照版本时光机（Pop-up Menu）**：
  - 将原静态纯文本 `#previewStatsSnap` 升级为互动式药丸按钮（如 `[ ⏱️ #1版最新 (19:18) ▴ ]`）；
  - 编写精美的向上浮层菜单（`.snapshot-popup-menu`），包含版本序号（`#1版`）、提取模板、敏感词命中项数、耗时与时间戳；
  - 点击底栏任意历史版本即刻无缝回放正文高亮与右侧审计清单，并高亮当前活跃版本标记（`✓`）；
  - 支持点击空白处或按 Esc 自动收起菜单。
- [x] 21.3 **样式打磨、无头浏览器回归核验与全链路测试**：
  - 适配 Geist-Neutral 极简主题；
  - 验证多文档切换、快照回放、新提取产生新快照后的实时联动；
  - 无头浏览器截图核验视觉质感。

---

## 阶段二十二：快照时光机置顶标题栏与字体缩放归位底栏 (已完成)
- [x] 22.1 **快照时光机上移至中间标题栏右侧**：
  - 迁移 `#snapshotPickerWrapper` 至 `.stage-toolbar-right`，与居中文档大标题在同一视觉水平线上；
  - 下拉菜单方向自适应改造：从原先的向上弹出（`bottom`）调整为向下展开（`top: calc(100% + 8px); right: 0;`），朝左下方优雅展开，带平滑淡入动效；
  - 箭头指示符由 `▴` 调整为 `▾`，激活时平滑旋转 180°。
- [x] 22.2 **正文字体缩放控制组归位至底栏右侧**：
  - 将「A- / 100% / A+ / 重置」控制组移动至 `.preview-footer-right`；
  - 底栏布局权责明确：左侧专注正文审计统计指标（字符数 · 字段/命中数），右侧常驻字号缩放微调组件。

---

## 阶段二十三：快照执行模型名称持久化与底栏居中展示 (已完成)
- [x] 23.1 **后端数据结构扩展与提取时模型自动绑定**：
  - 在 `src/session.rs` 的 `ExtractionSnapshot` 结构体中新增 `model_name: Option<String>` 字段，持久化落盘并保证兼容；
  - 在 `src/main.rs` 的敏感词提取链路中，自动探测并捕获当前实际执行提取的模型文件名（如 `LFM2.5-VL-450M-Q4_K_M.gguf` 或 `正则规则引擎`），写入快照数据记录中。
- [x] 23.2 **中间底栏绝对居中部署模型信息徽章**：
  - 在中间 Markdown 预览面板底栏部署 `.preview-footer-center`（`position: absolute; left: 50%; transform: translateX(-50%)`），实现几何绝对居中对齐；
  - 徽章精美呈现「🤖 模型: LFM2.5-VL-450M-Q4_K_M」，未提取时显示「模型: 未提取」；
  - 快照版本下拉菜单同步展示该历史版本所使用的模型名称提示，方便后期精细化对比与排查各模型提取效果。

---

## 阶段二十五：主工作台底部一体化模型下拉选择与启停控制 (方案一落地) (已完成)
- [x] 25.1 **界面结构升级与样式对齐（方案一）**：
  - 将 `.model-control-strip` 改造为一体化模型快速选择与启停容器；
  - 增加状态指示灯（`.status-indicator-dot`）、模型下拉选择框（`<select id="footerModelSelect" class="footer-model-select">`）与右侧启停拨杆（`#footerModelToggle`）；
  - 下拉框样式与整体系统表单风格保持一致（1px 细线边框、Geist 规范），支持展示就绪的本地 GGUF 模型及运行状态标识。
- [x] 25.2 **多模型列表动态拉取、实时切换与状态联动**：
  - 动态拉取本地 `models/` 目录下的所有可用 GGUF 模型与预设已就绪模型，填充至 `#footerModelSelect`；
  - 用户切换下拉选项时，若当前模型正在运行，可平滑无缝热切换或选择目标后拨动开关启动；
  - 联动「设置」面板内部的模型导入与下载操作，一旦有新模型入库实时更新下拉列表；
  - 解决无需打开繁琐设置弹窗即可在提取主界面自由切换并运行模型的诉求。
- [x] 25.3 **无头浏览器视觉验证与交互测试**：
  - 截图核验底部模型控制栏在亮暗模式与紧凑面板下的对齐效果；
  - 验证模型启动、停止、切换及提取结果中的模型名称联动。

---

## 阶段二十六：按钮文本居中对齐与脱敏导出机制规范 (已完成)
- [x] 26.1 **全系统按钮文本居中对齐排版强化**：
  - 在 `web/style.css` 的基础 `.btn` 组件中显式注入 `justify-content: center; text-align: center;`；
  - 彻底解决「导出清单 (CSV)」、「脱敏导出文档」及「← 返回修改规则重新提取」等弹性伸缩宽度按钮在 Flex 布局下文本偏左的问题，达成严谨的水平居中。
- [x] 26.2 **脱敏文档导出机制与排版保持原理梳理与交付**：
  - 详细剖析当前「脱敏导出文档」的执行逻辑与局限；
  - 给出能否保持源文档（如 Word/PDF/Excel）默认排版格式的权威技术分析与未来扩展方案。

---

## 阶段二十七：系统提示词面板 Tab 分段极简化重构 (已完成)
- [x] 27.1 **文案与层级极简化**：
  - 将原「系统提示词模板 (可自由编辑修改)：」精简重构为简洁直观的「编辑系统提示词」；
  - 将原「当前变量注入后的完整 System Prompt 预览：」精简重构为「系统提示词预览」；
  - 移除了长文本堆叠排版，降低用户的视觉与认知负荷。
- [x] 27.2 **分段选项卡（Tab）无缝切换交互落地**：
  - 将编辑区与预览区改为轻量分段 Tab 切换拨杆（`#tabPromptEditBtn` 与 `#tabPromptPreviewBtn`）；
  - 默认展示「编辑系统提示词」，需要核验变量注入效果时一键切到「系统提示词预览」实时预览，杜绝两块大文本框纵向重叠的拥挤感；
  - 当处于编辑 Tab 时优雅展示「恢复默认模板」辅助操作，处于预览 Tab 时自动隐藏，操作界面高度聚焦。

---

## 阶段二十八：设置面板固定 450px 高度与视口滚动适配 (已完成)
- [x] 28.1 **设置模态框尺寸与弹性容器规范**：
  - 将设置控制中心模态框默认固定高度调整为 `450px`（`height: 450px; display: flex; flex-direction: column;`）；
  - 标题栏 `modal-header` 设定 `flex-shrink: 0;` 保持常驻置顶，关闭按钮清晰可见；
  - 各选项卡面板（`#paneSetModel`、`#paneSetRules`）设定 `flex: 1; overflow-y: auto;`，内部内容丰富时优雅纵向平滑滚动，杜绝因内容膨胀导致弹窗被撑长变形的问题。

---

## 阶段二十九：提示词编辑框与预览框固定尺寸等高对齐 (已完成)
- [x] 29.1 **输入框与预览框几何尺寸与样式统一**：
  - 抽象 `.prompt-area-box` 规范类，设置固定 `height: 175px; width: 100%;`；
  - 统一应用 `box-sizing: border-box; font-family: var(--font-mono); font-size: 11px; line-height: 1.55; padding: 10px 12px;`；
  - 设置 `textarea` 的 `resize: none;` 禁止手动拖拽拉伸产生尺寸差异；
  - Tab 在「编辑系统提示词」与「系统提示词预览」之间切换时，几何边界保持完全静止，视觉零跳动。

---

## 阶段三十：系统设置中心新增“🎨 外观与显示”板块 (已完成)
- [x] 30.1 **主题模式切换功能（跟随系统 / 浅色模式 / 深色模式）**：
  - 在设置面板顶部 Tab 新增 `🎨 外观与显示`（`#tabSetAppearanceBtn`）；
  - 提供 💻 跟随系统、☀️ 浅色模式、🌙 深色模式 三项卡片式直观切换；
  - 采用标准 `data-theme` 属性无缝覆盖原有 `@media (prefers-color-scheme)` 规则，切换零延迟；
  - 状态持久化存入 `localStorage`，重新打开应用自动记忆并载入。
- [x] 30.2 **全局 UI 显示缩放比（100% ~ 300% 全自适应等比放大）**：
  - 提供连续平滑滑动条（100% ~ 300%，步长 5%）与快速阶梯选择 Pills（100%, 110%, 120%, 130%, 150%, 175%, 200%, 250%, 300%）；
  - 采用浏览器标准等比缩放规范（`zoom: var(--ui-scale)` 与 CSS 自适应变量），使得整个 SensiDoc 界面（顶部栏、左右侧面板、拖拽把手、按钮、字号、弹窗）等比锐利放大；
  - 缩放时各区域依旧保持严格的 Flexbox / Grid 弹性约束，无变形、无文字重叠，解决默认 100% 偏小偏紧的问题；
  - 缩放配置自动保存在本地，即改即显。

---

## 阶段三十一：主界面单按钮主题循环切换与 100%~150% 纯按钮缩放 (已完成)
- [x] 31.1 **主界面“设置”前置一键主题切换按钮**：
  - 在主界面顶部工具栏右侧、设置按钮前部署单个快捷按钮 `#topThemeToggleBtn`；
  - 点击单按钮即可在「💻 跟随系统 ➔ ☀️ 浅色模式 ➔ 🌙 深色模式」三态之间循环轮转切换；
  - 按钮文案与图标实时动态联动，零遮挡、零门槛。
- [x] 31.2 **设置面板主题按钮紧凑尺寸优化**：
  - 将外观面板中原大面积方块卡片重构为 32px 紧凑标准按钮，图标文字横向居中排列，视觉整洁不突兀。
- [x] 31.3 **UI 缩放体验极简化（移除滑动条，范围 100% ~ 150% 纯按钮）**：
  - 移除冗长滑块，仅保留 6 个常用阶梯快捷切换 Pill（`100% (默认)`、`110%`、`120%`、`130%`、`140%`、`150%`）；
  - 锁定 100% ~ 150% 黄金可读性区间，确保无论怎么缩放，三栏工作区始终完美自适应。

---

## 阶段三十三：前端 UI 全面引入 Lucide 矢量图标并替换 Emoji (已完成)
- [x] 33.1 **落地零外部网络依赖的按需内联 Lucide SVG 规范（方案 A）**：
  - 在 [`web/style.css`](file:///Users/icychick/Projects/SensiDoc/web/style.css) 中规范定义 `.lucide-icon`（涵盖默认 14px、`.sm` 12px、`.xs` 10px、`.lg` 16px 等多规格），严格对齐 `stroke: currentColor`，完美自适应明暗主题与文本颜色。
- [x] 33.2 **全站关键板块与按钮前置矢量图标**：
  - **顶部导航区**：品牌 Logo 增加 `ShieldCheck` 盾牌图标，设置按钮增加 `Settings` 齿轮图标，一键主题循环切换按钮全面改用 `Monitor` / `Sun` / `Moon` 矢量线条。
  - **左侧文档栏**：标题新增 `Files` 图标，导入文件按钮新增 `Upload` 图标，搜索清空与排序切换采用 `X` / `ArrowUpDown` 图标。
  - **中间主阅读视界**：预览与源码拨杆前置 `Eye` 与 `Code` 图标，快照时光机前置 `Clock` 图标，底部执行模型标识前置 `Cpu` 芯片图标。
  - **右侧审计控制台**：分段 Tab 前置 `SlidersHorizontal`（提取规则）与 `ClipboardCheck`（敏感清单）图标；场景预设、常用标签库、生效规则各板块标题分别增设对应辨识度图标；立即执行提取按钮前置 `Play` 图标，清单导出与脱敏导出分别前置 `Download` 与 `ShieldCheck` 图标。
- [x] 33.3 **设置中心与动态交互状态 Emoji 彻底清退**：
  - 系统设置模态框三大导航 Tab 采用 `Cpu` / `SlidersHorizontal` / `Palette` 矢量图标；
  - 设置面板中主题选择按钮（跟随系统、浅色、深色）全部使用 Lucide SVG 替代 Emoji；
---

## 阶段三十四：高比例 UI 缩放视口溢出修复与底部动作栏全适配 (已完成)
- [x] 34.1 **排查缩放溢出根因 (CSS Zoom vs Viewport Height)**：
  - 根因定位：在现代浏览器引擎下，`zoom: > 1` 时 `100vh` 仍以物理视口计算，导致在放大坐标系内元素高度超出可视区域；
  - 此外 `.app-layout` 原采用 `calc(100vh - 44px)` 固定计算，在缩放倍数提升后被成倍拉长，使得底部底栏被顶出屏幕下方。
- [x] 34.2 **弹性高度与 Flex 截断重构 (零溢出标准)**：
  - 将 `html`、`body` 改为标准 `height: 100%`，`body` 采用 `flex-direction: column`；
  - 顶部导航栏 `app-header` 设置 `flex-shrink: 0`；三栏工作区 `.app-layout` 改为 `flex: 1; min-height: 0;`；
  - 在 `.main-stage`、`.preview-container`、`.inspector-panel`、`.tab-content`、`.rules-pane-body`、`.audit-list`、`.sidebar` 关键弹性容器均补齐 `min-height: 0`；
- [x] 34.3 **全档位缩放无缝自动化验证 (100% ~ 200%)**：
  - 经由 Headless Chrome 针对 100%、115%、125%、150%、175%、200% 全量缩放档位及多条规则长列表进行测量：
---

## 阶段三十五：设置面板导航居中重构与模型下载文案精简 (已完成)
- [x] 35.1 **移除设置面板“系统控制中心”冗余标题并绝对居中选项卡**：
  - 移除 `.modal-header` 中原有的 `h3` 标题文案；
  - 重构 `.modal-header` 布局为 `position: relative; justify-content: center;`，使三大功能 Tab（离线模型 / 提取规则与提示词 / 外观与显示）完美水平居中对称；
  - 将“关闭”按钮通过绝对定位锚定在右侧，维持视觉平衡与极简现代感。
- [x] 35.2 **精简模型下载板块提示与按钮文案**：
  - 将原“魔搭 ModelScope 预设模型（点击一键流式下载）：”更新为“魔搭平台下载离线模型：”；
  - 将模型列表操作按钮文案由“魔搭下载”重命名为更加直观聚焦的“一键下载”。

---

## 阶段三十六：图标衍生设计系统评估与架构决策 (已完成)
- [x] 36.1 **应用 Icon 视觉基因提炼与原型验证**：
  - 基于应用新图标（3D 柔白文档 + 赛博透视目镜）提炼设计要素，并构建独立原型页面进行实机体验；
- [x] 36.2 **明确设计系统边界与决策定稿**：
  - **评估结论**：衍生设计系统色彩过多过杂，容易破坏政企/安全审计场景所需的高严谨度与低视觉噪点；
  - **架构决策**：彻底放弃引入高饱和/复杂色彩体系，坚定维持现有的 **`Geist-Neutral` 极简黑白灰中性色设计系统**（1px 细线、高对比度、无杂色干扰、极致阅读专注度）；
  - 清理临时验证原型文件，保持代码库精简干净。

---

## 阶段三十七：全平台官方应用图标生成与资产归档 (已完成)
- [x] 37.1 **基于 UI 盾牌矢量图元提取与 512×512 渲染**：
  - 精准提取 UI 品牌标志 `Lucide ShieldCheck`（盾牌校验）矢量图元，完成像素级抗锯齿与多规格高精度渲染；
- [x] 37.2 **视感平衡优化（大盾牌比例适配）**：
  - 将中央盾牌占比由 52% 提升至 64.3%（270px/320px），大幅强化小尺寸（Dock/任务栏/快捷方式）下的识别度与视觉张力；
- [x] 37.3 **跨平台规范与样式定稿**：
  - **最终定稿风格**：选定与 Geist 极简系统高度契合的“白瓷微渐变底座 + 纯黑大盾牌线条”方案；
  - **macOS 平台**：`sensidoc_mac.png`（512×512，macOS 官方规范 Squircle 超椭圆圆角与微阴影）；
  - **Windows 平台**：`sensidoc_win.png`（512×512，Windows 11 Fluent 规范圆角，全画幅无外缩 padding）；
- [x] 37.4 **根目录精简化与资产归档**：
  - 根目录仅保留简明别名 `sensidoc_mac.png` 和 `sensidoc_win.png`；
  - 历史长文件名、深色版与透明背景矢量版等素材全部归档至 `assets/icons/` 目录。

---

## 阶段三十八：长文本分块滑动窗口 (Overlap) 与跨块幂等消解 (已完成)
- [x] 38.1 **长文档分块滑动重叠窗口 (Overlap Window) 落地**：
  - 在 [`src/extractor.rs`](file:///Users/icychick/Projects/SensiDoc/src/extractor.rs) 中重构文本分块器，支持 `chunk_text_with_overlap`；
  - 默认保留 `200` 字符重叠滑动窗口，在换块时自动回溯保留上一块末尾完整的自然行作为新块的前置语境上下文，彻底消除临界敏感实体的上下文断裂（如前置字段名丢失）；
  - 针对极端超长无换行单行增加字符级滑动切分防溢出安全机制。
- [x] 38.2 **跨块重复实体全局幂等去重与绝对坐标回填**：
  - 充分复用 `merge_and_resolve` 的全局机制，通过 `full_text.match_indices(&ai_item.text)` 在完整全篇正文中全局重新定位所有真实出现点，完全不依赖分块相对坐标；
  - 实现集合级幂等去重与子串重叠消解，无论同一实体在重叠区被提取多少次，均自动归并为单一条目并准确回填全部正文位置。
- [x] 38.3 **单元测试覆盖与全链路回归验证**：
  - 新增 `test_chunking_with_overlap`（验证分块重叠区自然行保留）与 `test_duplicate_ai_items_resolution`（验证多 Chunk 重复提取实体的幂等归并与全文 3 处绝对坐标回填）；
  - 11 项 Cargo 单元测试 100% 通过（`cargo test` 全绿）。

---

## 阶段三十九：UI 细节优化与模型交互引导流程加固 (已完成) · [🔗 对话跳转](conversation://8373d05b-4a9b-4cb4-b124-1e2d655d8a6a)
- [x] 39.1 **规则卡片风险等级标签前置排版**：
  - 将生效规则卡片中的风险等级徽标（`中危/高危/低危`）移动至右侧操作区「存标签」按钮前面，设置 `flex-shrink: 0` 和 `gap: 6px`，显著提升视线右对齐的规整度。
- [x] 39.2 **魔搭模型下载完成按钮文案规范**：
  - 将魔搭平台已下载模型的按钮文案从「载入并启动」统一规范为「载入启动」。
- [x] 39.3 **空规则列表引导文案精炼**：
  - 空规则列表提示文案简化为「点击上方标签或「＋ 新增规则」添加规则」。
- [x] 39.4 **一键提取前置模型自动引导与拉起**：
  - 点击「立即执行提取」时，若未下载任何离线模型，弹窗引导前往设置面板下载或导入；
  - 若模型已存在但未启动，自动变更为 `正在启动模型...` 启动选中的模型后再无缝执行混合提取。
- [x] 39.5 **工具品牌标题重构**：
  - 网页标题与启动 Banner 统一更新为「`SensiDoc - 离线信息审计与脱敏工具`」。

---

## 阶段四十：端侧小模型（450M~1.5B）指令隔离、协议重塑与全字段精准检出 (已完成) · [🔗 对话跳转](conversation://8373d05b-4a9b-4cb4-b124-1e2d655d8a6a)
- [x] 40.1 **抗指令污染防线 (`<document>` 隔离包裹)**：
  - 在 [`src/extractor.rs`](file:///Users/icychick/Projects/SensiDoc/src/extractor.rs) 的系统提示词中增加 `<document>` 隔离包裹与安全声明，彻底消除合同正文末尾出现的“请提取合同中的所有企业名称”等测试指令对端侧小模型的指令污染。
- [x] 40.2 **结构化对象数组输出协议升级与字段纯净化**：
  - 将系统提示词输出协议由扁平字符串数组升级为结构化字段映射 `[{"field": "字段名", "text": "原文原词"}]`；
  - 在 `{FIELDS_DEFINITION}` 中剥离风险等级字样噪音，消除模型注意力分散，让端侧小模型 100% 聚焦在实体特征抽取上。
- [x] 40.3 **后端动态解析容错与多级别名上下文消解**：
  - 重构 `query_llm` 反序列化为动态 `Value` 解析，全面兼容对象数组、键值字典、别名映射等各种模型输出形态；
  - 升级 `merge_and_resolve`，支持精确字段匹配、模糊别名匹配及原文上下文回退匹配。
- [x] 40.4 **实测验证**：
  - 《合同1.docx》与《合同2.docx》在 LFM-450M 和 Qwen2.5-1.5B 下均实现 6/6 目标字段 100% 精准检出。

---

## 阶段四十一：模型实时状态双向探针、设置说明同步与全场景一键关闭 (已完成) · [🔗 对话跳转](conversation://8373d05b-4a9b-4cb4-b124-1e2d655d8a6a)
- [x] 41.1 **后端 8081 端口实时动态探针**：
  - 在 [`src/model_manager.rs`](file:///Users/icychick/Projects/SensiDoc/src/model_manager.rs) 中，`get_active_model` 与 `get_presets` 升级为异步主动探测本地 8081 端口获取当前实际承载模型，彻底根除页面刷新后内存状态丢失导致的“状态显示为未开启”问题。
- [x] 41.2 **设置面板提示词输出格式说明同步刷新**：
  - 设置面板左下角静态说明由旧版 `极简纯字符串数组 ["词1", "词2"]` 同步更新为 `结构化实体对象数组 [{"field": "字段名", "text": "原文"}]`。
- [x] 41.3 **全链路一键关闭与双重进程释放保障**：
  - 主界面底部 Toggle 拨杆开关支持一键关闭当前模型，释放 8081 端口与显存，指示灯自动恢复灰色离线状态；
  - 设置面板内运行中的模型展示为红色「关闭运行」，点击直接停止当前模型；
  - 后端 `stop_server` 引入多重进程清理与 `pkill` 机制，彻底杜绝孤儿进程与端口占用。

---

## 阶段四十二：文档与快照全生命周期删除支持及快照日期时间增强 (已完成) · [🔗 对话跳转](conversation://8fbdd9e2-6055-43cb-a0b5-748e43cd77d5)
- [x] 42.1 **左侧文档列表每个文件支持删除**：
  - 在 [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js) 的文档卡片顶部右侧增加垃圾桶删除图标（`.doc-delete-btn`），采用与右侧面板一致的 `.delete-btn` 样式与 Lucide Trash 图标；
  - 点击时阻止事件冒泡并弹窗二次确认，调用 `deleteDocument()` 请求 `DELETE /api/documents/{id}`；
  - 后端在 [`src/session.rs`](file:///Users/icychick/Projects/SensiDoc/src/session.rs) 与 [`src/main.rs`](file:///Users/icychick/Projects/SensiDoc/src/main.rs) 中实现文档删除与异步刷盘；若删除的是当前激活文档，自动切换至下一文档或优雅重置为空状态。
- [x] 42.2 **日志历史快照支持下拉菜单内逐项删除**：
  - 在快照下拉菜单 [`renderSnapshotPopupMenu()`](file:///Users/icychick/Projects/SensiDoc/web/app.js) 中为每个快照项右侧增加垃圾桶删除图标（`.snap-delete-btn`）；
  - 点击二次确认后调用 `deleteSnapshotRecord()` 请求 `DELETE /api/documents/{id}/snapshot/{snapshot_id}`；
  - 后端在 `delete_snapshot()` 中安全移除快照，自动将激活快照回退至最新剩余快照或重置未提取状态，并同步持久化落盘与侧边栏快照徽章。
- [x] 42.3 **日志快照增加完整日期与时间展示**：
  - 新增 `formatSnapshotDateTime()`，将快照 ISO 时间统一格式化为 `YYYY-MM-DD HH:mm`（例如 `2026-09-01 12:34`）；
  - 中间底栏时光机胶囊与下拉菜单列表中均统一呈现完整日期与时间，彻底解决无法区分跨日期快照的问题；
  - 在 [`web/style.css`](file:///Users/icychick/Projects/SensiDoc/web/style.css) 中将下拉菜单宽度由 `320px` 调整为 `390px`，确保模板名、命中数、完整时间与删除图标等排版舒适无换行。
- [x] 42.4 **单元测试与语法全链路验证**：
  - 在 `src/session.rs` 中新增 `test_document_and_snapshot_lifecycle` 单元测试，覆盖文档与快照的增删及激活回退状态机；
  - 12 项 Cargo 单元测试 100% 通过（`cargo test` 全绿），`node -c web/app.js` 语法验证通过。

---

## 阶段四十三：界面中央通用二次确认模态框重构 (全面替代浏览器原生弹窗) (已完成) · [🔗 对话跳转](conversation://8fbdd9e2-6055-43cb-a0b5-748e43cd77d5)
- [x] 43.1 **中央二次确认模态框组件 (`#confirmModal`)**：
  - 在 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 中构建居中对话框，包含圆形危险/提示图标徽标、标题、换行说明及取消/确认按钮组；
  - 在 [`web/style.css`](file:///Users/icychick/Projects/SensiDoc/web/style.css) 中定义 `.btn.danger` 与 `.confirm-modal-box`，严格遵循 Geist-Neutral 极简中性色与磨砂遮罩规范。
- [x] 43.2 **Promise 化通用调用驱动与键盘无障碍交互 (`showConfirmDialog`)**：
  - 在 [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js) 中封装 `showConfirmDialog({ title, message, confirmText, cancelText, isDanger }) -> Promise<boolean>`；
  - 自动聚焦主操作按钮，支持按 `Enter` 快捷确认、按 `Esc` 或点击半透明遮罩背景取消。
- [x] 43.3 **全系统 100% 清退浏览器原生 `confirm(...)` 弹窗**：
  - 文档删除、快照删除、场景模板删除、常用标签删除、规则字段删除、清空规则列表及未启动模型引导等全部 7+ 处确认交互统一切换为界面中央模态框。
## 阶段四十四：快照提取规则审查面板构建与两款端侧小模型提示词深度迭代评测 (已完成) · [🔗 对话跳转](conversation://a2de2406-6bc8-410f-b0b1-cc993b75ced8)
- [x] 44.1 **快照系统提示词数据持久化与「提取规则」审查 UI 落地**：
  - 后端数据结构：在 [`src/session.rs`](file:///Users/icychick/Projects/SensiDoc/src/session.rs) 的 `ExtractionSnapshot` 中扩展 `system_prompt: Option<String>` 字段，`add_snapshot` 接收并持久化落盘；
  - 提取链路固化：在 [`src/main.rs`](file:///Users/icychick/Projects/SensiDoc/src/main.rs) 的 `/api/extract` 中将组装得到的完整 System Prompt 写入快照数据中；
  - 前端 UI 按钮：在中间阅读面板 `.stage-toolbar-right` 中紧随快照时光机下拉菜单新增「提取规则」按钮（`#viewSnapshotRulesBtn`），无快照时智能禁用，有快照时激活；
  - 提示词审查模态框：新增 `#snapshotRulesModal`，完整展示当前快照执行的模型、模板、时间、耗时、启用字段定义清单以及本次提取实际使用的完整「系统提示词」，并支持一键复制提示词；
  - 12 项 Cargo 单元测试 100% 通过，前端 JS 语法校验通过。
- [x] 44.2 **构建企业信息审计与脱敏 10 大典型场景测试基准集**：
  - 编写 [`benchmark_data.py`](file:///Users/icychick/Projects/SensiDoc/tests/enterprise_benchmark/benchmark_data.py)，覆盖采购合同、高管保密协议、财务报销、安全事件应急、特权运维交接、离职资产交接、外包投标标书、客户KYC档案、内部合规调查、SLA服务协议 10 份高保真测试文档；
  - 明确标注每个测试文档的待提取目标字段定义（名称、描述、风险等级）与真实 Ground Truth 敏感实体标准答案。
- [x] 44.3 **Qwen2.5-1.5B vs LFM2.5-VL-450M 双轨提示词迭代对比评测**：
  - 针对端侧小模型“漏提取”核心痛点，设计多轮提示词优化策略（Baseline默认版 vs 纯正向实体扫描版 vs 结构化分步协议版 vs 字段增强描述版）；
  - 编写 [`run_benchmark.py`](file:///Users/icychick/Projects/SensiDoc/tests/enterprise_benchmark/run_benchmark.py)，自动化导入 10 份测试文档并调用 llama-server 执行真实提取，80 组测试结果已全部作为快照写入持久化；
  - 精确统计各模型、各提示词版本下的精确率 (Precision)、召回率 (Recall) 和 F1 值。
- [x] 44.4 **深度技术归因、最佳提示词沉淀与评测报告交付**：
  - 归纳端侧 450M~1.5B 小模型漏提取的底层机理（注意力分散、语义泛化弱、指令污染、格式负荷）；
  - 提炼两款模型各自的最佳系统提示词模板与字段定义编写黄金规范（Qwen2.5-1.5B 推荐 V4 极简版，F1 91.20%；LFM2.5-450M 推荐 V1 结构加固版，F1 85.71%）；
  - 正式输出完整技术报告：[`ENTERPRISE_SMALL_MODEL_PROMPT_BENCHMARK_REPORT.md`](file:///Users/icychick/Projects/SensiDoc/ENTERPRISE_SMALL_MODEL_PROMPT_BENCHMARK_REPORT.md)。

---

## 阶段四十五：模型专属最佳提示词自动联动与内置基准测试寻优引擎 (已完成) · [🔗 对话跳转](conversation://a2de2406-6bc8-410f-b0b1-cc993b75ced8)
- [x] 45.1 **模型专属提示词配置库与持久化支持**：
  - 在 [`src/session.rs`](file:///Users/icychick/Projects/SensiDoc/src/session.rs) 中新增 `ModelPromptProfile` 结构体，并在 `WorkspaceStore` 中增加 `model_prompt_profiles` 映射落盘持久化；
  - 实现默认模型智能匹配规则（Qwen 系列默认匹配 V4 极简版 F1 91.2%，LFM 系列默认匹配 V1 结构加固版 F1 85.7%）；
  - 在 [`src/main.rs`](file:///Users/icychick/Projects/SensiDoc/src/main.rs) 中新增 `GET /api/models/{filename}/prompt` 与 `POST /api/models/{filename}/prompt` 路由。
- [x] 45.2 **内置企业基准测试集与自动寻优引擎 (`src/benchmark.rs`)**：
  - 在 Rust 后端封装 [`benchmark.rs`](file:///Users/icychick/Projects/SensiDoc/src/benchmark.rs)，内置 10 份高保真测试文档与 4 种典型提示词模板（V1 基准版、V2 扫描版、V3 三步协议版、V4 极简版）；
  - 新增 `POST /api/models/{filename}/benchmark` 接口，支持对任意本地运行中的 GGUF 模型一键跑测并输出 Precision/Recall/F1 矩阵与最高 F1 胜出版本。
- [x] 45.3 **主界面模型切换自动同步最佳提示词模板**：
  - 在 [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js) 中，当用户切换并启动模型时，自动拉取其绑定的最佳提示词并同步至当前提取工作区；
  - 增加轻量反馈提示（展示当前绑定的最佳模板名称与 F1 分数）。
- [x] 45.4 **设置中心「⚡ 提示词寻优」操作与评测控制台模态框**：
  - 在 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 中为设置面板的模型列表项增设最佳提示词标签与「`⚡ 提示词寻优`」按钮；
  - 新增 `#autoBenchmarkModal` 模态框，展示实时跑测进度条、4 组指标对比表格、胜出结论与「✨ 设为此模型的默认最佳提示词」一键采纳写入按钮；
  - 在 [`web/style.css`](file:///Users/icychick/Projects/SensiDoc/web/style.css) 中落地专业美观的表格与徽章样式。
- [x] 45.5 **全链路端到端功能验证与回归测试**：
  - 12 项 Cargo 单元测试 100% 通过，端到端调用 `POST /api/models/.../benchmark` 成功跑出 V4 胜出结果，数据完整落盘。

---

## 阶段四十六：在线 OpenAI 兼容模型 API 配置面板与提示词深度进化寻优双轨引擎 (已完成) · [🔗 对话跳转](conversation://a2de2406-6bc8-410f-b0b1-cc993b75ced8)
- [x] 46.1 **在线 AI 模型 API 配置数据结构与持久化 (`online_ai_config`)**：
  - 在 [`src/session.rs`](file:///Users/icychick/Projects/SensiDoc/src/session.rs) 中新增 `OnlineAiConfig` 结构体（`enabled`, `base_url`, `api_key`, `model_id`, `temperature`），并在 `WorkspaceStore` 中持久化存储；
  - 在 [`src/main.rs`](file:///Users/icychick/Projects/SensiDoc/src/main.rs) 中提供 `GET /api/settings/online-ai`、`POST /api/settings/online-ai` 与 `POST /api/settings/online-ai/test` 接口；
  - 跑通连通性测试与落盘单元测试。
- [x] 46.2 **在线大模型驱动的提示词错题反思与进化生成引擎 (`src/benchmark.rs`)**：
  - 在 [`src/benchmark.rs`](file:///Users/icychick/Projects/SensiDoc/src/benchmark.rs) 中实现 OpenAI Chat Completion 请求客户端；
  - 实现双轨寻优逻辑：当在线 AI 开启时，先收集本地小模型在基准测试集上的漏报实体（False Negatives）与误报项（False Positives），将其构造成错题诊断上下文，请求在线大模型生成针对性深层优化的 `AI 深度进化版 Prompt`，再送入本地模型验证 F1。
- [x] 46.3 **设置中心「在线 AI 模型 (API)」配置面板 UI**：
  - 在 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 设置模态框中新增 Tab 标签 `在线 AI 模型` 与面板内容；
  - 包含独立启用/停用开关 Toggle、Base URL 输入框（带 DeepSeek / OpenAI / SiliconFlow 快捷填充）、API Key 密码输入框、Model ID 输入框与「测试连通性」操作按钮；
  - 遵循统一的 Lucide 矢量 SVG 图标与极简中性视觉规范。
- [x] 46.4 **自动寻优控制台 (`#autoBenchmarkModal`) 双轨感知与交互升级**：
  - 在模态框顶部根据在线模型开启状态自适应展示当前运行模式徽标（`离线范式池盲测` vs `在线 AI 深度进化 (DeepSeek/GPT-4o 驱动)`）；
  - 进化寻优模式下展示错题反思与进化版指标，支持一键采纳进化提示词。
- [x] 46.5 **全链路端到端功能验证与回归测试**：
  - 验证：API Key 配置与测试连通性；
  - 验证：在线 AI 开启与关闭状态下的不同寻优执行路径。

---

## 阶段四十七：提示词寻优功能归位整合与提取规则板块模型针对性调优重构 (已完成) · [🔗 对话跳转](conversation://a2de2406-6bc8-410f-b0b1-cc993b75ced8)
- [x] 47.1 **模型管理板块轻量化与职责解耦**：
  - 从 [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js) 的 `loadModelPresets` 模型卡片中移除「⚡ 提示词寻优」按钮，使「离线模型」Tab 专注于模型的生命周期管控（导入、下载、启动、停止）。
- [x] 47.2 **重构「提取规则与提示词」面板布局 (`#paneSetRules`)**：
  - 在 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 中将「提取规则与提示词」重构为三大逻辑层：
    1. **目标模型与专属提示词档案控制台**：包含目标模型下拉选择器 `#promptTargetModelSelect`、专属绑定档案/F1 徽章 `#promptModelProfileBadge`、「提示词自动寻优」按钮 `#triggerPromptBenchmarkBtn`、「恢复模型推荐」按钮 `#resetToModelDefaultPromptBtn`；
    2. **提示词编辑器与实时预览区**：支持针对当前选中模型编辑并保存为专属提示词（`#saveModelCustomPromptBtn`）；
    3. **业务规则预设模板库管理**：规则库管理、导入/导出与清空。
  - 全面采用标准 Lucide 矢量 SVG 图标与响应式中性配色。
- [x] 47.3 **前端模型联动与针对性寻优事件交互 (`web/app.js`)**：
  - 动态拉取全部已支持/已下载的模型列表，填充 `#promptTargetModelSelect`；
  - 监听下拉选择变更事件，自动拉取对应模型的专属提示词档案并同步至编辑器；
  - 绑定「提示词自动寻优」：基于当前选中模型触发寻优评测并在采纳后即时同步刷新；
  - 绑定「保存在线/专属提示词」与「恢复模型推荐」操作。
- [x] 47.4 **端到端链路验证与回归测试**：
  - 验证：在规则与提示词面板中切换不同模型（如 Qwen-1.5B 与 LFM-450M），提示词及绑定徽章即时联动；
  - 验证：在规则面板直接对选中模型触发自动寻优并一键采纳写入。

---

---

## 阶段四十九：提示词 AI 优化内聚式多策略下拉与指标直显交互升级 (已完成) · [🔗 对话跳转](conversation://22ce5631-64c9-4f1d-8fdf-1a913343aee1)
- [x] 49.1 **顶部目标模型调优区域极净化**：
  - 在 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 中彻底移除模型下拉框下方的“当前生效档案：...”多余行，顶部仅保留单行模型选择，彻底降噪。
- [x] 49.2 **「⚡ AI优化」Tab 内容内聚重构**：
  - 在 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 与 [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js) 中新增策略版本选择器 `#promptStrategySelect`，将预设/评测得到的候选提示词策略（如 `🏆 V4_超轻量极简直接抽取版`、`🔹 V1_默认结构基准版`、`🔹 V2_严格两阶段思考抽取版`、`🔹 V3_少样本示例加固版`、`🤖 在线 AI 深度进化版`）直接收录为下拉菜单；
  - 选项文案精简纯粹，去掉多余的括号说明（选中即最优）；
  - 顶部右侧保留 `[⚡ 重新自动寻优评测]` 按钮。
- [x] 49.3 **识别指标胶囊与提示词内容直接上屏**：
  - 在「AI优化」Tab 内直接设置 `#strategyMetricsBar`（动态展示综合 F1、查全召回率 R、精准率 P、单篇耗时及策略徽章）与只读提示词代码框 `#strategyPromptPreview`；
  - 切换下拉菜单时，指标条与提示词全文即时无缝联动。
- [x] 49.4 **底部操作栏智能采纳与保存**：
  - 在「AI优化」Tab 下，底部按钮自动切换为「采纳设为专属提示词」，一键将当前选中的策略设为该模型的专属提示词档案；
  - 精简底部输出规范文案为 `输出规范：[{"field": "字段名", "text": "原文"}]`。

---

## 阶段五十：场景模板管理居中模态弹窗与操作体验升级 (已完成) · [🔗 对话跳转](conversation://22ce5631-64c9-4f1d-8fdf-1a913343aee1)
- [x] 50.1 **居中「另存为场景模板」模态框落地 (`#saveTemplateModal`)**：
  - 在 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 中新增居中模态弹窗，提供模板名称、描述输入及规则数量统计；
  - 支持回车提交、Esc 取消；保存后自动刷新并选中新模板，彻底清退浏览器原生 `prompt()` 与 `alert()`。
- [x] 50.2 **模板删除与通用中央警告框**：
  - 删除模板采用居中二次确认模态框，删除后静默刷新；
  - 封装 `showAlertDialog()` 统一承接全站错误与空状态居中弹窗提示。
- [x] 50.3 **文档列表文件名恢复加粗显示**：
  - 在 [`web/style.css`](file:///Users/icychick/Projects/SensiDoc/web/style.css) 中将 `.file-name` 调整为 `font-weight: 600`。
- [x] 50.4 **移除设置面板内冗余的「工作区规则维护」区块**：
  - 从 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 中移除「工作区规则维护（清空当前规则）」冗余卡片，保持设置面板专注纯粹。
- [x] 50.5 **全系统 100% 清退浏览器原生 `alert()` / `prompt()` 弹窗**：
  - 全面排查并重构了“存标签”、“解析/上传报错”、“删除/复制失败”、“未选文档提示”、“在线 AI 保存”、“离线模型启停/下载”、“本地 GGUF 路径导入”等全部 20+ 处交互，统一采用与删除文档/快照完全一致的画面中央模态弹窗系统（`showAlertDialog` 等）。

---

## 阶段五十一：原生文件选择器挂载、魔搭 Q8 模型升级与设置面板弹性自适应布局重构 (已完成) · [🔗 对话跳转](conversation://a1903cc6-59cd-44db-85b8-6064978ee7aa)
- [x] 51.1 **系统原生文件选择对话框选取本地 GGUF 模型**：
  - 后端在 [`src/model_manager.rs`](file:///Users/icychick/Projects/SensiDoc/src/model_manager.rs) 中实现跨平台原生文件选择器 `pick_file_dialog()`（macOS 原生 Finder 对话框 / Windows OpenFileDialog / Linux 原生选择器）；
  - 新增 `POST /api/models/pick-and-import` 路由，用户点击「+ 选取本地 GGUF 模型」直达系统文件选择器，选定后自动秒级创建软链接挂载并弹出立即启动确认；
  - 彻底移除前端冗余的手动贴路径模态框 `#importModelModal`。
- [x] 51.2 **魔搭默认推荐模型升级为 Q8_0 高精度版本**：
  - 将内置推荐模型由 `LFM2.5-VL-450M-Q4_K_M.gguf` 升级为 `LFM2.5-VL-450M-Q8_0.gguf`（~360 MB，Q8_0 高精度量化）；
  - 彻底清理本地 `models/` 目录下已过期的 Q4 历史模型文件。
- [x] 51.3 **审计工作台分段选项卡文案统一**：
  - 将右侧主控制台的「敏感清单」统一重命名为「提取结果」，表达更精准客观。
- [x] 51.4 **文档列表文件名选中态加粗与默认常规字重**：
  - 文档列表中未选中项保持常规字重 `font-weight: 400`，当前激活项加粗 `font-weight: 600`。
- [x] 51.5 **设置模态框全板块弹性自适应与底部操作栏固底**：
  - 将「提取规则与提示词」板块与「在线 AI 模型」板块重构为 Flex 弹性布局，底部操作栏（包含输出规范、测试 API、保存提示词等按钮）设置 `flex-shrink: 0; margin-top: auto;` 吸底固定；
  - 上方核心内容（提示词只读预览区、自定义编辑文本框等）采用 `flex: 1; min-height: 0;` 动态填满面板剩余纵向空间，内容超出时仅在内部平滑滚动；
  - 移除了设置面板内各板块多余的 `padding-right: 4px;`，使所有板块右侧边缘与顶部「关闭」按钮精准对齐。

---

## 阶段五十二：提取规则与提示词板块极简化重构 (移除寻优跑测，按模型尺寸预设) (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 52.1 **彻底清退寻优评测与 AI 错题反思模块**：
  - 彻底删除 `#autoBenchmarkModal` 模态框 DOM、表格样式及所有相关 JS 跑测驱动逻辑；
  - 移除「重新自动寻优评测」按钮、「基准评测集：10 篇高保真文档」描述及 F1/召回率指标栏，消除普通用户的认知门槛。
- [x] 52.2 **按模型尺寸自动推荐内置最佳预设模板**：
  - 在目标模型选择器旁增设动态尺寸智能标签（`#promptModelSizeBadge`）：
    - 针对 Qwen 等 1.5B 级别模型，自动展示「1.5B 轻量推荐 (极简单抽)」，默认载入极速直抽模板；
    - 针对 LFM 等 450M 级别模型，自动展示「450M 超轻量加固 (防漂移)」，默认载入上下文隔离结构加固模板；
    - 针对大尺寸或通用模型，自适应匹配通用基准模板。
- [x] 52.3 **单层扁平提示词工作区与编辑/预览极简双拨杆**：
  - 移除「AI优化 / 自定义编辑 / 预览」三重割裂 Tab，重构为单一整洁工作区；
  - 右上角配备轻量拨杆：`[编辑提示词]` 与 `[实时预览]`，切换零晃动；
  - 底部操作栏规范精炼为 `[ 恢复推荐模板 ]` 与 `[ 保存提示词 ]`，支持随时一键还原出厂配置。
- [x] 52.4 **全链路端到端功能验证与回归测试**：
  - 13 项 Cargo 单元测试全部通过（100% 绿灯），前端 JS/CSS 语法检查通过。

---

## 阶段五十三：提示词变量插槽着色与提取规则板块极简化 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 53.1 **提示词变量插槽醒目着色与底栏说明精简**：
  - 在 [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js) 的 `renderPromptHighlight` 中将 `{FIELDS_DEFINITION}` 变量渲染为独立醒目的 `.prompt-var-tag` 标签着色；
  - 彻底移除底部操作栏中冗余的「底部的变量插槽：{FIELDS_DEFINITION}」说明文本，使操作区更加清爽。
- [x] 53.2 **设置中心 Tab 标签文案精简**：
  - 将设置面板顶部 Tab「提取规则与提示词」简化重命名为「**提取规则**」（[`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html)）。

---

## 阶段五十四：在线模型设置下拉化管理与双轨提取调度重构 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 54.1 **设置中心「在线模型」板块职责重塑**：
  - 将 Tab 标签更名为「**在线模型**」，彻底剥离原先提示词模板调优定位，转型为标准云端模型调用与管理中心；
  - 已保存模型重构为顶部下拉菜单管理（`#onlineModelSelect`），选中模型自动同步表单配置，支持新建、保存、删除与连通性测试。
- [x] 54.2 **表单纯净新建与参数拓展**：
  - 新建模型时表单完全清空（不预填充），BaseURL 默认展示 OpenAI 官方示例提示；
  - 参数配置区新增 `--top-k 50` 与 `--repeat-penalty 1.1` 自定义参数项。
- [x] 54.3 **主工作台底部提取模型双轨分组调度**：
  - 在 [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js) 的 `populateFooterModelSelect` 中将底部提取模型下拉框重构为双轨分组：
    - `离线本地模型 (GGUF)`：枚举当前已就绪的本地离线模型，展示运行中状态并支持启停；
    - `在线云端模型 (API)`：动态聚合已保存的在线模型配置；
  - 在「立即执行提取」时根据所选前缀（`offline:` / `online:`）无缝路由至本地小模型推理或云端大模型 API。

---

## 阶段五十五：离线模型列表加载修复与操作文案精简 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 55.1 **离线模型与测试评测记录加载恢复**：
  - 修复设置模态框打开与选项卡切换时离线模型列表未加载的问题，在模态框打开及 Tab 切换事件中主动调用 `loadModelPresets()`；
  - 恢复 `models/` 目录下已下载模型、魔搭推荐模型列表及模型专属评测记录（F1 分数徽标）的完整展示。
- [x] 55.2 **操作按钮文案极简化**：
  - 将离线模型列表中的操作按钮由「一键下载」与「载入启动」精简为「**下载**」与「**启动**」。

---

## 阶段五十六：外观显示 Tab 文案精简与全局 UI 缩放滑动条重构 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 56.1 **Tab 标签文案精简**：
  - 将设置中心顶部的「外观与显示」选项卡重命名为「**外观显示**」（[`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html)）。
- [x] 56.2 **全局 UI 缩放滑动条重构**：
  - 将离散 Pill 按钮组重构为缩放滑动条（`<input type="range" id="uiScaleSlider" min="100" max="200" step="5">`）；
  - 缩放刻度严格对齐既有比例（100% ~ 200%，步长 5%），覆盖 100%, 110%, 115%, 120%, 125%, 130%, 140%, 150%, 175%, 200%；
  - 底部均匀分布关键刻度标签（100% 默认、125%、150%、175%、200%），支持点击标签快速定位；
  - 滑动时实时联动右上角数值徽标、页面 CSS `--ui-scale` 与 `zoom`，并持久化到 `localStorage`。

---

## 阶段五十七：前端初始化异常排查、DOM 引用防御性加固与静态资源缓存控制 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 57.1 **缺失 DOM 引用补全与未捕获异常排查**：
  - 排查并修复了 `web/app.js` 的 `el` 对象中缺失 `rawJsonModal`、`viewRawJsonBtn`、`closeRawJsonModalBtn`、`copyRawJsonBtn`、`rawJsonCodeBlock` 导致 `initEventListeners` 抛出 `Uncaught TypeError` 并阻塞后续 `loadDocuments()` 与 `loadModelPresets()` 执行的问题。
- [x] 57.2 **全事件监听器防御性加固**：
  - 为 `web/app.js` 中所有事件监听器绑定增加空值安全防护（`if (el.xxx)`），确保个别 DOM 缺失时不阻断整个前端生命周期。
- [x] 57.3 **静态资源缓存控制与实机端到端验证**：
  - 在 [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html) 中为 `style.css` 与 `app.js` 引入版本查询参数（`?v=1.1.2`），避免浏览器加载旧版缓存脚本；
  - 经 Chrome Headless 实测验证，10 份测试文档、各版本历史快照时光机记录（14 个快照、10 个快照等）及模型评测徽标均已完整恢复正常展示。

---

## 阶段五十八：规则标签与提示词重构（「风险等级」升级为「优先级」） (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 58.1 **企业基准多方案离线实测对照**：
  - 编写并运行 [`tests/test_priority_benchmark.py`](file:///Users/icychick/Projects/SensiDoc/tests/test_priority_benchmark.py)，对 10 份企业基准测试文档进行 4 组策略全量测试；
  - 实测证明：行内注入 `(优先级: 高/中/低)` 使综合 F1 由 **77.70% 提升至 81.86%**，核心高优先级字段召回率由 **57.50% 显著提升至 65.00%**，精准率保持 100%（零误报）。
- [x] 58.2 **提示词字段定义生成引擎升级 (`src/extractor.rs`)**：
  - 在 `format_fields_definition` 中为每个启用字段注入 `(优先级: 高/中/低)` 行内权重提示，引导大模型在多字段长文本提取时进行合理注意力分配。
- [x] 58.3 **前端 UI 优先级文案与徽标精简重构 (`web/app.js` / `web/index.html`)**：
  - 规则卡片（`#ruleCardList`）、常用标签库（`#tagPool`）、快照审查模态框（`#snapshotRulesModal`）及审计命中清单（`#auditList`）中，原「高危/中危/低危」统一升级为精简的「**高**」、「**中**」、「**低**」优先级徽标与提示；
  - 导出 CSV 表头由「风险等级」更新为「优先级」，值对应输出「高/中/低」；
  - 保持底层数据模型完全向下兼容历史快照与保存模板。
- [x] 58.4 **全链路端到端功能验证与回归测试**：
  - 13 项 Cargo 单元测试 100% 绿灯通过，前端 JS 语法检查通过，Chrome 浏览器实机验证展示正常。

---

## 阶段五十九：新增提取规则居中模态弹窗与紧凑布局重构 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 59.1 **居中模态弹窗 UI 结构与极简紧凑布局构建 (`web/index.html`)**：
  - 新增居中模态弹窗 `#addRuleModal`，采用左右两列紧凑布局：左侧「字段名称」（必填自适应伸缩，单例提示「例如：甲方企业」），右侧并排「优先级」（92px 紧凑选择框，精简纯净「高 / 中 / 低」选项）；
  - 下方配置「提取特征描述 (选填)」文本框（默认适中 2 行舒适高度，单例提示「例如：合同采购方或甲方公司全称」），视觉层级清晰整齐；
  - 严格遵循 Geist 中性极简设计规范，使用纯 SVG 图标，自适应深浅色主题。
- [x] 59.2 **弹窗控制与表单提交逻辑重构 (`web/app.js`)**：
  - 点击工作台右侧「＋ 新增规则」触发 `openAddRuleModal()`，清空历史输入并自动聚焦到字段名称输入框；
  - 完善名称非空与防重复校验，提交后自动倒序插入至生效规则列表最顶部并即时刷新；
  - 支持勾选「同时保存至常用标签库」同步调用 `saveRuleAsTag()` 存库；
  - 支持 Esc 快捷键、点击关闭按钮及点击遮罩外部关闭弹窗。
- [x] 59.3 **端到端交互与回归测试验证**：
  - Cargo 13 项单元测试 100% 通过，JS 语法检查通过；
  - Chrome 浏览器实机验证弹窗弹出、表单提交添加、首项聚焦与规则刷新流程正常。
- [x] 59.4 **底部模型控制栏视觉精简**：
  - 移除工作台右下角「模型:」标签左侧冗余的状态指示圆点（`#footerModelDot`），由右侧拨杆开关与模型下拉列表直观表达启停状态，底栏视觉更清爽。

---

## 阶段六十：llama-server 专属冷门端口迁移 (18188) 与精准进程隔离治理 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 60.1 **冷门专属端口迁移 (`18188`)**：
  - 将项目中本地 llama-server 默认绑定的 `8081` 迁移至 SensiDoc 专属冷门端口 **`18188`**（[`src/model_manager.rs`](file:///Users/icychick/Projects/SensiDoc/src/model_manager.rs)），彻底避免与系统中常规 8080/8081/11434 等其他 AI/Web 项目产生端口冲突。
- [x] 60.2 **精准进程生命周期治理与误杀根治**：
  - 彻底移除原先盲目全局强杀的 `pkill -f llama-server`；
  - 重构 `stop_server`：优先通过 PID 句柄优雅终止当前拉起的子进程；Unix 兜底机制重构为仅精准查找并释放占用 `18188` 专属端口的孤儿残留（`lsof -ti :18188`），绝对不误触系统其他正在运行的 llama.cpp / llama-server 服务。
- [x] 60.3 **动态端口探测与全链路适配**：
  - `ModelManager::get_active_model`、`Extractor::query_llm`、`BenchmarkEngine::run_benchmark` 及主服务启动回调统一由 `server_port()` 动态参数驱动；
  - 13 项 Cargo 单元测试 100% 绿灯通过，服务热更新就绪。

---

## 阶段六十一：三栏顶部标题栏高度严格对齐与视觉规整 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 61.1 **三栏顶部标题栏高度统一基准 (`web/style.css`)**：
  - 左侧文档面板标题栏（`.sidebar-header`）、中间正文预览工具栏（`.stage-toolbar`）与右侧审计面板选项卡（`.segmented-control`）统一重构为严格一致的 **`48px`** 高度与 `box-sizing: border-box`；
  - 彻底消除原先中间工具条 `40px` 与左侧面板视觉高度不一的错落感，工作区三栏顶部水平基准线完全对齐。

---

## 阶段六十三：全系统数据契约与导出字段统一（`risk_level` 全量重构升级为 `priority`） (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 63.1 **后端结构体与序列化字段统一 (`src/extractor.rs` / `src/exporter.rs` / `src/benchmark.rs`)**：
  - `RuleField` 与 `SensitiveItem` 核心字段由 `risk_level` 全面更名为 `priority`；
  - 增加 `#[serde(default = "default_priority", alias = "risk_level")]` 属性，确保完美向下兼容历史工作区存储；
  - `Exporter::export_to_csv` 规范输出 `item.priority`，基准测试用例全部对齐 `priority` 字段。
- [x] 63.2 **前端全链路与原始 JSON 字段统一 (`web/app.js` / `web/style.css`)**：
  - 「原始 JSON」弹窗中 `detected_items` 与 `missed_fields` 统一输出 `priority: "high" | "medium" | "low"`；
  - 规则列表（`renderRulesTable`）、标签库（`renderFieldTags` / `saveRuleAsTag`）、新增规则（`handleAddRuleSubmit`）、快照审查（`renderSnapshotRulesModal`）及 CSV 导出全面对齐优先读取 `priority`；
  - CSS 选择器补齐 `mark.sensi-mark[data-priority="..."]`。
- [x] 63.3 **全链路回归与数据持久化验证**：
  - 13 项 Cargo 单元测试 100% 绿灯通过，JS 语法检查通过；
  - `.sensidoc_workspace.json` 与企业基准测试集已全量完成字段迁移。

---

## 阶段六十四：输出 SensiDoc v2 实施方案（全格式纯代码原生无损脱敏架构） (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 64.1 **完成 [`docs/IMPLEMENTATION_PLAN_v2.md`](file:///Users/icychick/Projects/SensiDoc/docs/IMPLEMENTATION_PLAN_v2.md) 编制**：
  - 汇总 1~63 阶段的所有架构演进（18188 专属端口隔离、48px 视界对齐、priority 优先级体系、双轨模型调度、方案 A 闭环审计）；
  - 全面确立 **v2 核心全格式纯代码原生脱敏导出引擎** 架构设计（DOCX/XLSX/PPTX 跨 Run XML 合并替换、97-2003 二进制资产升级转码、TXT/CSV 直接字符流替换、PDF 物理遮盖与文字流抹除）；
  - 规范定义 RESTful 导出 API、全量数据协议契约与五大实施里程碑（M1~M5）。

---

## 阶段六十五：SensiDoc v2 原生脱敏导出引擎落地与全链路验证 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 65.1 **原生脱敏引擎核心实现 (`src/desensitizer.rs`)**：
  - `mask_text`：智能等长掩码算法（<=2 字符全打码，>2 字符首尾保留+中间掩码）；
  - `desensitize_docx` / `desensitize_docx_xml`：ZIP 内存流解压，段落级跨 `<w:t>` 节点 Unicode 字符投影合并、敏感词等长打码与无损精确回写；
  - `desensitize_xlsx`：`xl/sharedStrings.xml` 字符串池与 worksheet 行内单元格脱敏；
  - `desensitize_pptx`：`ppt/slides/slide*.xml` 幻灯片文本框段落脱敏；
  - `desensitize_plain_text` / `desensitize_csv`：纯文本与 CSV 字符流就地打码；
  - `desensitize_document_auto`：基于扩展名自动路由分发并生成规范 MIME 类型与文件名。
- [x] 65.2 **原始文件暂存与后端脱敏接口接入 (`src/paths.rs` / `src/main.rs`)**：
  - `paths::get_uploads_dir()`：管理 `uploads/` 原始上传二进制存储目录；
  - `convert_document`：上传时自动暂存原始二进制至 `uploads/{doc_id}.bin`，`delete_document` 自动同步清理；
  - 注册 `POST /api/documents/{id}/desensitize` 路由，支持 `mode: "native" | "markdown"` 及指定快照，支持 RFC 5987 / RFC 6266 中文文件名规范响应。
- [x] 65.3 **前端 UI 导出菜单与交互重构 (`web/index.html` / `web/style.css` / `web/app.js`)**：
  - 将「脱敏导出」升级为自适应向上弹出的「脱敏导出 ▼」交互菜单（Geist-Neutral 极简微动效）；
  - 支持多维导出操作：
    - 📄 **导出原格式文档**：原生无损保留排版与样式 (`.docx` / `.xlsx` / `.pptx` / `.txt` / `.csv`)；
    - 📝 **导出脱敏 Markdown**：轻量纯文本 (`.md`)；
    - 📊 **导出敏感词清单**：包含频次、优先级与检出来源 (`.csv`)；
    - 📋 **导出全量审计 JSON**：包含已检出与未检出完整闭环数据 (`.json`)；
  - `app.js` 实现流式二进制文件下载与错误捕获。
- [x] 65.4 **全链路端到端功能验证与回归测试**：
  - 21 项 Cargo 单元测试 100% 绿灯通过（含 DOCX/XLSX/PPTX 内存 ZIP 还原测试与全格式自动分发测试）；
  - JS 语法检查通过，实机浏览器验证无阻断异常。

---

## 阶段六十六：原生 PDF 版式文档 Content Stream 纯代码脱敏引擎落地 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 66.1 **实施方案 v2 全面扩充更新 ([`docs/IMPLEMENTATION_PLAN_v2.md`](file:///Users/icychick/Projects/SensiDoc/docs/IMPLEMENTATION_PLAN_v2.md))**：
  - 在第 3.4 节详细补充原生 PDF 版式文档 Content Stream 字符级抹除架构与数据流；
  - 确立清晰边界：本阶段聚焦原生矢量/文字类 PDF 真脱敏，扫描件与图片型 PDF 的 OCR 像素级遮盖作为后续版本迭代任务；
  - 同步更新第 6 节工程目录与第 7 节 M6 里程碑。
- [x] 66.2 **引入纯 Rust PDF 底层解析引擎 (`Cargo.toml`)**：
  - 引入 `lopdf = { version = "0.38", default-features = false }`，零外部 C/C++ 依赖，二进制增量仅 ~320KB。
- [x] 66.3 **PDF 原生 Content Stream 算子解析与多编码打码器实现 (`src/desensitizer.rs`)**：
  - 实现 `Desensitizer::desensitize_pdf`：解析 PDF 对象树与页面 `/Contents` 流，捕获 `Tj`、`TJ`、`'`、`"` 文字绘制算子；
  - 实现 `desensitize_pdf_object` 与 `desensitize_pdf_tj_array`：
    - UTF-16BE 编码（`\xFE\xFF` 开头）自适应解码与等长 `*` 替换；
    - UTF-8 / WinAnsi 编码自适应等长打码；
    - GB18030 / GBK 编码自适应等长打码；
  - 自动清理文档级 `/Metadata`（XMP 敏感元数据）与 `/Info`；
  - `desensitize_document_auto` 正式接入 `.pdf` 原生脱敏路由，自动生成 `xxx_脱敏.pdf`（`application/pdf`）。
- [x] 66.4 **全链路端到端功能验证与回归测试**：
  - 新增 `test_pdf_content_stream_pipeline` 测试用例，验证 PDF 对象构造、算子打码、元数据清理与重新序列化；
  - 全部 **22 项 Cargo 单元测试 100% 绿灯通过**。
- [x] 66.5 **脱敏导出菜单视觉去重与纯净化 (`web/index.html`)**：
  - 移除「脱敏导出」下拉菜单中各选项标题前残留的 emoji 表情（如 `📄`、`📝`、`📋`），统一使用高精细度的纯矢量 SVG 图标，整体视觉更加中性专业、规整干练；
  - 精简第一项子描述文案为 `保留原文件格式与排版 (.docx/.pdf/.xlsx/.txt/.csv等)`。
- [x] 66.6 **脱敏导出菜单布局与等高规整优化 (`web/index.html` / `web/style.css`)**：
  - 在「导出原格式文档」与「导出脱敏 Markdown」之间补充横向分割线；
  - 规范化 3 个菜单选项的高度为统一的 46px，图标纵向居中对齐，排版彻底规整对称。

---

## 阶段六十七：SensiDoc 统一 CLI 命令行与 Agent 自动化调用支持 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 67.1 **引入 `clap` 并搭建 CLI 参数规范架构 (`src/cli.rs`)**：
  - 引入精简特性 `clap`，定义 `audit`、`mask`、`convert`、`templates`、`serve` 5 大顶级子命令；
  - 实现参数隔离：`stdout` 纯输出标准结构化数据（JSON/脱敏流），进度与警告统一输出至 `stderr`；
  - 规范化退出码（0: 成功放行，1: 命中敏感阻断，2: 参数错误，3: 推理异常）。
- [x] 67.2 **场景预设模板与规则决策链打通**：
  - 支持 `-t / --template` 读取 `.sensidoc_workspace.json` 中保存的场景模板；
  - 支持 `--rules` 命令行追加、`--rules-file` JSON 导入与 `--regex-only` 毫秒级免模型兜底模式；
  - 实现 `sensidoc templates list` 查看当前所有可用场景模板及规则字段清单。
- [x] 67.3 **核心执行引擎打通与服务探针复用**：
  - `audit`：统一串联 anydoc 转换 ➔ 正则/LLM 抽取 ➔ 冲突消解 ➔ JSON 结构化输出；
  - `mask`：直接生成原生脱敏文件（保留 DOCX/PDF/XLSX/PPTX 等原生排版）；
  - `convert`：命令行快速预览任意文档为 Markdown 纯文本；
  - 智能服务探针：检测 `18188` 端口，优先复用已常驻的 `llama-server` 实例，无实例时按需单次拉起并在退出时清理。
- [x] 67.4 **主入口兼容重构 (`src/main.rs`) 与全链路测试**：
  - 改造 `main.rs`：无子命令时自动回退至默认桌面 GUI / Web 服务启动，保持完全向前兼容；
  - 22 项 Cargo 单元测试 100% 绿灯通过；实测 `audit`、`mask`、`convert`、`templates` 命令及管道 `jq` 解析全部通过。
- [x] 67.5 **生成详细的 CLI 使用说明与 Agent 集成文档 ([`docs/CLI_GUIDE.md`](file:///Users/icychick/Projects/SensiDoc/docs/CLI_GUIDE.md))**：
  - 包含命令行语法、参数手册、标准输出 JSON Schema、Python/Node.js Agent 快速接入指南。
- [x] 67.6 **CLI 调用记录与 Web 界面「文档列表」实时双向联动 (`src/cli.rs` & `src/session.rs`)**：
  - **自动入库与同名复用**：CLI 执行 `audit` / `mask` 时自动解析文档并入库，追加标有 `[CLI] 模板名` 的提取快照；
  - **原始二进制转储**：原始文件写入 `uploads/{doc_id}.bin`，实现从 CLI 到 Web 界面随时点击「脱敏导出」下载原生版式打码文档；
  - **跨进程磁盘热感知**：`SessionManager` 引入基于 `mtime` 状态机的 `sync_from_disk_if_modified()`，常驻 Web 服务零重启自动感知外部 CLI 新写入；
  - **无痕开关 `--no-record`**：满足 Agent 高频批量处理或 CI/CD 扫描时的零磁盘留痕诉求；
  - **全链路测试通过**：新增 `test_cross_instance_disk_sync` 单元测试，全部 23 项测试 100% 绿灯；实机验证 CLI 审计后 Web API 立即返回最新文档卡片与快照。
- [x] 67.7 **魔搭离线模型预设库扩充 Tessera-4B-Preview (`src/model_manager.rs` & `src/session.rs`)**：
  - **新增预设配置**：引入 `sahilchachra/Tessera-4B-Preview-GGUF` 的 `Tessera-4B-Preview-Q4_K_M.gguf` 量化版本（~2.6 GB，基于 Qwen3.5-4B 深度微调的高性能推理模型）；
  - **下载探测优化**：在断点续传中增加对 `Content-Range` 响应头的解析，精准获取超大模型文件体积；
  - **提示词自动映射**：在 `get_model_prompt_profile` 中将 `tessera` 自动映射至 `V4_超轻量极简直接抽取版`；
  - **直链验证通过**：通过 `test_modelscope_connection` 单元测试验证 ModelScope 直链 Range 分块请求成功，Web 界面「设置」模态框即时呈现下载卡片。
- [x] 67.8 **离线模型列表评测记录徽章移除与信息精简 (`web/app.js`)**：
  - **精简视觉层次**：彻底移除模型卡片中原先展示的“评测记录: V4_超轻量极简直接抽取版 (实测冠军) (F1: 91.2%)”等信息补充徽章，保持列表干净清爽。
- [x] 67.9 **模型下载过程取消与临时缓存自动清除闭环 (`src/model_manager.rs`, `src/main.rs`, `web/app.js`)**：
  - **后端取消控制通道**：`ModelManager` 引入基于 `tokio::sync::oneshot` 的下载中断信号表 `download_cancellations`，下载数据流中通过 `tokio::select!` 监听取消信号并自动删除 `models/{filename}.part`；
  - **新增取消接口**：注册 `/api/models/download/cancel` 路由，提供即时终止与磁盘残余缓存安全删除；
  - **前端状态机闭环**：点击“下载”后按钮动态切换为醒目的红色边框“取消下载”按钮；点击取消后重置进度并弹出反馈对话框（“已终止模型下载，并自动清除了已下载的临时缓存文件”），按钮恢复为“下载”；
  - **实机与单元测试验证**：`test_cancel_download_cleans_cache` 单元测试通过，并通过 Chrome DevTools 自动化实测验证取消、弹窗反馈与 `models/` 磁盘零缓存残留。
- [x] 67.10 **魔搭推荐模型库轮换与参数大小排序、下载红色“取消”按钮加固 (`src/model_manager.rs`, `web/app.js`)**：
  - **按参数大小升序排序**：推荐模型顺序调整为 `LFM2.5-VL-450M (Q8_0)` (450M) $\rightarrow$ `Qwen2.5-1.5B-Instruct (Q4_K_M)` (1.5B) $\rightarrow$ `Qwen3.5-2B (Q5_K_M)` (2B) $\rightarrow$ `Tessera-4B-Preview (Q4_K_M)` (4B)，Qwen3.5-2B 紧排在 Qwen2.5-1.5B 之后；
  - **后端权威下载态**：`ModelPreset` 引入 `is_downloading` 状态字段，从服务端权威告知当前正在下载的模型，彻底杜绝前端刷新或二次渲染导致的按钮脱敏；
  - **下载中红色“取消”按钮常驻**：下载启动及 SSE 流式传输期间，右侧操作区强制保持红色的“取消”按钮（`border-color: var(--danger); color: var(--danger); background: rgba(239, 68, 68, 0.08);`），直到下载完成或用户点击取消；
  - **全链路实机回归**：通过 Chrome DevTools 自动化实机实测验证，下载过程中红色的“取消”按钮稳定常驻，取消时自动清除缓存并复原。

---

## 阶段六十八：老旧二进制文档 (.doc / .xls / .ppt 97-2003) 升级转码与脱敏导出支持 (待办)
- [ ] 68.1 **旧版 Excel (.xls) 纯 Rust 升级转码与脱敏方案**：
  - 调研与集成 `calamine` + `rust_xlsxwriter`，实现内存读取旧版 BIFF8 二进制流，执行敏感词等长打码，并重构导出为现代 `.xlsx` 格式；
- [ ] 68.2 **旧版 Word (.doc) 二进制流解析与提取方案**：
  - 探索基于 `cfb`（复合文档二进制格式解析器）读取 WordDocument Stream，提取段落文本并完成脱敏转码导出为 `.docx`；
- [ ] 68.3 **旧版 PPT (.ppt) 兼容性评估与路由适配**：
  - 评估轻量提取幻灯片文本框并升级打包为 `.pptx` 的可行性；
- [ ] 68.4 **后端脱敏路由升级与单元测试**：
  - 扩展 `Desensitizer::desensitize_document_auto`，实现 `.doc` $\rightarrow$ `.docx`、`.xls` $\rightarrow$ `.xlsx` 自动升级转码导出；
  - 编写旧版二进制格式脱敏单元测试与回归测试用例。

---

## 阶段六十九：超大文档 (>50MB) 流式解压与高并发压测优化 (待办)
- [ ] 69.1 **超大文档流式解压与分块打码 (Streaming & Chunking)**：
  - 针对数十万行大表格与嵌套百张高分图的 DOCX/XLSX/PDF，优化内存解压与重打包机制，避免全量载入引发的内存峰值；
- [ ] 69.2 **内存水位控制与零拷贝 (Zero-Copy / Disk Spooling)**：
  - 实现超过特定阈值（如 30MB）时的临时磁盘缓冲（Spooling）机制，确保低配单机环境下平稳运行，彻底杜绝 OOM；
- [ ] 69.3 **多用户并发压测与性能基准报告**：
  - 编写自动化高并发压测脚本，模拟多文档并行提取与高频脱敏导出，输出吞吐量（TPS）、平均响应延迟与内存占用曲线。

---

## 阶段七十：扫描件与图片型 PDF 的 OCR 像素级光栅化遮盖 (已于阶段一百一十五闭环完成)
- [x] 70.1 **纯本地轻量 OCR 定位引擎集成**：
  - 针对无矢量文字层的扫描件 PDF 与纯图片文档，利用原生 OCR 定位敏感词对应的图像像素矩形框（Bounding Box）；
- [x] 70.2 **图像像素级光栅化遮盖与 PDF 重新压制**：
  - 实现基于 `image` crate 的自适应局部物理高斯模糊涂抹，提取各页 XObject Stream 重新压制生成高保真脱敏 PDF 与原格式脱敏图片。

---

## 阶段七十一：全面剔除 `risk_level` 历史兼容冗余，确立纯 `priority` 单轨架构 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 71.1 **后端数据结构彻底精简 (`src/extractor.rs`)**：
  - 移除 `RuleField` 和 `SensitiveItem` 中遗留的 `risk_level` 别名及所有 Helper 辅助转换反序列化器；
  - 确立 `priority` 检索辅助字段为唯一标准，保留 `#[serde(default = "default_priority")]` 容错；
  - 更新单元测试 `test_rule_field_deserialization` 100% 绿灯。
- [x] 71.2 **前端全链路剔除 `risk_level` 赋值与降级回退 (`web/app.js`)**：
  - 规则表格渲染、标签添加/保存、快照审核、JSON 原始视图、Markdown 高亮、CSV 与全量 JSON 导出中全面清理 `risk_level`；
  - `getCleanRulesPayload()` 仅构造标准 `priority` 字段，彻底根绝 Axum 422 反序列化冲突。
- [x] 71.3 **测试集与运行态全链路验证**：
  - `test_priority_benchmark.py` 清理所有 `risk_level` 依赖；
  - 25 项 Cargo 单元测试全部通过，前端端到端实机验证成功完成离线提取。

---

## 阶段七十二：文档排序功能重构（精简表述、Lucide 图标浮层、默认时间从新到旧） (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 72.1 **替换原生丑陋 `<select>` 为 Geist 现代自适应浮层下拉菜单 (`web/index.html` / `web/style.css`)**：
  - 弃用系统原生蓝色 select 弹窗，构建带有自适应微阴影、毛玻璃与圆角的 `.sidebar-sort-dropdown`；
  - 精简选项表述，移除冗余括号描述（如原“添加时间 (最新在前)”简化为“时间从新到旧”）；
  - 每项排序规则统一配备精准的 Lucide 图标（`clock`、`history`、`arrow-down-a-z`、`arrow-up-a-z`、`arrow-down-wide-narrow`、`arrow-up-narrow-wide`）及激活态勾选符号。
- [x] 72.2 **默认排序规则升级为按时间从新到旧 (`web/app.js`)**：
  - 将 `state.docSortRule` 默认初始值由 `time_asc` 升级为 `time_desc`，新导入或新生成的文档默认置顶展示；
  - 实现点击触发、选项切换、外部点击自动收起闭环交互。
- [x] 72.3 **实机视觉与交互验证**：
  - Chrome 真机截图确认下拉浮层视觉与等宽规范排布正常，即时切换生效。

---

## 阶段七十三：文件格式筛选重构（单行前置、多选下拉浮层、默认全选） (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 73.1 **移除平铺胶囊并构建单行前置筛选组件与多选浮层 (`web/index.html`)**：
  - 彻底移除第二行 `.sidebar-filter-pills`，释放侧边栏垂直 32px 空间；
  - 在搜索框左侧前置 `.sidebar-filter-wrapper`，包含图标、自适应文本与下拉指示器；
  - 构建包含复选框的多选下拉浮层 `.sidebar-filter-dropdown`（全部格式、Word、PDF、Excel、PPT、纯文本、CSV，及底部全选/清空快捷操作）。
- [x] 73.2 **Geist 现代自适应多选浮层样式构建 (`web/style.css`)**：
  - 针对 `.sidebar-filter-btn`、`.sidebar-filter-dropdown` 编写对标排序组件的精致阴影与动效；
  - 解决图标与复选框主题变量适配（采用 `--accent` 单轨），实现自适应文本排布、复选框选中高亮及 `.has-filter` 激活高亮态。
- [x] 73.3 **JavaScript 多选状态机与联动过滤升级 (`web/app.js`)**：
  - `state.docExtFilters` 升级为 Set 集合，默认全部勾选；
  - 封装 `getEffectiveDocExt` 智能识别 AnyDoc 转换后缀（如 `.docx.md` 识别为 Word，`.xlsx.md` 识别为 Excel）；
  - 折叠态按钮自适应文案（默认“全部”，1项显示格式名如“Word”，多项显示“N 项”）；
  - 实现“全部格式”与单项智能互斥联动、列表即时响应过滤。
- [x] 73.4 **全链路实机验证与回归测试**：
  - 通过 Chrome DevTools 自动化实机测试验证单行工具栏渲染、多选勾选交互、搜索+排序组合过滤；
  - 25 项 Rust 后端单元测试持续全绿通过。
- [x] 73.5 **多选面板精简与底部全选反选优化 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 移除顶部冗余的“全部格式”复选框与分割线；
  - 隐藏各格式右侧扩展名文本（`.docx / .doc` 等），宽度收敛至极简 140px；
  - 底部“全选”升级为智能切换按键（全选时显示“取消全选”，非全选时显示“全选”），点击即刻在全选与清空之间无感切换。

---

## 阶段七十四：审计结果动作栏精简与按钮规格统一 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 74.1 **移除冗余导航按钮 (`web/index.html`)**：
  - 彻底移除右侧审计主控面板底部的“返回修改规则重新提取”按钮（`#backToRulesBtn`），精简操作认知负荷。
- [x] 74.2 **导出按钮尺寸与栅格重构 (`web/index.html`, `web/style.css`)**：
  - 底部操作区采用 `display: grid; grid-template-columns: 1fr 1fr; gap: 8px;` 实现严格 50%-50% 对称均分；
  - “导出清单 (CSV)”（`#exportCsvBtn`）与“脱敏导出”（`#exportDesensBtn`）尺寸升级对齐顶部“提取规则”按钮（高度 32px，字号 12px，字重 500）；
  - 脱敏导出下拉菜单（`#exportDropdownMenu`）保留精准向上停靠与层级逻辑。
- [x] 74.3 **实机视觉对齐与全链路测试回归**：
  - Chrome DevTools 实机验证两按钮宽度完全一致（170px vs 170px，高度 32px），与顶部标签栏形成视觉呼应；
  - 25 项 Rust 单元测试全部通过。

---

## 阶段七十五：文档卡片信息重构（审计状态指示 + 智能添加时间） (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 75.1 **精简卡片结构与视觉布局优化 (`web/app.js`, `web/style.css`)**：
  - 彻底移除卡片第二行低价值的“字符数统计”与“快照数量统计”；
  - 采用方案 1 微型药丸风格：在卡片第一行（文件名右侧、删除垃圾桶按钮前）新增类似「存标签」样式的无小圆点微型状态标签（`[已审计]` 翡翠绿底色与边框 / `[待提取]` 中性浅灰底色与边框）；
  - 卡片第二行专注呈现智能添加时间（如 `今天 HH:mm`、`昨天 HH:mm`、`MM-DD HH:mm`），悬停浮动秒级绝对时间；
- [x] 75.2 **全系统设计语言统一与实机验证**：
  - 标签与右侧主控台规则项的「存标签」/「优先级」视觉规格（字号 10px、内边距 1px 5px、圆角 3px）形成 100% 呼应；
  - 补充 light/dark 主题 `--success` 规范变量与微动效，通过 Chrome DevTools 自动化实机测试验证浅色与深色模式下的真实渲染；
  - 25 项 Rust 单元测试持续全绿通过。
- [x] 75.3 **文档卡片激活态底色常驻修复 (`web/style.css`)**：
  - 将 `.file-item.active` 的背景底色由原先回退的 `var(--surface)` 修正为常驻高亮浅灰 `var(--surface-hover)`；
  - 移除了内层 `.file-card-inner:hover` 引起的底色竞争，确保鼠标移开后当前选中项依然清晰稳定保持浅灰底色与深色边框。

---

## 阶段七十六：主控台核心下拉菜单自定义伪下拉（Popover Select）重构 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 76.1 **统一伪下拉菜单样式体系构建 (`web/style.css`)**：
  - 构建 Geist 极简风格的 `.custom-select-wrapper`、`.custom-select-btn` 与浮层 `.custom-select-dropdown`；
  - 支持向下展开与针对底部控制条的向上弹出（`.drop-up`）；
  - 提供 `.custom-select-item`（含 hover/active 态与激活勾选图标 `✓`）、`.custom-select-group-header`（分组大写标签）与 `.custom-select-divider`；
  - 深度适配浅色/深色双色彩模式与阴影。
- [x] 76.2 **场景预设模板下拉框重构 (`web/index.html`, `web/app.js`)**：
  - 将原生 `#presetSelect` 升级为自定义浮层选择器，底层保留隐藏的 `<select>` 作为状态机与事件源；
  - 实现 `syncPresetSelectUi()` 同步当前选中模板文案与激活勾选；
  - 支持列表点击无感切换场景模板、自动同步规则定义并更新“删除此模板”按钮。
- [x] 76.3 **底部提取模型选择器重构 (`web/index.html`, `web/app.js`)**：
  - 将底部 `#footerModelSelect` 改造为紧凑型自定义选择器，结合 `.drop-up` 向上弹出，避免视口底部溢出；
  - 支持 `optgroup` 自动解析（“离线本地模型 (GGUF)”与“在线云端模型 (API)”）；
  - 实现模型切换、在线云端激活与离线本地模型状态联动的平滑同步；
  - 增加全局外部点击（Click Outside）与 ESC 键自动关闭所有浮层的逻辑。
- [x] 76.4 **实机视觉与功能回归测试**：
  - Chrome DevTools 实机验证浅色/深色主题下的渲染效果、展开/收起过渡动效与选择交互；
  - 25 项 Rust 后端单元测试全部通过。

---

## 阶段七十七：桌面客户端沉浸式标题栏、窗口拖拽与跨平台安全区域适配 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 77.1 **原生桌面窗口（Tao + Wry）沉浸式无边框架构重构 (`src/main.rs`)**：
  - 去除传统系统标题栏与菜单栏，启用 `EventLoopBuilder::<UserEvent>::with_user_event()` 事件循环；
  - **macOS**：配置 `with_title_hidden(true)`、`with_titlebar_transparent(true)`、`with_fullsize_content_view(true)`，网页全屏沉浸，系统原生三色红黄绿交通灯浮动于左上角；
  - **Windows**：配置 `with_decorations(false)` 无边框窗口，移除原生标题栏；
  - 建立 Wry IPC 消息通道，支持 `drag_window`、`minimize`、`maximize`、`close` 事件。
- [x] 77.2 **平台安全区域（Safe Insets）与窗口拖动适配 (`web/style.css`, `web/index.html`)**：
  - 顶栏 `.app-header` 添加 `-webkit-app-region: drag` 与硬件级拖拽支持，按钮及操作区声明 `no-drag`；
  - **macOS**：自动增加 `padding-left: 78px`，避让左上角系统原生三色交通灯按钮；
  - **Windows**：右上角增加标准 Windows 风格无边框三联控制按钮（最小化、最大化/还原、关闭），关闭按钮悬停红色高亮；
  - **Web 模式**：纯浏览器访问保持零额外内边距与无 Windows 按钮，无侵入式平稳降级。
- [x] 77.3 **前端环境嗅探与双重拖拽事件保障 (`web/app.js`)**：
  - 注入 `window.__SENSIDOC_DESKTOP__` 与平台参数，在 `DOMContentLoaded` 第一时间动态赋予平台类名；
  - 绑定鼠标左键拖拽（`drag_window`）与双击顶栏最大化/还原（`maximize`）双重保障。
- [x] 77.4 **实机视觉与跨平台回归测试**：
  - Chrome DevTools 实机验证 macOS 模式（左侧 78px 边距）、Windows 模式（右上角三联按键）与纯 Web 浏览器模式（标准无留白）；
  - 25 项 Rust 自动化单元测试（`cargo test`）全绿通过。
---

## 阶段七十八：macOS交通灯垂直居中、300px侧边栏、设置面板自定义下拉重构与仅DMG构建 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 78.1 **macOS 交通灯按钮垂直居中优化 (`src/main.rs`)**：
  - 引入 `tao::dpi::LogicalPosition`，设置 `.with_traffic_light_inset(LogicalPosition::new(16.0, 15.0))`；
  - 使红黄绿控制按钮在 44px 高度的沉浸式顶栏内垂直居中，并与 SensiDoc 软件标题在同一水平中轴线上。
- [x] 78.2 **“文档列表”左侧面板默认显示宽度调整为 300px (`web/style.css`, `web/app.js`)**：
  - 更新 CSS 根变量 `--sidebar-width: 300px;`；
  - 更新 JS 默认宽度逻辑，并自动将旧默认值 250px 迁移为 300px。
- [x] 78.3 **“设置”面板原生 `<select>` 自定义伪下拉改造 (`web/index.html`, `web/app.js`)**：
  - 将“在线 AI 模型配置”中的 `#onlineModelSelect` 与“提取规则设定”中的 `#promptTargetModelSelect` 改造为 `.custom-select-wrapper`；
  - 保持底层原生 `<select style="display: none;">` 零破坏兼容，上层提供自定义触发按钮与浮层菜单；
  - 实现双向联动：选项点击后同步赋值原生 select 并分发 `change` 事件；在模型加载、表单命名输入、规则切换时自动同步自定义 UI 状态与选中对勾；
  - 接入全局外部点击关闭与 ESC 快捷键响应。
- [x] 78.4 **macOS 打包产物精简为纯 DMG (`scripts/build_mac_app.sh`, `.github/workflows/release.yml`)**：
  - `build_mac_app.sh` 移除生成 `.zip` 压缩包步骤，专注生成 `.dmg` 安装映像；
  - GitHub Actions 构建工作流移除 macOS 版本的 zip 收集项，仅上传与发布 DMG。
- [x] 78.5 **实机交互验证与测试回归**：
  - Chrome DevTools 实机验证设置面板两个下拉菜单的展开、切换联动及 300px 默认侧边栏宽度；
  - 25 项 Rust 单元测试（`cargo test`）全绿通过；
  - 本地仅 commit，不推送到 GitHub 远端。

---

## 阶段七十九：Windows标准安装包构建、应用图标嵌入、微软雅黑字体统合、窗口关闭二次确认与macOS双架构Universal兼容 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 79.1 **Windows Inno Setup 标准安装包流水线 (`scripts/installer.iss`, `scripts/build_win_installer.ps1`, `.github/workflows/release.yml`)**：
  - 编写官方级 Inno Setup 脚本，产出单一便携的 `sensidoc-v{VERSION}-windows-x86_64-setup.exe` 安装向导程序；
  - 自动创建桌面快捷方式（带自定义图标）、开始菜单项、标准 Windows 卸载程序（`unins000.exe`）及安装完成立即启动向导；
  - CI 工作流改为自动编译调用 Inno Setup 生成安装包并发布。
- [x] 79.2 **Windows PE 可执行文件图标与元数据嵌入 (`build.rs`, `Cargo.toml`)**：
  - 引入 `winres = "0.1"` 作为 Windows 目标平台的构建依赖；
  - 新建 `build.rs`，在 Windows 编译时自动将 `assets/sensidoc_win.ico` 嵌入 `sensidoc.exe` 的 PE 资源段中；
  - 解决 Windows 资源管理器、任务栏和快捷方式显示为默认空白通用图标的问题。
- [x] 79.3 **Windows 界面字体统合为“微软雅黑” (`web/style.css`, `web/app.js`)**：
  - 更新 `--font-sans` 回退链，针对 Windows 优先匹配 `"Segoe UI", "Microsoft YaHei", "微软雅黑"`；
  - 针对 `platform-win` 声明强覆盖，并确保所有 `button`, `input`, `select`, `textarea` 继承字体；
  - 彻底解决 Windows 下由于系统默认无衬线字体回退机制导致的宋体与黑体混杂显示问题。
- [x] 79.4 **跨平台窗口关闭统一二次确认 (`src/main.rs`, `web/app.js`)**：
  - 前端封装 `handleWindowCloseRequest()`，调用现有的标准 `showConfirmDialog` 模态框，支持取消与确认退出；
  - Windows 原生端点击右上角 `#winCloseBtn` 触发确认框；
  - 后端 Tao 窗口收到 `WindowEvent::CloseRequested`（点击 macOS 交通灯红点或按 Alt+F4 / Cmd+W）时拦截直接关闭，向 webview 发送脚本拉起前端退出确认框；
  - 只有用户在前端点击“确认退出”后，才分发 `force_close` 并由后端安全停止模型推理服务并退出应用。
- [x] 79.5 **macOS Universal 架构（M 处理器 + Intel 处理器）双兼容与产物命名 (`scripts/build_mac_app.sh`, `.github/workflows/release.yml`)**：
  - `build_mac_app.sh` 增加 `aarch64-apple-darwin` 与 `x86_64-apple-darwin` 目标构建与 `lipo -create` 通用二进制合并；
  - 一键产出兼顾 M1/M2/M3/M4 与 Intel Mac 的 Universal DMG；
  - 产物采用规范全小写通用标识：`sensidoc-v{VERSION}-macos-universal.dmg` 与 `sensidoc-v{VERSION}-windows-x86_64-setup.exe`。
- [x] 79.6 **实机视觉与功能回归测试**：
  - Chrome 实机验证 Windows 模式下字体计算值（全部统一为 Segoe UI + Microsoft YaHei）与退出二次确认弹窗的弹出与交互；
  - 25 项 Rust 后端自动化单元测试（`cargo test`）全绿通过。

---

## 阶段八十：本地全功能网页服务启动与联调测试 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 80.1 **历史进程安全清理与新版本重新编译构建**：
  - 终止旧版长驻后台进程（PID 93329），干净释放 3000 端口；
  - 基于最新代码完成增量构建，生成载入最新 UI 与拦截逻辑的独立服务端二进制。
- [x] 80.2 **本地无头 HTTP API 服务拉起与系统浏览器联动**：
  - 以 `sensidoc --server` 模式拉起本地后台服务，监听 `http://127.0.0.1:3000`；
  - 验证本地模型接口 `/api/models/local` 响应就绪；
  - 通过系统级调用自动在默认浏览器中打开服务页面供实机测试。
- [x] 80.3 **用户测试覆盖项对齐**：
  - 300px 默认文档侧边栏宽度舒展性体验；
  - 设置面板在线模型与规则提示词模型的自定义伪下拉弹出、选项切换与选中对勾；
  - Windows 环境微软雅黑与 Segoe UI 字体统合，杜绝宋体混杂；
  - 关闭窗口防误触二次确认保护。

---

## 阶段八十一：sensidoc agent 外部智能体自调优与自进化 CLI 模块 (待办 / 规划中)
- [ ] 81.1 **`sensidoc agent inspect` (内省与环境导出)**：
  - 支持导出当前模型清单、各参数量模型提示词档案与模板规则配置为机器可读纯 JSON；
  - 供外部 Agent (如 Claude Code, Cursor, Antigravity) 自动化了解当前软件运行基线。
- [ ] 81.2 **`sensidoc agent eval` (沙盒评测与量化打分体系)**：
  - 基于内置 benchmark 真实多场景样本集，支持传入候选 prompt 或策略进行沙盒干跑（dry-run）；
  - 输出机器友好结构化评估报告 JSON (Precision, Recall, F1 分数、推理耗时及具体错题样本)。
- [ ] 81.3 **`sensidoc agent apply` (策略与提示词安全回写)**：
  - 支持 Agent 自动化将调优后的轻量端侧模型专属提示词与场景模板安全持久化；
  - 实现从“评测-微调-回写-生效”的自闭环进化。

---

## 阶段八十二：生效规则定义「AI智能生成」两段式端云协同提炼模块 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 82.1 **后端 API 提取策略智能提炼接口 (`src/main.rs`)**：
  - 新增 `/api/rules/ai-generate` POST 路由与 `ai_generate_rules` 控制器；
  - 根据选中的 `model_id` 自动索引已配置的在线大模型（如 DeepSeek-V3）；
  - 构造包含输出规范、防幻觉约束与严格 JSON 格式的元提示词，向在线大模型请求敏感字段提炼并鲁棒解析为规则列表。
- [x] 82.2 **右侧面板操作按钮统合、标题极简化与方案三（主次分流·底部添加）重构 (`web/index.html`, `web/style.css`)**：
  - 标题彻底精简为纯粹的「**规则列表**」，移除末尾冗余的 `(X 项)` 统计内容；
  - 标题行右侧精简统一为两个高度对称的轻量实体按钮：「**AI生成**」与「**标签库**」，彻底消除“实体按钮 + 蓝色纯文字”在标题行的混搭拥挤感；
  - 将「**新增规则**」重构为规则卡片列表底部的常驻扩展入口（`#addFieldBtn`），采用全宽虚线轻质卡片设计（`＋ 点击添加自定义规则`，Notion / Linear 现代工作台风格），符合清单逐行追加直觉；
  - 抽屉面板视觉重塑：纯白底色（`var(--surface)`）+ 极细浅灰边框（`var(--border)`）+ 8px 圆角与柔和微阴影，移除右上角叉号，界面通透聚焦。
- [x] 82.3 **模型下拉双层架构重构与原生另存模板弹窗联动 (`web/app.js`)**：
  - **双层架构模型下拉**：保留底层原生 `<select>`（设为 `display: none`），所有表单读写、API 参数绑定原封不动；顶层呈现统一的伪下拉 Trigger 按钮与浮层菜单，点击选项自动同步并触发 `change` 事件；
  - **候选字段审查与模态联动**：AI 生成结果实时渲染为多选候选列表（包含字段名、风险优先级徽标与提取特征描述）；
  - 点击「仅应用到当前规则」直接注入当前生效规则并收起抽屉；
  - 点击「保存为模板并生效」先更新生效规则，再自动呼出既有的原生「另存为场景模板」模态弹窗（`#saveTemplateModal`），保持操作体验一致。
- [x] 82.4 **端到端实机验证与质量闭环**：
  - 25 项 Rust 单元测试（`cargo test`）全绿通过；
  - Chrome 实机端到端模拟测试（标题纯粹无尾缀、标题行双轻按钮对齐、底部虚线添加卡片点击拉起弹窗、抽屉展开/收起/互斥联动、伪下拉展开收起与双向绑定）全部正常。

---

## 阶段八十三：全系统双层架构自定义伪下拉组件即时同步与闭环加固 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 83.1 **点击项即时同步逻辑全面重构 (`web/app.js`)**：
  - 根因定位：旧逻辑在 `select.value !== val` 时只派发 `change` 事件而遗漏了立即调用 `syncXxxSelectUi()`，且部分原生 `<select>` 缺少 `change` 监听，导致折叠后 Trigger 文本不刷新、需二次展开才更新；
  - 重构全系统 5 个自定义下拉组件的点击事件处理：先设置 `select.value = val` 并**立即执行** `syncXxxSelectUi()` 刷新 Trigger 文案与选中高亮，值发生变化时派发 `change` 事件通知业务层，最后关闭浮层。
- [x] 83.2 **双向联动保底与文本查找健壮性加固 (`web/app.js`)**：
  - 为 `aiGenRulesModelSelect` 补充原生的 `change` 事件监听联动 `syncAiGenRulesModelSelectUi`；
  - 在 `presetSelect`、`footerModelSelect`、`onlineModelSelect` 的 `change` 回调中补充即时 UI 同步；
  - 加固 5 个 `syncXxxSelectUi` 函数中的选项文本查找算法（优先使用 `Array.from(options).find(o => o.value === currentVal)`），彻底避免隐藏 select 在特定时序下 `selectedIndex` 滞后。
- [x] 83.3 **端到端实机验证与测试闭环**：
  - 静态资源版本缓存升级至 `v=1.2.8`；
  - Chrome 实机测试全部 5 个下拉组件（AI 抽屉模型、场景预设模板、底部执行模型、设置面板在线模型、设置面板提示词模型），选项切换后折叠即刻显示最新选中项；
  - 25 项 Rust 后端单元测试（`cargo test`）全绿通过。
- [x] 83.4 **规则列表空状态引导文案精简与方位纠偏 (`web/app.js`)**：
  - 修正原“从上方选取标签或点击「+ 新增规则」添加”中“新增规则”方位与当前底部入口脱节的问题；
  - 精炼为两行清晰指引：「暂无生效规则 / 可从上方选取标签/AI生成，或点击下方添加」，言简意赅。
- [x] 83.5 **文档列表与提取结果列表选中态外边框线宽视觉对齐 (`web/style.css`)**：
  - 为 `.file-item.active` 引入与 `.audit-card.active` 相同的 `box-shadow: 0 0 0 1px var(--text);` 扩散轮廓与过渡效果；
  - 呈现一致且不占盒模型宽度的 2px 黑色微加粗选中边框，两端交互视觉感受完全统合。
- [x] 83.6 **文档列表选中项底纹颜色与提取结果卡片对齐 (`web/style.css`)**：
  - 将 `.file-item.active` 的背景色由 `var(--surface-hover)` 改为 `var(--surface-active)`；
  - 实现左侧文档卡片与右侧提取结果实体卡片在选中态的浅灰色底纹与 2px 边框线宽 100% 视觉统合。

---

## 阶段八十四：v1.3.0 正式版本发布与 GitHub 自动化构建 (进行中) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 84.1 **工程版本号升级与安装脚本同步**：
  - `Cargo.toml` 与 `Cargo.lock` 正式升级至 `v1.3.0`；
  - 同步更新 Windows 安装包配置 `scripts/installer.iss` 与 `scripts/build_win_installer.ps1` 默认版本号至 `1.3.0`；
  - 静态资源版本标识同步更新。
- [x] 84.2 **全套回归测试与本地验证**：
  - 运行 `cargo test`，25 项单元测试全绿通过。
- [x] 84.3 **Git 分支与 Release Tag 推送至 GitHub**：
  - 本地 master 分支最新提交已成功推送到 `origin master`；
  - 成功创建并推送 Git Tag `v1.3.0`，触发 GitHub Actions 跨平台自动打包与发布流水线。
- [x] 84.4 **GitHub Actions 跨平台应用构建初次触发**：
  - 工作流首次运行排查，定位到 Windows 端 Inno Setup 缺失语言包导致中断。

---

## 阶段八十五：Windows 构建异常修复与流水线闭环监控 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 85.1 **根因排查与定位**：
  - Windows CI 环境下 Inno Setup 6 默认不包含非官方中文语言包，构建时抛出 `Couldn't open include file ChineseSimplified.isl` 导致编译中断；
  - 随后识别到缺失 `README.md` 导致的次级编译中断。
- [x] 85.2 **语言包内置与打包流水线增强**：
  - 仓库内置官方标准 `scripts/ChineseSimplified.isl`，修复 `scripts/installer.iss` 中的相对路径依赖；
  - 升级 `scripts/build_win_installer.ps1` 与 `.github/workflows/release.yml`，同步生成 `.exe` 安装程序与 `.zip` 绿色免安装版；
  - 根目录补全标准 `README.md`，并在 `installer.iss` 中为外部文件追加 `skipifsourcedoesntexist` 容错标记。
- [x] 85.3 **更新代码与 Tag 并推送至 GitHub**：
  - 推送 master 分支与更新后的 `v1.3.0` Tag，触发全新 GitHub Actions 流水线。
- [x] 85.4 **持续监控构建全过程直至发布成功**：
  - 实时监控 macOS（2m14s）、Windows（22m39s）跨平台任务与 Release 发布任务（17s），所有阶段 100% 成功闭环。
- [x] 85.5 **CI 构建产物 Artifact 名称统合与补充软件名前缀 (`.github/workflows/release.yml`)**：
  - 将 Actions 页面展示的 `macos-app-v*` 与 `windows-app-v*` 统一调整为 `sensidoc-v*-macos` 与 `sensidoc-v*-windows`，确保构件包名带有明确的 `sensidoc` 软件标识。

---

## 阶段八十六：左侧文档空状态区域滚动条异常排查与消除 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 86.1 **CSS 盒模型根因精确定位**：
  - 通过 Chrome DevTools 实机测量空状态盒模型参数：`.file-list` 实际可用高度 622px（642px - 20px padding）；
  - 子元素 `.file-list-empty` 同时设置了 `height: 100%`（622px）与 `margin: 12px`，导致计算总高度为 646px，超出包含块 24px，触发父级 `overflow-y: auto` 的纵向滚动条渲染。
- [x] 86.2 **自适应布局重构与双重保险机制**：
  - `web/style.css` 中将 `.file-list` 改为弹性列布局（`display: flex; flex-direction: column;`），空状态卡片采用 `flex: 1; box-sizing: border-box;` 自动撑满剩余空间，彻底移除外层 12px 冗余边距；
  - 增加 `.file-list:has(.file-list-empty) { overflow-y: hidden; }`，在空状态及搜索无果状态下彻底关闭滚动条；
  - 为 `.file-item` 补充 `flex-shrink: 0;`，保证多文档列表正常滚动且项高不受压迫。
- [x] 86.3 **静态资源缓存版本迭代与实机验证**：
  - `web/index.html` 引用版本提升为 `style.css?v=1.2.11`；
  - 实机验证无文档空状态与搜索无果状态：`hasScrollbar: false`，`scrollHeight === clientHeight`，无任何滚动条；多文档加载状态下滚动条正常工作。

---

## 阶段八十七：规则列表交互微调与执行按钮精简 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 87.1 **添加自定义规则按钮移位至列表上方**：
  - 将 `button#addFieldBtn` 从卡片流底部迁移至规则列表顶部（抽屉下方、卡片流上方），与顶部添加后倒序插入最顶部的业务逻辑完全契合；
  - 更新样式 `.add-rule-top-btn`（上下外边距 `margin: 6px 0 8px;`），视觉与列表项无缝呼应。
- [x] 87.2 **规则列表空状态引导文案与居中文本对齐**：
  - 空状态文案同步升级为“暂无生效规则 / 可从上方选取标签、AI生成或添加自定义规则”；
  - 新增 `.rule-card-empty` 弹性全高居中类（`flex: 1; justify-content: center;`），彻底解决空状态提示偏上问题。
- [x] 87.3 **底部模型控制条融入底部与执行按钮文案精简**：
  - 移除 `.model-control-strip` 的灰色底纹与外边框，使其透明平滑融入 `.inspector-footer`；
  - 将底部主执行按钮文案由“立即执行提取”精简为“立即执行”，同步更新审计空状态对应文案与 JS 状态复位逻辑（保留 SVG 图标）。

---

## 阶段八十八：底部操作区方案三一体化复合执行按钮重构 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 88.1 **淘汰冗余 Toggle 拨杆与单行重载结构**：
  - 消除混杂在单行内的物理启停开关与“模型:”纯文本标签，将后端进程启停完全收敛至“按需自动拉起”直觉模型；
  - 在 `web/index.html` 中采用 `.split-extract-group` 一体化复合组件重构右侧底座。
- [x] 88.2 **左侧模型选择器深度适配 Split Button**：
  - 左半区集成 `.split-model-btn`，无缝保留完整的自定义下拉弹出层交互与样式（包含本地 GGUF 与云端 API 分组、选中对勾、圆角阴影等）；
  - 向上弹出菜单（drop-up）精准定位于按钮上方，选定后自动同步显示当前模型。
- [x] 88.3 **右侧高亮执行动作区联动**：
  - 右半区采用主题适配的 `.split-extract-btn`，深浅色主题自适应（浅色纯黑高对比 / 深色亮白高对比），平滑衔接左侧选择器；
  - 页面静态资源版本更新至 `v=1.2.14`，实机验证深浅色主题及模型切换功能 100% 正常工作。

---

## 阶段八十九：离线模型独立参数配置（采样温度、候选范围、重复惩罚）抽屉架构 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 89.1 **离线模型 Profile 持久化与数据结构 (`src/session.rs`, `src/main.rs`)**：
  - `SessionData` 新增 `offline_model_profiles: HashMap<String, OfflineModelProfile>`，实现各模型参数独立持久化存储至 `.sensidoc_workspace.json`；
  - 注册 `GET /api/models/offline-profiles` 与 `POST /api/models/offline-profiles` 路由；
  - 启动离线提取时动态读取当前模型的独立参数并传入提取引擎。
- [x] 89.2 **离线模型卡片内嵌式折叠抽屉交互 (方案一) (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 在已下载模型与本地扫描模型卡片中增加齿轮参数配置按钮（`[⚙]`）；
  - 点击平滑展开嵌入式参数抽屉，支持采样温度 (Temp)、候选范围 (Top-K)、重复惩罚 (Repeat) 的即时调节；
  - 支持失焦/修改自动防抖保存，提供“恢复默认”一键重置功能。

---

## 阶段九十：离线与在线模型思考模式 (Thinking / CoT) 开关与容错架构 (方案二：4 列单行紧凑并排) (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 90.1 **推理思考开关数据结构与双轨透传 (`src/session.rs`, `src/main.rs`)**：
  - `OfflineModelProfile` 与 `OnlineModelProfile` 结构体新增 `#[serde(default)] pub enable_thinking: bool`，默认关闭（`false`）；
  - `/api/models/offline-profiles` 与 `/api/settings/online-models` 接口全面支持 `enable_thinking`；
  - 在线与离线推理调度流程动态读取思考开关配置并透传至提取引擎。
- [x] 90.2 **提取引擎与思维链标签智能剥离容错 (`src/extractor.rs`, `src/cli.rs`, `src/benchmark.rs`)**：
  - `Extractor::query_llm` 与 `Extractor::query_online_llm` 接入 `enable_thinking` 参数，开启思考模式时自动扩展生成长度上限；
  - `Extractor::parse_llm_json_response` 增加智能思维链截断：若检测到 `</think>` 标签，优先截取标签后的正文内容，彻底杜绝 DeepSeek-R1、QwQ 等模型思考过程干扰 JSON 解析；
  - 编写并通过单元测试 `test_parse_llm_json_response_with_thinking`。
- [x] 90.3 **前端 4 列并排紧凑网格与 Toggle 按钮落地 (方案二) (`web/style.css`, `web/index.html`, `web/app.js`)**：
  - `.model-param-grid` 升级为 4 列并排（采样温度、候选范围、重复惩罚、思考模式）；
  - 新增 `.param-toggle-btn` 样式（高度 28px 与输入框严格等高对齐，浅色/深色主题自适应）；
  - 思考模式状态指示直观醒目：`[●──] 未开启` / `[──●] 已开启`，默认全关闭；
  - 在线模型配置表单与离线模型参数抽屉同步对齐 4 列网格设计；
  - 支持一键“恢复默认”（一键还原为关闭思考及基准数值）。
- [x] 90.4 **全链路端到端功能验证与回归测试**：
  - 26 项 Rust 单元测试（`cargo test`）100% 绿灯通过；
  - 静态资源缓存标识升级至 `v=1.2.16`；
  - Chrome 实机端到端验证深浅色主题、4 列对齐、点击切换、持久化落盘与一键恢复默认。

---

## 阶段九十一：最大输出 Token (Max Tokens) 约束、双行充裕布局 (方案 B) 与执行中手动打断停止功能 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 91.1 **Max Tokens 参数结构与全链路透传 (`src/session.rs`, `src/extractor.rs`, `src/main.rs`, `src/cli.rs`, `src/benchmark.rs`)**：
  - `OfflineModelProfile` 与 `OnlineModelProfile` 新增 `#[serde(default = "default_max_tokens")] pub max_tokens: u32`（离线默认 1024，在线默认 2048），有效防范小模型幻觉复读与死循环；
  - `Extractor::query_llm` 与 `Extractor::query_online_llm` 接收并传递 `max_tokens` 至请求体；
  - `src/main.rs` 与 `src/cli.rs` 动态读取并透传对应 Profile 的 `max_tokens`。
- [x] 91.2 **模型参数面板双行充裕布局重构 (方案 B) (`web/style.css`, `web/index.html`, `web/app.js`)**：
  - **第 1 行（4 列数值网格）**：采样温度 (Temp)、候选范围 (Top-K)、重复惩罚 (Repeat)、最大Token (Max)；
  - **第 2 行（独立状态与操作行）**：
    - 左侧：`思考模式 (Think)：` + 紧凑 Toggle 按钮（`[●──] 未开启` / `[──●] 已开启`） + `(针对推理模型)` 辅助提示；
    - 右侧：状态指示与 `[ 恢复默认 ]` 一键还原按钮（离线抽屉重置为 0.1 / 50 / 1.1 / 1024 / 关闭思考）；
  - 在线模型表单与离线模型抽屉视觉语言 100% 对齐。
- [x] 91.3 **执行中原位切换「停止执行」与前后端打断闭环 (`web/app.js`, `web/style.css`)**：
  - **前端状态流转**：点击「立即执行」后，右侧复合按钮原位切换为醒目的高亮危险红 `[ ⏹ 停止执行 ]`（带呼吸脉冲动效），左侧模型选择器锁定禁用；
  - **用户手动打断**：执行过程中再次点击「停止执行」，触发 `AbortController.abort()` 立即中断 HTTP 请求，前端友好捕获 `AbortError`，零报错弹窗优雅复位；
  - **后端算力释放**：客户端连接断开时，底层 Tokio 自动取消任务并断开与 `llama-server` / API 连接，释放 GPU/CPU 计算资源。
- [x] 91.4 **全链路实机验证与回归测试**：
  - 26 项 Rust 自动化单元测试全部通过（100% 绿灯）；
  - 静态资源版本升级至 `v=1.2.17`；
  - Chrome 实机端到端验证抽屉数值输入、失焦保存、恢复默认、在线模型配置与执行中红色打断停止按钮交互正常。

---

<<<<<<< HEAD
## 阶段九十二：纸质文档与表格 OCR 识别原生集成 (PP-OCRv6 + SLANet_plus + 卷帘透视比对) (已完成) · [🔗 对话跳转](conversation://6efdb7c6-eaca-4a22-8acc-97baeea17ede)
- [x] 92.1 **基础设施与 ONNX 模型按需下载器 (`Cargo.toml`, `src/paths.rs`, `src/model_manager.rs`, `src/main.rs`)**：
  - 集成 Rust `ort 2.x`（ONNX Runtime 原生绑定）、`ndarray`、`image`、`imageproc` 依赖；
  - `src/paths.rs` 增加 `get_ocr_models_dir()`、4 大组件文件名常量与 `get_ocr_status()` / `is_ocr_ready()` 检测；
  - `src/model_manager.rs` 增加 OCR 专属模型包（`PP-OCRv6_det_small.onnx`, `PP-OCRv6_rec_medium.onnx`, `slanet-plus.onnx`, `ppocrv6_dict.txt`）下载支持，复用 ModelScope RapidAI 官方直链、HTTP Range 断点续传与取消清理；
  - `src/main.rs` 注册 `/api/ocr/status`、`/api/ocr/download`、`/api/ocr/cancel`、`/api/ocr/delete` 接口；
  - 跑通 `test_ocr_bundle_metadata` 与 `test_cancel_ocr_download_cleans_cache` 等 28 项单元测试（100% 绿灯通过）。
- [x] 92.2 **纯 Rust 核心 OCR 推理引擎流水线 (`src/ocr/`)**：
  - `src/ocr/preprocessor.rs`：图像标准化、自适应 Letterbox 缩放与 NCHW 张量组装；
  - `src/ocr/text_detector.rs`：基于 `PP-OCRv6_small_det` (RepLKFPN) 的文本行定位与二值化轮廓提取；
  - `src/ocr/text_recognizer.rs`：基于 `PP-OCRv6_medium_rec` (LightSVTR) 的文本切片推理与 50 种语言统一字典 CTC 解码；
  - `src/ocr/table_structure.rs`：基于 `slanet-plus` 的无线/复杂表格 HTML 骨架预测与 Cell BBox 坐标回归；
  - `src/ocr/matcher.rs`：基于空间几何中心点包含与 Coverage 面积比的单元格-文本拓扑关联匹配器；
  - `src/ocr/markdown_builder.rs`：结构化表格转标准 GFM Markdown / 兼容 HTML `<table>`，并自动规整单据头尾键值对；
  - `src/ocr/mod.rs`：对外暴露 `OcrEngine` 统一入口并编写单元测试，32 项测试全部通过。
- [x] 92.3 **转换引擎多格式分流与扫描版 PDF 提取 (`src/converter.rs`, `src/main.rs`)**：
  - `DocConverter::convert_bytes` 新增图片拦截（`.jpg/.jpeg/.png/.bmp/.webp/.tiff` 及魔数探测）直接分流至 `OcrEngine`；
  - 针对 `.pdf`：当 `anydoc` 提取字符数过少（纯扫描件）时，利用 `lopdf` 提取内嵌图像流并按页聚合 OCR 输出；
  - 提供原图访问接口 `GET /api/documents/{id}/file`，支持内联访问与无损展示。
  - 跑通 `test_is_image_detection` 与 `test_image_ocr_routing_when_not_ready`，34 项单元测试 100% 绿灯通过。
- [x] 92.4 **前端设置面板重构与首拖情境引导卡片 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 设置模态框 Tab 1 顶部集成「纸质单据与密集表格 OCR 引擎」组件卡片（实时展示就绪状态、89.5MB 体积、一键下载/取消/删除与进度条）；
  - 首次将图片拖入软件且未安装模型时，弹出友好的一键下载引导卡片（`#ocrPromptModal`），支持原地流式展示进度并在下载完成后自动恢复识别处理。
- [x] 92.5 **方案二：视界拨杆与卷帘透视比对视图落地 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 中间主舞台工具栏增加智能四态视界拨杆：`[渲染] | [卷帘比对] | [原图] | [源码]`（图片文档自适应激活）；
  - 基于 CSS `clip-path` 与 `var(--curtain-pos)` 实现 60fps 硬件加速自由左右拖拽垂直卷帘滑轨，底层为原始扫描件、顶层为结构化表格；
  - 接入双向滚动联动同步，敏感词高亮标注无缝映射至卷帘顶层，点击单元格敏感词平滑联动右侧审计面板。
- [x] 92.6 **全链路端到端功能验证与回归测试**：
  - 真实跑通 ModelScope 官方源静默按需下载（94.39MB 完整套件就绪）；
  - 端到端真实单据图像 OCR 提取测试通过（原生耗时 ~700ms，生成语义化 HTML/Markdown）；
  - 原图内联访问接口 `GET /api/documents/{id}/file` 验证无损可用；
  - 全部 34 项 Cargo 单元测试 100% 绿灯通过，静态资源版本升级为 `v1.3.0`。
- [x] 92.7 **表格 HTML 骨架与单元格对齐修复 + 文档列表即时入库与状态机扩展 (`src/ocr/`, `web/`)**：
  - **表格拓扑与词表对齐**：彻底排查并对齐 SLANet_plus 官方 50-token 词表，修复因词表偏移导致的 `<tr>`/`<td>` 解码紊乱；
  - **8点坐标与等比缩放修复**：重构 `TableStructurePredictor`，正确支持 8 点多边形 `[min, max]` 提取，并按 `max_dim` 正确映射回原图物理像素坐标；
  - **无畸变预处理与表格组装**：预处理改为按最长边等比例缩放并在 488x488 补 0，对齐 RapidTable 标准；重构 `MarkdownBuilder` 单元格注入逻辑，无 span 自动降级为标准 GFM 管道表格，有合并单元格输出语义化 HTML 表格；
  - **文档列表即时呈现（乐观入库）**：上传文件（图片/文档）瞬间立即插入左侧文件列表并高亮选中，中间视图呈现优雅的骨架加载动画与提示；
  - **文档生命周期状态机扩展**：状态药丸标签扩展支持 `OCR识别中...`（带呼吸脉冲）、`解析中...`、`待提取`、`已审计` 与 `解析失败`，转换完成后无缝切换为正式状态。
- [x] 92.8 **长图海报核心表格单调拓扑对齐与非表格正文段落解包 (`src/ocr/table_structure.rs`, `src/ocr/matcher.rs`, `src/ocr/markdown_builder.rs`)**：
  - **核心多列表格智能识别**：自动探测并提取连续多列 (`cols >= 2`) 的核心表格网格，剥离长图海报顶部标题与底部说明文字的伪 `colspan` 单元格包裹；
  - **行级单调拓扑对齐 (Row-level Monotonic Topological Alignment)**：采用中位数 $y$ 确定稳健表格上下界，将表格候选文本行与核心表格结构行进行单调一对一对应，彻底解决行重叠（Ghost Row）导致的空行与数据合并错位；
  - **单元格精准水平映射与多行换行保护**：同行文本按水平列序一对一注入单元格；单元格内多行垂直文本用 `<br>` 连接并在 HTML 转义中保留；
  - **表外正文与章节小标题自然 Markdown 流化**：表格上方的单据大标题与分类提取为一级/二级标题；表格下方的流程图与申请规则等条款，智能提取为 `### 小节标题` 并逐行渲染为 Markdown 列表项，彻底根治换行丢失与文本压扁问题；
  - **全量测试通过**：37 项单元测试 100% 绿灯，端到端长图海报 (`售前激励政策.jpeg`) 真实提取验证完美无误。
- [x] 92.9 **扫描版/图片型 PDF 智能路由引擎与密集文本框全序排序修复 (`src/converter.rs`, `src/ocr/text_detector.rs`, `src/ocr/matcher.rs`, `src/ocr/markdown_builder.rs`)**：
  - **根本原因诊断**：针对多页扫描版账单 PDF (`CHAY DA IMPORT-C1-MAY-2026-$ 8,545.18.pdf`)，anydoc 明确抛出无文本层扫描件异常；但在路由至 OCR 推理时，底层文本检测器原有的阅读顺序排序函数因阶梯状坐标分布违反了数学全序（Total Order violation），导致 Rust 标准库排序算法 panic 崩溃；且未就绪时未返回 `OCR_NOT_READY` 提示；
  - **严格数学全序重构**：`text_detector.rs` 与 `matcher.rs` 彻底重构为基于行分桶与 IEEE 浮点严格 `total_cmp` 的排序逻辑，彻底根治 driftsort panic 崩溃；
  - **多页扫描版 PDF 自动路由闭环**：
    - `converter.rs` 新增智能探测：当 anydoc 提取字符极少或明确提示 Scanned 时，自动无缝判定为图片类型 PDF 并路由至 OCR；
    - 模型未就绪时精准返回 `OCR_NOT_READY`，前端立即唤起一键按需下载；
- [x] 92.10 **扫描版 PDF 页面旋转规范自适应与高精度文本/表格识别修复 (`src/converter.rs`, `src/ocr/mod.rs`, `src/main.rs`)**：
  - **根本原因诊断**：参考 `industry_PDF` 实践并深入底层 PDF 字典结构分析发现，该 4 页扫描件内部内嵌的原始扫描图像尺寸为横向 (`2340x1654`)，但在 PDF 页面标准中标记了 `/Rotate 270`（逆时针 90° 倒向）。旧代码直接提取原始 DCT 流送入 OCR，导致文本检测器将纵向文本切成细碎横条，识别出乱码字符碎片（如 `33 商 I i 原`、`0 n 7` 等），且 SLANet 强行将横向切片预测为混乱表格；而 `industry_PDF` 通过 MuPDF 渲染时自动执行了旋转与 DPI 转换；
  - **PDF 旋转属性继承与自动矫正**：
    - 在 `src/converter.rs` 中新增 `get_page_rotation`，严格遵从 ISO 32000-1 规范，支持递归溯源 `/Parent` 树继承的 `/Rotate` 属性（支持 90°、180°、270° 等任意倍数角度）；
    - 提取内嵌图像后，智能根据页面角度自动调用 `img.rotate270()` 等精准矫正为直立人眼阅读方向；同时过滤小于 100x100 的小图标/噪点；
  - **动态图像直传与推理优化**：
    - `src/ocr/mod.rs` 新增 `OcrEngine::recognize_image(&DynamicImage)` 接口，扫描件解码旋转后直接在内存中传递给检测器与识别器，免去重复 JPEG 编解码开销；
    - `src/main.rs` 的 `/api/convert` 接口使用 `tokio::task::spawn_blocking` 执行计算密集型 OCR 推理，彻底避免大文档推理对 Tokio 异步运行时的事件饥饿；
  - **全量实机验证**：
    - 矫正后全文档 4 页端到端识别准确率极大提升：
      - 第 1 页：检测出 251 个文字区域，完整准确提取出 `SUPERL (CAMBODIA) CO., LTD.`、`PAYMENT REQUISITION FORM`、各项发票单号（`2605011672` 等）与明细金额；
      - 第 2 页：检测出 51 个文字区域，完整提取出英文审批邮件链（`From: Cyrus Yu`、`Approved` 等）；
      - 第 3 & 4 页：检测出 168 与 196 个文字区域，完整提取出物流对账单（`CHAY DA LOGISTICS CO., LTD.`）与月度明细表格；
    - 所有 38 项 Cargo 单元测试 100% 绿灯通过，服务稳定运行于 3000 端口。

---

## 阶段九十三：OCR 底座解耦重构——全面迁移至 anyocr 独立引擎与冗余剥离 (已完成) · [🔗 对话跳转](conversation://179180f8-e7e7-45e2-aaf7-aada2d975a7b)
- [x] 93.1 **接入 anyocr 独立感知底座依赖与 Cargo 清理 (`Cargo.toml`)**：
  - 引入 `anyocr = { path = "../anyocr" }` 本地独立路径依赖；
  - 解耦对底层 `ort`、`ndarray`、`imageproc` 等冗余重型依赖的手工维护，统一由 `anyocr` 统一管理与按需抽象。
- [x] 93.2 **核心推理适配代理层重塑 (`src/ocr/mod.rs`)**：
  - 将 `src/ocr/mod.rs` 改造为极简、高并发安全的门面适配器（Facade Pattern），底层全权委托 `anyocr::Engine`；
  - 完整保留原有向后兼容公共类型与接口签名（`OcrBoxItem`, `OcrResult`, `OcrEngine::recognize_bytes`, `recognize_image`, `is_loaded`, `unload`）；
  - 保留并加固 3 分钟空闲自动卸载监控（Tokio 后台轻量轮询检测 `OCR_IDLE_TIMEOUT` 与 `Arc::strong_count == 1`），确保长时间无任务时自动回收内存。
- [x] 93.3 **彻底删除 6 个历史冗余底层模块 (解耦瘦身约 5 万行代码)**：
  - 彻底清理删除 `src/ocr/` 内部不再需要的 6 个历史重型文件：
    - `src/ocr/markdown_builder.rs`
    - `src/ocr/matcher.rs`
    - `src/ocr/preprocessor.rs`
    - `src/ocr/table_structure.rs`
    - `src/ocr/text_detector.rs`
    - `src/ocr/text_recognizer.rs`
  - 业务层与底层深度学习推理引擎实现彻底解耦，维护复杂度大幅降低。
- [x] 93.4 **测试并发安全加固与转换异常熔断 (`src/ocr/mod.rs`, `src/converter.rs`)**：
  - 加固单元测试多线程竞争：引入测试互斥锁与动态轮询重试机制，杜绝并行测试时强引用交叉导致的卸载断言失败；
  - `src/converter.rs` PDF 转换增加 `catch_unwind` 异常捕获，防范第三方 PDF 解析排序违反数学全序时导致的未捕获异常退出，优雅回退至 OCR 图像提取流。
- [x] 93.5 **全链路回归测试与业务验证**：
  - 跑通全部 32 项 Cargo 单元测试与端到端测试（100% 绿灯通过）；
  - 验证敏感信息审计、正则/LLM 抽取、快照落盘、前端原图卷帘对比视图无缝衔接。

---

## 阶段九十四：anyocr 引擎底座同步升级 (已完成)
- [x] 94.1 **同步最新 anyocr 上游版本 (`Cargo.lock`)**：
  - 更新 anyocr 锁版本至最新 commit (`11f8d42`)；
  - 继承上游 CoreML 动态形状优化、多核 CPU SIMD、超大图 2560px 自适应视窗钳制保护与原图物理坐标还原。
- [x] 94.2 **适配 EngineConfig 新增配置字段 (`src/ocr/mod.rs`)**：
  - 适配并对齐 `max_dimension: Some(2560)` 配置项，防止极端高分扫描件内存膨胀。
- [x] 94.3 **编译与 OCR 单元测试验证**：
  - `cargo check` 与 `cargo build` 顺利编译；
  - OCR 端到端推理与引擎生命周期卸载测试全部通过（100% 绿灯）。

---

## 阶段九十五：OCR 识别模型轻量化——切换至 PP-OCRv6_rec_small (已完成)
- [x] 95.1 **路径与模型平滑回退设计 (`src/paths.rs`)**：
  - 将默认识别模型文件名更新为 `PP-OCRv6_rec_small.onnx`；
  - `get_ocr_rec_path()` 实现向后兼容回退：优先加载更轻量的 `small` 模型，若本地仅有旧版 `medium` 模型则平滑回退，保障老用户无缝过渡。
- [x] 95.2 **下载器规格与校验重塑 (`src/model_manager.rs`, `scripts/download_ocr_models.sh`)**：
  - 更新 `OCR_DOWNLOAD_SPECS` 中识别模型配置为 `PP-OCRv6_rec_small.onnx` (体积由 ~76.6MB 骤降至 ~21.2MB，OCR 全套总包由 ~94MB 瘦身至 ~39MB)；
  - 对齐 ModelScope RapidAI 官方直链与对应字典路径；
  - 完善 `delete_ocr_bundle` 兼容清理旧版残留 `PP-OCRv6_rec_medium.onnx`；
  - 更新 `test_ocr_bundle_metadata` 断言范围（35MB ~ 45MB）。
- [x] 95.3 **用户交互与提示文案同步更新 (`src/ocr/mod.rs`, `src/converter.rs`, `web/index.html`, `web/app.js`)**：
  - 前后端所有 OCR 模型套件大小提示统一对齐为 `~39MB`；
  - 设置面板 OCR 介绍更新为「PP-OCRv6 轻量极速版 (Small Det + Small Rec)」。
- [x] 95.4 **全量编译与端到端测试闭环**：
  - 单元测试与端到端表格 OCR 推理测试全部通过，推理时延与内存占用显著下降。

---

## 阶段九十六：OCR 底座质量加固与架构技术债收敛 (P1) (已完成) · [🔗 实施计划](plan/OCR_TECH_DEBT_REMEDIATION_PLAN.md)
- [x] 96.1 **依赖与私有路径治理 (Task 1 & Task 2) (`anyocr`, `Cargo.toml`)**：
  - 剥离 `anyocr/src/asset/downloader.rs` 中遗留的开发者本地绝对路径，改为标准工作区与缓存目录探测并推送到 GitHub (`8525874b`)；
  - 清理 SensiDoc `Cargo.toml` 中冗余声明的 `ort`, `ndarray`, `imageproc` 直接依赖，仅保留直接引用的 `image` crate，消除版本漂移与重复编译风险；
  - 跑通 `anyocr` 13 项单元测试与 SensiDoc 全量 32 项单元测试。
- [x] 96.2 **模型下载与就绪校验引入 SHA-256 强校验与 Commit 版本锁定 (Task 3) (`src/model_manager.rs`, `src/paths.rs`, `scripts/download_ocr_models.sh`)**：
  - 在 `OcrDownloadFileSpec` 中为 4 项 OCR 组件（Det、Rec、Table、Dict）补充权威 SHA-256 哈希指纹；
  - 将 ModelScope 下载 URL 由浮动 `master` 分支锁定为**不可变 Git Commit Revision**（`RapidOCR: 7d07816`、`RapidTable: a484f11`），杜绝云端静默突变导致哈希失效；
  - 在 `src/model_manager.rs` 实现 `verify_file_sha256()` 校验函数；在已存在组件复用与临时下载流 `.part` 转正前执行强哈希校验，遇截断或哈希不匹配自动阻断并删除损坏缓存；
  - 校准 `src/paths.rs` 中各组件大小就绪阈值（Det > 8MB, Rec > 18MB, Table > 6MB, Dict > 50KB）；
  - 新增 `test_verify_file_sha256` 与 `test_ocr_specs_sha256_against_local_files_if_exist`，验证 4 款本地模型 SHA-256 与定义完全匹配。
- [x] 96.3 **全局并发锁中毒 (PoisonError) 显式告警与自动恢复 (Task 4) (`src/ocr/mod.rs`)**：
  - 封装 `acquire_ocr_state()` 核心助手函数，统一拦截并自愈 `PoisonError`；
  - 遭遇锁中毒时记录显式 `tracing::error!` 告警，执行 `OCR_STATE.clear_poison()` 并经由 `into_inner()` 安全重置状态，杜绝非可重入锁自死锁与进程瘫痪；
  - 增加 `test_ocr_state_poison_recovery` 单元测试模拟异常线程中断，验证锁自愈与服务自动恢复 100% 通过。
- [x] 96.4 **模型体积指标 API 动态驱动化与前后端文案解耦 (Task 5) (`src/paths.rs`, `src/converter.rs`, `src/ocr/mod.rs`, `web/app.js`, `web/index.html`)**：
  - `OcrStatus` 接口扩展提供 `expected_total_bytes: u64` 动态数据字段；
  - 前端 `web/app.js` 统一封装 `formatModelSize()` 辅助函数，数据驱动渲染所有界面体积文案；
  - 统一收敛后端错误返回文案，彻底消除硬编码 `~39MB` 文本分散维护的技术债。
- [x] 96.5 **构建与全量回归测试闭环**：
  - `cargo check`、`cargo build` 零报错，编译干净稳定；
  - 全量 35 项单元测试 100% 绿灯通过。

---

## 阶段九十七：高精度规则校验与企业高频特征扩展 (P0/P1) (已完成) · [🔗 对话跳转](conversation://835b13a1-6c65-4227-9f68-4beb7806c8c7)
- [x] 97.1 **中国二代身份证高精校验算法落地 (ISO 7064:1983.MOD 11-2) (`src/extractor.rs`)**：
  - 实现纯 Rust 加权求和校验函数（加权因子 `[7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2]`，校验码映射 `10X98765432`）；
  - 补充前 6 位行政区划码有效性初筛与出生年月日有效性过滤（含闰年与月份天数判断）；
  - 正则初筛后通过算法强制校验，彻底杜绝合同中 18 位长数字串/流水号/时间戳被误报为身份证。
- [x] 97.2 **银行卡/信用卡号多制式正则与 Luhn 模 10 校验算法 (`src/extractor.rs`)**：
  - 扩展正则支持主流 16~19 位卡号（国内银联 62、Visa、MasterCard、Amex 等）；
  - 实现纯 Rust 零依赖 Luhn (Mod 10) 校验逻辑；
  - 过滤掉格式形似银行卡但校验和不匹配的合同订单号、报关单号等纯数字串。
- [x] 97.3 **企业统一社会信用代码 (USCC 18位) 与组织机构代码 (9位) 国标算法 (`src/extractor.rs`)**：
  - 统一社会信用代码 (USCC 18位)：正则匹配登记管理部门与机构类别，实现基于 **GB 32100-2015** 的第 18 位校验码计算，精准识别企业营业执照核心税号；
  - 组织机构代码 (9位)：正则匹配 `\b[0-9A-Z]{8}-[0-9A-Z]\b`，实现基于 **GB 11714-1997 MOD 11-2** 校验算法，覆盖历史老版合同档案。
- [x] 97.4 **国际银行账户号码 (IBAN) 正则与 ISO 7064 MOD 97-10 校验算法 (`src/extractor.rs`)**：
  - 正则匹配国际银行账户 `\b[A-Z]{2}\d{2}[A-Z0-9]{11,30}\b`；
  - 实现国别代码长度校验与大数整数字符串 `MOD 97 == 1` 模除校验逻辑；
  - 满足出海企业、跨国商务贸易合同与外币结算单据的高精审计诉求。
- [x] 97.5 **全场景联络通讯补齐：E.164 国际电话与中国座机/400 服务热线 (`src/extractor.rs`)**：
  - 国际电话：匹配 `\+(?:[1-9]\d{6,14})\b`，利用 `+` 强特征天然过滤数字流水号，补齐海外号码盲区；
  - 中国固定电话与客服热线：匹配带区号座机 `\b(?:0\d{2,3}-)?[1-9]\d{6,7}\b` 与 400 服务热线 `\b400[-\s]?\d{3}[-\s]?\d{4}\b`。
- [x] 97.6 **高危 IT 数字资产与凭证密钥检出 (`src/extractor.rs`)**：
  - 主流云厂商 AccessKey：AWS (`\bAKIA[0-9A-Z]{16}\b`)、阿里云 (`\bLTAI[0-9A-Za-z]{16,20}\b`)、腾讯云 (`\bAKID[0-9A-Za-z]{32}\b`)；
  - 加密私钥块头：`-----BEGIN (?:RSA\s|EC\s|DSA\s|OPENSSH\s)?PRIVATE KEY-----`，零误报捕获；
  - 规则触发项自动定级为“高危 (high)”，防止技术交接与接口文档泄漏绝密密钥。
- [x] 97.7 **规则意图路由与分类映射优化 (`src/extractor.rs`)**：
  - 扩充 `wants_category` 意图词库，覆盖“税号”、“纳税人识别号”、“开户行账号”、“SecretKey”等常见行业别名；
  - 确保各新增规则能与前端自定义字段名及标签库精准自适应绑定。
- [x] 97.8 **假阳性抑制测试用例与全链路基准验证 (`src/extractor.rs`)**：
  - 编写涵盖真实号码、极端假号（错位、无效校验位、18位连续流水、纯数字货号）的完备单元测试集；
  - 确保 100% 拦截假阳性，`cargo test` 全量通过（58 项单元测试全部绿灯）。

---

## 阶段九十八：脱敏策略多样化——硬抹除 (Redaction) 模式与脱敏预览支持 (已完成)
- [x] 98.1 **后端脱敏策略扩展与接口支持 (`src/desensitizer.rs`, `src/main.rs`, `src/cli.rs`)**：
  - 抽象脱敏模式枚举 `MaskStyle`：`Masking`（保留首尾星号掩码 `张*三`）与 `Redaction`（硬抹除 `████`）；
  - `desensitize_plain_text` 与多格式导出支持按选定策略执行替换；
  - `/api/documents/{id}/desensitize` 接口支持 `style` 查询参数及 POST 结构体；
  - CLI `sensidoc mask` 增加 `--style` 参数支持（`masking` / `redaction`）。
- [x] 98.2 **原生 Office/PDF XML 容器硬抹除支持 (`src/desensitizer.rs`)**：
  - 确保跨 XML `<w:t>`、`<a:t>` 节点及 PDF 内容流支持等长字符硬抹除，不破坏文档原有排版结构。
- [x] 98.3 **设置面板增加脱敏模式选项与全局偏好持久化 (`web/index.html`, `web/app.js`, `web/style.css`)**：
  - 设置面板中增加「脱敏策略」配置卡片，支持切换默认脱敏模式（星号掩码 vs 块状硬抹除）；
  - 偏好自动持久化至 `localStorage`，作为全局默认设置生效。
- [x] 98.4 **中间面板工具栏新增“脱敏”拨杆按钮与即时脱敏渲染 (`web/index.html`, `web/app.js`, `web/style.css`)**：
  - 工具栏拨杆由 `[渲染] [源码]` 扩展为 `[渲染] [脱敏] [源码]`，保持 2 个汉字极简对齐；
  - 激活“脱敏”视图时，根据设置面板的默认策略（掩码 vs 硬抹除）即时打码渲染，点击打码项无缝联动审计列表。
- [x] 98.5 **前端脱敏导出交互适配与参数透传 (`web/app.js`)**：
  - 右下角「脱敏导出 ▼」在请求后端导出时根据当前选定脱敏模式透传 `style` 参数。
- [x] 98.6 **端到端测试与全格式导出回归验证 (`src/desensitizer.rs`)**：
  - 编写并执行单元测试，验证纯文本、Markdown、DOCX、XLSX、PPTX、PDF 在两种模式下的脱敏正确性。
---

## 阶段九十九：4B 与小微模型快慢协同架构天梯榜评测与离线模型精简 (已完成) · [🔗 对话跳转](conversation://3e7df9a1-379d-4d5b-aa2f-a04ec14a8434)
- [x] 99.1 **Benchmark 测试集大扩建与 Ground Truth 对齐 (`src/benchmark.rs`)**：
  - 扩展中文用例至包含长篇多段落、角色混淆、诱饵数字与长定语从句等复杂真实场景；
  - 新增 10 份英文/跨国真实业务文档（跨国 MSA、HIPAA 医疗隐私、GDPR DPO、AWS 安全事件 RCA、M&A Term Sheet、AML 等）；
  - 为 20 份文档构建完整中英文字段规则（RuleField）与无歧义标准真实标注（Ground Truth），通过 `test_benchmark_documents_integrity` 严谨回归验证。
- [x] 99.2 **抽象 `DualModelStrategy` 与本地 Ollama 多模型适配器 (`src/extractor.rs`, `src/benchmark.rs`)**：
  - 封装通用的本地 Ollama 客户端，无缝调用 `127.0.0.1:11434` 上的本地模型；
  - 抽象多模型协作策略枚举：`BaselineSingle4B`、`ProposalAndJudge`、`ConfidenceRouter`、`SpanAssigner`。
- [x] 99.3 **实现协同策略数据流与思维链截断协议 (`src/extractor.rs`)**：
  - 顶层支持 `"think": false` 零思维链直出；
  - `clean_json_text` 智能识别并剥离 `</think>` 标签，确保干净 JSON 解析；
  - 接入小模型宽松海选提案 + 慢模型靶向终审判决二分类。
- [x] 99.4 **矩阵基准对决与黄金组合实测定型**：
  - 多轮对决验证跑出最佳黄金组合：
    * **快前置**：`Qwen3.5-text-0.8B (Q6_K)`（单篇 8.2 秒，海选召回 83.3%，F1 0.909 夺冠）；
    * **慢终审**：`MiniCPM5-2B (Q4_K_M)`（终审精确率 100.0%，完全平替 4B）。
- [x] 99.5 **设置面板离线模型预设收敛**：
  - 精简后端离线模型配置，只保留 `Qwen3.5-text-0.8B-Q6_K`（魔搭源: `icychick/Qwen3.5-text-0.8B-GGUF`）与 `MiniCPM5-2B-Q4_K_M`（魔搭源: `OpenBMB/MiniCPM5-2B-gguf`）；
  - 单元测试与魔搭 Range 断点续传链路 100% 验证通过。

---

## 阶段一百：端云全模打通、协同引擎解耦与列表极简交互重塑 (已完成) · [🔗 对话跳转](conversation://04cf921b-b2be-4504-9d91-f82fe19c2d13)
- [x] 100.1 **左侧文档列表轻量化与卡片对称交互设计 (`web/app.js`, `web/style.css`)**：
  - **原生/扫描精准探测**：结合文档扩展名与 OCR 分页特征，准确识别并标注文档类型；
  - **极简矢量图标纯净化**：去除“扫描”/“原生”汉字标签，彻底剥离背景底纹（background）、边框（border）及 padding，仅保留 15px 精致矢量图标（扫描件显示扫描取景图标、原生文档显示文本文件图标），悬浮保留原生 tooltip；
  - **高度不变居中与对称交互**：重构单行卡片结构，保持 58px 黄金高度居中呈现；默认状态下“待提取/已审计”状态标签替代删除按钮位置以实现视觉对称，鼠标悬浮（hover）时删除按钮平滑浮现并将标签左推。
- [x] 100.2 **端侧协同引擎子模型独立管理与极简状态呈现 (`web/app.js`, `web/index.html`, `web/style.css`)**：
  - **子模型解耦独立下载**：设置面板中协同引擎的每个子模型（Qwen3.5-0.8B、MiniCPM5-2B）支持单独触发下载与状态同步；
  - **状态指示三态收敛**：移除冗余的前置“● 就绪”与独立的“启动/运行中”操作，统一收敛为【下载】/【下载中】/【已就绪】三态标签；
  - **纸质 OCR 已就绪样式修复**：修复模型下载完成后“已就绪”胶囊标签的圆角与内边距样式，保持全站设计语言统一。
- [x] 100.3 **底部主栏独立离线模型与协同引擎双模选择 (`web/app.js`, `web/index.html`)**：
  - **三轨选择分组**：模型选择器分为“端侧组合模式 (协同引擎)”、“独立端侧模型 (单个离线)”与“在线云端模型 (API)”三大组，用户既可选用快慢协同引擎，也可独立选择单个离线小模型（如 Qwen3.5-0.8B、MiniCPM5-2B）；
  - **纯净文案与去除运行图标**：彻底移除模型名称后的冗余描述与前置播放/运行中图标（▷），仅通过纯净模型名称与选中状态呈现。
- [x] 100.4 **设置面板提取规则提示词全模支持 (`web/app.js`)**：
  - 目标模型下拉框统一接入离线端侧模型与在线云端模型双轨分组；
  - 彻底移除“(当前运行中)”文字提示；
  - 支持为每一个在线模型与离线模型独立配置并持久化保存系统提示词。
- [x] 100.5 **AI 生成规则抽屉全模双轨打通与智能路由 (`src/main.rs`, `web/app.js`, `web/index.html`)**：
  - **抽屉下拉多模选择**：AI 生成规则面板的模型下拉菜单由仅支持在线模型扩展为同时支持离线端侧模型与在线云端模型，自定义组件支持 optgroup 分组渲染；
  - **后端双轨路由执行**：`/api/rules/ai-generate` 接口支持智能路由：选择离线模型时，自动按需拉起底层 llama-server 并在专属端口执行推理，输出遵从标准 JSON 数组；选择在线模型时无缝代理 API；
  - **离线模型小尺寸稳定输出**：优化面向小模型的 System Prompt 示例约束，增强 JSON 数组提取与容错反序列化，实现离线端侧模型规则秒级生成。

## 阶段一百零四：单据智能识别三档复核架构与流式体验重塑 (P0) (已完成) · [🔗 对话跳转](conversation://1a5fe8d6-fc99-404c-b60b-11f40ab67aeb)
- [x] 104.1 **顶部空间释放与方案一三档胶囊分段器落地 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 隐藏中间面板顶部的 `#docMeta` 冗余文件名标题，黄金空间让渡给功能按钮；
  - 落地方案一三档胶囊控制器：`[基础OCR]`、`[快速VLM ❸]`、`[全量VLM]`；
  - 采用纯 Lucide 矢量图标（杜绝 Emoji），支持未选中/激活态/待复核徽标联动；
  - 扫描件/图片格式自动亮出三档胶囊组，普通非扫描文本档自动静默隐藏。
- [x] 104.2 **低置信度可疑区块探测与自适应上下文边界扩展算法 (`src/ocr/mod.rs`, `src/converter.rs`, `src/session.rs`)**：
  - 在 `DocumentItem` 增加 `raw_boxes`、`low_confidence_count`、`base_markdown`、`fast_vlm_markdown`、`full_vlm_markdown` 与图像尺寸元数据支持（`#[serde(default)]` 兼容旧版）；
  - OCR 推理产出中按阈值（score < 0.88）筛选可疑区块；
  - 实现基于字高 $H$ 的动态安全扩展（上下微扩 15%、左右扩 1.3$H$ 并探测整行/单元格）微切片裁剪服务 `crop_image_box` 与零依赖 Base64 快速编码。
- [x] 104.3 **VLM 快速复核局部原地骨架流式与全量复核流式落地 (`src/main.rs`, `web/app.js`, `web/style.css`)**：
  - 针对低置信度区块进行 VLM 定向纠错接口（`POST /api/documents/{id}/vlm-fast`），前端实现局部原地骨架微光与渐进替换，修正项以淡入绿光动态反馈；
  - 全量复核支持 SSE 流式打字机输出（`GET /api/documents/{id}/vlm-full/stream`），骨架自上而下逐行溶解收缩，文字打字机光标滚动实时渲染；
  - 后端提供 OCR 识别档位无缝切换接口（`POST /api/documents/{id}/ocr-tier`），三档识别排版在前端即时切换回退且不丢数据；
  - 统一后续“AI 敏感字段提取”流式事件协议（SSE: chunk / done / error）。
- [x] 104.4 **三档胶囊与独立「识别/重新识别」动作按钮架构升级与全闭环体验 (`web/index.html`, `web/style.css`, `web/app.js`, `src/main.rs`)**：
  - **交互职责解耦**：将三档胶囊定位为“档位选择与视图比对”，在胶囊右侧新增高对比度独立操作按钮（`[▶ 识别 / ↻ 重新识别]`）；
  - **智能状态感知**：档位已生成时秒切呈现，按钮显示为「重新识别」；档位未生成时展示引导卡片，按钮呈现「识别」并带呼吸光环引导点击；
  - **三档后端闭环打通**：
    - `基础OCR`：新增 `POST /api/documents/{id}/ocr-base` 支持清空缓存重新执行端侧 PP-OCRv6 与 SLANet_plus 解析；
    - `快速VLM`：触发 Qwen3.5-0.8B 局部自适应微切片纠偏与 `.vlm-corrected` 呼吸高亮；
    - `全量VLM`：触发 Qwen3.5-0.8B 端到端全图重构，自上而下逐行骨架溶解与 SSE 打字流式渲染；
  - **全局现代 Toast 机制落地**：封装轻量全局浮层通知组件（`showToast` / `showNotification`），提供操作状态即时反馈。
- [x] 104.5 **动作按钮尺寸严丝合缝、动态置信度极简下拉与局部骨架虚线框重塑 (`web/index.html`, `web/style.css`, `web/app.js`, `src/main.rs`)**：
  - **尺寸与高度完美对齐**：重构 `.ocr-action-btn` 尺寸（高度 25px、字号 11.5px、内边距 0 8px），与切换胶囊分段器完全平齐，彻底消除突兀感；
  - **动态极简百分数置信度下拉框**：
    - 收起常态下仅显示极简百分数（如 `< 85% ▾`，宽度缩减至 58px），完全不挤占顶栏空间；
    - 点击展开原生面板展示各档置信度范围及详细定义（40% ~ 95% 平滑阶梯，最低可下探至 40% 严重损毁排查）；
    - 切换置信度时动态重新计算当前文档命中可疑片段数，胶囊角标即刻联动更新；
  - **快速 VLM 局部行内骨架脉冲**：执行快速复核时不再全局清屏，保留背景排版，仅在需要复核的低置信度文本片段上原地闪烁微光骨架；
  - **高保真虚线框与科技感标识**：复核完成后，对修正项赋予紫青色细虚线框与 `[⚡ 修正]` 徽标，对确认项赋予浅青细虚线框与 `[✓ 复核]` 徽标，悬浮查看原词与置信度对比。

---

## 阶段一百零一：提取全链路轨迹 (Trajectory) 可视化与排查体系构建 (待办)
- [ ] 101.1 **执行轨迹数据模型扩展与向后兼容契约 (`src/session.rs`)**：
  - 定义 `ExecutionTrajectory`、`TracePhase`、`TokenUsage` 等结构体；
  - 在快照 `ExtractionSnapshot` 中扩展 `pub trajectory: Option<ExecutionTrajectory>` 字段，`#[serde(default)]` 实现向后无缝兼容。
- [ ] 101.2 **提取引擎全链路指标采集与打桩 (`src/extractor.rs`)**：
  - 采集预处理分块（Chunk 数量、字数范围与偏移量）、正则兜底耗时与命中明细；
  - 采集每轮模型推理真实耗时、Token 统计、思考链（Reasoning）与原始响应（Raw Response）。
- [ ] 101.3 **后端 SSE 流式进度广播接口落地 (`src/main.rs`)**：
  - 新增 `POST /api/extract/stream` 端点（基于 Axum SSE），实现执行过程事件流（`trace_start`、`trace_step`、`trace_done`）实时推送。
- [ ] 101.4 **前端轨迹视图三段式界面落地 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 构建仿 deepseek-harness 轨迹模态框：概览工具栏、泳道时间线甘特图（Timeline）、阶段流水账（Ledger）与详情检查器抽屉（Inspector）。
- [ ] 101.5 **中间面板顶部按钮升级与双模联动 (`web/app.js`, `web/index.html`)**：
  - 中间面板顶部原「提取规则」按钮升级为「⏱️ 提取轨迹」，执行中呈现流式呼吸动画，点击实时观测；
  - 联动快照时光机历史版本回溯，点击秒级展示任意历史版本的完整执行轨迹与排查凭据。

---

## 阶段一百零二：单据图片比对视图与卷帘透视深度优化 (待办)
- [ ] 102.1 **单据原图全景视图优化 (`web/app.js`, `web/index.html`, `web/style.css`)**：
  - 重新设计原图全景展示交互，支持高分辨率单据平滑缩放、拖拽平移与 OCR 选框无损高亮对齐；
  - 增强跨平台手势与滚轮缩放体验。
- [ ] 102.2 **卷帘透视比对 (Curtain Comparison) 交互重构 (`web/app.js`, `web/index.html`, `web/style.css`)**：
  - 优化滑轨拖拽手感、底层原图与表层 Markdown/表格的双向严格同步滚动；
  - 解决文字换行或行高不一致时的基线错位问题，支持分块锚点对齐；
  - 完善后重新在顶部文档工具栏放开「卷帘比对」与「原图」拨杆切换按钮。

---

## 阶段一百零三：AI 智能生成提取策略与规则抽屉交互深度重构 (待办)
- [ ] 103.1 **AI 生成规则功能交互与视觉重构 (`web/app.js`, `web/index.html`, `web/style.css`)**：
  - 暂行隐藏「AI生成」规则抽屉与触发按钮，留待全新交互范式设计；
  - 重新规划自然语言需求提炼、元提示词生成规则的交互链路（例如模态向导、行内智能建议或对话式提示词工作流）；
  - 优化离线小模型与在线大模型双轨生成时的实时流式反馈、候选字段一键勾选预览与一键合并体验；
  - 待交互方案与视觉设计定型后重新上线该功能模块。

---

## 阶段一百零五：VLM 全量视觉模型升级、离线管理架构重构与双模即时就绪 (已完成) · [🔗 对话跳转](conversation://ed03d807-d42e-4aae-ad08-efe8ad55169c)
- [x] 105.1 **内置快模型与视觉 VLM 全面升级为 Qwen3.5-0.8B (带视觉塔)**：
  - 弃用旧版纯文本模型 `qwen3.5-text-0.8b-q6_k`，全面替换升级为支持视觉多模态的 `Qwen3.5-0.8B-Q4_K_M.gguf` (~508 MB)；
  - 绑定官方配对视觉塔 `mmproj-BF16.gguf` (~198 MB)，实现快前置初筛与视觉 VLM（图片单据、跨列表格排版重构）一库两用；
  - 下载 Qwen3.5-0.8B 时后端自动同步下载挂载视觉塔权重，删除主模型时自动联动清理附属视觉塔。
- [x] 105.2 **“设置 - 离线模型”管理面板架构彻底重构（方案一独立卡片化）**：
  - 移除“核心快慢协同引擎”外层大组合框，将所有离线模型解耦为平级独立卡片（卡片1: Qwen3.5-0.8B、卡片2: MiniCPM5-2B、卡片3: 纸质单据与表格 OCR 套件、卡片4: 自定义 GGUF 模型）；
  - 精简卡片内所有繁琐说明文案，统一收敛至标题旁的问号气泡提示符（`(?)` Tooltip）；
  - Qwen3.5-0.8B 卡片内清晰呈现依赖树：`├── 主模型 (~508 MB)` 与 `└── 视觉塔 (~198 MB)`，展示各子文件独立就绪状态。
- [x] 105.3 **纸质单据与表格 OCR 套件一键全套下载与子组件状态呈现**：
  - 统一为一个下载按钮一键下载全套 4 个组件（Det、Rec、Table、Dict）；
  - 树形结构展示 4 个子组件各自的独立状态（`[✓ 已就绪]` / `[× 失败] ↺` / `[⏳ 待下载]`），支持组件级断点续传与失败重试。
- [x] 105.4 **自定义 GGUF 模型去重过滤与视觉塔关联支持**：
  - 严格过滤已下载的官方预设主模型以及从属视觉塔 `mmproj*.gguf` 文件，杜绝重复展示；
  - “暂无已导入的第三方模型”极简优雅提示；
  - 参数抽屉新增“多模态视觉塔 (mmproj)”绑定配置，自动按同名前缀智能匹配，并支持手动下拉选择或纯语言模式。
- [x] 105.5 **“核心快慢协同引擎”功能正确归位与条件置灰**：
  - 将协同引擎从设置面板中解脱出来，归位至右侧栏底部【立即执行】前的模型选择下拉菜单；
  - 逻辑联动判定：仅在快模型（Qwen3.5）与慢模型（MiniCPM5）均已下载就绪时可选并作为默认推荐项；若缺少任意模型则置灰锁定（`disabled`）并提示下载。
- [x] 105.6 **双向毫秒级即时就绪检测与死锁修复**：
  - 后端在 `ModelPreset` 新增 `is_main_downloaded` 与 `is_mmproj_downloaded` 明确标识，并提供 `/api/models/mmprojs` 独立探针；
  - 前端改造 `loadModelPresets` 与 `renderModelPresets`，基于本地磁盘扫描与预设标记进行双保险即时判定，彻底消除“检测中...”死锁，瞬间点亮就绪态。

---

## 阶段一百零六：OCR 引擎按需激活与生命周期管理规范化（15分钟自动卸载与按钮对齐）(已完成) · [🔗 对话跳转](conversation://ed03d807-d42e-4aae-ad08-efe8ad55169c)
- [x] 106.1 **OCR 卡片状态药丸与操作按钮规范化对齐**：
  - 移除孤立特殊的绿色胶囊，完全复用标准 `.module-status-pill`（未运行时呈现灰底 `● 已就绪`，运行时呈现亮绿 `● 运行中`）；
  - 操作按钮与上方大模型完全一致：未运行时呈现 `[ 启动 ]`（黑底白字 primary）与 `[ 删除 ]`（红底白字 danger）；运行时呈现 `[ 关闭运行 ]` 与 `[ 删除 ]`；
  - 修复关闭运行按钮内联样式覆写导致的字体隐形问题，统一为清晰鲜明的红底白字 danger 按钮。
- [x] 106.2 **OCR 引擎按需冷启动与 15 分钟空闲自动卸载**：
  - 默认状态下 OCR 引擎保持未载入（非运行）状态，零显存/内存常驻占用；
  - 当用户传入图片、扫描件或触发 OCR 复核时，后端自动无感冷启动激活 OCR 引擎；
  - 后端空闲监控超时时间从 3 分钟提升为 15 分钟（`OCR_IDLE_TIMEOUT = 15 * 60`），15 分钟内无图片任务自动卸载引擎并释放内存；
  - 后端新增 `/api/ocr/start` 与 `/api/ocr/stop` 接口，前端按钮支持用户随时手动预热或手动即刻释放内存。
- [x] 106.3 **全流程无头浏览器与端到端回归验证**：
  - 45 项单元测试 100% 通过；
  - 使用 Chrome Devtools 对初始就绪、手动启动、手动关闭、自动识别全流程完成截图实机验收。

---

## 阶段一百零七：anyocr 版面识别引擎升级与复合单据网格重构 (已完成) · [🔗 对话跳转](conversation://cc806494-2958-431d-a84e-cfdaa32bf2b8)
- [x] 107.1 **根因剖析与 SLANet 表格合法性检验熔断保护 (`anyocr::layout::table_matcher`)**：
  - 深入排查复合单据（整页包含标题、33行大表格、银行信息、审批签字等）直接输入 SLANet 时产生畸形 `rowspan="14"` 巨型单元格吞噬全页 200+ 文本框的根因；
  - 增加 `TableMatcher::is_slanet_valid` 质量检验与单元格利用率校验，在检测到欠分割或漂移时自动熔断 SLANet 并平滑降级。
- [x] 107.2 **引入启发式坐标网格表格构建器 (`anyocr::layout::grid_table`)**：
  - 基于 OCR 文本框物理坐标 `[x1, y1, x2, y2]` 投影实现逻辑行聚类（`cluster_into_rows`）；
  - 自动识别多列特征与表头关键词（`find_table_row_ranges`），精准发现明细表格真实起止范围；
  - 基于 1D 空间密度聚类划定列区间（`compute_column_intervals`），将 33 行密集明细高精还原为规整 GFM Markdown / HTML 表格。
- [x] 107.3 **多表格与复合版面流式交织重构 (`anyocr::layout::mod`)**：
  - 废弃全局单表格假设，升级为多表格与正文流式交织组装管线（`process_multi_table_flow`）；
  - 支持单页单据中同时包含表头键值对、33行明细表格、银行信息正文、底部审批表格、手写笔迹并保序输出。
- [x] 107.4 **单据键值对保护与标题前缀规范 (`anyocr::layout::para_merger`, `heading`)**：
  - 增强 `is_key_value_line` 保护表单字段标签，防止单据字段被折行误合并；
  - 排除公式与列表前缀，确保手写笔记作为自然正文输出。
- [x] 107.5 **远程 Gitee 同步、SensiDoc 依赖更新与磁盘空间深度清理**：
  - `anyocr` 提交至 Gitee（commit `e53b32f`）；
  - SensiDoc 执行 `cargo update anyocr` 锁定最新 revision；
  - 清理 Debug 及历史交叉编译中间产物，释放 ~7.6 GB 磁盘空间。

---

## 阶段一百零八：离线模型选型极简收敛与端侧组合策略移除 (已完成)
- [x] 108.1 **架构决策与极简原则收敛 (去掉离线端侧模型协作组合模式)**：
  - **背景与痛点复盘**：此前尝试的快慢协同/多模型组合策略（如 Qwen3.5-0.8B 初筛海选 + MiniCPM5-2B 终审判决）虽然在理想 benchmark 矩阵下有理论互补，但在真实业务交互中链路冗长、级联误差不易控制、各阶段耗时与幻觉边界难以精准量化衡量；
  - **极简主义决策（KISS 原则）**：遵循“简单至上，杜绝过度设计”原则，果断废弃复杂的端侧组合策略，全面回归单一明确、行为可预期、易排查易调试的独立端侧模型（Single Local Model）推理体系。
- [x] 108.2 **主执行栏模型选择器纯净化重塑 (`web/app.js`)**：
  - 从右侧栏底部【立即执行】主按钮前的模型下拉菜单中彻底剔除「端侧组合策略 (协同引擎)」分组及相关组合选项；
  - 模型分组收敛为清晰直观的两大轨制：**`独立端侧模型 (Local)`** 与 **`在线云端模型 (API)`**；
  - 优化默认选中与安全回退逻辑，剥离多余的协同引擎热启/停止分支，让模型状态（指示灯/拨杆）与实际运行进程 100% 精准对应。
- [x] 108.3 **全流程回归与质量验证**：
  - 45 项单元测试与自动化验证全部通过（44 passed, 1 ignored, 0 failed）；
  - 确保单一离线端侧模型与在线云端大模型在主界面提取与 AI 规则生成抽屉中无缝切换且零残余副作用。

---

## 阶段一百零九：单据扫描工具栏浮动胶囊重塑、下拉精简与全局通知视域归位 (已完成) · [🔗 对话跳转](conversation://adaf0358-cf78-4c80-b055-53cb3e667465)
- [x] 109.1 **扫描工具栏重塑为预览面板顶部悬浮毛玻璃胶囊 (方案一 Floating Island) (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - **顶栏空间释放**：彻底从顶部全局标题栏剥离三档扫描切换与识别执行按钮，顶部控制栏回归纯净的 `[渲染/脱敏/源码]` 与快照时光机；
  - **预览区居中悬浮**：将扫描工具栏重定位至中间工作舞台 `.preview-container` 顶部绝对居中（`top: 14px; left: 50%`），具备自适应磨砂毛玻璃质感（`backdrop-filter: blur(16px)`）；
  - **智能显隐与折叠展开**：仅在扫描件或图片格式时自动显示，常规文本静默隐藏；支持一键折叠为 Mini 药丸胶囊 `[ ⚡ 扫描档位: 基础OCR ⌄ ]`，并记住用户本地偏好。
- [x] 109.2 **置信度下拉纯净化与防折行强制单行模式 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - **下拉菜单文案精简**：彻底剥离选项括号内冗长描述，收敛为统一纯净形式：`置信度 < 95%`、`置信度 < 90%` 等；
  - **强制单行与动态宽度**：为浮动胶囊与三档按钮施加 `white-space: nowrap` 与 `flex-shrink: 0`，解决切换“快速VLM”出现置信度下拉和执行按钮时文字挤压折成两行的问题，无论如何伸缩均保持单行优雅对齐。
- [x] 109.3 **全局 Toast 消息通知归位至中间面板底部信息栏上方 (`web/style.css`, `web/app.js`)**：
  - **告别右下角遮挡**：移除右下角全局固定定位，将 Toast 宿主限定在中间面板 `.preview-container` 内部，定位至 `bottom: 46px; left: 50%` 水平绝对居中；
  - **视线聚焦与零遮挡**：悬浮在底部信息栏上方约 8px 处，彻底释放右下角模型控制与字号缩放按钮，多条消息自下而上优雅堆叠；
  - **样式原汁原味还原**：完全保留原版设计样式（微圆角 `border-radius: 8px`、字号 13px、支持多行文本自然换行），坚决杜绝大圆角胶囊与单行截断。
- [x] 109.4 **语法健壮性保证与 Chrome DevTools 自动化实机验收**：
  - 通过 `node -c web/app.js` 静态语法校验，避免脚本加载中断；
  - 基于 Chrome DevTools 完成文档列表加载、三档切换、折叠展开、下拉菜单以及 Toast 绝对居中几何位置（水平误差 < 0.01px）的端到端全闭环验收。

---

## 阶段一百一十：端侧小微多模态模型（0.8B）死循环攻克、全量 VLM 提示词优化与双轨协同实测 (已完成) · [🔗 对话跳转](conversation://b245ac33-c1e5-440d-95a6-4d98f95c5292)
- [x] 110.1 **快速 VLM 几何切片边界收敛与自回归解码超参收敛 (`src/ocr/mod.rs`, `src/extractor.rs`, `src/main.rs`)**：
  - **切片几何纠偏**：排查并修复 `expand_crop_box` 将 `[min_x, min_y, max_x, max_y]` 当作 `[x, y, w, h]` 导致宽度翻倍暴涨吞入数十行单号的 bug，水平扩展引入字高严格约束（`pad_x = (h * 0.4).max(6.0)`）；
  - **文本搜索与行距防越界**：针对单字符全局模糊搜索误碰大标题，施加垂直行高间距校验（$\Delta Y < 40\text{px}$）与有效词长（$\ge 2$）过滤；
  - **解码参数收敛**：微切片提取 `max_tokens` 由 1024 收敛至 128，引入 `temperature: 0.0`、`frequency_penalty: 0.3` 与 `presence_penalty: 0.2`，彻底杜绝自回归死循环。
- [x] 110.2 **纯视觉感知路线确立与 OCR 文本注入假设证伪**：
  - 在财务单据（`/Users/icychick/Pictures/CHAY DA IMPORT-C1-MAY-2026-$ 8,545.18_1.png`）上实测验证在 Prompt 中注入 OCR 文本作为参考的假设；
  - 揭示端侧小微模型严重的**文本先验偏见（Text Prior Bias）**：小模型的文本注意力压倒视觉注意力，导致 OCR 识别错字（如 `$ 10.51` 误识为 `$ 10.5f`）被模型原封不动抄袭；
  - 果断废弃 OCR 文本注入，确立纯视觉多模态像素直读路线。
- [x] 110.3 **全量 VLM 100% 通用系统提示词拓扑解耦与防挤占设计 (`src/main.rs`)**：
  - 针对 0.8B 模型将首列“序号+公司名”拆分并占用第二列、导致 33 行采购单号整列被吞（0% 召回）的问题，提炼纯正向、无业务硬编码的系统提示词；
  - 明确分表解耦（抬头、主表、底部独立分块）、首列复合字段防挤占规范、关键列（采购单号、金额）零丢失保底；
  - 实测实现 33 行采购单号 100% 完整找回，金额与币种零污染。
- [x] 110.4 **工业品说明书图纸交叉盲测与小微模型物理边界探针**：
  - 在工业说明书图纸（`/Users/icychick/Pictures/PixPin_2026-09-14_19-24-43.png`）上进行标准基准（Ground Truth）对比盲测；
  - 验证通用提示词成功消除公差双负号幻觉（`+/- -0.4` 纠正为 `+/- 0.4 mm`）、纠正单位拼写退化（`sqqm` 纠正为 `sqmm`）、实现条款独立分行；
  - 发现全图视野下大标题对顶部边缘 18px 小字的“视觉显著性压制（Salience Suppression）”，并通过微切片探针验证局部切片下 0.8B 模型小字识别准确率达 100%。
- [x] 110.5 **测试报告归档与全局工程技能知识持久化**：
  - 编写并归档详尽的实测报告：[`docs/SMALL_VLM_PROMPT_OPTIMIZATION_TEST_REPORT.md`](file:///Users/icychick/Projects/SensiDoc/docs/SMALL_VLM_PROMPT_OPTIMIZATION_TEST_REPORT.md)；
  - 遵循 `/learn` 提案工作流，更新全局技能 [`small-vlm-engineering/SKILL.md`](file:///Users/icychick/.gemini/config/skills/small-vlm-engineering/SKILL.md)，沉淀提示词三要三不要法则与“全量拓扑 + 局部微切片”双轨协同工程架构。

---

## 阶段一百一十一：CLI 扫描件识别与三级 OCR/VLM 流水线落地 (--ocr base/fast-vlm/full-vlm) (已完成) · [🔗 对话跳转](conversation://f6902e6f-a03c-40c6-8de7-ac6b72114c0e)
- [x] 111.1 **VLM 复核与重构核心底层解耦与独立模块化 (`src/ocr/vlm.rs`, `src/ocr/mod.rs`)**：
  - 剥离原先内嵌在 `src/main.rs` 内部的 VLM 推理调度与算法，创建独立模块 `src/ocr/vlm.rs` 并从 `src/ocr/mod.rs` 统一导出；
  - 提炼结构体 `VlmEngineInfo`、`FastVlmCorrection`、`FastVlmInspectedItem`、`FastVlmResult`、`MergedOcrCluster`；
  - 封装端到端管道函数：`run_fast_vlm_pipeline`（自动基于原生基础 OCR 候选框执行空间聚类、Markdown 上下文挂载与定向纠偏）与 `run_full_vlm_pipeline`（全量多模态排版高保真重构）；
  - `src/main.rs` 中的 Web API Handler（`fast_vlm_review_handler`）全量改造为委托调用，消除重复代码 240+ 行。
- [x] 111.2 **统一命令行 CLI `--ocr` 参数设计与多档位集成 (`src/cli.rs`, `src/converter.rs`)**：
  - 新增枚举 `OcrTier`（`base` | `fast-vlm` | `full-vlm`），实现 `ValueEnum` 与 Display/Serde；
  - 为 `convert`、`audit`、`mask` 三大核心子命令增加统一 `--ocr` 参数，配置 `default_missing_value = "base"` 与 `num_args = 0..=1`，优雅支持无值标志位（`--ocr` 默认基础 OCR）与显式指定（`--ocr fast-vlm` / `--ocr full-vlm`）；
  - 在 `src/converter.rs` 中新增 `convert_file_detailed` 接口，支持以路径直读并返回包含 `doc_type`、`raw_boxes` 与 `low_confidence_count` 的 `ConvertResult`。
- [x] 111.3 **双轨与三级流水线执行链路打通 (`prepare_document_markdown`)**：
  - 严格落实用户设计准则：指定 `--ocr fast-vlm` 时，系统**自动先执行基础 OCR**，识别文字与表格结构后，自适应定位低置信度（< 0.88）疑难文字块与单元格，拉起轻量 VLM 进行局部微切片快速纠偏；
  - 指定 `--ocr full-vlm` 时，对图像执行缩放与 Base64 编码，调用全量视觉多模态大模型进行高保真段落与表格重构；
  - 指定 `--ocr base` 或省略参数时，快速走原生基础 ONNX OCR 毫秒级解析；对原生文档（docx/xlsx/txt）自动直通，杜绝冗余开销。
- [x] 111.4 **全量回归验证与单元测试覆盖**：
  - 在 `src/cli.rs` 中追加 7 个单元测试（覆盖 `--ocr` 缺省回退 base、`fast-vlm`、`full-vlm`、不带参数、子命令解析以及序列化）；
  - 修复 `test_expand_crop_box` 的坐标约定断言；
  - 全工程 51 个单元测试（44 个 main/模块测试 + 7 个 CLI 测试）100% 通过；CLI 帮助命令与转换命令端到端验证无误。
- [x] 111.5 **CLI 开发者指南与主说明文档全量同步 (`docs/CLI_GUIDE.md`, `README.md`)**：
  - 同步升级 `docs/CLI_GUIDE.md` 至 v1.3.3；
  - 在核心子命令速查与详细用法（`convert` / `audit` / `mask`）中全面补充 `--ocr [<TIER>]` 语法与实操示例；
  - 新增第 3.4 节“扫描件与图像 OCR/VLM 三级识别流水线 (`--ocr` 深度说明)”与 ASCII 决策流；
  - 更新 Python / Node.js AI Agent 工具封装示例，增加 OCR 档位支持；
  - 在 `README.md` 的快速上手模块中补充命令行审计与扫描件转换示例。

---

## 阶段一百一十二：快速 VLM 单元格防炸列与全量 VLM 底部签批栏通用解耦优化 (已完成) · [🔗 对话跳转](conversation://0e8f3321-a76e-4ddf-b12d-6dfddc98446c)
- [x] 112.1 **快速 VLM 单元格微切片防炸列与管道符清洗隔离 (`src/ocr/vlm.rs`)**：
  - **Prompt 刚性负向约束**：在微切片系统提示词中增加刚性要求：`若复核对象为表格单元格，严禁输出表格竖线分隔符 '|'，必须保持为单单元格连续纯文本`；
  - **单元格文本安全清洗**：新增 `sanitize_fast_vlm_output` 函数，在 `is_cell == true` 场景下，强制消除模型生成的非法管道符 `|`，规避 Markdown 表格“炸列”与整行数据向右错位漂移；
  - **单测护航**：编写单元测试验证管道符消除与表格列数前后严格一致性。
- [x] 112.2 **前端徽标（Badge）挂载与 DOM 容器精准匹配重构 (`web/app.js`)**：
  - **新词优先匹配**：优化 `applyVlmReviewBadges` 匹配逻辑，优先按纠偏后的新词（`newText`）检索 DOM 节点，避免因仅检索旧上下文导致匹配失准；
  - **长度安全比例兜底**：在包含匹配逻辑中增加最小长度与比例校验（`normBlock.length >= 6 && normBlock.length >= normTarget.length * 0.6`），彻底杜绝单数字（如 `"26"`）因弱包含误吸附修正徽标。
- [x] 112.3 **全量 VLM 底部审批签名栏通用排版解耦与防人名虚构 (`src/ocr/vlm.rs`, `src/main.rs`)**：
  - **防人名脑补与空白如实还原**：针对端侧小微模型草书字形辨识弱和自回归贪婪完形填空本能，在 `FULL_VLM_SYSTEM_PROMPT` 中施加通用抽象约束：手写草书统一标注为 `[手写签名]`，无笔迹空白处如实标注为 `[未签署/空白]`，100% 杜绝虚构拼音或假名；
  - **结构化列表解耦**：签批栏统一采用键值列表排版（`- **角色名称:** [签署状态] (签署日期)`），中英文对照角色标签保持在同一项不拆列，签署日期紧随对应角色，彻底消除“首列纵向堆叠 5 行日期”的空间几何坍塌；
  - **端到端纯视觉无硬编码验证**：在财务单据样本上实测验证，33 行采购单号 100% 完整保留，底部审批栏 0 虚构、0 错位。
- [x] 112.4 **工程最佳实践文档同步与多模态工程技能知识固化 (`docs/SMALL_VLM_RECOGNITION_BEST_PRACTICES.md`)**：
  - 在最佳实践指南中更新系统提示词黄金模板，增加第 3 条“底部审批与签批栏通用规范”；
  - 在基准对照证据表（Benchmark Evidence）中追加“全量 VLM 底部审批栏”调优前后对比项；
  - 客户端热重启验证通过，服务正常监听于 `http://127.0.0.1:3000`。

---

## 阶段一百一十三：规则列表卡片高度塌陷截断修复与紧凑精致排版优化 (已完成) · [🔗 对话跳转](conversation://b7224c21-bbb2-4d89-8a89-22e2fe4ff604)
- [x] 113.1 **规则卡片底部描述截断与 Flex 布局压缩根因排查 (`web/style.css`)**：
  - **根因定位**：`.rule-card-list` 作为列向 Flex 容器（`flex-direction: column`），在容器高度受限（约 437px）且规则项较多时，子项 `.rule-card` 因默认 `flex-shrink: 1` 遭遇纵向挤压（高度从正常的 ~88px 被硬性挤扁至 57px），叠加上卡片自身的 `overflow: hidden`，导致底部 `.rule-desc-row`（特征描述文字）下半截被硬性裁切；
  - **Flex 防收缩锁死**：给 `.rule-card` 显式设置 `flex-shrink: 0`，确保单张卡片尺寸由其内部真实内容撑起，当总高度超出容器时由 `rule-card-list` 的 `overflow-y: auto` 规范接管滚动；
  - **滚动层级解耦**：将上层 `.tab-content` 的 `overflow-y: auto` 修正为 `overflow: hidden`，消除双层嵌套滚动争抢与无限伸展陷阱。
- [x] 113.2 **规则卡片上下边距与信息密度紧凑化重构 (`web/style.css`)**：
  - **上层标题行瘦身**：`.rule-card-header` 垂直内边距由 `9px 12px` 缩减为 `5px 10px`，最小高度由 `44px` 优化为 `34px`，字段名输入框高度由 `26px` 优化为 `24px`；
  - **下层描述行瘦身**：`.rule-desc-row` 垂直内边距由 `9px 12px 9px 34px` 缩减为 `5px 10px 5px 32px`，最小高度由 `44px` 优化为 `32px`；

---

## 阶段一百一十四：设置面板视觉统一重构（离线模型分段大灰底卡片集 + 在线模型大灰底工作台） (已完成) · [🔗 对话跳转](conversation://fff70847-7e58-4988-9ef2-72dcf2f3b48b)
- [x] 114.1 **离线模型板块（Tab 1）重构为分段大灰底卡片集（方案 B）与概念升级**：
  - **概念重塑**：彻底清退“初筛/终审”旧概念，将 Qwen3.5-0.8B 定位升级为“极速型引擎 (端侧极速响应 & 视觉重构)”，MiniCPM5-2B 定位升级为“稳定型引擎 (稳定型引擎 & 深度提取)”；
  - **视觉降噪与留白优化**：大段解释性文字全部收纳进 `tooltip-badge [?]`，界面清爽整洁；
  - **幽灵危险按钮引入**：新增 `.ghost-danger-btn` 样式，将大红“删除”按钮降噪为平时与底色融为一体的低调幽灵小按钮（灰白边框+垃圾桶图标，hover 悬停才显露警示红），大幅降低视觉干扰；
  - **状态标签降噪**：优化 `.dep-status-tag` 饱和度，去除刺眼高饱和纯绿，选用内敛的中性灰底与克制绿色圆点；依赖树字体从等宽代码体统一为系统主界面字体。
- [x] 114.2 **在线模型板块（Tab 2）重构为镜像提取规则的双层大灰底工作台（方案 A）**：
  - **双层大灰底结构对齐**：使用外层大灰底卡片（`var(--surface-2)`）整体包裹，内部分解为两个白底子卡片（`var(--surface)`）：
    - **基础连接凭据卡片**：配置名称与 Model ID 采用双列并排，Base URL 与 API Key 单列舒展；
    - **推理控制与思考模式卡片**：顶部右侧集成思考模式（Think）极简拨杆，下方紧凑排列 Temp / Top-K / Repeat / Max 四列超参数网格；
  - **无缝平替与 DOM 接口 100% 保持**：完整保留所有原有控件 ID 与交互逻辑，顶部删除按钮同步升级为幽灵危险按钮；
- [x] 114.4 **离线模型卡片细节打磨与交互升维（圆角修复 + 降噪清退 + 详情折叠）**：
  - **大灰底圆角修复**：在 CSS `:root` 中补齐全局 `--radius: 8px;` 变量，并将 `.module-engine-card` 的圆角固化为 `var(--radius-md, 8px)`，消除直角突兀感；
  - **清退“系统预设”与“已就绪”噪点**：彻底移除各卡片标题栏中的静态 `系统预设` / `可选增强` 标签，并在未运行就绪状态下隐藏 `● 已就绪` 状态 Pill，保持界面清爽克制；
  - **模型详情树支持折叠与状态记忆**：每个卡片标题左侧增设精致的旋转 Chevron 图标，点击标题行即可极速展开/收起下方白底依赖树，折叠后单卡高度从 115px 紧凑压缩为 46px，并基于 `localStorage` 自动持久化用户的折叠习惯。
- [x] 114.5 **在线模型默认参数对齐与高级参数区块默认折叠**：
  - **默认推理参数与思考模式对齐**：采样温度 (Temp) 更新为 `0.7`，候选范围 (Top-K) 保持 `50`，重复惩罚 (Repeat) 更新为 `1`，最大 Token (Max) 更新为 `4096`，思考模式 (Think) 默认为 `未开启`；
  - **高级参数卡片默认折叠**：将“推理控制与思考模式”子卡片改造为可折叠面板，默认收起不显示，仅保留极简标题条与展开指示器，点击可平滑展开/收起超参数网格与思考模式开关，主视觉极致聚焦于基础凭据填写。
- [x] 114.6 **在线模型参数面板横向溢出彻底锁死与纯纵向平滑滚动优化**：
  - **彻底清除左右滚动**：在外层 `#paneSetOnlineAi` 与中间大灰底卡片容器上显式锁定 `overflow-x: hidden !important`，严禁任何水平横向溢出与横向滚动条；
  - **网格弹性防撑破锁死**：将配置名称/Model ID 双列网格和 4 列参数矩阵由 `1fr` 升级为 `minmax(0, 1fr)`，所有输入控件强制配置 `min-width: 0` 与 `max-width: 100%`，防止长内容或滚动条挤压引发宽度溢出；
  - **上下平滑垂直滚动**：保留大灰底主工作区的 `overflow-y: auto !important`，确保参数展开或内容超出时仅顺畅上下滚动浏览，交互稳定无晃动。
- [x] 114.7 **在线模型高级参数精净化（保留温度/最大Token/思考开关单行排布）**：
  - **精简核心超参数**：清退在线 API 场景极少用到的“候选范围 (Top-K)”和“重复惩罚 (Repeat)”控件，仅聚焦保留“温度 (Temp)”、“最大Token (Max)”与“思考开关 (Think)”；
  - **单行精致对齐排布**：采用 `minmax(0, 1fr) minmax(0, 1fr) auto` 单行网格，Label 与 26px 控件高度严格平齐，展开后仅占用极小高度，视觉与操作体验达到极简极致。

---

## 阶段一百一十五：星号掩码全量彻底替换与图像/扫描件物理高斯模糊涂抹脱敏 (已完成)
- [x] 115.1 **星号掩码全量彻底替换（不保留首尾）**：
  - 修改后端 `src/desensitizer.rs` 中 `MaskStyle::Masking.mask_text()` 算法，移除首尾保留字符，统一改为等长 `*`（`"*".repeat(len)`）；
  - 同步更新旧模块 `src/exporter.rs` 中的脱敏逻辑；
  - 更新前端 `web/app.js` 的 `getMaskedText` 为全星号替换；
  - 更新设置面板 `web/index.html` 中的脱敏策略卡片说明与示例（`张*三` 改为 `全星号掩码 ***`）；
  - 更新所有相关单元测试断言，确保回归测试 100% 通过。
- [x] 115.2 **图像敏感区域定位与物理高斯模糊涂抹引擎 (`src/desensitizer.rs`)**：
  - 采用安全优先的整框模糊匹配策略（OCR 框包含敏感词即判定命中并添加上下文微 padding，彻底杜绝插值漂移漏敏）；
  - 实现基于 `image` crate 的局部 ROI 动态高斯模糊涂抹算子（根据框高自适应计算 sigma，确保字符笔画彻底融化不可逆）；
  - 实现纯图片（PNG/JPG/WEBP/BMP）物理脱敏接口 `desensitize_image`，若未提前提取坐标则自动按需调用 OCR 兜底。
- [x] 115.3 **扫描件 PDF 物理脱敏与全格式自动路由接入 (`src/desensitizer.rs`, `src/main.rs`, `src/cli.rs`)**：
  - 扩展 `desensitize_document_auto_detailed` 分发能力，透传 `raw_boxes`，优先复用解析态 OCR 结果；
  - 对扫描件 PDF 提取内嵌页面图片流并执行物理高斯模糊涂抹，重新写回 XObject Stream 并更新字典，生成高保真脱敏 PDF；
  - Web 端点击“导出原格式文档”时，图片格式原生导出脱敏图片（不再降级为 Markdown），扫描件 PDF 原生导出脱敏 PDF；
  - CLI `sensidoc mask` 同步支持图片与扫描件 PDF 原生物理脱敏。
- [x] 115.4 **全链路端到端功能验证与单元测试覆盖**：
  - 编写图片物理模糊、敏感框定位与自动分发单元测试（`test_locate_sensitive_boxes`、`test_image_physical_blur_desensitization`、`test_auto_dispatch_image_file`）；
  - 全量 61 项 Cargo 单元测试 100% 绿灯通过，CLI 与 Web 脱敏导出链路全部贯通。
- [x] 115.5 **“外观显示”面板细节优化与完全脱敏表述对齐**：
  - 将标题“全局 UI 界面缩放比例”更名为“**界面缩放比例**”，保持 100%～200% 范围与 5% 步长平滑联动；
  - 将“默认脱敏打码策略”更名为“**脱敏打码策略**”；
  - 描述文案明确说明“星号掩码所有文本换成*完全脱敏”，消除历史遗留歧义，与全量等长星号算法完全对齐。
- [x] 115.6 **离线模型卡片命名规范收敛**：
  - 将离线模型面板中原“纸质单据与密集表格 OCR 套件 (PP-OCRv6)”卡片标题更名为「**PPOCR-v6 (高性能离线OCR套件)**」，视觉更加精炼直观。
- [x] 115.7 **首次运行向导高信噪比重构（方案 A 落地）**：
  - 弃用旧版 2.1GB 快慢双模型协同方案，主推端侧多模态轻量组合 **Qwen3.5-0.8B**（~706 MB，含主模型与视觉塔）；
  - 剔除“（纯电子文档无需勾选）”、“ (带视觉塔) ”、“初筛/终审”等陈旧或冗余描述，说明文案精简为直达要点的纯本地离线介绍；
  - 配套更新 `web/app.js` 检查逻辑与下载联动，仅勾选核心引擎时仅触发 Qwen3.5-0.8B 下载并自适应就绪关闭。
- [x] 115.8 **规则列表卡片主次层级视觉重构**：
  - 调优规则卡片上下两层高度对比：上层字段名头部 `.rule-card-header` 最小高度提升至 `42px`（内边距增至 `9px 12px`），输入框增至 `28px`，凸显第一视觉中心主体地位；
  - 下层辅助描述区块 `.rule-desc-row` 收敛至 `28px`（内边距微调为 `4px 12px`，输入框 `22px`），形成鲜明的 `42px : 28px` 黄金主次呼吸对比。
- [x] 115.9 **模型卡片“运行中”药丸转指示点与左侧前置状态重塑**：
  - 彻底清退右侧拥挤的「● 运行中」大药丸胶囊标签，释放操作按钮排布空间；
  - 在卡片左侧模型名称与图标前增设精致的 `.engine-status-dot` 状态指示点（折叠箭头后、模型图标前）；
  - 全面支持指示点多态映射：高饱和绿色常亮+呼吸光晕（运行中）、琥珀色呼吸（启动中）、科技蓝呼吸（下载中）、静止灰（离线/待机），鼠标悬停实时展示状态 Tooltip，覆盖预设离线模型与自定义第三方 GGUF 模型。
- [x] 115.10 **左侧面板区块标题字号与图标尺寸联动调优 (`web/style.css`, `web/index.html`)**：
  - 将 `.pane-section-title`（对应「预设模板」与「规则列表」区块标题）字号由 `11.5px` 统一提升至 `12px`；
  - 同步将其前置图标（`.lucide-icon`）由 10px (`.xs`) 调整至 **12px (`.sm`)**，实现文字字号与图标尺寸 1:1 精确平齐，增强整体视觉饱满度与可读性。
- [x] 115.11 **“提取规则”与“提取结果”卡片头部高度精致自然化 (`web/style.css`)**：
  - **规则卡片头部**：`.rule-card-header` 最小高度由 `42px` 调整为更自然的 **`35px`**（内边距由 `9px 12px` 收敛为 `6px 12px`，输入框适配为 `25px`），与底部 `28px` 形成舒适的 7px 高差呼吸节奏，消除空洞臃肿感；
  - **结果卡片头部**：`.audit-card-main` 内边距由 `12px 14px` 优化为 **`7px 12px`**（最小高度 `36px`），与底部 `26px` 的元信息形成优雅自然的比例层次，列表整体视觉更加紧凑专业。
- [x] 115.12 **导出交互逻辑归类重构——未脱敏类收敛至左侧「提取导出 ▼」折叠列表 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - **逻辑对称收敛**：将原底部孤立的单按钮 `[ 导出清单 (CSV) ]` 升级重构为左侧次级折叠下拉菜单 `[ 提取导出 ▼ ]`（左对齐弹出，带 Geist-Neutral 极简微动效与互斥关闭交互）；
  - **未脱敏项归类**：将属于未脱敏类别的「导出清单 (CSV)」与原本混在右侧下拉菜单中的「导出全量提取 JSON」统一收敛于左侧「提取导出 ▼」列表中；
  - **脱敏项聚焦解耦**：右侧主按钮 `[ 🛡 脱敏导出 ▼ ]` 彻底剥离未脱敏的 JSON 项，专属聚焦脱敏产物（保留顶部的掩码/硬抹除策略切换、导出原格式文档、导出脱敏 Markdown），形成左右鲜明的“左侧未脱敏原始数据 vs 右侧已脱敏文档”清晰逻辑阵列。
- [x] 115.13 **前端中文界面与交互文案全面去“审计”化，统一收敛为“提取” (`web/index.html`, `web/app.js`, `src/main.rs`, `src/session.rs`)**：
  - **标题与手柄**：网页及原生桌面窗口标题统一更名为 `SensiDoc - 敏感信息提取与脱敏工具`，侧边栏拖拽手柄更新为 `拖动调整提取面板宽度`，原生文件选择对话框提示对齐为 `请选择要导入提取的文档文件`；
  - **生命周期状态与引导**：结果面板空状态更新为 `暂无提取记录，点击「立即执行」开始提取`，文档列表状态标签由 `已审计` 统一更名为 `已提取`（生命周期完整闭合：待提取 ➔ 提取中 ➔ 已提取）；
  - **导出菜单与产物命名**：菜单项更名为 `导出全量提取 JSON`，默认导出文件名同步规范对齐为 `敏感词提取清单_*.csv` 与 `全量提取数据_*.json`；
  - **系统提示词与模型预设**：首次运行向导与模型卡片更名为 `核心提取引擎`、`深度语义提取版 (MiniCPM5-2B 推荐)`，Prompt 提示词模板统一聚焦为“敏感数据深度提取引擎”；
  - **底层英文代码零风险隔离**：严格只替换中文用户可见界面文本，底层的英文标识符（如 `auditPane`, `auditList`, `exportAuditJson`, `status: "audited"` 等）、CSS 类名及 API 路由完全不动，确保运行时零破坏。

---

## 阶段一百一十六：流式提取响应与单卡片动态探针骨架加载交互体系 (已完成)
- [x] 116.1 **后端流式抽取接口与增量 JSON 实体解析引擎 (`src/main.rs`, `src/extractor.rs`)**：
  - 新增 `POST /api/extract/stream` SSE 流式端点，支持 POST JSON 请求体；
  - 快速通道：正则规则优先在 <10ms 内扫描，一旦检出即刻通过 SSE 发送 `event: item`；
  - AI 增量解析：实现 `StreamingEntityExtractor` 与 `parse_sse_delta`，开启模型 `stream: true`，编写增量式 JSON 对象闭合探测算法，每当解析出一个合法的 `{ "field": "...", "text": "..." }` 实体，校验原文包含度并计算出现次数（`positions`、`count`），立即通过 SSE 向前端推送 `event: item`；
  - 终验与落盘：推理全部结束后，执行 `merge_and_resolve`，更新 Session 快照，通过 SSE 发送 `event: done`（携带完整的快照 ID、统计指标等）。
- [x] 116.2 **前端单卡片动态探针骨架与流式接收渲染通道 (`web/style.css`, `web/app.js`)**：
  - CSS 骨架动效：编写 `.audit-card.skeleton-probe`、微光扫描动画、`.skeleton-block` 呼吸条以及科技感脉冲蓝点与卡片渐入动效（`auditCardSlideIn`）；
  - 点击即跳：`triggerExtraction` 点击瞬时（0ms）立即切换 Inspector Tab 至 “audit”（提取结果面板），清空旧列表，并在最底部挂载唯一的动态骨架探针；
  - 增量接收：采用 `fetch` + `ReadableStream` 监听 SSE 数据流：
    - 每收到 `item`：在最底部的探针骨架上方插入真实卡片（伴随流畅的渐入动效），更新 Tab 计数徽标；探针骨架保持在最尾部呼吸等待；
    - 收到 `done`：从 DOM 中平滑移除探针骨架，渲染未检出字段分割线及灰色卡片，刷新文档快照树，恢复按钮状态；
  - 中断取消：支持用户点击“停止执行”时触发 `AbortController`，中断 SSE 并平滑清理骨架探针。
- [x] 116.3 **端到端综合验证与回归测试**：
  - 运行全量 Cargo 单元测试（64 项测试通过，涵盖流式增量解析与 SSE 探测），`node -c` 语法检查 100% 绿灯；
  - 验证本地离线与在线模型下，点击即跳转、正则项瞬出、AI 项逐项冒出、尾部单骨架呼吸推进与完成收敛的整体体验。

---

## 阶段一百一十七：双平台多架构 llama.cpp 运行时构建补齐与 macOS 独立拆包瘦身 (已完成) · [🔗 对话跳转](conversation://00699197-537e-4af3-b6f9-ad5df82ec551)
- [x] 117.1 **Rust 端跨平台路径与无黑框进程守护治理 (`src/paths.rs`, `src/model_manager.rs`)**：
  - 扩展 `src/paths.rs` 的 `get_llama_bin_path`：区分 Windows 下 `.exe` 扩展名，并在开发环境下优先智能探测对应架构子目录（`bin/windows-x86_64/`、`bin/windows-arm64/`、`bin/macos-arm64/`、`bin/macos-x86_64/`）再回退至根 `bin/`；
  - 在 `src/model_manager.rs` 中适配 Windows 环境：配置 `creation_flags(0x08000000)`（`CREATE_NO_WINDOW`），防止启动模型弹出控制台黑框；注入 `llama_bin_path` 所在目录至环境变量 `PATH`，保证同级 Windows DLL 顺利装载。
- [x] 117.2 **Windows 双架构依赖自动获取脚本 (`scripts/download_llama_windows.ps1`)**：
  - 新建自动化拉取脚本，支持 `-Arch`（`x86_64` 或 `arm64`，默认 `x86_64`）与 `-Version`；
  - 针对 `x86_64` 抓取官方 `llama-*-bin-win-avx2-x64.zip`，针对 `arm64` 抓取官方 `llama-*-bin-win-arm64.zip`；解压并自动提取至 `bin/windows-$Arch/`。
- [x] 117.3 **Windows Inno Setup 与一键打包流水线升级 (`scripts/installer.iss`, `scripts/build_win_installer.ps1`)**：
  - 升级 `scripts/installer.iss`：接收 `RuntimeBinDir` 宏，将对应架构的 `bin` 运行时打包进安装包 `{app}\bin\`，若存在 `onnxruntime.dll` 则一并打入应用根目录；
  - 升级 `scripts/build_win_installer.ps1`：基于已有的 `$Arch` 参数，前置校验并按需触发自动下载，在制作绿色免安装 ZIP 时打包对应架构运行时目录与依赖库。
- [x] 117.4 **macOS 打包流水线独立拆包与显著瘦身 (`scripts/build_mac_app.sh`)**：
  - 彻底废除 `lipo` 暴力合并双架构逻辑，主程序体积直降 50%；
  - 支持参数化构建：`./scripts/build_mac_app.sh [version] [arm64|x86_64|all]`，分别编译并生成清晰明了的 `sensidoc-v*-macos-arm64.dmg` 与 `sensidoc-v*-macos-x86_64.dmg`；
  - 按目标架构自动选取匹配的 `llama-server` 与动态库，彻底杜绝架构错配导致的运行崩溃。
- [x] 117.5 **跨架构脚本语法与全量单元测试验收**：
  - 运行 `cargo test` 验证 `paths.rs` 与 `model_manager.rs`，全量 64 项单测 100% 绿灯（63 passed, 1 ignored, 0 failed）；
  - `bash -n scripts/build_mac_app.sh` 语法静态检查通过（Code 0）；
  - 逐项审查 14 个原始诊断问题全部确认已修复：Windows `.exe` 后缀适配、多架构子目录探测、`CREATE_NO_WINDOW` 黑框抑制、PATH 注入、下载脚本双架构支持、Inno Setup `RuntimeBinDir` 宏与 `onnxruntime.dll` 打包、绿色 ZIP 收录运行时、`lipo` 彻底移除、架构错配杜绝、DMG 命名区分等；
  - 遗留非阻塞备忘：`stop_server()` 的孤儿端口清理仅 `#[cfg(unix)]`（通过 `lsof`），Windows 下暂无对应兜底，但 `child.kill()` 的 `TerminateProcess` 语义已足够可靠，不影响功能正确性。
- [x] 117.6 **GitHub Actions 远端 CI/CD 流水线矩阵升级 (`.github/workflows/release.yml`, `scripts/download_llama_windows.ps1`)**：
  - 将 macOS 构建重构为 Matrix 并行矩阵（`arm64` 与 `x86_64`），各自独立拉取对应架构 target 并调用 `./scripts/build_mac_app.sh "$VER" "${{ matrix.arch }}"`，彻底消除 lipo 并行翻倍产物；
  - 优化 `scripts/download_llama_windows.ps1`，针对 GitHub Actions CI 环境直连 GitHub Releases 官方源，确保 Windows 双架构（`x86_64` / `arm64`）自动打包与 llama.cpp 运行时注入 100% 稳定；
  - 完善 Release 归档逻辑，自动收集并公开发布双系统双架构全量 12 份标准与离线增强 DMG / Setup / Zip 产物。

---

## 阶段一百一十八：提取面板顶部实时流式响应气泡条与可折叠终端抽屉 (已完成)
- [x] 118.1 **后端流式抽取通道增设 Token Delta 实时派发 (`src/main.rs`)**：
  - 在 `extract_sensitive_info_stream` 中，当解析到大模型（本地/在线）流式吐出的 `delta` 片段时，通过 `Event::default().event("delta").data(...)` 实时推送，为前端提供逐字打字机数据流；
- [x] 118.2 **提取面板顶部实时气泡条与可折叠暗黑终端构建 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - **DOM 部署**：在 `#auditPane` 的工具栏正下方部署 `#streamStatusBar`；
  - **CSS 动效**：编写 `.stream-status-bar`、`.stream-ticker` 单行流光条、`.stream-terminal-drawer` 等宽暗黑终端抽屉及平滑展开/收起过渡动效；
  - **流式渲染通道**：
    - 开始时挂载状态条，初始化为“正在初始化推理管道...”；
    - 收到 `delta`：实时更新单行 Ticker 最新片段，并在终端抽屉中打字机滚动置底；
    - 收到 `item`：单行 Ticker 高亮提示“捕获敏感项: [分类] 原词”；
    - 收到 `done`：收敛为极淡灰色状态线“✔ 本次提取已完成 · 耗时 X.Xs · 共检出 Y 项 [查看日志 ▾]”，支持事后随时回溯；
- [x] 118.3 **端到端综合编译与测试回归**：
  - 跑通全量 64 项 Cargo 单元测试与 `node -c` 前端语法校验，编译并重启服务验证完整交互。

---

## 阶段一百一十九：重新提取时清空预览框选与流式提取中中间文档实时框选同步 (已完成)
- [x] 119.1 **点击“立即执行”前置清理中间面板旧框选与聚焦态 (`web/app.js`)**：
  - 在 `executeExtractionStream` 开始时，立即复位 `state.selectedSensiText` 与 `state.highlightIndices`；
  - 传入空列表调用 `renderMarkdownWithHighlights(doc.markdown, [])`，清除中间文档预览区与卷帘透视容器中所有残留的 `<mark class="sensi-mark">` 框选边框与高亮；
- [x] 119.2 **流式提取循环中增量同步中间面板框选与视觉脉冲 (`web/app.js`)**：
  - 在 `executeExtractionStream` 的 SSE 数据流消费循环中，每当捕获到新的实体卡片（`event: item`）时：
    - 立即调用 `renderMarkdownWithHighlights(doc.markdown, currentHitItems)` 增量刷新中间文档插桩；
- [x] 119.3 **端到端综合回归与全功能验收**：
  - 执行 `node -c web/app.js` 语法静态检查通过；
  - 真实全流程验证：点击“立即执行”旧框选瞬间清空、流式提取中实体增量捕获时中间面板实时圈选同步、卡片点击轮转跳转与脉冲聚焦完全正常。

---

## 阶段一百二十：修复流式提取完成后耗时统计显示为 0.0s 问题 (已完成)
- [x] 120.1 **快照耗时字段名对齐与高精度客户端兜底 (`web/app.js`)**：
  - 根因定位：后端快照中耗时字段名为 `execution_ms`，前端此前误写为 `execution_time_ms`，导致读取为 `undefined` 从而计算出 `0.0s`；
  - 在 `executeExtractionStream` 开头记录 `streamStartTime = performance.now()`；
  - 闭环处优先使用 `finalSnapshot.execution_ms`，若缺失则优雅回退至客户端端到端毫秒差 `clientElapsedMs`；小于 100ms 呈现 `< 0.1s`，正常范围呈现 `X.Xs`，并在 tooltip 与终端日志中同步输出精确毫秒值；
- [x] 120.2 **综合验证与回归验收**：
  - `node -c web/app.js` 语法检查通过；
  - 真实流式提取实测：耗时精确显示为 4.3s（4344ms），暗黑终端抽屉与气泡条状态均准确无误。

---

## 阶段一百二十一：深色主题重塑与全界面沉浸石墨深灰风格 (已完成) · [🔗 对话跳转](conversation://92481bbf-2c09-4fed-acbb-6f2461637639)
- [x] 121.1 **Design Tokens 重塑与多层级深灰调色 (`web/style.css`)**：
  - 参考微信桌面端深色模式，将深色主题主背景由死黑重构为石墨深灰 `#191919`；
  - 左侧文档列表卡片（`.file-item`）与右侧规则列表卡片（`.rule-card`）统一定制为浅黑 `#232326`，凸显浮动卡片的微立体层次感；
  - 软件顶部标题栏（`.app-header`）与左侧文档列表栏（`.sidebar-header`）统一绑定 `var(--surface-2)`（`#242428` / `#fafafa`），保持顶底一致；
- [x] 121.2 **OCR 悬浮工具条与全局组件深色修复 (`web/style.css`)**：
  - 修复单据图片 OCR 悬浮工具条收起胶囊与展开态在深色模式下未切换（白底白字隐形）的 CSS 选择器语法缺陷；
  - 统一接入 `--surface-floating`（`rgba(38, 38, 41, 0.92)`）与毛玻璃模糊滤镜；
  - 将「预设模板」与「规则列表」之间的 1px 分割线设为透明，预设模板下拉框与按钮适配 `var(--surface-2)` 浅灰底纹。

---

## 阶段一百二十二：规则管理操作流闭环与 AI 提炼面板浅灰底纹统一 (已完成) · [🔗 对话跳转](conversation://92481bbf-2c09-4fed-acbb-6f2461637639)
- [x] 122.1 **点击“添加新规则”自动收回 AI 生成面板 (`web/app.js`)**：
  - 在 `openAddRuleModal` 中前置过滤 `_isAiPrompt` 临时卡片，调用 `closeAiGenRulesDrawer` 并自动复位「AI生成」按钮的激活状态；
  - 收起面板后直接在列表首位插入并平滑聚焦至空白规则草稿卡片（Draft Card），操作逻辑自然闭环；
- [x] 122.2 **「AI提炼规则」面板浅灰底纹与输入框白底对比统一 (`web/style.css`)**：
  - 将 `.rule-card.ai-prompt-card` 及抽屉底纹设置为与「添加新规则（草稿卡片）」完全一致的浅灰色 `#f2f2f2`（深色模式下 `#21262d`）；
  - 内部 Prompt 输入框采用纯白 `#ffffff`（深色 `#191919`）与细线边框，形成清晰的输入层级对比。

---

## 阶段一百二十三：OCR 快速 VLM 档位控件交互与微切片对照体验深度优化 (已完成) · [🔗 对话跳转](conversation://92481bbf-2c09-4fed-acbb-6f2461637639)
- [x] 123.1 **置信度下拉框位置与动态联动显示 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 位置调整：将置信度下拉框移至「快速VLM」与「全量VLM」之间，紧跟快速VLM胶囊，逻辑自然连贯；
  - 动态联动：移除 CSS 中的 `!important` 强行覆盖，严格仅在选中「快速VLM」时呈现置信度下拉框，切换到「基础OCR」或「全量VLM」时完全隐藏（`display: none`）；
  - 文案纯净化：移除下拉按钮上的 `<` 符号，精简呈现纯百分数（如 `85%`）；
  - 消除切换噪音：移除切换三档识别视图时底部弹出的冗余 Toast 消息；
- [x] 123.2 **消除悬浮工具条加载跳动 Bug 与加深分段器底色 (`web/style.css`)**：
  - 根治跳动：移除外层 `.ocr-floating-toolbar` 上的关键帧位移动画，避免起始帧冲掉 `transform: translateX(-50%)` 导致从右侧跳到中间的问题，确保绝对水平正中稳定加载；
  - 分段器底色加深：将分段器容器背景调深为 `var(--surface-active, #ebebeb)`，未选中项自然呈现中灰底色，选中项保持纯白卡片与微投影，对比鲜明一目了然；
- [x] 123.3 **快速 VLM 修正文本长按对比原文本与悬浮提示精炼 (`web/app.js`, `web/style.css`)**：
  - 长按瞬时对比：对被纠偏的文本（`.vlm-text-corrected`）绑定 `mousedown` / `touchstart` 事件，鼠标按下长按瞬时切换呈现原文本（无中划线干扰），松开鼠标（`mouseup` / `mouseleave`）即刻恢复纠正后文本；
  - 提示精炼：悬浮提示文本精简为 `原文本：${oldText}`，去除冗长前缀。

---

## 阶段一百二十四：快照相关信息工作台升级、流式日志深度集成与中间面板底部信息栏移除 (已完成) · [🔗 对话跳转](conversation://3c4d86cb-a505-4033-8c42-a3832939cffc)
- [x] 124.1 **顶部工具栏「提取规则」按钮重构为「相关信息」与底部信息栏移除 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 顶部工具栏时光机右侧原「提取规则」按钮正式更名为「相关信息」，图标更新为标准信息详情（Info）图标，鼠标悬浮提示同步升级为“查看当前快照的相关信息、统计指标与执行详情”；
  - 彻底移除中间文档阅读面板底部的 `.preview-footer-bar`（包含字符数、处标记、执行模型及字体缩放按钮），让 Markdown 渲染与源码模式纵向视野纯净舒展、自适应占满容器；
  - 前端脚本对被移除的 DOM 节点做好防御性安全兼容，避免任何 null 引用风险。
- [x] 124.2 **快照元数据与流式执行日志（Streaming Logs）持久化与后端支持 (`src/session.rs`, `src/main.rs`, `src/cli.rs`)**：
  - 数据结构扩展：在 `ExtractionSnapshot` 结构体中新增 `#[serde(default)] pub execution_log: Option<String>`，实现向后无缝兼容；
  - 流式过程收集：在 `POST /api/extract/stream` 执行管道中实时收集正则秒级初筛规则数、模型推理吐出的全部 Tokens Delta、实体捕获事件与最终耗时汇总，在 `add_snapshot` 时持久化落盘至快照；
  - 保证历史快照回放时可随时调出当时完整的推理轨迹与日志。
- [x] 124.3 **重构「快照相关信息」深度审查工作台为 5 栏网格与圆角药丸 Tab 切换 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - **5 栏工整指标看板**：将原底部信息栏与快照核心元数据解耦重构为清晰规范的 5 栏网格（执行模型、文档规模、实体检出、执行耗时、记录时间），执行耗时精简去毫秒（纯秒显示，如 `4.4s`），单项职责明确；
  - **圆角矩形药丸 Tab 切换条 (Pill Tab)**：彻底摒弃原先直角生硬的灰色矩形方块，外层采用平滑圆角卡槽（`border-radius: 8px`），激活项呈现独立的纯白/深色圆角矩形药丸（`border-radius: 6px` + 微柔悬浮投影），完美包裹住标签文本与图标；
  - **三大排查板块**：支持无缝切换「⚡ 执行日志 (Streaming Logs)」（暗黑等宽高对比度终端，带一键复制日志与行数统计）、「📝 系统提示词 (Prompt)」与「📋 生效规则字段」。
- [x] 124.4 **快照版本实时联动同步与综合验收回归 (`web/app.js`)**：
  - 联动响应：在时光机下拉（`selectSnapshot`）或重新提取自动激活新快照时，若「快照相关信息」弹窗正处于打开状态，即刻自动重新调用 `openSnapshotRulesModal()` 动态刷新当前选中快照的所有指标、流式日志、提示词与规则，无需关闭重开；
  - 全量回归：跑通 `cargo check`、`cargo test`（64 项单元测试 100% 绿灯，63 passed, 1 ignored）及 `node -c web/app.js` 语法静态检查。

---

## 阶段一百二十五：文档本地源文件重新关联机制与底图缓存防丢自愈体系 (已完成) · [🔗 对话跳转](conversation://22ade210-5b05-4199-a973-928f91585f1e)
- [x] 125.1 **数据模型扩展与底图状态探测能力 (`src/session.rs`)**：
  - 在 `DocumentItem` 增加 `pub fn has_original_file(&self) -> bool`，智能检测 `source_path` 物理源文件或 `uploads/{id}.bin` 缓存有效性；
  - 新增 `pub async fn relink_document_source(...)` 方法，安全重写 `uploads/{id}.bin` 物理镜像、更新 `source_path` 元数据并自动重解析图像尺寸后落盘保存。
- [x] 125.2 **后端多模重新关联接口与原生文件选择对话框整合 (`src/main.rs`)**：
  - 封装跨平台原生单选文件对话框 `pick_single_document_dialog(prompt_text)`（适配 macOS `osascript` 与 Windows `PowerShell OpenFileDialog`）；
  - 注册 `GET /api/documents/{id}/file-status` 提供底图完整性健康状态查询；
  - 注册 `POST /api/documents/{id}/pick-and-relink` 支持桌面端一键唤起系统原生选择器直连物理源文件；
  - 注册 `POST /api/documents/{id}/relink` 支持 Web 浏览器端 Multipart 文件上传或指定路径补齐。
- [x] 125.3 **前端右键上下文菜单扩展与双模自适应关联流程 (`web/app.js`)**：
  - 在侧边栏文档项右键菜单中新增「重新关联源文件...」选项；
  - 封装 `relinkDocumentFile(docId)`：桌面客户端运行环境下优先调用原生对话框，网页端或降级场景动态唤起隐藏 `<input type="file">`，关联成功后自动触发视图与底图缓存无缝热更新。
- [x] 125.4 **VLM/OCR 底图缺失主动拦截与带操作引导的全局通知体系 (`web/app.js`)**：
  - 封装 `showToastWithAction(message, actionLabel, onAction, type)` 高交互 Toast 组件；
  - 在全量 VLM 重构、快速 VLM 复核及基础 OCR 识别捕获到“原始文件不存在，本地暂存缓存已丢失”时，主动抛出带有 `[重新关联文件]` 操作按钮的浮层提示，点击直达关联流程；
  - 桌面端侧边栏监听 DOM `drop` 事件时增加 `window.__SENSIDOC_DESKTOP__` 判定防护，全面让渡给底层系统级 `DragDropEvent` 原生拖拽，确保新拖入文件永久绑定宿主绝对路径。
- [x] 125.5 **生命周期单元测试覆盖与全量功能回归 (`src/session.rs`)**：
  - 新增 `test_document_relink_lifecycle` 单元测试，完整覆盖“底图丢失探测 ➔ 重新关联物理源路径 ➔ 恢复读取并同步工作区”的全生命周期闭环；
  - 全工程 65 项单元测试（OCR、脱敏、解析、推理等）全部绿灯通过（64 passed, 0 failed, 1 ignored）。

---

## 阶段一百二十六：OCR 多余空行清洗与冗余表格标签自动过滤 (已完成) · [🔗 对话跳转](conversation://fb5c8714-4c4f-4b30-9b2d-91bec0eb221e)
- [x] 126.1 **anyocr 表格渲染空行过滤与全空表格剔除 (`anyocr::layout::table_matcher`)**：
  - 在 `render_tokens_to_markdown` 阶段，自动识别并过滤纯空表格行（`<tr><td></td></tr>`、`<tr><td colspan="*"></td></tr>` 及仅含空白字符的空单元格行）；
  - 若整张表格在剔除空行后无实质有效内容，自动消除外层 `<table>...</table>` 标签，杜绝空表格与单列长图伪表格渲染；
  - anyocr 变更已本地验证通过并同步推送至 Gitee 及 GitHub 远程仓库（commit `af9ef3a`），SensiDoc 通过 `cargo update anyocr` 锁定最新 revision。
- [x] 126.2 **SensiDoc 核心空行与脏标签清洗器落地 (`src/ocr/mod.rs::sanitize_redundant_empty_lines`)**：
  - 构建高并发、零开销的 `LazyLock` 预编译正则流水线；
  - 自动过滤 HTML 空表格行、清理空 thead/tbody/table 节点、剔除 Markdown 管道符纯空数据行 (`| | |`)；
  - 清理无实质文本内容的 `<p></p>` 与 `<div></div>` 标签；
  - 将连续 3 个及以上的连续多余换行统一收敛归一化为标准的 2 个换行 (`\n\n`)，保持段落干净整洁。
- [x] 126.3 **OCR / VLM / PDF 转码全链路接入与结果清洗 (`src/ocr/mod.rs`, `src/ocr/vlm.rs`, `src/converter.rs`)**：
  - `OcrEngine::convert_doc_to_result` 产出的 Markdown 自动挂接空行清洗；
  - `run_fast_vlm_pipeline`（快速 VLM 微切片纠偏）与 `run_full_vlm_pipeline`（全量 VLM 重构）产出自动挂接空行清洗；
  - `DocConverter::extract_scanned_pdf` 输出自动执行空行清洗。
- [x] 126.4 **大模型实体抽取 Prompt 刚性防线构建 (`src/extractor.rs`, `src/main.rs`, `src/cli.rs`)**：
  - `query_llm` 与 `query_online_llm` 在组装 `<document>` 标签前对正文执行严格空行清洗，彻底杜绝无意义空行与空表格标签污染 LLM 上下文；
  - `extract_sensitive_info` 与 `extract_sensitive_info_stream`（非流式/流式提取）在执行初筛前对全文进行清洗与对齐；
  - `cli.rs` 在命令行提取流程中挂接清洗器，保障无头审计与 CI 产物清洁度。
- [x] 126.5 **单元测试与端到端真实单据图像回归验证**：
  - 编写涵盖 HTML 空表格行、Markdown 空表格行、正文连续多余换行、用户实际长图文多场景的自动化单测（`test_sanitize_redundant_empty_lines`）；
  - 全工程 66 项 Cargo 单元测试 100% 绿灯通过；
  - 在真实样本 `《免费：商业的未来》深度解读.png` 上进行端到端验证，原本产生的 40 处空 `<tr>` 标签与 26 行连续横条被 100% 消除，“★★★高”与“★★中”自然衔接。

---

## 阶段一百二十七：macOS 原生文件选择器焦点置顶与重新关联交互容错加固 (已完成) · [🔗 对话跳转](conversation://73fa04b2-2d02-4c05-ae6b-56f1031d0d4b)
- [x] 127.1 **macOS 操作系统原生选择器前台激活与焦点置顶加固 (`src/main.rs`, `src/model_manager.rs`)**：
  - 根因消除：解决无宿主 AppleScript `choose file` 对话框被 SensiDoc 桌面主窗口遮挡在底层、无前台焦点的缺陷，在脚本中注入 `tell application (path to frontmost application as text) activate`，确保原生选择器直接浮现在当前应用窗口最上层；
  - 覆盖文档多选 (`pick_files_dialog_native`)、单文档重新关联 (`pick_single_document_dialog`) 与 GGUF 模型文件选择 (`pick_file_dialog`) 全场景。
- [x] 127.2 **原生对话框多级容错兜底与异常精准透传 (`src/main.rs`, `src/model_manager.rs`)**：
  - 建立三级降级链路：特定文档类型/UTI 过滤 ➔ 全类型兼容唤起 ➔ `activate me` 纯原生兜底；
  - 异常暴露收敛：彻底修复底层执行失败时误返回 `Ok(None)` 伪装成用户主动取消的 BUG，真实系统失败时准确返回 HTTP 500 与具体 stderr 报错信息，保证前端进入异常分支并感知原因。
- [x] 127.3 **前端用户手势 (User Gesture) 安全降级与交互兜底重构 (`web/app.js`)**：
  - 抽取独立的同步本地选择器 `triggerLocalRelinkFileInput(docId)`；纯网页端直接同步触发，保留浏览器原生手势上下文；
- [x] 127.4 **编译与自动化回归验证**：
  - `cargo check` 零报错通过；
  - 跑通 `cargo test` 全量 66 项测试（65 passed, 1 ignored），验证各项生命周期与底图重构链路稳定。

---

## 阶段一百二十八：Windows 离线推理运行时构建升级 (Qwen3.5 架构适配) 与全生命周期日志系统 (已完成) · [🔗 对话跳转](conversation://9e72670a-3b2d-4b2d-a252-4a0924c3b94e)
- [x] 128.1 **Windows 预编译 llama.cpp 运行时构建升级与现代资产命名适配 (`scripts/download_llama_windows.ps1`)**：
  - 定位 Windows ARM 与 x86 双架构离线模型启动失败根本原因：`llama_model_load: error loading model: error loading model architecture: unknown model architecture: 'qwen35'`，旧版脚本锁定了 2025 年初的 `b4600`，不识别 2026 年较新的 Qwen 3.5 架构；
  - 默认版本升级至官方支持 `qwen35` 的最新稳定构建 `b11026`；
  - 适配官方现代 Release 资产命名（`llama-${Version}-bin-win-cpu-x64.zip`、`llama-${Version}-bin-win-cpu-arm64.zip`、`llama-${Version}-bin-win-opencl-adreno-arm64.zip` 等），确保本地打包及 GitHub Actions CI 自动获取官方预编译包（方案 A）。
- [x] 128.2 **安装阶段全过程日志记录与自动归档 (`scripts/installer.iss`)**：
  - 在 Inno Setup `[Setup]` 配置段开启 `SetupLogging=yes`，自动记录安装解压、权限检查与文件部署；
  - 编写 `[Code]` 脚本段的 `DeinitializeSetup()` 钩子：当安装完成、异常中断或失败时，自动将 Inno Setup 安装日志归档保存至用户数据目录 `%APPDATA%\SensiDoc\logs\installer.log`。
- [x] 128.3 **应用日志目录与关键日志片段读取基础能力 (`src/paths.rs`)**：
  - 新增 `get_logs_dir()`（Windows: `%APPDATA%/SensiDoc/logs`，macOS: `~/Library/Application Support/SensiDoc/logs`）；
  - 新增 `get_sensidoc_log_path()`、`get_llama_log_path()` 与 `get_installer_log_path()`；
  - 新增 `read_recent_log_snippet()` 安全读取最近 N 行有效日志文本。
- [x] 128.4 **后端双轨日志落盘与系统级日志目录一键直达 (`Cargo.toml`, `src/main.rs`)**：
  - 引入 `tracing-appender = "0.2"` 依赖；
  - 在 `main()` 入口通过 `init_logging()` 初始化非阻塞文件写入器，将主服务运行日志写入 `sensidoc.log`，同时保持控制台标准输出；
  - 注册 `GET /api/system/logs`（查询日志文件状态与最新日志片段）与 `POST /api/system/logs/reveal`（在 Windows 资源管理器 / macOS 访达中一键高亮打开日志目录）。
- [x] 128.5 **llama-server 启动异常毫秒级捕获与错误日志穿透透传 (`src/model_manager.rs`)**：
  - 将子进程标准输出与错误输出重定向至 `llama-server.log`；
  - 启动健康检查探测中，通过 `child.try_wait()` 在 500ms 内即时捕获子进程提前退出/崩溃（如缺失模型架构、缺失 VC++ DLL），彻底废除原先盲目干等 25 秒超时的糟糕体验；
  - 自动抓取 `llama-server.log` 最近 10 行日志直接组装进错误信息中返回前端。
- [x] 128.6 **前端 UI 错误展示与查看日志交互闭环 (`web/index.html`, `web/app.js`)**：
  - 设置弹窗右上角增加【查看日志】按钮，一键调起 Windows 资源管理器打开日志目录；
  - 二次确认 / 提示弹窗支持 `white-space: pre-wrap;` 与滚动条展示完整报错，且在涉及模型或日志异常时自动呈现【查看详细日志】按钮，点击直达日志目录；

---

## 阶段一百二十九：OCR 推理吞吐架构调优与识别模型轻量化降维升级 (已完成) · [🔗 对话跳转](conversation://28ffd366-73fb-426f-bf1e-99dd72430155)
- [x] 129.1 **CPU 纯文本推理 Padding 膨胀瓶颈定位与 Batch 架构调优 (`src/ocr/mod.rs`)**：
  - 实测排查定位在 CPU 笔记本环境下多行切片识别严重耗时（原 70s 级）的深层根因：开启 Batch 处理时不同切片必须 Padding 补零对齐到批内最长文本宽度，在 CPU 缺乏海量张量核心的环境下引发数倍无效计算，同时触发 L1/L2 Cache 频繁失效；
  - 将 `anyocr::EngineConfig` 的 `max_batch_size` 调优为 `1`（紧凑逐行串行计算），在 M4 / CPU 环境下单行识别耗时立即从 9.8s 暴降至 3.3s。
- [x] 129.2 **识别模型降维轻量化切换与兼容回退 (`src/paths.rs`)**：
  - 将默认识别模型由 `PP-OCRv6_rec_medium.onnx`（34.5M, 76.6MB）切换为更轻快、高性价比的 `PP-OCRv6_rec_small.onnx`（7.7M, 21.2MB）；
  - `get_ocr_rec_path()` 优先检索并加载 Small 模型，若本地旧环境仅存在 Medium 模型则平滑兼容回退，确保零破坏性迁移。
- [x] 129.3 **设置面板与离线模型套件下载链路同步重构 (`src/model_manager.rs`, `web/index.html`, `scripts/download_ocr_models.sh`)**：
  - 更新 `model_manager.rs` 中的 `OCR_DOWNLOAD_SPECS`：文本识别组件直链切换为 Small 模型，更新精确尺寸（`21_234_383` 字节）与 SHA-256 校验码（`6f327246b50388f3c176ae304bd95767ea6dc0c9ae92153ef8cbe210b3c14884`）；
  - `web/index.html` 设置面板组件状态树文案同步更新为 `PP-OCRv6_rec_small.onnx (21.2 MB)`；
  - 更新 `scripts/download_ocr_models.sh` 离线打包拉取脚本，将 medium 替换为 small 模型直链。
- [x] 129.4 **GitHub Actions CI/CD 流程与打包缓存升级 (`.github/workflows/release.yml`)**：
  - 将 macOS (arm64) 与 Windows (x86_64/arm64) 构建工作流中的 OCR 模型缓存键统一由 `ocr-models-v1` 升级为 `ocr-models-v2`；
  - 保证 CI 构建离线全量版应用包时自动拉取并打包最新的 Small 模型，同时让离线安装包体积直接瘦身约 55MB。
- [x] 129.5 **端到端实测验证与性能飞跃闭环**：
  - 针对实测单据图片（968×1310，24行文本）全量验证：端到端推理总耗时由 5.67s 骤降至 **0.65s (648ms)**，识别部分由 5560ms 压降至 **534ms**（单行仅需 22ms），提速达 **10.4 倍**，解析字符完整准确且低置信度为 0；在常规 4 核 CPU 笔记本上整体耗时将从 70s 直接压降进 3~5s 黄金区间；
  - `cargo check` 与 `cargo build --release` 编译全部通过。
