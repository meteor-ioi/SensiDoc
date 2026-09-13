#!/usr/bin/env python3
"""
todo.md 自动化拆分工具
将庞大的单文件 todo.md 结构化拆分为符合 Doska 规范的独立 Markdown 卡片文件。
零 Token 消耗，毫秒级就绪。
"""

import os
import sys
import re
import argparse
from pathlib import Path
from datetime import datetime

# 中文数字转阿拉伯数字辅助
CN_NUM = {
    '一': 1, '二': 2, '三': 3, '四': 4, '五': 5, '六': 6, '七': 7, '八': 8, '九': 9, '十': 10,
    '十一': 11, '十二': 12, '十三': 13, '十四': 14, '十五': 15, '十六': 16, '十七': 17, '十八': 18, '十九': 19, '二十': 20,
    '二十一': 21, '二十二': 22, '二十三': 23, '二十四': 24, '二十五': 25, '二十六': 26, '二十七': 27, '二十八': 28, '二十九': 29, '三十': 30,
}

def parse_phase_num(text):
    # 匹配 "阶段九十六" 或 "阶段 96" 或 "阶段一"
    m = re.search(r"阶段\s*([0-9一二三四五六七八九十百]+)", text)
    if not m:
        return 999
    val = m.group(1)
    if val.isdigit():
        return int(val)
    # 简易中文解析
    if val in CN_NUM:
        return CN_NUM[val]
    # 处理 "九十六"
    res = 0
    if "百" in val:
        parts = val.split("百")
        res += (CN_NUM.get(parts[0], 1)) * 100
        val = parts[1] if len(parts) > 1 else ""
    if "十" in val:
        parts = val.split("十")
        ten = CN_NUM.get(parts[0], 1) if parts[0] else 1
        res += ten * 10
        val = parts[1] if len(parts) > 1 else ""
    if val:
        res += CN_NUM.get(val, 0)
    return res if res > 0 else 999

def parse_priority(text):
    if "p0" in text.lower() or "高危" in text or "紧急" in text:
        return "high"
    if "p1" in text.lower() or "high" in text.lower():
        return "high"
    if "p2" in text.lower() or "medium" in text.lower() or "中危" in text:
        return "medium"
    if "low" in text.lower() or "低危" in text:
        return "low"
    return "medium"

def clean_title(title):
    # 清理类似 "(已完成)", "(进行中)", "(待办)", "· [🔗 实施计划](...)" 等
    cleaned = re.sub(r"\(已完成\)|\(进行中\)|\(待办[^\)]*\)|\(规划中[^\)]*\)", "", title)
    cleaned = re.sub(r"·\s*\[🔗[^\]]+\]\([^\)]+\)", "", cleaned)
    cleaned = re.sub(r"\(P[0-9/]+\)", "", cleaned)
    cleaned = re.sub(r"^阶段[0-9一二三四五六七八九十百]+[：:]\s*", "", cleaned)
    cleaned = re.sub(r'[\\/*?:"<>|]', "_", cleaned)
    return cleaned.strip()

