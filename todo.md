# SensiDoc 开发任务清单与进度跟踪 (todo.md)

> 本文档实时记录 SensiDoc 项目各模块的开发实施进度，严格按检查点验证推进。  
> 标记说明：`[ ]` 待办 | `[-]` 进行中 | `[x]` 已完成并通过测试  
> 💬 **上下文会话导航**：[🔗 点击跳转到本次聊天对话记录 (Conversation ID: 8fbdd9e2-6055-43cb-a0b5-748e43cd77d5)](conversation://8fbdd9e2-6055-43cb-a0b5-748e43cd77d5)

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

## 阶段五十九：规则特征描述输入框常态化可见与添加交互优化 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)
- [x] 59.1 **规则特征描述输入框样式常态化 (`web/style.css`)**：
  - 将 `.rule-desc-input` 从原有的透明边框与透明背景（容易误认为是不可编辑纯文本）重构为具象常态输入框样式：`border: 1px solid var(--border); background: var(--surface);`；
  - 补充 `:hover` 边框微亮与 `:focus` 聚焦光晕外圈，使用户无需点击也能清晰感知该区域为输入框。
- [x] 59.2 **添加字段交互与占位符透出优化 (`web/app.js`)**：
  - 点击「＋ 新增规则」时，新建规则的描述默认值由原硬编码的 `"提取特征与上下文模式描述"` 改为空字符串 `""`，使原生的 `placeholder="请输入提取目标的特征描述"` 清晰透出提示，引导用户直观输入；
  - 在 `renderRulesTable` 中对 `rule.description` 做空值安全回退处理。
- [x] 59.3 **端到端实机验证与全链路回归**：
  - 经 Headless Chrome 自动化实机交互与截图验证，点击添加规则后，字段名输入框与特征描述输入框均以清晰内嵌输入框常态呈现，视觉边界清晰且占位符指引明确。
