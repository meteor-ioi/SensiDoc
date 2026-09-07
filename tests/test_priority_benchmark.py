# -*- coding: utf-8 -*-
"""
对比测试：在字段定义中引入“优先级（高/中/低）”后，对离线模型提取效果（Precision / Recall / F1）的影响
"""

import json
import time
import urllib.request
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "enterprise_benchmark"))
from benchmark_data import TEST_DOCUMENTS

LLAMA_URL = "http://127.0.0.1:8081/v1/chat/completions"

# 提示词模板 V4 极简版 (Qwen2.5 默认最佳模板)
PROMPT_TEMPLATE_V4 = """【指令】：从文本中提取所有符合定义的敏感信息，输出纯 JSON 数组。

【字段定义】：
{FIELDS_DEFINITION}

【规则】：
1. 逐行扫描提取所有出现的敏感原词，不要漏掉任何一个。
2. 仅输出 JSON 对象数组：
[
  {"field": "字段名", "text": "原文原词"}
]
无任何多余解释。"""

def format_fields_baseline(fields):
    """原版：不带任何优先级标注"""
    lines = []
    for f in fields:
        if f.get("is_enabled", True):
            lines.append(f"- 字段[{f['name']}]：{f['description']}")
    return "\n".join(lines)

def format_fields_priority_inline(fields):
    """方案 1：行内括号标注 (优先级: 高/中/低)"""
    lines = []
    for f in fields:
        if f.get("is_enabled", True):
            risk = f.get("risk_level", "medium")
            pri = "高 (必须优先穷尽提取)" if risk == "high" else ("中" if risk == "medium" else "低")
            lines.append(f"- 字段[{f['name']}] (优先级: {pri})：{f['description']}")
    return "\n".join(lines)

def format_fields_priority_bracket(fields):
    """方案 2：方括号标注 [高优先级 / 中优先级 / 低优先级]"""
    lines = []
    for f in fields:
        if f.get("is_enabled", True):
            risk = f.get("risk_level", "medium")
            pri = "高优先级" if risk == "high" else ("中优先级" if risk == "medium" else "低优先级")
            lines.append(f"- 字段[{f['name']}] [{pri}]：{f['description']}")
    return "\n".join(lines)

def format_fields_priority_grouped(fields):
    """方案 3：按优先级分组排序"""
    high_fields = [f for f in fields if f.get("is_enabled", True) and f.get("risk_level") == "high"]
    other_fields = [f for f in fields if f.get("is_enabled", True) and f.get("risk_level") != "high"]
    lines = []
    if high_fields:
        lines.append("【高优先级核心字段（必须彻底穷尽提取，严禁遗漏）】：")
        for f in high_fields:
            lines.append(f"- 字段[{f['name']}]：{f['description']}")
    if other_fields:
        lines.append("【一般优先级字段】：")
        for f in other_fields:
            lines.append(f"- 字段[{f['name']}]：{f['description']}")
    return "\n".join(lines)