def split_todo(todo_path: Path, kanban_dir: Path, active_only: bool = False, dry_run: bool = False):
    if not todo_path.exists():
        print(f"❌ 未找到源文件: {todo_path}")
        return

    content = todo_path.read_text(encoding="utf-8")

    # 按照 "## 阶段" 或 "## 模块" 切割
    sections = re.split(r"(?=^##\s+(?:阶段|模块))", content, flags=re.MULTILINE)
    
    phases = []
    for sec in sections:
        sec = sec.strip()
        if not sec.startswith("## "):
            continue

        lines = sec.split("\n")
        raw_title_line = lines[0].replace("## ", "").strip()
        body_lines = lines[1:]

        # 分析子任务
        subtasks = re.findall(r"^-\s*\[([\sXx\-])\]\s*(.+)$", sec, flags=re.MULTILINE)
        total_tasks = len(subtasks)
        done_tasks = sum(1 for status, _ in subtasks if status.lower() in ('x', '-'))

        # 判定状态列
        is_explicit_done = "已完成" in raw_title_line
        is_explicit_in_prog = "进行中" in raw_title_line
        is_explicit_todo = "待办" in raw_title_line or "规划中" in raw_title_line or "backlog" in raw_title_line.lower()

        if is_explicit_done or (total_tasks > 0 and done_tasks == total_tasks):
            target_col = "04_Done"
        elif is_explicit_in_prog or (0 < done_tasks < total_tasks):
            target_col = "03_In_Progress"
        elif is_explicit_todo or "规划" in raw_title_line:
            target_col = "02_To_Do"
        else:
            target_col = "02_To_Do"

        phase_num = parse_phase_num(raw_title_line)
        priority = parse_priority(raw_title_line)

        # 提取关联计划或文件
        plan_links = re.findall(r"\[🔗\s*([^\]]+)\]\(([^)]+)\)", raw_title_line)

        phases.append({
            "num": phase_num,
            "raw_title": raw_title_line,
            "clean_title": clean_title(raw_title_line),
            "target_col": target_col,
            "priority": priority,
            "total_tasks": total_tasks,
            "done_tasks": done_tasks,
            "body": "\n".join(body_lines).strip(),
            "plan_links": plan_links
        })

    print(f"📋 共解析出 {len(phases)} 个开发阶段/任务项")

    # 排序：按阶段序号
    phases.sort(key=lambda x: x["num"])

    # 统计分布
    col_counts = {"01_Backlog": 0, "02_To_Do": 0, "03_In_Progress": 0, "04_Done": 0}
    for p in phases:
        col_counts[p["target_col"]] = col_counts.get(p["target_col"], 0) + 1

    print("📊 任务状态分布:")
    for col, cnt in col_counts.items():
        print(f"  • {col}: {cnt} 项")

    # 过滤策略
    to_export = []
    if active_only:
        print("\n🔍 模式: [活跃任务优先] (仅同步待办、进行中与近期核心阶段)")
        for p in phases:
            # 未完成的必须导出
            if p["target_col"] in ("02_To_Do", "03_In_Progress", "01_Backlog"):
                to_export.append(p)
            elif p["num"] >= 90:  # 最近已完成的大阶段
                to_export.append(p)
    else:
        print("\n🔍 模式: [全量同步] (全量导出所有卡片)")
        to_export = phases

    print(f"✨ 预计生成/同步 {len(to_export)} 张卡片\n")

    if dry_run:
        print("💡 [Dry Run 演练模式] 预览将生成的卡片清单:")
        for item in to_export:
            print(f"  [{item['target_col']}] Prio:{item['priority']} | 阶段{item['num']}: {item['clean_title'][:40]}... ({item['done_tasks']}/{item['total_tasks']})")
        return

    # 创建目标目录
    for col in ["01_Backlog", "02_To_Do", "03_In_Progress", "04_Done", "_files", "_trash"]:
        (kanban_dir / col).mkdir(parents=True, exist_ok=True)

    today = datetime.now().strftime("%Y-%m-%d")
    written = 0

    for item in to_export:
        p_num_str = f"{item['num']:02d}" if item['num'] < 100 else f"{item['num']}"
        safe_name = f"{p_num_str}-{item['clean_title'][:50]}.md"
        card_file = kanban_dir / item["target_col"] / safe_name

        # 提取标签 Tags
        tags = ["phase"]
        if "ocr" in item["clean_title"].lower():
            tags.append("ocr")
        if "ui" in item["clean_title"].lower() or "界面" in item["clean_title"]:
            tags.append("frontend")
        if "rust" in item["clean_title"].lower() or "架构" in item["clean_title"] or "引擎" in item["clean_title"]:
            tags.append("backend")
        if "模型" in item["clean_title"] or "prompt" in item["clean_title"].lower():
            tags.append("ai")

        links_section = ""
        if item["plan_links"]:
            links_section = "\n### 🔗 关联实施计划\n" + "\n".join([f"- [{title}]({link})" for title, link in item["plan_links"]])

        card_content = f"""---
id: phase-{item['num']}
title: {item['clean_title']}
priority: {item['priority']}
tags: [{', '.join(tags)}]
created: {today}
---

# {item['clean_title']}

### 📌 任务概述
{item['raw_title']}

### 📋 子任务清单 (Checklist)
{item['body'] if item['body'] else "- [ ] 核心功能开发与联调"}
{links_section}

### 📝 开发记录与进度
- *{today}*：由 todo.md 自动化同步生成。当前完成度: [{item['done_tasks']}/{item['total_tasks']}]。
"""
        card_file.write_text(card_content, encoding="utf-8")
        written += 1

    print(f"✅ 成功将 {written} 张任务卡片落盘至: {kanban_dir}")

def main():
    parser = argparse.ArgumentParser(description="todo.md 自动化拆分至 Doska 看板")
    parser.add_argument("--todo", default="todo.md", help="源 todo.md 路径")
    parser.add_argument("--kanban", default="kanban", help="目标 kanban/ 文件夹路径")
    parser.add_argument("--all", action="store_true", help="全量同步所有历史阶段 (默认优先同步活跃阶段)")
    parser.add_argument("--dry-run", action="store_true", help="仅预览演练，不物理写盘")

    args = parser.parse_args()

    todo_file = Path(args.todo).resolve()
    kanban_path = Path(args.kanban).resolve()

    split_todo(
        todo_path=todo_file,
        kanban_dir=kanban_path,
        active_only=not args.all,
        dry_run=args.dry_run
    )

if __name__ == "__main__":
    main()
