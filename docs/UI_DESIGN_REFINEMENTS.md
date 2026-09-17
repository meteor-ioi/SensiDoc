# SensiDoc 界面视觉风格与微调规范指引 (UI Design System Refinements)

本文档总结了近期对 SensiDoc 前端界面（规则列表、提取结果列表、文档列表等核心视图）进行交互重构与细节微调所沉淀的设计哲学、视觉规范与组件实践，供后续演进全套设计系统（Design System Tokens / DESIGN.md）参考使用。

---

## 一、 核心设计哲学 (Design Philosophy)

1. **纯净一体化卡片 (Unified Single-Surface Cards)**
   - **摒弃双色夹心与生硬硬切**：取消以往卡片内部“上白下灰”的双色分段设计与虚线分割线（Dashed Dividers），避免多张卡片纵向排列时产生视觉条纹噪点（斑马线效应）。
   - **单一容器认知**：遵循“一张卡片即为一个独立纸片实体”的心理隐喻，整张卡片统一使用 `var(--surface)` 纯净底色。

2. **以排版层级替代生硬色块 (Typographic Hierarchy over Background Contrast)**
   - **主次分离**：不再依赖大面积背景色差来区分“主要字段”与“辅助信息”，而是依靠**字阶（13px vs 11.5px / 11px）**、**字重（600 vs 400/500）**、**色彩深浅（`var(--text)` vs `var(--text-mute)`）**以及**合理的负空间（Padding / Indentation）**自然拉开信息层次。

3. **隐式操作与轻量内联交互 (Subtle Hover Actions & Inline Confirmations)**
   - **按需显露（Progressive Disclosure）**：高频查看、低频操作的危险动作（如卡片删除）默认收起或透明隐藏，鼠标 Hover 时才丝滑展开，保持日常界面的最高整洁度。
   - **零弹窗二次确认（Zero-Modal Inline Confirm）**：高危险删除动作摒弃阻断式 Modal 弹窗，就地变为紧凑微型“删除”确认按钮，超时自动归位，保证心流不被打断。

4. **严格一致的垂直呼吸节奏 (Consistent Vertical Rhythm & Spacing)**
   - 杜绝外边距（`margin`）与容器间隙（`gap`）盲目叠加导致间距失控。左右双栏、各列表卡片之间的纵向间距严格遵循同一基准（如 `7px`）。

---

## 二、 核心组件设计规范与参数细节

### 1. 规则卡片与结果卡片 (Rule Card & Audit Card)

#### (1) 背景与容器外框
- **容器背景**：`var(--surface)`（纯白 / 深色模式纯黑灰背景）。
- **边框规范**：`1px solid var(--border)`，圆角 `var(--radius-md)`（8px）。
- **投影体系**：静态微弱投影 `box-shadow: 0 1px 3px rgba(0, 0, 0, 0.03)`；Hover 浮起 `box-shadow: 0 3px 8px rgba(0, 0, 0, 0.06)`，伴随 `translateY(-1px)`。
- **列表间距**：列表容器取消 `gap: 0`，卡片外边距统一为 `margin-bottom: 7px`，保证左右两栏滚动列表间隙完全对齐。

#### (2) 内部主副内容垂直留白节奏 (Vertical Padding Balance)
- **主行信息区**：
  - Padding: `8px 12px 6px 12px`（底部留出 6px 间隙，避免紧贴副信息）。
  - 主文案：字号 `13px`，字重 `600`，字色 `var(--text)`。
- **辅助元信息区**：
  - Padding: `2px 12px 8px 12px`（顶部 2px 与主行留白呼应，底部 8px 形成卡片内边距收尾）。
  - 辅助文案：字号 `11px`，字重 `400`，字色 `var(--text-mute)`。

---

### 2. 内联二次确认删除组件 (Inline Confirm Delete Pattern)

在文档列表项（`.file-item`）与规则卡片（`.rule-card`）中统一落地：

1. **常态（Idle / Hover）**：
   - 默认通过 `opacity: 0` / `width: 0` 收缩隐藏，不侵占卡片常态可用宽度。
   - Hover 时平滑展开为 `20px` 宽度的垃圾桶图标（`var(--text-mute)`，Hover 时变为红色危险色）。
2. **首次触发（First Click）**：
   - 不弹出破坏性 Modal 弹窗。
   - 图标就地过渡为紧凑型状态药丸标签样式的「**删除**」按钮（仿照 `.file-status-tag`）：
     - **字阶字重**：`10px`，`font-weight: 500`，`line-height: 1.2`。
     - **内边距圆角**：`padding: 1px 5px`，`border-radius: 3px`，避免按钮突兀膨胀。
     - **配色方案**：`background: var(--danger-soft)`，文字 `var(--danger)`，边框 `1px solid rgba(220, 38, 38, 0.25)`；Hover 时高亮反白。
3. **确认或超时（Confirm or Timeout）**：
   - 再次点击立即执行删除。
   - 2.5 秒内无二次点击动作，自动平滑恢复为垃圾桶图标状态。

---

### 3. 卡片行内就地编辑与待处理提示 (Inline Creation & Draft State)

用于“添加新规则”操作，彻底取代传统的全屏模态弹窗：

1. **入口迁移**：
   - 从原本在列表顶部霸占全宽的虚线大按钮，迁移收纳至“规则列表”标题右侧工具栏（文案“添加新规则”，配合 `btn sm` 规范与标签库按钮并列对齐）。
2. **就地插入与高亮指引**：
   - 点击后直接在规则列表第 1 行插入空规则卡片，无需弹窗。
   - 焦点自动命中首行字段名输入框。
3. **底部轻量操作栏（Draft Actions）**：
   - 未保存的草稿卡片底部增加轻量操作栏（`.rule-card-draft-actions`），右对齐呈现「取消」与「确认」按钮。
   - 点击「确认」进行空值与同名字段查重校验，校验通过即转为正式规则卡片。
   - 放弃直接点「取消」，移除草稿卡片。

---

## 三、 设计系统迁移建议 (Design System Action Items)

后续在重构全局 `DESIGN.md` 或 UI 样式库时，建议直接吸纳以下 Token 与规则：

- [ ] **增加微型药丸标签尺寸规范 (`tag-xs` / `pill-xs`)**：
  `font-size: 10px; line-height: 1.2; padding: 1px 5px; border-radius: 3px;`。
- [ ] **统一卡片间隙与容器边距 Token**：
  卡片间隙基准设定为 `7px`（`--card-gap: 7px`），容器内统一禁用复合 `gap` + `margin-bottom` 叠加。
- [ ] **沉淀 Inline-Confirm 行为规范**：
  对于删除、清空、重置等低频局部动作，优先推荐“展开微型标签 + 2.5s 自动回弹”的无弹窗方案。
