---
priority: high
tags: [frontend, ui, css, geist]
created: 2026-09-04
---

# M4 - Geist 极简交互三栏工作台与 UI 体系

### 📌 任务目标
遵循 Geist-Neutral 极简现代设计规范，打造高可读性、高自适应度的三栏式敏感资产审计工作台（左侧文档资产树、中间高亮预览、右侧闭环审计结果与规则配置），统合浅色/深色主题，彻底清退原生丑陋弹窗与下拉框。

### 📋 子任务清单 (Checklist)
- [x] 300px 响应式左侧文档侧边栏与微型状态胶囊（`[已审计]` / `[待提取]`）
- [x] 文档排序、关键词过滤与多选格式自适应筛选浮层
- [x] 中间富文本高亮（`mark.sensi-mark`）与多色优先级视觉映射
- [x] 右侧闭环审计控制台（命中卡片高亮、多级过滤、频次统计）
- [x] 全站清退浏览器原生 `alert()` / `prompt()`，统一采用居中模态弹窗
- [x] 全站原生 `<select>` 重构为双层架构自定义伪下拉浮层（Popover Select）
- [x] 全局 UI 缩放滑动条（100%~200%）与深浅主题即时无缝切换

### 📝 开发记录与进度
- *2026-09-04*：完成三栏式基础布局与高亮联动。
- *2026-09-07*：完成全站自定义伪下拉组件重构与即时双向绑定加固。
- *2026-09-08*：消除文档空状态滚动条异常，完成列表与卡片 2px 边框统合。

### 🔗 关联文件 / 依赖
- 前端结构：[`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html)
- 样式设计：[`web/style.css`](file:///Users/icychick/Projects/SensiDoc/web/style.css)
- 交互状态：[`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js)
