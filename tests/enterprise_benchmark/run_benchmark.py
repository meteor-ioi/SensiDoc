# -*- coding: utf-8 -*-
"""
SensiDoc 企业信息审计基准评测运行器
对 10 个测试文档执行自动化批量推理、快照落盘与 Precision/Recall/F1 准确率评测
"""

import json
import time
import urllib.request
import urllib.parse
from benchmark_data import TEST_DOCUMENTS
from prompt_templates import PROMPT_VERSIONS

BASE_URL = "http://127.0.0.1:3000"

def http_post(url, data):
    req = urllib.request.Request(
        url,
        data=json.dumps(data).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.loads(resp.read().decode("utf-8"))

def http_get(url):
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.loads(resp.read().decode("utf-8"))

def ensure_server_healthy():
    for _ in range(10):
        try:
            res = http_get(f"{BASE_URL}/api/health")
            if res.get("status") == "ok":
                return True
        except Exception:
            time.sleep(1)
    return False

def start_model(model_filename):
    print(f"\n[模型调度] 正在切换并启动模型: {model_filename} ...")
    res = http_post(f"{BASE_URL}/api/models/start", {"filename": model_filename})
    print(f"[模型调度] 响应: {res.get('message', res)}")
    # 等待端口就绪
    for i in range(15):
        try:
            active = http_get(f"{BASE_URL}/api/models/active")
            if active.get("active_model") == model_filename:
                print(f"[模型调度] 模型 {model_filename} 端口就绪！")
                time.sleep(2)
                return True
        except Exception:
            pass
        time.sleep(1)
    return True

def setup_documents():
    """导入 10 篇测试文档，若已存在则清理后全新录入"""
    print("\n[文档初始化] 正在准备 10 份企业场景测试基准文档...")
    existing = http_get(f"{BASE_URL}/api/documents")
    existing_map = {d["filename"]: d["id"] for d in existing}

    doc_ids = {}
    for doc in TEST_DOCUMENTS:
        filename = doc["filename"]
        # 如果已存在，先删除旧文档以确保快照全新记录
        if filename in existing_map:
            req = urllib.request.Request(f"{BASE_URL}/api/documents/{existing_map[filename]}", method="DELETE")
            try:
                urllib.request.urlopen(req)
            except Exception:
                pass
        
        # 导入新文档 (通过 multipart 转换接口或者直接模拟 upsert)
        boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW"
        body = (
            f"--{boundary}\r\n"
            f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'
            f"Content-Type: text/markdown; charset=utf-8\r\n\r\n"
            f"{doc['markdown']}\r\n"
            f"--{boundary}--\r\n"
        ).encode("utf-8")

        req = urllib.request.Request(
            f"{BASE_URL}/api/convert",
            data=body,
            headers={"Content-Type": f"multipart/form-data; boundary={boundary}"}
        )
        with urllib.request.urlopen(req) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            doc_ids[doc["id"]] = data["doc_id"]
            print(f"  ✓ 已载入文档: {filename} -> ID: {data['doc_id']}")

    return doc_ids

def evaluate_extraction(extracted_items, ground_truth, doc_markdown):
    """
    计算精准率 (Precision)、召回率 (Recall) 和 F1 分数
    ground_truth: {"字段名": ["期望实体1", "期望实体2"]}
    extracted_items: [{"text": "...", "category": "..."}]
    """
    # 提取出的所有合法文本 (去重并剔除空)
    extracted_texts = set()
    for it in extracted_items:
        t = it.get("text", "").strip()
        if t and t in doc_markdown:
            extracted_texts.add(t)

    # 展开全部期望的标准答案实体
    expected_texts = set()
    for cat, truths in ground_truth.items():
        for gt in truths:
            expected_texts.add(gt.strip())

    if not expected_texts:
        return 1.0, 1.0, 1.0, 0, 0, 0

    # 命中判定 (支持包含或子串匹配，如 "￥1,860,000.00" 与 "1,860,000.00")
    tp_set = set()
    for exp in expected_texts:
        for ext in extracted_texts:
            if exp == ext or (len(exp) >= 4 and (exp in ext or ext in exp)):
                tp_set.add(exp)
                break

    tp = len(tp_set)
    fn = len(expected_texts) - tp
    # 误抽判定：extracted 中既不属于 expected 也不与任何 expected 重叠
    fp = 0
    for ext in extracted_texts:
        matched = False
        for exp in expected_texts:
            if exp == ext or (len(exp) >= 4 and (exp in ext or ext in exp)):
                matched = True
                break
        if not matched:
            fp += 1

    precision = tp / (tp + fp) if (tp + fp) > 0 else 0.0
    recall = tp / len(expected_texts) if len(expected_texts) > 0 else 0.0
    f1 = (2 * precision * recall) / (precision + recall) if (precision + recall) > 0 else 0.0

    return precision, recall, f1, tp, fn, fp

def run_suite():
    if not ensure_server_healthy():
        print("[错误] SensiDoc 服务未在 http://127.0.0.1:3000 启动，请先运行服务！")
        return

    doc_ids = setup_documents()
    models = [
        "qwen2.5-1.5b-instruct-q4_k_m.gguf",
        "LFM2.5-VL-450M-Q4_K_M.gguf"
    ]

    all_results = {}

    for model_name in models:
        start_model(model_name)
        all_results[model_name] = {}

        print(f"\n=======================================================")
        print(f" 开始评测模型: {model_name}")
        print(f"=======================================================")

        for prompt_key, prompt_info in PROMPT_VERSIONS.items():
            prompt_name = prompt_info["name"]
            custom_template = prompt_info["template"]
            print(f"\n--- 运行提示词版本: {prompt_name} ---")

            doc_scores = []
            total_time_ms = 0
            total_tp = 0
            total_fn = 0
            total_fp = 0

            for doc in TEST_DOCUMENTS:
                d_id = doc_ids[doc["id"]]
                extract_payload = {
                    "doc_id": d_id,
                    "template_name": f"{prompt_name} 自动化评测",
                    "fields": doc["fields"],
                    "use_ai": True,
                    "custom_prompt": custom_template
                }

                start_t = time.time()
                try:
                    snap = http_post(f"{BASE_URL}/api/extract", extract_payload)
                    cost_ms = int((time.time() - start_t) * 1000)
                    total_time_ms += cost_ms

                    items = snap.get("items", [])
                    p, r, f1, tp, fn, fp = evaluate_extraction(items, doc["ground_truth"], doc["markdown"])
                    total_tp += tp
                    total_fn += fn
                    total_fp += fp

                    doc_scores.append({
                        "doc_filename": doc["filename"],
                        "precision": p,
                        "recall": r,
                        "f1": f1,
                        "tp": tp,
                        "fn": fn,
                        "fp": fp,
                        "extracted_count": len(items),
                        "cost_ms": cost_ms
                    })
                    print(f"  [{doc['filename'][:16]}...] 召回率: {r*100:.1f}%, 准确率: {p*100:.1f}%, F1: {f1*100:.1f}% (命中:{tp}, 漏:{fn}, 耗时:{cost_ms}ms)")
                except Exception as e:
                    print(f"  [提取失败] {doc['filename']}: {e}")

            # 汇总该提示词下的整体宏观表现
            macro_recall = (total_tp / (total_tp + total_fn)) if (total_tp + total_fn) > 0 else 0.0
            macro_precision = (total_tp / (total_tp + total_fp)) if (total_tp + total_fp) > 0 else 0.0
            macro_f1 = (2 * macro_precision * macro_recall) / (macro_precision + macro_recall) if (macro_precision + macro_recall) > 0 else 0.0
            avg_time_ms = total_time_ms / len(TEST_DOCUMENTS) if TEST_DOCUMENTS else 0

            print(f"\n>> 【{prompt_name}】 汇总指标:")
            print(f"   平均查全召回率 (Recall):    {macro_recall*100:.2f}% (总命中 {total_tp} / 期望 {total_tp+total_fn})")
            print(f"   平均精准率 (Precision):    {macro_precision*100:.2f}% (误报数: {total_fp})")
            print(f"   综合 F1 分数:              {macro_f1*100:.2f}%")
            print(f"   单篇平均耗时:              {avg_time_ms:.1f} ms")

            all_results[model_name][prompt_key] = {
                "name": prompt_name,
                "macro_recall": macro_recall,
                "macro_precision": macro_precision,
                "macro_f1": macro_f1,
                "avg_time_ms": avg_time_ms,
                "total_tp": total_tp,
                "total_fn": total_fn,
                "total_fp": total_fp,
                "doc_scores": doc_scores,
                "template": custom_template
            }

    # 保存完整的测试结果 JSON
    result_path = "tests/enterprise_benchmark/benchmark_results.json"
    with open(result_path, "w", encoding="utf-8") as f:
        json.dump(all_results, f, ensure_ascii=False, indent=2)
    print(f"\n[评测完成] 完整数据已落盘至: {result_path}")

if __name__ == "__main__":
    run_suite()