def query_llm(system_prompt, markdown_text):
    payload = {
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": f"<document>\n{markdown_text}\n</document>"}
        ],
        "temperature": 0.1,
        "max_tokens": 2048,
        "stream": False
    }
    req = urllib.request.Request(
        LLAMA_URL,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    t0 = time.time()
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            content = data["choices"][0]["message"]["content"].strip()
            elapsed_ms = int((time.time() - t0) * 1000)
            
            # 解析 JSON
            start = content.find("[")
            end = content.rfind("]")
            if start != -1 and end != -1 and end > start:
                json_str = content[start:end+1]
                try:
                    items = json.loads(json_str)
                    return items, elapsed_ms, None
                except Exception as e:
                    return [], elapsed_ms, f"JSON parse error: {e}"
            return [], elapsed_ms, "No JSON array found"
    except Exception as e:
        return [], 0, str(e)

def evaluate(extracted_items, ground_truth, fields):
    high_fields = {f["name"] for f in fields if f.get("risk_level") == "high"}

    total_gt = 0
    high_gt = 0
    for field, vals in ground_truth.items():
        total_gt += len(vals)
        if field in high_fields:
            high_gt += len(vals)

    total_pred = len(extracted_items)

    tp = 0
    high_tp = 0
    gt_copy = {k: list(v) for k, v in ground_truth.items()}

    for item in extracted_items:
        cat = item.get("field") or item.get("category") or ""
        txt = (item.get("text") or "").strip()
        
        matched = False
        if cat in gt_copy:
            for g in list(gt_copy[cat]):
                if txt == g or txt in g or g in txt:
                    tp += 1
                    if cat in high_fields:
                        high_tp += 1
                    gt_copy[cat].remove(g)
                    matched = True
                    break
        
        if not matched:
            for other_cat, other_list in gt_copy.items():
                for g in list(other_list):
                    if txt == g and len(txt) >= 4:
                        tp += 1
                        if other_cat in high_fields:
                            high_tp += 1
                        other_list.remove(g)
                        matched = True
                        break
                if matched:
                    break

    fp = total_pred - tp
    fn = total_gt - tp

    precision = (tp / total_pred) if total_pred > 0 else 1.0
    recall = (tp / total_gt) if total_gt > 0 else 0.0
    f1 = (2 * precision * recall / (precision + recall)) if (precision + recall) > 0 else 0.0
    high_recall = (high_tp / high_gt) if high_gt > 0 else 1.0

    return {
        "tp": tp, "fp": fp, "fn": fn,
        "total_gt": total_gt, "total_pred": total_pred,
        "precision": precision, "recall": recall, "f1": f1,
        "high_gt": high_gt, "high_tp": high_tp, "high_recall": high_recall
    }

def run_suite(suite_name, template, formatter):
    print(f"\n=======================================================")
    print(f"🚀 正在测试策略: {suite_name}")
    print(f"=======================================================")

    results = []
    total_time = 0

    for i, doc in enumerate(TEST_DOCUMENTS, 1):
        fields_str = formatter(doc["fields"])
        prompt = template.replace("{FIELDS_DEFINITION}", fields_str)
        
        items, time_ms, err = query_llm(prompt, doc["markdown"])
        total_time += time_ms

        if err:
            print(f"  [{i}/10] ❌ {doc['filename']}: {err} ({time_ms}ms)")
        else:
            metrics = evaluate(items, doc["ground_truth"], doc["fields"])
            results.append(metrics)
            print(f"  [{i}/10] ✓ {doc['filename']}: 提取={len(items)} 期望={metrics['total_gt']} | P={metrics['precision']*100:.1f}% R={metrics['recall']*100:.1f}% F1={metrics['f1']*100:.1f}% [高危R={metrics['high_recall']*100:.1f}%] ({time_ms}ms)")

    avg_p = sum(r["precision"] for r in results) / len(results) if results else 0
    avg_r = sum(r["recall"] for r in results) / len(results) if results else 0
    avg_f1 = sum(r["f1"] for r in results) / len(results) if results else 0
    avg_high_r = sum(r["high_recall"] for r in results) / len(results) if results else 0
    total_tp = sum(r["tp"] for r in results)
    total_fp = sum(r["fp"] for r in results)
    total_fn = sum(r["fn"] for r in results)

    print(f"\n📊 【{suite_name} 汇总统计】:")
    print(f"  - Macro Precision : {avg_p * 100:.2f}%")
    print(f"  - Macro Recall    : {avg_r * 100:.2f}%")
    print(f"  - Macro F1        : {avg_f1 * 100:.2f}%")
    print(f"  - 高优先级召回率  : {avg_high_r * 100:.2f}%")
    print(f"  - TP / FP / FN   : {total_tp} / {total_fp} / {total_fn}")
    print(f"  - 平均单篇耗时    : {total_time // len(TEST_DOCUMENTS)}ms")

    return {
        "name": suite_name,
        "macro_precision": avg_p,
        "macro_recall": avg_r,
        "macro_f1": avg_f1,
        "high_recall": avg_high_r,
        "avg_time_ms": total_time // len(TEST_DOCUMENTS)
    }

def main():
    print("================================================================")
    print(" SensiDoc 优先级 Prompt 效果基准对照实验 (Qwen2.5-1.5B 实测)")
    print("================================================================")

    res_base = run_suite("1. 基准版 (无优先级标注)", PROMPT_TEMPLATE_V4, format_fields_baseline)
    res_inline = run_suite("2. 方案一 (行内标注: 优先级 高/中/低)", PROMPT_TEMPLATE_V4, format_fields_priority_inline)
    res_bracket = run_suite("3. 方案二 (方括号: [高优先级/中优先级])", PROMPT_TEMPLATE_V4, format_fields_priority_bracket)
    res_grouped = run_suite("4. 方案三 (按优先级分组强调)", PROMPT_TEMPLATE_V4, format_fields_priority_grouped)

    print("\n\n" + "="*70)
    print("🏆 【全方案最终综合评测对比表】")
    print("="*70)
    print(f"{'评测方案':<32} | {'精准率 P':<10} | {'召回率 R':<10} | {'综合 F1':<10} | {'高优先级 R':<12}")
    print("-"*70)
    for r in [res_base, res_inline, res_bracket, res_grouped]:
        print(f"{r['name']:<32} | {r['macro_precision']*100:>7.2f}% | {r['macro_recall']*100:>7.2f}% | {r['macro_f1']*100:>7.2f}% | {r['high_recall']*100:>9.2f}%")
    print("="*70)

if __name__ == "__main__":
    main()
