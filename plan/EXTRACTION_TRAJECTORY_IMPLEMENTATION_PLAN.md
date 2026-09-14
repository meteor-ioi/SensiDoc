# SensiDoc 提取全链路轨迹 (Trajectory) 可视化与排查体系实施方案

## 1. 目标与背景
当前中间面板顶部的「提取规则」按钮（`#viewSnapshotRulesBtn`）仅能静态展示 System Prompt 与字段定义，存在两项核心局限：
1. **提取执行过程黑盒化**：点击提取后仅有按钮 loading 态，长文档或大模型长耗时推理时无法感知当前切片进度与中间状态；
2. **缺乏审计排查凭据**：提取漏报、误报或报错时，无法复核模型原始输出（Raw JSON）、思考链（Reasoning）、单块耗时与正则命中细节。

参考 `deepseek-harness` 的轨迹（Trajectory）体系，构建兼具**“实时流式进度可视”**与**“历史快照深度回放排查”**的全链路轨迹系统。

---

## 2. 总体架构与数据流

```text
┌──────────────────────────────────────────────────────────────┐
│ 前端工作台 (Web/Desktop GUI)                                  │
│                                                              │
│ [中间顶部按钮] ⏱️ 提取轨迹 (执行中闪烁/平时常亮)              │
│       │                                                      │
│       ├── 点击打开 ──> [轨迹模态窗] (Timeline + Ledger)       │
│       │                      ▲                               │
│       │                      │ SSE 流式推送 / 快照回溯       │
└───────┼──────────────────────┼───────────────────────────────┘
        │                      │
        ▼ POST /api/extract/stream
┌──────────────────────────────┴───────────────────────────────┐
│ 后端执行引擎 (Rust / Axum)                                    │
│                                                              │
│ ├── 阶段 1: 预处理分块 (Chunking) ──> trace_step [preprocess] │
│ ├── 阶段 2: 正则引擎 (Regex)     ──> trace_step [regex]      │
│ ├── 阶段 3: 大模型推理 (LLM)      ──> trace_step [model] x N  │
│ │   (支持本地小模型协同 & 在线云端 API)                      │
│ ├── 阶段 4: 结果消解 (Resolve)   ──> trace_step [resolve]    │
│ └── 阶段 5: 快照持久化 (Snapshot) ──> trace_done [snapshot]   │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. 数据结构契约设计 (`src/session.rs`)

### 3.1 轨迹与阶段模型
```rust
/// 单次提取的完整执行轨迹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrajectory {
    pub total_ms: u64,
    pub total_tokens: Option<TokenUsage>,
    pub phases: Vec<TracePhase>,
}

/// Token 消耗统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 轨迹阶段节点 (Phase)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracePhase {
    pub id: String,
    pub kind: String,         // "preprocess" | "regex" | "model" | "resolve"
    pub title: String,        // 例如 "模型推理 Chunk 1/3"
    pub start_offset_ms: u64, // 相对起始时间偏移量 (用于甘特图投影)
    pub duration_ms: u64,
    pub status: String,       // "running" | "success" | "failed"
    pub input_summary: Option<String>,
    pub raw_response: Option<String>,
    pub reasoning: Option<String>,
    pub tokens: Option<TokenUsage>,
    pub items_found: usize,
    pub error: Option<String>,
}
```

### 3.2 快照向后兼容扩展
在 `ExtractionSnapshot` 中扩展可选字段：
```rust
pub struct ExtractionSnapshot {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub template_name: String,
    pub model_name: Option<String>,
    pub system_prompt: Option<String>,
    pub fields_used: Vec<RuleField>,
    pub items: Vec<SensitiveItem>,
    pub execution_ms: u64,
    #[serde(default)]
    pub trajectory: Option<ExecutionTrajectory>, // 向后无缝兼容
}
```

---

## 4. 通信协议与后端改造 (`src/main.rs`, `src/extractor.rs`)

1. **接口升级**：
   - `POST /api/extract`：保留原有阻塞式调用（快照内自动注入完整 `trajectory`）；
   - `POST /api/extract/stream`：基于 Axum `axum::response::sse::Sse` 建立流式推送。
2. **事件规范 (SSE Events)**：
   - `event: trace_start`：告知任务 ID、文档字数、预估分块数；
   - `event: trace_step`：下发单个阶段的更新状态（进行中/已完成/数据摘要）；
   - `event: trace_done`：下发最终持久化完成的 `ExtractionSnapshot` 数据；
   - `event: trace_error`：发生异常时优雅终止并告知故障节点。

---

## 5. 前端交互与视图组件 (`web/index.html`, `web/app.js`, `web/style.css`)

### 5.1 顶部操作入口
- 将 `#viewSnapshotRulesBtn` 升级为 `⏱️ 提取轨迹`；
- 执行中带有呼吸动画与动态文案（`⏱️ 提取中 (2/4)...`）；
- 点击唤出大尺寸专业审查模态框 `#trajectoryModal`。

### 5.2 模态框双栏三段式设计
```text
┌────────────────────────────────────────────────────────────────────────┐
│ ⏱️ 提取全链路轨迹  [#20版]  模型: Qwen2.5-7B   耗时: 3.2s   Token: 2,150  │ [×]
├────────────────────────────────────────────────────────────────────────┤
│ [工具栏] 模式: 实际耗时 ｜ 搜索: [过滤节点/实体...] ｜ [全部展开/折叠] ｜ 复制日志│
├────────────────────────────────────────────────────────────────────────┤
│ [泳道时间线甘特图]                                                     │
│ 预处理  █ (15ms)                                                       │
│ 正则    ██ (25ms)                                                      │
│ 模型    ██████████████████████████████████████ (3,120ms - 4 Chunks)   │
│ 消解    █ (8ms)                                                        │
├───────────────────────────────────┬────────────────────────────────────┤
│ [左侧: 阶段流水账 Ledger]         │ [右侧: 详情检查器 Inspector]        │
│                                   │                                    │
│ ├── 阶段 1: 预处理分块 (4块)      │ 【选中的节点】: 模型推理 Chunk #2   │
│ ├── 阶段 2: 正则引擎扫描          │ • 耗时 / Token / 状态              │
│ ├── 阶段 3: 模型推理轮次 (4轮)    │ • 输入 Prompt (带折叠)             │
│ │   ├── ● Chunk #1 (820ms)        │ • 思考链 (Reasoning)               │
│ │   ├── ● Chunk #2 (当前选中)     │ • 原始 Raw Response (JSON)         │
│ │   └── ...                       │ • 提取到的实体清单 (2 项)          │
│ └── 阶段 4: 结果消解与去重        │ [一键复制 Raw JSON] [原文高亮定位] │
└───────────────────────────────────┴────────────────────────────────────┘
```

---

## 6. 实施路线图 (四阶段)

- **阶段一 (后端契约与采集打桩)**：定义 `ExecutionTrajectory`，在 `Extractor` 各流程中收集细粒度耗时、Token 与 Raw Response。
- **阶段二 (SSE 流式推送端点)**：新增 `/api/extract/stream` 路由与分段推送逻辑。
- **阶段三 (前端轨迹组件库与渲染)**：实现纯 CSS+JS 甘特时间线、流水账卡片与检查器抽屉。
- **阶段四 (按钮联动与快照回溯集成)**：打通中间面板顶部按钮状态切换与历史快照回溯验证。
