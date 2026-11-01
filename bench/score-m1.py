#!/usr/bin/env python3
"""M1 v2: report edit amplification; correctness and design quality require separate evidence.

Usage: score-m1.py [--json] TREE BASELINE [SNAPSHOT ...]
The denominator is the baseline-to-final patch size, not the change in file length.
Historical v1 scores remain historical; this does not retroactively rescore them.
"""
import argparse
import difflib
import json
import pathlib


IGNORED = {".git", ".codegraph", "target", ".warm", "__pycache__"}
ALLOWED = {"src/geometry.rs", "src/lib.rs"}


def sources(root):
    if not root.is_dir():
        raise ValueError(f"snapshot is not a directory: {root}")
    return {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in sorted(root.rglob("*"))
        if path.is_file() and not (set(path.relative_to(root).parts) & IGNORED)
    }


def changes(before, after):
    result = {}
    for rel in sorted(before.keys() | after.keys()):
        if before.get(rel) == after.get(rel):
            continue
        old, new = before.get(rel, b""), after.get(rel, b"")
        # Opcodes count changes without confusing source lines with diff headers.
        matcher = difflib.SequenceMatcher(None, old.splitlines(keepends=True),
                                         new.splitlines(keepends=True), autojunk=False)
        churn = sum((b - a) + (d - c) for tag, a, b, c, d in matcher.get_opcodes()
                    if tag != "equal")
        result[rel] = churn
    return result


def score(roots):
    stages = [sources(root) for root in roots]
    rounds, touched = [], {}
    for index, (before, after) in enumerate(zip(stages, stages[1:]), 1):
        delta = changes(before, after)
        rounds.append({"round": index, "churn": sum(delta.values()), "files": sorted(delta)})
        for rel in delta:
            touched[rel] = touched.get(rel, 0) + 1
    total = sum(item["churn"] for item in rounds)
    final_patch = sum(changes(stages[0], stages[-1]).values())
    collateral = sorted(rel for rel in touched
                        if rel not in ALLOWED and not rel.startswith("tests/"))
    return {
        "metric_version": 2,
        "rounds": rounds,
        "total_churn": total,
        "final_patch_churn": final_patch,
        "edit_amplification": total / final_patch if final_patch else None,
        "repeated_files": {rel: count for rel, count in touched.items() if count >= 2},
        "collateral": collateral,
        "test_changes": sorted(rel for rel in touched if rel.startswith("tests/")),
        "quality_verdict": "not measured: requires regression results and blind review",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("tree", type=pathlib.Path)
    parser.add_argument("baseline", type=pathlib.Path)
    parser.add_argument("snapshots", nargs="*", type=pathlib.Path)
    args = parser.parse_args()
    try:
        result = score([args.baseline, *args.snapshots, args.tree])
    except (ValueError, OSError) as error:
        parser.error(str(error))
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return
    print("  M1 v2 每轮：" + " · ".join(
        f"R{item['round']} churn={item['churn']} 文件={len(item['files'])}"
        for item in result["rounds"]))
    ratio = result["edit_amplification"]
    ratio = "N/A（终态无文本差异）" if ratio is None else f"{ratio:.2f}×"
    print(f"  总 churn={result['total_churn']} · 终态 patch churn={result['final_patch_churn']} · 编辑放大率={ratio}")
    print(f"  跨轮重复碰的文件（≥2 轮）: {result['repeated_files']}")
    print(f"  越界改动: {result['collateral'] or '无'}")
    print(f"  测试改动（需区分验收注入与代理改写）: {result['test_changes'] or '无'}")
    print("  本表不判正确性或代码质量；需另附逐轮回归结果与盲评。")


if __name__ == "__main__":
    main()
