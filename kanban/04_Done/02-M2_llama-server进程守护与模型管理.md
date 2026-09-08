---
priority: high
tags: [backend, model, llama-cpp]
created: 2026-09-02
---

# M2 - llama-server 进程守护与模型管理

### 📌 任务目标
实现本地大语言模型（端侧 GGUF）的自动化生命周期管理，包括从魔搭社区（ModelScope）断点续传流式下载推荐模型、专属冷门端口（18188）进程隔离拉起、子进程 PID 句柄精准管理与防误杀安全释放。

### 📋 子任务清单 (Checklist)
- [x] ModelScope 国内镜像源断点续传下载器与 SSE 实时进度推流
- [x] 下载过程即时「取消」控制与临时 `.part` 缓存自动清理机制
- [x] 推荐模型库梯队搭建：LFM-450M, Qwen2.5-1.5B, Qwen3.5-2B, Tessera-4B
- [x] SensiDoc 专属冷门端口（18188）迁移与多实例端口冲突治理
- [x] 精准 PID 进程树生命周期守护与退出兜底释放（杜绝误杀系统其他实例）
- [x] 跨平台原生文件选择器一键导入本地 GGUF 模型（自动软链接挂载）

### 📝 开发记录与进度
- *2026-09-02*：初步实现 llama-server 后台子进程拉起与探针。
- *2026-09-05*：完成 18188 专属端口迁移与精准 PID 句柄治理。
- *2026-09-07*：完善魔搭取消下载与 Tessera-4B 预设支持。

### 🔗 关联文件 / 依赖
- 模型管理器：[`src/model_manager.rs`](file:///Users/icychick/Projects/SensiDoc/src/model_manager.rs)
- 服务调度：[`src/main.rs`](file:///Users/icychick/Projects/SensiDoc/src/main.rs)
