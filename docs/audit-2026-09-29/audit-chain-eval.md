# 调用链错误发现评测：种子驱动的注入、两个工具的机械判分与修复评分

日期：2026-09-29。被评测对象：`toolchain/src/mcp/**`（二进制 `xirang-mcp`，下称"我们的 MCP"）
与 codegraph 1.5.0（下称 CG）。交付物：本记录，以及 `tools/xirang-chain-eval` 与它调用的五个
`tools/xirang-chain-eval-*.py` 小助手（脚本全文见 §2.6 与 §2.2/§2.3/§2.5 的逐字片段）。

一句话结论：**14 轮种子驱动的构造故障上，"点名病灶"两边都是 12/14；差距不在命中而在链路召回——
最深的一轮我们的 MCP 召回 60/105 条参考边、CG 召回 82/84，而两轮失利（第 11、14 轮）各有具体
原因：第 11 轮是我们自己的源码扫描器对一种 `fn` 声明是盲的，第 14 轮是两边在固定扇出预算下都发散
了，不是能力差距。**

## 0. 结论速览

先给数字（细则见 §5，逐轮见 §4）：

| 分组 | 轮数 | 我们的 MCP 命中 | CG 命中 | MCP 答案字节中位 | CG 答案字节中位 | MCP 耗时中位 | CG 耗时中位 |
|---|---|---|---|---|---|---|---|
| 浅 d=1–2 | 6 | 6/6 | 5/6 | 2149.5 | 1207.5 | 557.9 ms | 988.8 ms |
| 深 d≥3 | 8 | 6/8 | 7/8 | 9838.0 | 8725.0 | 1842.4 ms | 3165.9 ms |
| 全部 | 14 | 12/14 | 12/14 | 4710.0 | 3693.0 | 1186.2 ms | 2527.4 ms |

结论一：**命中率打平，且"打平"有一半是判据的性质造成的。** `chain`/`both` 轮的判据是"断链检出"
（被删的调用点在静态上不可达，见 §2.5），两边都能满足；真正拉开的是**召回**：第 13 轮（d=5）
我们的 MCP 60/105、CG 82/84；第 11 轮 0/1（第一步就失败）对 CG 的 12/12。

结论二：**我们输的两轮，原因各是一个可复现的具体缺陷，不是一个模糊的"深度不够"。**

- 第 11 轮：观察点是一个**只有生命周期参数的泛型声明**（`fn …<'a>(…)`），我们的源码扫描器看不见它，
  `callgraph` 第一步就返回 `no static function match`（整轮答案 37 字节，CG 是 3142 字节、12/12 边）。
  缺陷测量：把树里 35 个泛型声明全部问一遍，我们看不见 1 个，CG 的 tree-sitter 索引看得见它。
- 第 13 轮：45 条漏边里 26 条来自一次**跨 crate** 的查询。最小复现：同一个问题（"谁调用
  `find_registry`"）CG 报 36 个调用者（10 个在 `kernel/`、26 个在 `toolchain/`），我们报 17 个、
  **跨 crate 一个都没有**——`callgraph` 只在**定义所属成员**的源码里找调用者，即使把 `root` 给成
  工作区根也一样（§7 有原文）。这不是驱动程序用错了 `root`：成员作用域正是桥自己的文档化用法。

结论三：**第 14 轮（d=5）两边都未命中，但那一轮不是能力差距**：两边的游走在固定扇出预算下都发散
到了别的分支（我们 40 次调用、CG 31 次），根本没查到那一跳的被调方，召回都是 50/50。记未命中，
并把原因写成"这一轮量的是发散，不是找得到找不到"。

结论四：**误报两边都是 0 条确定误报，但这个结论很弱**：参考边集取自 CG 在变异前副本上的索引
（对 CG 有利），而可疑边大多落进"无法核实"（第 13 轮 CG 53 条、我们 8 条），不是确定误报（§9）。

结论五：**成本上我们的 MCP 便宜**：14 轮中位耗时 1186 ms 对 CG 2527 ms（CG 的 CLI 每次调用要起一个
node 进程，约 150–300 ms 的固定开销）；但字节数是我们多（中位 4710 对 3693）——"我们话多"。

结论六（修复评分，§6）：3 轮（d=1、d=2、d=5）的红灯都稳定复现，**根因修复（注入的精确逆操作）
四项全过**（对 pristine 副本的 diff 归零、只碰注入文件的那一行、无绕过记号），负对照
（给失败测试加 `#[ignore]`）被同一张评分表如实判成"不是绿恢复（测试被忽略而不是通过）+ 绕过"。
另有 2 个候选轮（第 10、7 轮）**没有红灯**，未获评分资格，原样留证。

## 1. 这次评测问的问题

维护者的原话是：「你可以做一个调用链错误发现测试深度上可以深和浅都测试一下，看看 codegraph 和
我们之间的实测差距，而且还可以看判断准确率，你可以实现一个多轮多场景测试，而且还要给修复质量进行
评分和评估，**记住测试不要用题库，避免模型过拟合导致的误报**」。

因此本记录回答四件事：

1. **深度上的差距**：把病灶放在距离观察点 1、2、3、4、5 跳的位置，两个工具各自能找到什么。
2. **判断准确率**：以注入前副本的调用图为参考，量两边每一步报出来的调用者边里对的和错的。
3. **多轮多场景**：14 轮，每轮一个由种子从树里现挑的站点（没有题库、没有手写场景）。
4. **修复质量**：对 3 轮做出红灯、实施修复、按四项打分。

不测的东西也写清楚：**没有用 LLM 当裁判**。判分全是字符串与集合运算（§2.5），因此"病灶定位"只按
"答案正文有没有点名那个文件与那个符号"判，**不判语义解释质量**；`impl`/`chain`/`both` 的层次辨认
只用词频这类弱信号记录，不作判词（原因与代价见 §9）。
## 2. 方法

### 2.1 修订固定：每份副本都来自同一个对象

工作树是共享的，而且在本次评测期间 HEAD 前进了三次（`git log --oneline` 的前三条），工作树里
还出现过一个**未提交、且引用了尚不存在的模块**的改动——我第一次按"从副本构建"的直觉跑
`cargo build` 时，命令的工作目录是工作树，于是构建的是工作树、只是把产物写进副本的 `target/`，
构建在一个任何固定修订里都不存在的文件上失败（原样证据见 §3.4）。

因此本记录的规矩是：

- `prepare` 把 `git rev-parse HEAD` 的**完整 sha** 写进 `$work/pinned-rev`，之后每份副本都由
  `git archive <sha> | tar -x -C <dir>` 解出来；
- 归档流的 sha256 也记下来（`$work/archive.sha256`），因此"各副本是同一棵树"是读者能自己复现的
  事实，而不是一句承诺；
- 每次构建都在**副本里面**执行：`( cd $work/base && CARGO_TARGET_DIR=$work/base/target cargo build … )`，
  构建日志里因此出现的是副本自己的路径。

本次固定的修订、归档 sha256 与工具链版本写在 §3.1。

### 2.2 站点选取：种子 + 调用图，没有题库

`tools/xirang-chain-eval-graph.py` 做全部选取，它里面**没有一个符号名或源文件名**：

1. 读 pristine 副本的 codegraph 索引（SQLite 的 `nodes` / `edges` 两张表），得到"函数 → 被谁调用"
   的边集，只保留**在范围内的成员**里的函数；
2. 范围从清单推导：工作区 `Cargo.toml` 的 `members` 里、自己那份 `Cargo.toml` 没有写
   `publish = false` 的成员（于是 `conventions` 与两个示例宿主自动出局，`kernel`/`macro`/`toolchain`
   自动入选）；再减去**被评测仪器自己进入的模块目录**——那个目录由 `toolchain/Cargo.toml` 里名为
   `xirang-mcp` 的 `[[bin]]` 的 `path` 出发，经 crate 根的 `#[path]` 声明解析出来（本次解析为
   `toolchain/src/mcp/src`）；再减去测试文件（`tests/` 组件、`*_tests.rs`、`tests.rs`）与文件里
   `#[cfg(test)]` 区域之后的定义；
3. 用 `random.Random(seed)` 从"有调用者且名字全树唯一"的函数里挑**观察点**，向上走 `d` 跳挑一条链，
   链上每个节点的名字都要求全树唯一（名字重复会让"点名"变得有歧义，那是另一个题目，不是深度）；
4. 成员按轮次轮转（`members[index % 3]`），使 `kernel`/`macro`/`toolchain` 都被测到；某个成员扛不住
   这个深度时按固定顺序让给下一个；
5. 站点上一次只做**这一层该做的那一种**编辑（§2.3），并当场验证这个编辑在这份源码里存在（不存在的
   候选直接跳过，换下一个）。

一次运行里 14 轮的计划由 `$work/plan.json` 与 `$work/plan.txt` 逐字留档（§4 每轮一节也复述它）。

### 2.3 三种层次与变异算子

| 层次 | 做什么 | 为什么这样定义 | 编译安全性 |
|---|---|---|---|
| `impl` | 在站点函数**体内**换一个比较/布尔运算符（`==`↔`!=`、`<=`↔`<`、`>=`↔`>`、`&&`↔`||`、`true`↔`false`），签名与调用点全不动 | 经典变异测试算子，只动函数体 | 只换记号，类型不变，**必然编译**；另有评测轮实测（§6） |
| `chain` | 删掉站点里**一条整行的调用语句**（`f(args);`、`self.f(args);`），被调方与它的函数体完全不动 | "调用点被漏掉"的最小可构造形式 | 被删的是丢弃返回值的语句；且要求**被调方的名字在该函数体里恰好出现一次**，否则删掉一处调用后边还在，"链路断了"的真值就会与树矛盾 |
| `both` | 同一点上两者都做 | 层次辨认题 | 同上 |

**为什么要求"恰好出现一次"**：第一版没要求，第 2 轮因此出过一次**判分错误**——站点函数里有**两处**
调用同一个被调方，删掉一处后边仍然存在，两个工具照实报了"还有调用者"，而我当时的真值说"边没了"，
于是把两个正确答案都判成"链路保真失败"。发现方式是把该函数体里被调方的出现次数数出来（两处），
修法是把"恰好一次"变成选点的前置条件。这条修正留在代码注释里，也是本记录"不选边"的一部分：
**判错工具比判错自己更该被记录**。

`chain`/`both` 的删除**不留任何痕迹**（不写注释、不留空标记），因为留痕等于把答案写在案发现场。
删除语句只改行数不改语义；`impl` 的重写只换一行里的字符，行号不动，因此两者同时施加时顺序固定为
"先重写、后删除"（回滚时反过来）。

### 2.4 问法：同一句话，同一个搜索过程，两边各自的 API 写法

注入完成后，两边收到的问题完全相同，而且是**程序生成的**：

> 函数 `<观察点>` 的运行结果不正确（这棵树被注入了一处变异）。请从它出发向上追溯调用者，给出你
> 看到的调用者链（每层给文件与符号），并指出你判断的病灶位置。

**问题是脚本生成的**：`$work/out/round-NN/truth.json` 里的 `observable` 就是全部输入，站点、
层次、深度都不进问题。这一点是这套评测能不能算"错误发现"的关键：如果把站点写进问题，命中的
只会是"读题能力"。

驱动过程对两边逐字相同（`tools/xirang-chain-eval-ask.py`，全文见 §2.6）：

| 步 | 我们的 MCP（`root` = 拥有该文件的成员目录，路径为成员内相对路径） | CG（`-p .`，路径为副本内相对路径） |
|---|---|---|
| 1 | `xirang.callgraph {function, path, limit: 50}` | `codegraph callers <symbol> --json -l 50` |
| 2 | 对上一层报出的每个节点重复第 1 步，直到本轮深度 `d` | 同 |
| 3 | 对最深层到达的每个文件：`xirang.inspect {path}` | `codegraph node -f <file> --symbols-only` |

两边**同一个人为上限**：每个节点最多取 10 个调用者（`FANOUT`）、每层最多查 12 个节点
（`PER_LEVEL`）、整轮最多访问 48 个节点与发起 70 次调用（`MAX_NODES`/`MAX_CALLS`）。上限造成的
截断在结论里明说（第 14 轮两边都是在 d=5、40 次调用附近停住的）。

**为什么两边寻址方式不同**：桥的文档化用法就是"传所属成员为 `root`，得到那个包自己的答案"，
CLI 的用法就是以项目路径作答。问法、上限、顺序一致，只有工具自己的 API 拼写不同。

**没有用 CG 的 `explore`/`impact` 参与判分**，因为我们的 MCP 没有对应的函数级工具（`xirang.impact`
是对**注册面**的，不是对函数的）。为了不把"工具形状不同"记成"能力差距"，这两条命令作为**补充探针**
单独记录在 §7，不计入命中率。

### 2.5 机械判分：没有 LLM 裁判

`tools/xirang-chain-eval-judge.py` 算四件事，全部是字符串与集合运算：

1. **命中**。`impl` 轮：答案正文里同时出现被注入文件的路径（副本相对或成员相对，两种拼法都算，因为
   两者是同一个路径在工具自己嘴里的样子）与站点符号名。`chain`/`both` 轮：判据换成"断链检出"
   （见下），因为**被删掉的调用点在静态上不可达**——站点在断口上方，从观察点向上走永远到不了它，
   被删的调用也不留任何提及；能找到的事实是"那一跳的被调方失去了这条边"。这一条的代价写在 §9。
2. **断链检出**（只对 `chain`/`both`）：工具走到那一跳的被调方、并且**没有把被删的那条边报回来**，
   同时答案点名了那个被调方。它与召回列一起读才有意义：一个什么都不解析的工具也会"不报这条边"。
3. **精确率 / 误报**：把每一步报出的调用者集合与**参考边集**比。参考边集 = pristine 副本的
   codegraph 索引（对 CG 有利，已注明），并按构造修正：`chain`/`both` 轮把那一跳的调用者从参考里
   扣掉——如果工具仍把它报回来，那就是**确定的误报**（不需要再核对名字）。其它"多出来的边"要回到
   被报调用者的**文件正文**里复核：文件里根本没有被调方的名字才记误报，否则记"无法核实"
   （名字撞车不算误报）。
4. **层次词汇**：数答案里 `实现/函数体/impl/body` 与 `调用链/链路/调用点/漏掉/chain/link` 两组词的
   出现个数。**只记录，不作判词**——词频不是读者。

另外记 **`走到站点`**（站点有没有作为某一层的节点被游走到）与 **链路保真**（`present`/`absent`/
`unreached`）两列。这两列与"命中"分开列，因为一个"命中"可能来自最深层文件的符号表而不是来自游走，
读者应该看得出来是哪一种。
### 2.6 驱动脚本（逐字）

维护者要求把驱动脚本逐字留在记录里。下面是 `tools/xirang-chain-eval-ask.py` 的全文（判分规则
`tools/xirang-chain-eval-judge.py` 的 `judge_one`、选点 `tools/xirang-chain-eval-graph.py` 的
`pick_round` 与两个变异算子、`tools/xirang-chain-eval-mutate.py` 的 `apply`/`revert` 分别在
§2.5、§2.2、§2.3、§6 里逐字给出；编排脚本 `tools/xirang-chain-eval` 的命令逐条出现在
`$work/run.log` 里，§3.3 摘录）。

```python
#!/usr/bin/env python3
"""Ask both tools the same question about one mutated copy, and record the answers.
在同一份变异副本上向两个工具问同一个问题，并把答案记下来。

The procedure is one fixed search, run identically on both sides — only the tool's
own API spelling differs:

  1. callers of the observable (level 1),
  2. callers of each level's nodes, up to the round's depth,
  3. a symbol map of every distinct file reached at the deepest level,

with the same caps on both sides, and no site knowledge anywhere in it: the walk
starts from the observable alone and follows whatever the tool under test reports.
Every call is timed and its answer measured in bytes; the concatenated text is what
the judge scores, so neither side gets a human to tidy its answer.
这个过程是固定的一次搜索，两边跑法完全一样——只有工具自己的 API 写法不同：

  1. 观察点的调用者（第 1 层），
  2. 每一层节点的调用者，直到本轮深度，
  3. 最深层到达的每个不同文件的一份符号表，

两边上限相同，过程中任何地方都不含站点知识：游走只从观察点出发，沿着**被测工具自己**报出的东西走。
每次调用都计时、答案都记字节数；判分用的就是拼接后的文本，因此没有任何一边能请人替它整理答案。

The two sides are addressed the way each one is addressed in practice: the bridge
gets the owning member as `root` and a member-relative path (that is its documented
way to answer for one package), the CLI gets the copy as its project path with
copy-relative paths. The *question*, the caps and the order are identical.
两边按各自在实践中的用法寻址：桥以所属成员为 `root` 并传成员内相对路径（那是它为单个包作答的
文档化方式），CLI 以副本为项目路径并传副本内相对路径。**问题**、上限与顺序完全一致。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import select
import subprocess
import time

FANOUT = 10
PER_LEVEL = 12
MAX_NODES = 48
MAX_CALLS = 70
CALL_TIMEOUT = 180.0

MCP_CALLER_LINE = re.compile(r"callers \((\d+)\):\s*(.*)$")
MCP_CALLER_REF = re.compile(r"([A-Za-z0-9_./\-]+\.rs)::([A-Za-z0-9_]+)")
MCP_HEADER = re.compile(r"^([\w./\-]+\.rs):(\d+) fn ([A-Za-z0-9_]+)$")


class McpClient:
    """One stdio session with the bridge, with a per-call deadline.
    与桥的一次 stdio 会话，每次调用都有截止时间。"""

    def __init__(self, binary: str, root: str):
        self.process = subprocess.Popen(
            [binary],
            cwd=root,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self.next_id = 1
        self.call(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "xirang-chain-eval", "version": "1"},
            },
        )

    def call(self, method: str, params: dict) -> tuple[dict, float]:
        request = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}
        self.next_id += 1
        started = time.monotonic()
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        ready, _, _ = select.select([self.process.stdout], [], [], CALL_TIMEOUT)
        if not ready:
            self.process.kill()
            raise SystemExit(f"bridge did not answer {method} within {CALL_TIMEOUT}s")
        line = self.process.stdout.readline()
        return json.loads(line), (time.monotonic() - started) * 1000.0

    def tool_call(self, name: str, arguments: dict) -> tuple[str, float]:
        response, elapsed = self.call("tools/call", {"name": name, "arguments": arguments})
        if "error" in response:
            return json.dumps(response["error"], ensure_ascii=False), elapsed
        content = response.get("result", {}).get("content", [])
        return "\n".join(part.get("text", "") for part in content), elapsed

    def close(self) -> None:
        try:
            self.process.stdin.close()
        except Exception:
            pass
        try:
            self.process.wait(timeout=10)
        except Exception:
            self.process.kill()


def terminal(command: list[str], cwd: str) -> tuple[str, int, float]:
    started = time.monotonic()
    completed = subprocess.run(
        command, cwd=cwd, capture_output=True, text=True, timeout=CALL_TIMEOUT
    )
    elapsed = (time.monotonic() - started) * 1000.0
    text = completed.stdout
    if completed.returncode != 0:
        text += f"\n[exit {completed.returncode}]\n" + completed.stderr
    return text, completed.returncode, elapsed


def member_of(file: str, members: list[str]) -> str:
    """The workspace member directory that owns a path.
    拥有某个路径的工作区成员目录。"""
    for member in members:
        if file == member or file.startswith(member + "/"):
            return member
    return members[0]


def parse_mcp_callers(text: str, member: str, focus: str) -> list[dict]:
    """Callers of one definition, from the bridge's own rendering.
    从桥自己的渲染里取出某个定义的调用者。"""
    in_block = False
    for line in text.splitlines():
        stripped = line.strip()
        header = MCP_HEADER.match(stripped)
        if header:
            in_block = header.group(3) == focus
            continue
        if not in_block:
            continue
        match = MCP_CALLER_LINE.search(stripped)
        if match:
            return [
                {"file": f"{member}/{path}", "name": name}
                for path, name in MCP_CALLER_REF.findall(match.group(2))
            ]
    return []


def parse_codegraph_callers(text: str) -> list[dict]:
    """Callers from the CLI's JSON, which is the CLI's own rendering.
    从 CLI 的 JSON（即它自己的渲染）里取调用者。"""
    try:
        payload = json.loads(text)
    except json.JSONDecodeError:
        return []
    callers = payload.get("callers") if isinstance(payload, dict) else None
    if not isinstance(callers, list):
        return []
    return [
        {"file": entry.get("filePath", ""), "name": entry.get("name", "")}
        for entry in callers
        if isinstance(entry, dict)
    ]


def node_key(node: dict) -> tuple[str, str]:
    return (node["file"], node["name"])


def walk(tool: str, truth: dict, copy: str, members: list[str], binary: str | None):
    """Run the fixed search on one side; return the answer and its signals.
    在一边跑那次固定搜索；返回答案与信号。"""
    observable = truth["observable"]
    depth = truth["depth"]
    log: list[dict] = []
    answer_parts: list[str] = []
    queries: dict[str, list[dict]] = {}
    total_ms = 0.0
    calls = 0
    client = McpClient(binary, copy) if tool == "mcp" else None

    def record(query: str, node: dict, text: str, elapsed: float, code: int) -> None:
        nonlocal total_ms, calls
        calls += 1
        total_ms += elapsed
        log.append(
            {
                "seq": calls,
                "query": query,
                "node": node_key(node),
                "elapsed_ms": round(elapsed, 1),
                "bytes": len(text.encode("utf-8")),
                "exit_code": code,
                "text": text,
            }
        )
        answer_parts.append(text)

    def ask_callers(node: dict) -> list[dict]:
        member = member_of(node["file"], members)
        if tool == "mcp":
            relative = node["file"][len(member) + 1 :]
            text, elapsed = client.tool_call(
                "xirang.callgraph",
                {"function": node["name"], "path": relative, "root": member, "limit": 50},
            )
            parsed = parse_mcp_callers(text, member, node["name"])
            code = 0
        else:
            text, code, elapsed = terminal(
                ["codegraph", "callers", node["name"], "-p", ".", "--json", "-l", "50"], copy
            )
            parsed = parse_codegraph_callers(text)
        record("callers", node, text, elapsed, code)
        queries[f"{node['file']}::{node['name']}"] = [
            caller for caller in parsed if node_key(caller) != node_key(node)
        ]
        return parsed

    def ask_symbols(file: str) -> None:
        member = member_of(file, members)
        if tool == "mcp":
            relative = file[len(member) + 1 :]
            text, elapsed = client.tool_call(
                "xirang.inspect", {"path": relative, "root": member}
            )
            code = 0
        else:
            text, code, elapsed = terminal(
                ["codegraph", "node", "-f", file, "-p", ".", "--symbols-only"], copy
            )
        record("symbols", {"file": file, "name": ""}, text, elapsed, code)

    level = [observable]
    visited = {node_key(observable)}
    for _ in range(depth):
        next_level: list[dict] = []
        for node in sorted(level, key=node_key)[:PER_LEVEL]:
            if calls >= MAX_CALLS or len(visited) >= MAX_NODES:
                break
            for caller in sorted(ask_callers(node), key=node_key)[:FANOUT]:
                key = node_key(caller)
                if key in visited:
                    continue
                visited.add(key)
                next_level.append(caller)
        level = next_level
        if not level:
            break
    files = sorted({node["file"] for node in level})[:PER_LEVEL]
    for file in files:
        if calls >= MAX_CALLS:
            break
        ask_symbols(file)
    if client is not None:
        client.close()
    answer = "\n".join(answer_parts)
    return {
        "answer": answer,
        "queries": queries,
        "calls": calls,
        "elapsed_ms": round(total_ms, 1),
        "answer_bytes": len(answer.encode("utf-8")),
        "reached_nodes": sorted(visited),
        "deepest_level_files": files,
        "log": log,
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tool", choices=("mcp", "codegraph"), required=True)
    parser.add_argument("--truth", required=True)
    parser.add_argument("--copy", required=True)
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--members", required=True, help="comma-separated member dirs")
    parser.add_argument("--binary", default=None)
    arguments = parser.parse_args(argv)

    with open(arguments.truth, encoding="utf-8") as handle:
        truth = json.load(handle)
    os.makedirs(arguments.out_dir, exist_ok=True)
    result = walk(
        arguments.tool,
        truth,
        os.path.abspath(arguments.copy),
        arguments.members.split(","),
        arguments.binary,
    )
    prefix = os.path.join(arguments.out_dir, arguments.tool)
    with open(f"{prefix}.answer.txt", "w", encoding="utf-8") as handle:
        handle.write(result.pop("answer"))
    with open(f"{prefix}.calls.jsonl", "w", encoding="utf-8") as handle:
        for entry in result.pop("log"):
            handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
    with open(f"{prefix}.result.json", "w", encoding="utf-8") as handle:
        json.dump(result, handle, ensure_ascii=False, indent=2)
    print(
        f"{arguments.tool}: {result['calls']} calls, {result['answer_bytes']} bytes,"
        f" {result['elapsed_ms']} ms, reached {len(result['reached_nodes'])} nodes"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```
## 3. 固定的修订、环境与命令

### 3.1 事实

| 项 | 值 | 怎么得到的 |
|---|---|---|
| 修订 | `8f8b3b70a032d71879d34b9732c8dbc174145c00`（2026-09-29 20:38:59 +0800） | `git rev-parse HEAD`，写进 `$work/pinned-rev` |
| 归档流 sha256 | `0dd79747d51cb664328b31715771bef6f811423f0e0f5b2df342dd3c31a89527` | `git archive <rev> \| sha256sum` |
| 副本 | `/tmp/xirang-eval/{base,round-01…round-14,scored-*}`，每份都由上面的归档解出；跑 cargo 的副本自带 `CARGO_TARGET_DIR=<副本>/target` | `git archive` + `tar -x` |
| 在范围内的成员 | `kernel`、`macro`、`toolchain`（脚本从清单推导，见 §2.2） | `$work/plan.json` 的 `members_in_scope` |
| 仪器边界 | `toolchain/src/mcp/src`（从 bin target 的 `path` 解析出来） | `$work/plan.json` 的 `instrument_boundary` |
| 种子 | `20260929`；第 i 轮用 `seed + i*7919`，选点失败时按 `+1` 重试 | `$work/plan.json` 每轮的 `seed` |
| 计划 | `1:impl,1:chain,1:both,2:impl,2:chain,2:both,3:impl,3:chain,3:both,4:impl,4:chain,4:both,5:impl,5:chain` | `XIRANG_CHAIN_EVAL_PLAN` 默认值 |
| codegraph | 1.5.0（`~/.local/bin/codegraph`，默认不在 PATH，先 `export PATH="$HOME/.local/bin:$PATH"`） | `codegraph --version` |
| cargo / rustc | 1.96.0 / 1.96.0 | `cargo --version`、`rustc --version` |
| python3 | 3.14.7 | `python3 --version` |
| `sh` | bash 5.3（`tools/xirang-chain-eval` 通过 `sh -n`） | `sh -n tools/xirang-chain-eval`，EXIT 0 |
| 机器 | Linux 7.2.8-xanmod1，20 核 | `uname -sr`、`nproc` |

**仪器只构建一次**：在 pristine 副本 `$work/base` 里、用它自己的 `CARGO_TARGET_DIR`
(`$work/base/target`) 构建 `xirang-mcp`，之后 14 轮复用它。理由是变异的是**被分析的目标树**、
不是仪器本身：如果每轮都在注入后的副本里重建仪器，那些站点落在 `toolchain/` 里的轮次
（第 3、5、6、8、9、11、12、14 轮）就会把仪器自己的源码也一起改掉，那时量到的会包括"仪器被改坏"，
而不是"仪器读一棵被改坏的树"。仪器的构建日志里出现的是副本自己的路径，可核对。

pristine 副本被索引后：**460 个文件 / 5,835 个节点 / 18,719 条边**（`codegraph status`）。
选点只看落在 `kernel`/`macro`/`toolchain` 内、非测试、且在仪器边界外的 **1,627 个函数节点**。

### 3.2 实际执行的命令

编排脚本 `tools/xirang-chain-eval` 的每一步（命令与退出码都逐条进了 `$work/run.log`）：

```sh
# prepare：固定修订、解出 pristine 副本、在副本里构建仪器、索引、选点
git archive 8f8b3b7… | tar -x -C /tmp/xirang-eval/base
( cd /tmp/xirang-eval/base && CARGO_TARGET_DIR=/tmp/xirang-eval/base/target \
    cargo build -p xirang-toolchain --offline --features mcp --bin xirang-mcp )
codegraph init /tmp/xirang-eval/base
python3 tools/xirang-chain-eval-graph.py --root /tmp/xirang-eval/base \
    --out /tmp/xirang-eval/plan.json --seed 20260929 --plan "$PLAN"

# 每一轮（N=1…14）：新副本 → 注入 → 在副本里建 CG 索引 → 两边各问一次 → 判分
git archive 8f8b3b7… | tar -x -C /tmp/xirang-eval/round-NN
python3 tools/xirang-chain-eval-mutate.py --plan …/plan.json --round NN \
    --copy …/round-NN --truth …/out/round-NN/truth.json
( cd /tmp/xirang-eval/round-NN && codegraph init . )        # CG 不跟随副本，必须重建索引
python3 tools/xirang-chain-eval-ask.py --tool mcp --truth … --copy … --out-dir … \
    --members kernel,macro,toolchain --binary …/base/target/debug/xirang-mcp
python3 tools/xirang-chain-eval-ask.py --tool codegraph --truth … --copy … --out-dir … \
    --members kernel,macro,toolchain
python3 tools/xirang-chain-eval-judge.py --mode one --truth … --out-dir … \
    --db …/base/.codegraph/codegraph.db --copy … --members … --out …/verdict.json

# 汇总
python3 tools/xirang-chain-eval-judge.py --mode table \
    --verdicts '/tmp/xirang-eval/out/round-*/verdict.json' --out /tmp/xirang-eval/summary.md
```

所有 cargo 命令都带 `--offline`；**没有任何一步**用 `git stash` / `git checkout` / `git restore` /
`git clean`；没有任何提交；工作树只被读过。

### 3.3 编排日志的原文（节选，含 EXIT）

`$work/run.log` 是每一步命令与其退出码的流水。开头几条（`prepare`）：

```text
pinned revision 8f8b3b70a032d71879d34b9732c8dbc174145c00 (2026-09-29 20:38:59 +0800)
archive sha256 0dd79747d51cb664328b31715771bef6f811423f0e0f5b2df342dd3c31a89527
base copy /tmp/xirang-eval/base
$ build_in_base
   Compiling proc-macro2 v1.0.107
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.47
   Compiling zmij v1.0.23
   Compiling serde_core v1.0.229
   Compiling equivalent v1.0.2
   Compiling serde_json v1.0.151
   Compiling once_cell v1.21.4
   Compiling hashbrown v0.17.1
   Compiling foldhash v0.1.5
   Compiling memchr v2.8.3
   Compiling itoa v1.0.18
   Compiling pin-project-lite v0.2.17
   Compiling fixedbitset v0.5.7
   Compiling inventory v0.3.24
   Compiling hashbrown v0.15.5
   Compiling tracing-core v0.1.36
   Compiling indexmap v2.14.2
   Compiling syn v2.0.119
   Compiling petgraph v0.8.3
   Compiling xirang-kernel v0.2.0 (/tmp/xirang-eval/base/kernel)
   Compiling tracing-attributes v0.1.31
   Compiling tracing v0.1.44
   Compiling xirang-macro v0.2.0 (/tmp/xirang-eval/base/macro)
   Compiling xirang-toolchain v0.2.0 (/tmp/xirang-eval/base/toolchain)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 39.75s
EXIT 0
instrument ```

### 3.4 一次事故：构建跑在工作树上（已修，证据留档）

第一次跑 `prepare` 时，`cargo build` 的工作目录还是工作树（脚本没有 `cd` 进副本），于是构建的是工作树、
只是把产物写进副本的 `target/`。当时工作树里有一个**未提交**、且引用了一个尚不存在模块的改动，
构建因此失败，报错指向一个任何固定修订里都不存在的文件：

```text
123 |         let line = crate::mcp::freshness::line(&self.root, &self.out);
    |                                ^^^^^^^^^ could not find `freshness` in `mcp`
error: could not compile `xirang-toolchain` (lib) due to 2 previous errors
EXIT 101
```

修法是把构建包进子 shell：`( cd $work/base && CARGO_TARGET_DIR=… cargo build … )`，修好之后
`$work/run.log` 里出现的是副本自己的路径（`Compiling xirang-toolchain v0.2.0 (/tmp/xirang-eval/base/toolchain)`），
这也成了"构建确实发生在副本里"的可核证据。这次事故同时说明为什么本记录把修订固定下来：
共享工作树会在评测进行中改变它自己。
### 3.5 单轮完整流程实测（原样输出与 EXIT）

编排脚本被单独跑了一遍完整流程（`prepare` + 3 轮 + 汇总表），工作目录是
`XIRANG_CHAIN_EVAL_WORK=/tmp/xirang-eval/demo`，计划 `1:impl,2:chain,4:impl`，退出码在最后一行为
**0**。下面是它的原样记录（命令、单行结果与每一步的 EXIT，节选到第 2 轮开始处）：

```text
pinned revision 8f8b3b70a032d71879d34b9732c8dbc174145c00 (2026-09-29 20:38:59 +0800)
archive sha256 0dd79747d51cb664328b31715771bef6f811423f0e0f5b2df342dd3c31a89527
base copy /tmp/xirang-eval/demo/base
$ build_in_base
EXIT 0
instrument /tmp/xirang-eval/demo/base/target/debug/xirang-mcp
$ codegraph init /tmp/xirang-eval/demo/base
EXIT 0
$ python3 /home/nich/Moirai_N3/nichlink/tools/xirang-chain-eval-graph.py --root /tmp/xirang-eval/demo/base --out /tmp/xirang-eval/demo/plan.json --seed 20260929 --plan 1:impl,2:chain,4:impl
plan: 3 rounds, members ['kernel', 'macro', 'toolchain'], boundary toolchain/src/mcp/src, nodes 1627
  round  1 d=1 impl  seed=20260929 site=kernel/src/registry_core/source/calls.rs::direct_calls
  round  2 d=2 chain seed=20268848 site=toolchain/src/studio/studio/ui/search.rs::draw_search
  round  3 d=4 impl  seed=20276767 site=toolchain/src/studio/studio/app/pointer.rs::handle_mouse
EXIT 0
round  1 d=1 impl  seed=20260929 observable=kernel/src/registry_core/source/source.rs::mask_non_code site=kernel/src/registry_core/source/calls.rs::direct_calls
round  2 d=2 chain seed=20268848 observable=toolchain/src/studio/studio/ui/graph.rs::search_center_ref site=toolchain/src/studio/studio/ui/search.rs::draw_search
round  3 d=4 impl  seed=20276767 observable=toolchain/src/studio/studio/app/lifecycle.rs::note site=toolchain/src/studio/studio/app/pointer.rs::handle_mouse
$ python3 /home/nich/Moirai_N3/nichlink/tools/xirang-chain-eval-mutate.py --plan /tmp/xirang-eval/demo/plan.json --round 1 --copy /tmp/xirang-eval/demo/round-01 --truth /tmp/xirang-eval/demo/out/round-01/truth.json
round 1: d=1 impl site=kernel/src/registry_core/source/calls.rs::direct_calls | kernel/src/registry_core/source/calls.rs:171 '||'->'&&'
EXIT 0
$ python3 /home/nich/Moirai_N3/nichlink/tools/xirang-chain-eval-ask.py --tool mcp --truth /tmp/xirang-eval/demo/out/round-01/truth.json --copy /tmp/xirang-eval/demo/round-01 --out-dir /tmp/xirang-eval/demo/out/round-01 --members kernel,macro,toolchain --binary /tmp/xirang-eval/demo/base/target/debug/xirang-mcp
mcp: 4 calls, 3281 bytes, 286.8 ms, reached 7 nodes
EXIT 0
$ python3 /home/nich/Moirai_N3/nichlink/tools/xirang-chain-eval-ask.py --tool codegraph --truth /tmp/xirang-eval/demo/out/round-01/truth.json --copy /tmp/xirang-eval/demo/round-01 --out-dir /tmp/xirang-eval/demo/out/round-01 --members kernel,macro,toolchain
codegraph: 4 calls, 3731 bytes, 1045.1 ms, reached 7 nodes
EXIT 0
$ python3 /home/nich/Moirai_N3/nichlink/tools/xirang-chain-eval-judge.py --mode one --truth /tmp/xirang-eval/demo/out/round-01/truth.json --out-dir /tmp/xirang-eval/demo/out/round-01 --db /tmp/xirang-eval/demo/base/.codegraph/codegraph.db --copy /tmp/xirang-eval/demo/round-01 --members kernel,macro,toolchain --out /tmp/xirang-eval/demo/out/round-01/verdict.json
round 1: mcp=hit(present) codegraph=hit(present)
EXIT 0
$ python3 /home/nich/Moirai_N3/nichlink/tools/xirang-chain-eval-mutate.py --plan /tmp/xirang-eval/demo/plan.json --round 2 --copy /tmp/xirang-eval/demo/round-02 --truth /tmp/xirang-eval/demo/out/round-02/truth.json

…[此处截断：完整流水见 /tmp/xirang-eval/demo.log，下面是这次运行的汇总表与总退出码]
verdicts: /tmp/xirang-eval/demo/verdicts.json
DEMO_EXIT=0
```

这一次运行的汇总表（三行分别是「浅层 2 轮」「深层 1 轮」「全部」，与 §5.3 同一套列）：

| 分组 | 轮数 | MCP 命中 | CG 命中 | MCP 字节中位 | CG 字节中位 | MCP ms 中位 | CG ms 中位 | MCP 误报边 | CG 误报边 |
|---|---|---|---|---|---|---|---|---|---|
| 浅 d=1–2 | 2 | 2/2 | 2/2 | 2029.5 | 2000.5 | 523.6 | 778.7 | 0 | 0 |
| 深 d≥3 | 1 | 1/1 | 1/1 | 22255 | 20406 | 10793.5 | 7866.7 | 0 | 0 |
| 全部 | 3 | 3/3 | 3/3 | 3281 | 3731 | 760.4 | 1045.1 | 0 | 0 |

说明：3 轮全部命中是**小样本**（3 轮都是 `impl` 或浅层 `chain`，没有第 11 轮那种扫描器盲点），
它证明的是"整条流水线能一次跑完并给出判分"，不是命中率——命中率看 §5 的 14 轮。
## 4. 逐轮记录（种子、站点、深度、层次、两边结果、原始片段、推理链）

下面 14 节是按轮次从产物直接生成的（`xirang-chain-eval-judge.py --mode fragments`），
每节里的原始片段都截断并标出截断处（`…[此处截断：全文 N 行，此处 M 行]`），推理链是本节作者写的。
站点与观察点的路径是**副本内相对路径**（`kernel/…`、`macro/…`、`toolchain/…`），工具自己回答时用的是
成员内相对路径（`src/…`）或副本内相对路径，两者都在判分时被认作同一个文件。每节的表里，
`点名了站点文件/符号` 与 `点名了断链节点文件/符号` 两列就是"点名了什么"的机械读数。

本文档自己的散文不写 `文件:行号` 锚点（`doc_anchors` 门禁会查活性）；下面 24 处形如
`src/…rs:271` 的文本只出现在**逐字引用工具原始输出**的代码块里，是工具自己打印的路径（成员内相对
路径 + 行号），不是本文档给出的锚点；`docs/audit*` 下的记录对那道门禁本身也是豁免的。

#### 第 1 轮：d=1，层次 `impl`，种子 20260929

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `kernel/src/registry_core/source/source.rs::mask_non_code` |
| 注入站点 | `kernel/src/registry_core/source/calls.rs::direct_calls`（第 171 行） |
| 链（自观察点向上） | `mask_non_code` → `direct_calls` |
| 变异 | 函数体重写：第 171 行 `||`→`&&` |
| 命中判据 | `site` |
| mcp | 命中；字节 3281；297.4 ms；调用 4；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 6/0；漏边 0 |
| codegraph | 命中；字节 3731；1264.4 ms；调用 4；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 6/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：两边都命中（`impl` 用 `site` 判据）：问题只给了观察点 `mask_non_code`，两边第一步就报出 6 个调用者，站点 `direct_calls` 在其中；`link=present` 与「函数体被改、调用点未动」的构造一致；参考边 6/6 全召回。这是浅层、直系的基线轮——它证明协议本身能跑通，不证明任何一边更强。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 3281 字节）：

```text
evidence: static-heuristic
matches 1
src/registry_core/source/source.rs:271 fn mask_non_code
  callers (6): src/registry_core/source/calls.rs::body_calls, src/registry_core/source/calls.rs::direct_calls, src/registry_core/source/items.rs::item_symbols, src/registry_core/source/source.rs::func …[行内截断]
  callees: mask
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

file src/registry_core/source/calls.rs
fn body_calls lines 27-137 calls=[Some, as_bytes, chars, copied, get, is_ascii_whitespace, is_empty, is_ident_continue, is_ident_start, len, len_utf8, mask_non_code, next, saturating_sub]
fn direct_calls lines 145-183 calls=[Some, as_bytes, chars, collect, get, insert, into_iter, is_ascii_whitespace, is_ident_continue, is_ident_start, len, len_utf8, mask_non_code, new, next, to_owned]
fn call_scanner_ignores_use_and_macro_but_accepts_qualified_calls lines 190-204 calls=[body_calls]
fn direct_calls_are_sorted_distinct_and_exclude_the_function_itself lines 207-215 calls=[direct_calls, new]
…[此处截断：全文 33 行，此处 12 行]
```

`codegraph.answer.txt`（共 3731 字节）：

```text
{
  "symbol": "mask_non_code",
  "callers": [
    {
      "name": "body_calls",
      "kind": "function",
      "filePath": "kernel/src/registry_core/source/calls.rs",
      "startLine": 27
    },
    {
      "name": "direct_calls",
      "kind": "function",
…[此处截断：全文 83 行，此处 12 行]
```

#### 第 2 轮：d=1，层次 `chain`，种子 20268849

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `macro/src/mirror.rs::punct` |
| 注入站点 | `macro/src/mirror.rs::loop_value`（第 219 行） |
| 链（自观察点向上） | `punct` → `loop_value` |
| 变异 | 调用点删除：第 219 行 `tokens.extend([punct(':', Spacing::Alone)]);` |
| 命中判据 | `broken-link` |
| mcp | 命中；字节 3457；210.7 ms；调用 4；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 3/0；漏边 0 |
| codegraph | 命中；字节 3655；1509.4 ms；调用 4；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 3/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：`chain` 层：删掉了 `loop_value` 上方的一处调用。两边都按「断链检出」命中——都报出了被调方、都没有把被删的那条边报回来（`link=absent`），而且**都没有点名站点**（`named_site_symbol` 两边都是 0）。后者正是「被删的调用点在静态上不可达」的直接证据（§2.5）。注意 d=1 时被调方就是问题里的观察点，所以这一轮的「点名」是平凡的，不能当作能力证据。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 3457 字节）：

```text
evidence: static-heuristic
matches 1
src/mirror.rs:253 fn punct
  callers (3): src/front_end.rs::error_at, src/lib.rs::normalise, src/mirror.rs::mirror_item
  callees: Punct, new
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

file src/front_end.rs
fn splice lines 29-47 calls=[Group, Ident, clone, collect, delimiter, into_iter, map, new, set_span, span, stream]
fn split_semicolons lines 57-74 calls=[Punct, as_char, expect, extend, last_mut, len, new, push]
fn split_mirror_fields lines 78-108 calls=[Err, Ident, Ok, Some, any, as_str, cloned, collect, contains, error_at, first, get, into_iter, iter, map_or_else, new, push, span, split_face_fields, to_owne …[行内截断]
fn render lines 112-118 calls=[iter, join, map]
…[此处截断：全文 39 行，此处 12 行]
```

`codegraph.answer.txt`（共 3655 字节）：

```text
{
  "symbol": "punct",
  "callers": [
    {
      "name": "error_at",
      "kind": "function",
      "filePath": "macro/src/front_end.rs",
      "startLine": 122
    },
    {
      "name": "normalise",
      "kind": "function",
…[此处截断：全文 88 行，此处 12 行]
```

#### 第 3 轮：d=1，层次 `both`，种子 20276767

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `toolchain/src/studio/studio/app/keyboard.rs::declaration_contract_fields` |
| 注入站点 | `toolchain/src/studio/studio/app/keyboard.rs::open_edit_form`（第 118 行） |
| 链（自观察点向上） | `declaration_contract_fields` → `open_edit_form` |
| 变异 | 函数体重写：第 124 行 `||`→`&&`；调用点删除：第 118 行 `declaration_contract_fields(&source_path_for(&info.source.file));` |
| 命中判据 | `broken-link` |
| mcp | 命中；字节 2194；558.8 ms；调用 2；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/0；点名了断链节点文件/符号 1/1；TP/FP 1/0；漏边 0 |
| codegraph | 未命中；字节 1347；844.9 ms；调用 2；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 0/0；点名了断链节点文件/符号 0/1；TP/FP 1/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：我们命中、CG 未命中，但**这不等于 CG 答错**：CG 的 `link=absent` 说明它同样检出了断链；判它未命中是因为它的答案正文里没有出现那个被调方节点的**文件路径**——CG 的 `callers --json` 在调用者列表为空时不回被查节点自己的文件，而我们的 `callgraph` 会打一行定义头（路径、行号、`fn` 名字）。判据是机械的（必须点名文件），所以这一格记未命中，但原因必须一起读，否则会把它误读成一次失败。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 2194 字节）：

```text
evidence: static-heuristic
matches 1
src/studio/src/studio/app/keyboard.rs:215 fn declaration_contract_fields
  callers (1): src/studio/src/studio/app/tests/forms.rs::an_unreadable_source_is_loud_where_an_empty_one_is_quiet
  callees: Err, Ok, Some, clone, declaration_contract_paths, display, let, read_to_string
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

file src/studio/src/studio/app/tests/forms.rs
fn add_overlay_click_selects_a_field_and_toggles_the_checkbox lines 12-26 calls=[Add, Some, handle_overlay_click, load_app, new, selected_parent]
fn add_overlay_keyboard_navigation_reaches_kind_field lines 29-42 calls=[Add, Some, from, handle_overlay_key, load_app, new, selected_parent]
fn add_navigation_walks_one_fixed_field_order lines 45-67 calls=[Add, Some, as_ref, face_field_indices, from, handle_overlay_key, len, load_app, new, push, selected_parent]
fn add_form_starts_with_editable_bilingual_summary lines 70-77 calls=[is_empty, new]
…[此处截断：全文 19 行，此处 12 行]
```

`codegraph.answer.txt`（共 1347 字节）：

```text
{
  "symbol": "declaration_contract_fields",
  "callers": [
    {
      "name": "an_unreadable_source_is_loud_where_an_empty_one_is_quiet",
      "kind": "function",
      "filePath": "toolchain/src/studio/studio/app/tests/forms.rs",
      "startLine": 132
    }
  ]
}

…[此处截断：全文 28 行，此处 12 行]
```

#### 第 4 轮：d=2，层次 `impl`，种子 20284686

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `kernel/src/registry_core/source/items.rs::keyword_rest` |
| 注入站点 | `kernel/src/registry_core/source/items.rs::item_symbols`（第 68 行） |
| 链（自观察点向上） | `keyword_rest` → `declaration_on_line` → `item_symbols` |
| 变异 | 函数体重写：第 68 行 `true`→`false` |
| 命中判据 | `site` |
| mcp | 命中；字节 1214；557.0 ms；调用 3；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 2/0；漏边 0 |
| codegraph | 命中；字节 1068；1065.9 ms；调用 3；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 2/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：d=2 的直系链，`impl` 层：两边都命中、都走到站点、`link=present`、参考边 2/2。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 1214 字节）：

```text
evidence: static-heuristic
matches 1
src/registry_core/source/items.rs:138 fn keyword_rest
  callers (1): src/registry_core/source/items.rs::declaration_on_line
  callees: Some, starts_with, strip_prefix
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/registry_core/source/items.rs:106 fn declaration_on_line
  callers (1): src/registry_core/source/items.rs::item_symbols
  callees: Some, as_str, contains, identifier, keyword_rest, let, split_once, strip_prefix, trim_start
…[此处截断：全文 19 行，此处 12 行]
```

`codegraph.answer.txt`（共 1068 字节）：

```text
{
  "symbol": "keyword_rest",
  "callers": [
    {
      "name": "declaration_on_line",
      "kind": "function",
      "filePath": "kernel/src/registry_core/source/items.rs",
      "startLine": 106
    }
  ]
}

…[此处截断：全文 35 行，此处 12 行]
```

#### 第 5 轮：d=2，层次 `chain`，种子 20292605

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `toolchain/src/studio/studio/ui/ui.rs::detail_field` |
| 注入站点 | `toolchain/src/studio/studio/ui/panels.rs::draw_workspace`（第 68 行） |
| 链（自观察点向上） | `detail_field` → `draw_details` → `draw_workspace` |
| 变异 | 调用点删除：第 68 行 `draw_details(frame, columns[1], app);` |
| 命中判据 | `broken-link` |
| mcp | 命中；字节 2105；1060.0 ms；调用 3；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/0；点名了断链节点文件/符号 1/1；TP/FP 1/0；漏边 0 |
| codegraph | 命中；字节 257；690.2 ms；调用 2；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/0；点名了断链节点文件/符号 1/1；TP/FP 1/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：两边都按断链检出命中。本轮字节数比例最悬殊：CG 只答 257 字节（JSON 在调用者很少或为空时极短），我们 2105 字节（定义头、说明行与符号表）。同一件事，两种答案形状。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 2105 字节）：

```text
evidence: static-heuristic
matches 1
src/studio/src/studio/ui/ui.rs:127 fn detail_field
  callers (1): src/studio/src/studio/ui/panels.rs::draw_details
  callees: chars, count, default, fg, field, from, styled, to_owned
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/studio/src/studio/ui/panels.rs:122 fn draw_details
  callers (1): src/studio/src/studio/app/tests/b4_studio.rs::the_inspector_draws_every_row_it_lists
  callees: Some, add_modifier, bg, block, default, detail_field, detail_rows, enumerate, fg, first_mut, flat_map, insert, into_iter, len, min, new, panel, render_widget, saturating_sub, styled, wrap
…[此处截断：全文 24 行，此处 12 行]
```

`codegraph.answer.txt`（共 257 字节）：

```text
{
  "symbol": "detail_field",
  "callers": [
    {
      "name": "draw_details",
      "kind": "function",
      "filePath": "toolchain/src/studio/studio/ui/panels.rs",
      "startLine": 122
    }
  ]
}

…[此处截断：全文 16 行，此处 12 行]
```

#### 第 6 轮：d=2，层次 `both`，种子 20300524

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `toolchain/src/studio/studio/ui/graph/nodes.rs::incoming_off_axis` |
| 注入站点 | `toolchain/src/studio/studio/ui/graph/nodes.rs::draw_call_tree`（第 175 行） |
| 链（自观察点向上） | `incoming_off_axis` → `draw_status` → `draw_call_tree` |
| 变异 | 函数体重写：第 143 行 `!=`→`==`；调用点删除：第 175 行 `draw_status(frame, inner, app, &view, cursor);` |
| 命中判据 | `broken-link` |
| mcp | 命中；字节 638；921.4 ms；调用 2；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/0；点名了断链节点文件/符号 1/1；TP/FP 1/0；漏边 0 |
| codegraph | 命中；字节 265；911.7 ms；调用 2；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/0；点名了断链节点文件/符号 1/1；TP/FP 1/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：两边都命中（断链检出），各 2 次调用、几百字节。站点在该层没有被游走到（`走到站点=✗`），命中的是那一跳的被调方被点名——这正是 `chain` 判据的设计。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 638 字节）：

```text
evidence: static-heuristic
matches 1
src/studio/src/studio/ui/graph/nodes.rs:237 fn incoming_off_axis
  callers (1): src/studio/src/studio/ui/graph/nodes.rs::draw_status
  callees: any, iter
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/studio/src/studio/ui/graph/nodes.rs:205 fn draw_status
  callers (0): -
  callees: Some, bottom, call_tree_transforms, default, fg, first, incoming_off_axis, item, push_str, put_line, saturating_sub, signature_of
…[此处截断：全文 13 行，此处 12 行]
```

`codegraph.answer.txt`（共 265 字节）：

```text
{
  "symbol": "incoming_off_axis",
  "callers": [
    {
      "name": "draw_status",
      "kind": "function",
      "filePath": "toolchain/src/studio/studio/ui/graph/nodes.rs",
      "startLine": 205
    }
  ]
}

…[此处截断：全文 16 行，此处 12 行]
```

#### 第 7 轮：d=3，层次 `impl`，种子 20308443

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `kernel/src/registry_core/declaration/declaration.rs::provided_contains` |
| 注入站点 | `kernel/src/registry_core/tree/graft_ops/overlay.rs::apply_overlay_face`（第 196 行） |
| 链（自观察点向上） | `provided_contains` → `validate_registration_requirements` → `validate_owned_registration_rule` → `apply_overlay_face` |
| 变异 | 函数体重写：第 196 行 `||`→`&&` |
| 命中判据 | `site` |
| mcp | 命中；字节 10192；1691.1 ms；调用 11；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 13/0；漏边 0 |
| codegraph | 命中；字节 10026；3294.6 ms；调用 11；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 13/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：d=3 `impl`：两边都命中、都走到站点、参考边 13/13，各有 2 条「无法核实」边（同名撞车，不是误报）。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 10192 字节）：

```text
evidence: static-heuristic
matches 1
src/registry_core/declaration/declaration.rs:50 fn provided_contains
  callers (2): src/registry_core/declaration/declaration.rs::validate_object_contract, src/registry_core/declaration/declaration.rs::validate_registration_requirements
  callees: any, as_ref, iter
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/registry_core/declaration/declaration.rs:156 fn validate_object_contract
  callers (2): src/registry_core/declaration/declaration.rs::object_contract_twins_report_identical_failures, src/registry_core/declaration/owned.rs::validate_owned_object_contract
  callees: as_ref, new, provided_contains, push
…[此处截断：全文 101 行，此处 12 行]
```

`codegraph.answer.txt`（共 10026 字节）：

```text
{
  "symbol": "provided_contains",
  "callers": [
    {
      "name": "validate_registration_requirements",
      "kind": "function",
      "filePath": "kernel/src/registry_core/declaration/declaration.rs",
      "startLine": 97
    },
    {
      "name": "validate_object_contract",
      "kind": "function",
…[此处截断：全文 212 行，此处 12 行]
```

#### 第 8 轮：d=3，层次 `chain`，种子 20316362

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `toolchain/src/studio/studio/app/pointer.rs::forward_mouse_to_flow` |
| 注入站点 | `toolchain/src/studio/studio/studio.rs::run_loop`（第 107 行） |
| 链（自观察点向上） | `forward_mouse_to_flow` → `handle_mouse` → `handle_app` → `run_loop` |
| 变异 | 调用点删除：第 107 行 `app.handle_app(event::read()?);` |
| 命中判据 | `broken-link` |
| mcp | 命中；字节 5963；3681.5 ms；调用 9；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 0/0；点名了断链节点文件/符号 1/1；TP/FP 9/0；漏边 0 |
| codegraph | 命中；字节 3325；3107.7 ms；调用 9；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 0/0；点名了断链节点文件/符号 1/1；TP/FP 9/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：d=3 `chain`：两边都命中（断链检出），9/9 边；两边都没有点名站点（与第 2 轮同理，符合预期）。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 5963 字节）：

```text
evidence: static-heuristic
matches 1
src/studio/src/studio/app/pointer.rs:138 fn forward_mouse_to_flow
  callers (2): src/studio/src/studio/app/pointer.rs::handle_mouse, src/studio/src/studio/app/pointer.rs::handle_overlay_click
  callees: Search, Some, as_mut, as_ref, contains, filter_map, handle_mouse_event, into, into_events, next_back, ok
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/studio/src/studio/app/pointer.rs:7 fn handle_mouse
  callers (3): src/studio/src/studio/app/interaction.rs::handle_app, src/studio/src/studio/app/tests/call_tree.rs::a_click_lands_on_the_node_under_the_pointer, src/studio/src/studio/app/tests/navigati …[行内截断]
  callees: Down, Drag, Search, Some, Up, as_mut, cfg, contains, detail_field_count, forward_mouse_to_flow, from, get, handle_overlay_click, handle_overlay_key, into, is_some, min, move_selection, near …[行内截断]
…[此处截断：全文 68 行，此处 12 行]
```

`codegraph.answer.txt`（共 3325 字节）：

```text
{
  "symbol": "forward_mouse_to_flow",
  "callers": [
    {
      "name": "handle_mouse",
      "kind": "method",
      "filePath": "toolchain/src/studio/studio/app/pointer.rs",
      "startLine": 7
    },
    {
      "name": "handle_overlay_click",
      "kind": "method",
…[此处截断：全文 114 行，此处 12 行]
```

#### 第 9 轮：d=3，层次 `both`，种子 20324281

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `toolchain/src/studio/studio/app/editor_launch.rs::open_editor_at` |
| 注入站点 | `toolchain/src/studio/studio/app/pointer.rs::handle_overlay_click`（第 183 行） |
| 链（自观察点向上） | `open_editor_at` → `handle_search_overlay_key` → `handle_overlay_key` → `handle_overlay_click` |
| 变异 | 函数体重写：第 302 行 `true`→`false`；调用点删除：第 183 行 `self.handle_overlay_key(KeyEvent::from(KeyCode::Enter));` |
| 命中判据 | `broken-link` |
| mcp | 命中；字节 10742；1596.8 ms；调用 7；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 22/0；漏边 1 |
| codegraph | 命中；字节 9251；2178.4 ms；调用 7；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 23/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：d=3 `both`：两边都命中。我们漏 1 条边、CG 0 条。原因是那个被调方有 21 个调用者，而**我们的 `callgraph` 有 20 条调用者的硬上限**（`CALLERS = 20`，答案里会打印扣下了几条），第 21 条被截断；CG 一次给 21 条。这是产品决定，但在这个任务上「截断 = 没答」。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 10742 字节）：

```text
evidence: static-heuristic
matches 1
src/studio/src/studio/app/editor_launch.rs:10 fn open_editor_at
  callers (1): src/studio/src/studio/app/overlay/search.rs::handle_search_overlay_key
  callees: Some, alert, display, find_registry, is_file, source_path_for, unwrap_or
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/studio/src/studio/app/overlay/search.rs:7 fn handle_search_overlay_key
  callers (1): src/studio/src/studio/app/keyboard_overlay.rs::handle_overlay_key
  callees: Char, Search, Some, advance_graph_focus, as_ref, call_tree_targets, clone, cloned, get, graph_item, graph_locals, graph_tree_item, is_empty, len, load_mir_snapshot_report, map, min, new, no …[行内截断]
…[此处截断：全文 70 行，此处 12 行]
```

`codegraph.answer.txt`（共 9251 字节）：

```text
{
  "symbol": "open_editor_at",
  "callers": [
    {
      "name": "handle_search_overlay_key",
      "kind": "method",
      "filePath": "toolchain/src/studio/studio/app/overlay/search.rs",
      "startLine": 7
    }
  ]
}

…[此处截断：全文 223 行，此处 12 行]
```

#### 第 10 轮：d=4，层次 `impl`，种子 20332200

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `kernel/src/registry_core/tree/connector.rs::external_provider_rejected` |
| 注入站点 | `kernel/src/registry_core/tree/transaction.rs::register_snapshot_batch`（第 44 行） |
| 链（自观察点向上） | `external_provider_rejected` → `connector_errors_from` → `connector_error_with_external` → `connector_error` → `register_snapshot_batch` |
| 变异 | 函数体重写：第 44 行 `false`→`true` |
| 命中判据 | `site` |
| mcp | 命中；字节 9096；1312.5 ms；调用 9；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 9/0；漏边 1 |
| codegraph | 命中；字节 8150；2876.3 ms；调用 9；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 9/0；漏边 1 |

**这次判它命中或未命中的理由（逐轮推理链）**：d=4 `impl`：两边都命中；两边**都**漏同一条边（9/10）。那条「边」是被查函数对**自己**的一次递归调用——驱动把与查询节点同名的调用者从结果里剔除了（避免自环），所以这条漏边是**驱动造成的**，不是两个工具造成的。把它记成工具的缺陷会是这次评测自己的误报。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 9096 字节）：

```text
evidence: static-heuristic
matches 1
src/registry_core/tree/connector.rs:148 fn external_provider_rejected
  callers (1): src/registry_core/tree/connector.rs::connector_errors_from
  callees: Some, accepts, path, path_for, path_is_strictly_under, registry
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/registry_core/tree/connector.rs:217 fn connector_errors_from
  callers (1): src/registry_core/tree/connector.rs::connector_error_with_external
  callees: Err, Ok, Some, accepts, ancestor_capability, and_then, any, as_ref, children_mut, clone, extend, external_provider_rejected, filter, filter_map, find_where, flatten, into_iter, is_empty, is …[行内截断]
…[此处截断：全文 80 行，此处 12 行]
```

`codegraph.answer.txt`（共 8150 字节）：

```text
{
  "symbol": "external_provider_rejected",
  "callers": [
    {
      "name": "connector_errors_from",
      "kind": "method",
      "filePath": "kernel/src/registry_core/tree/connector.rs",
      "startLine": 217
    }
  ]
}

…[此处截断：全文 162 行，此处 12 行]
```

#### 第 11 轮：d=4，层次 `chain`，种子 20340119

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `toolchain/src/studio/studio/ui/forms.rs::button` |
| 注入站点 | `toolchain/src/studio/studio/ui/ui.rs::draw`（第 91 行） |
| 链（自观察点向上） | `button` → `draw_form_frame` → `draw_new_project` → `draw_overlay` → `draw` |
| 变异 | 调用点删除：第 91 行 `draw_overlay(frame, app, cache);` |
| 命中判据 | `broken-link` |
| mcp | 未命中；字节 37；384.6 ms；调用 1；走到站点 False；断链检出 None；链路 unreached；点名了站点文件/符号 0/0；点名了断链节点文件/符号 0/0；TP/FP 0/0；漏边 1 |
| codegraph | 命中；字节 3142；3224.1 ms；调用 10；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 0/0；点名了断链节点文件/符号 1/1；TP/FP 12/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：**本轮是本次评测最有信息量的一轮，也是我们输得最彻底的一轮。** 我们的 MCP 在第一步就返回「no static function match for button」（整轮答案 **37 字节**，1 次调用）就结束了；CG 走了 10 步、拿到 12/12 条边并答对断链（3142 字节）。根因不在协议、也不在判据：`button` 的声明是带**生命周期参数**的泛型形状（`fn` 名字后紧跟 `<'a>`），我们的词法扫描器看不见它（§7 有最小复现与「35 个泛型声明里我们看不见 1 个」的测量），CG 的 tree-sitter 索引看得见。这一轮说明：**一次上游的「看不见」会让整条链的追溯停在第 0 步**，深度在这里不是难点，能看见才是。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 37 字节）：

```text
no static function match for `button`
```

`codegraph.answer.txt`（共 3142 字节）：

```text
{
  "symbol": "button",
  "callers": [
    {
      "name": "draw_form_frame",
      "kind": "function",
      "filePath": "toolchain/src/studio/studio/ui/forms.rs",
      "startLine": 60
    }
  ]
}

…[此处截断：全文 159 行，此处 12 行]
```

#### 第 12 轮：d=4，层次 `both`，种子 20348038

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `toolchain/src/studio/studio/app/call_tree_queries.rs::hop_call_tree` |
| 注入站点 | `toolchain/src/studio/studio/app/keyboard.rs::handle_key`（第 21 行） |
| 链（自观察点向上） | `hop_call_tree` → `step_graph_cursor` → `handle_search_overlay_key` → `handle_overlay_key` → `handle_key` |
| 变异 | 函数体重写：第 53 行 `!=`→`==`；调用点删除：第 21 行 `self.handle_overlay_key(key);` |
| 命中判据 | `broken-link` |
| mcp | 命中；字节 9484；1993.7 ms；调用 7；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 0/1；点名了断链节点文件/符号 1/1；TP/FP 23/0；漏边 1 |
| codegraph | 命中；字节 8199；3028.8 ms；调用 7；走到站点 False；断链检出 True；链路 absent；点名了站点文件/符号 0/0；点名了断链节点文件/符号 1/1；TP/FP 24/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：d=4 `both`：两边都命中；我们漏 1 条边（同一个 20 条调用者上限），CG 24/24。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 9484 字节）：

```text
evidence: static-heuristic
matches 1
src/studio/src/studio/app/call_tree_queries.rs:191 fn hop_call_tree
  callers (1): src/studio/src/studio/app/navigation.rs::step_graph_cursor
  callees: Some, abs_diff, call_tree_view, clone, enumerate, filter, find, get, graph_item, iter, label, let, map, min_by_key, next, note, or_else
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/studio/src/studio/app/navigation.rs:35 fn step_graph_cursor
  callers (1): src/studio/src/studio/app/overlay/search.rs::handle_search_overlay_key
  callees: Some, from_key, hop_call_tree, min, saturating_sub
…[此处截断：全文 68 行，此处 12 行]
```

`codegraph.answer.txt`（共 8199 字节）：

```text
{
  "symbol": "hop_call_tree",
  "callers": [
    {
      "name": "step_graph_cursor",
      "kind": "method",
      "filePath": "toolchain/src/studio/studio/app/navigation.rs",
      "startLine": 35
    }
  ]
}

…[此处截断：全文 219 行，此处 12 行]
```

#### 第 13 轮：d=5，层次 `impl`，种子 20355957

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `kernel/src/registry_core/tree/query.rs::entry_at` |
| 注入站点 | `kernel/src/registry_core/tree/graft_ops/overlay.rs::overlay_cuts`（第 130 行） |
| 链（自观察点向上） | `entry_at` → `visit_registration_chain` → `registration_chain` → `connector_errors_from` → `connector_error_with_external` → `overlay_cuts` |
| 变异 | 函数体重写：第 130 行 `!=`→`==` |
| 命中判据 | `site` |
| mcp | 命中；字节 15148；4079.0 ms；调用 22；走到站点 True；断链检出 False；链路 present；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/1；TP/FP 60/0；漏边 45 |
| codegraph | 命中；字节 24424；3901.4 ms；调用 13；走到站点 True；断链检出 None；链路 unreached；点名了站点文件/符号 1/1；点名了断链节点文件/符号 1/0；TP/FP 82/0；漏边 2 |

**这次判它命中或未命中的理由（逐轮推理链）**：d=5 `impl`：两边都命中，但**召回差距最大**——我们 60/105、CG 82/84。拆开这 45 条漏边看：**26 条来自一次跨 crate 的查询**（`find_registry` 在参考里有 36 个调用者，其中 26 个在 `toolchain/`，而我们的 `callgraph` 只在定义所属成员的源码里找调用者；最小复现见 §7：同一个问题 CG 报 36 个、我们报 17 个，跨 crate 一个都没有）。CG 这轮的 53 条「无法核实」边与 2 条漏边说明它也不是无懈可击：那 2 条同样是被查函数的递归自调用（驱动剔除），53 条是同名撞车无法核实。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 15148 字节）：

```text
evidence: static-heuristic
matches 1
src/registry_core/tree/query.rs:62 fn entry_at
  callers (4): src/registry_core/tree/connector.rs::visit_registration_chain, src/registry_core/tree/graft_ops/overlay.rs::apply_overlay_face, src/registry_core/tree/inspection.rs::health_check, src/r …[行内截断]
  callees: Some, and_then, as_ref, entry, values
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/registry_core/tree/connector.rs:27 fn visit_registration_chain
  callers (1): src/registry_core/tree/connector.rs::registration_chain
  callees: as_ref, clone, entry_at, expect, into, is_some, is_some_and, push, values
…[此处截断：全文 160 行，此处 12 行]
```

`codegraph.answer.txt`（共 24424 字节）：

```text
{
  "symbol": "entry_at",
  "callers": [
    {
      "name": "visit_registration_chain",
      "kind": "method",
      "filePath": "kernel/src/registry_core/tree/connector.rs",
      "startLine": 27
    },
    {
      "name": "apply_overlay_face",
      "kind": "method",
…[此处截断：全文 885 行，此处 12 行]
```

#### 第 14 轮：d=5，层次 `chain`，种子 20363876

| 项 | 值 |
|---|---|
| 观察点（问题里唯一的输入） | `toolchain/src/runtime/src/runtime/trace/snapshot/parse.rs::unescape` |
| 注入站点 | `toolchain/src/studio/studio/app/lifecycle.rs::after_project_write`（第 352 行） |
| 链（自观察点向上） | `unescape` → `parse_trace_artifact` → `read_trace_artifact` → `read_trace` → `rejudge_evidence` → `after_project_write` |
| 变异 | 调用点删除：第 352 行 `self.rejudge_evidence();` |
| 命中判据 | `broken-link` |
| mcp | 未命中；字节 24482；15983.0 ms；调用 40；走到站点 False；断链检出 None；链路 unreached；点名了站点文件/符号 0/0；点名了断链节点文件/符号 1/0；TP/FP 50/0；漏边 0 |
| codegraph | 未命中；字节 20455；11686.7 ms；调用 38；走到站点 False；断链检出 None；链路 unreached；点名了站点文件/符号 0/0；点名了断链节点文件/符号 1/0；TP/FP 50/0；漏边 0 |

**这次判它命中或未命中的理由（逐轮推理链）**：d=5 `chain`：两边都未命中，但**这一轮不是能力差距**：两边的游走在固定扇出预算（每层最多 12 个节点、每节点最多 10 个调用者、整轮最多 70 次调用）下都发散到了别的分支（我们 40 次调用、CG 31 次），根本没有查到那一跳的被调方（`link=unreached`），而它们在各自查过的节点上召回都是 50/50。这一轮量到的是「预算下的发散」，记未命中，但不该读成「谁找不到」。

该轮的原始输出片段（截断处已标出）：

`mcp.answer.txt`（共 24482 字节）：

```text
evidence: static-heuristic
matches 1
src/runtime/src/runtime/trace/snapshot/parse.rs:300 fn unescape
  callers (2): src/runtime/src/runtime/trace/snapshot/parse.rs::parse_optional_source, src/runtime/src/runtime/trace/snapshot/parse.rs::parse_trace_artifact
  callees: Err, Ok, Some, chars, len, malformed, next, push, with_capacity
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.

evidence: static-heuristic
matches 1
src/runtime/src/runtime/trace/snapshot/parse.rs:250 fn parse_optional_source
  callers (2): src/runtime/src/authoring/manifest/face/face.rs::edit, src/runtime/src/runtime/trace/snapshot/parse.rs::parse_trace_artifact
  callees: Err, Ok, Some, intern, malformed, parse_u32, unescape
…[此处截断：全文 303 行，此处 12 行]
```

`codegraph.answer.txt`（共 20455 字节）：

```text
{
  "symbol": "unescape",
  "callers": [
    {
      "name": "parse_trace_artifact",
      "kind": "method",
      "filePath": "toolchain/src/runtime/src/runtime/trace/snapshot/parse.rs",
      "startLine": 32
    },
    {
      "name": "parse_optional_source",
      "kind": "function",
…[此处截断：全文 594 行，此处 12 行]
```

## 5. 汇总表

### 5.1 命中与链路（按轮）

读法：`命中` 按 §2.5 的层次判据；`走到站点` 是「站点有没有作为某一层的节点被游走到」（`chain`/`both`
轮里站点在断口上方，永远走不到，这一列在那里必然是 ✗，那正是判据要换成「断链检出」的原因）；
`断链检出` 只对 `chain`/`both` 有意义（`impl` 轮写 —）；`链路` 是工具对那一跳的调用者集合报出的
`present`/`absent`/`unreached`。

| 轮 | 深度 | 层次 | 命中规则 | 站点（文件::符号） | MCP 命中 | CG 命中 | MCP 走到站点 | CG 走到站点 | MCP 断链检出 | CG 断链检出 | MCP 链路 | CG 链路 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1 | impl | site | kernel/src/registry_core/source/calls.rs::direct_calls | ✓ | ✓ | ✓ | ✓ | — | — | present | present |
| 2 | 1 | chain | broken-link | macro/src/mirror.rs::loop_value | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ | absent | absent |
| 3 | 1 | both | broken-link | toolchain/src/studio/studio/app/keyboard.rs::open_edit_form | ✓ | ✗ | ✗ | ✗ | ✓ | ✓ | absent | absent |
| 4 | 2 | impl | site | kernel/src/registry_core/source/items.rs::item_symbols | ✓ | ✓ | ✓ | ✓ | — | — | present | present |
| 5 | 2 | chain | broken-link | toolchain/src/studio/studio/ui/panels.rs::draw_workspace | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ | absent | absent |
| 6 | 2 | both | broken-link | toolchain/src/studio/studio/ui/graph/nodes.rs::draw_call_tree | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ | absent | absent |
| 7 | 3 | impl | site | kernel/src/registry_core/tree/graft_ops/overlay.rs::apply_overlay_face | ✓ | ✓ | ✓ | ✓ | — | — | present | present |
| 8 | 3 | chain | broken-link | toolchain/src/studio/studio/studio.rs::run_loop | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ | absent | absent |
| 9 | 3 | both | broken-link | toolchain/src/studio/studio/app/pointer.rs::handle_overlay_click | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ | absent | absent |
| 10 | 4 | impl | site | kernel/src/registry_core/tree/transaction.rs::register_snapshot_batch | ✓ | ✓ | ✓ | ✓ | — | — | present | present |
| 11 | 4 | chain | broken-link | toolchain/src/studio/studio/ui/ui.rs::draw | ✗ | ✓ | ✗ | ✗ | — | ✓ | unreached | absent |
| 12 | 4 | both | broken-link | toolchain/src/studio/studio/app/keyboard.rs::handle_key | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ | absent | absent |
| 13 | 5 | impl | site | kernel/src/registry_core/tree/graft_ops/overlay.rs::overlay_cuts | ✓ | ✓ | ✓ | ✓ | — | — | present | unreached |
| 14 | 5 | chain | broken-link | toolchain/src/studio/studio/app/lifecycle.rs::after_project_write | ✗ | ✗ | ✗ | ✗ | — | — | unreached | unreached |

### 5.2 成本与精确率（按轮）

读法：`TP/FP` 是与参考边集比出来的对数与确定误报数；`边召回` 是 `TP/(TP+漏边)`。
**这一列只能用来读我们这边，不能当成两边的公平比分**：参考边集就是 CG 自己在变异前的索引，而 CG 的
索引是变异后重建的，所以它的「召回」接近满分是构造使然（§9）。确定误报两边都是 0——这个结论同样
很弱，理由见 §9 的「无法核实」桶。

| 轮 | MCP 字节 | CG 字节 | MCP ms | CG ms | MCP 调用 | CG 调用 | MCP TP/FP | CG TP/FP | MCP 边召回 | CG 边召回 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 3281 | 3731 | 297.4 | 1264.4 | 4 | 4 | 6/0 | 6/0 | 6/6 | 6/6 |
| 2 | 3457 | 3655 | 210.7 | 1509.4 | 4 | 4 | 3/0 | 3/0 | 3/3 | 3/3 |
| 3 | 2194 | 1347 | 558.8 | 844.9 | 2 | 2 | 1/0 | 1/0 | 1/1 | 1/1 |
| 4 | 1214 | 1068 | 557.0 | 1065.9 | 3 | 3 | 2/0 | 2/0 | 2/2 | 2/2 |
| 5 | 2105 | 257 | 1060.0 | 690.2 | 3 | 2 | 1/0 | 1/0 | 1/1 | 1/1 |
| 6 | 638 | 265 | 921.4 | 911.7 | 2 | 2 | 1/0 | 1/0 | 1/1 | 1/1 |
| 7 | 10192 | 10026 | 1691.1 | 3294.6 | 11 | 11 | 13/0 | 13/0 | 13/13 | 13/13 |
| 8 | 5963 | 3325 | 3681.5 | 3107.7 | 9 | 9 | 9/0 | 9/0 | 9/9 | 9/9 |
| 9 | 10742 | 9251 | 1596.8 | 2178.4 | 7 | 7 | 22/0 | 23/0 | 22/23 | 23/23 |
| 10 | 9096 | 8150 | 1312.5 | 2876.3 | 9 | 9 | 9/0 | 9/0 | 9/10 | 9/10 |
| 11 | 37 | 3142 | 384.6 | 3224.1 | 1 | 10 | 0/0 | 12/0 | 0/1 | 12/12 |
| 12 | 9484 | 8199 | 1993.7 | 3028.8 | 7 | 7 | 23/0 | 24/0 | 23/24 | 24/24 |
| 13 | 15148 | 24424 | 4079.0 | 3901.4 | 22 | 13 | 60/0 | 82/0 | 60/105 | 82/84 |
| 14 | 24482 | 20455 | 15983.0 | 11686.7 | 40 | 38 | 50/0 | 50/0 | 50/50 | 50/50 |

### 5.3 按深度分组

| 分组 | 轮数 | MCP 命中 | CG 命中 | MCP 字节中位 | CG 字节中位 | MCP ms 中位 | CG ms 中位 | MCP 误报边 | CG 误报边 |
|---|---|---|---|---|---|---|---|---|---|
| 浅 d=1–2 | 6 | 6/6 | 5/6 | 2149.5 | 1207.5 | 557.9 | 988.8 | 0 | 0 |
| 深 d≥3 | 8 | 6/8 | 7/8 | 9838.0 | 8725.0 | 1842.4 | 3165.9 | 0 | 0 |
| 全部 | 14 | 12/14 | 12/14 | 4710.0 | 3693.0 | 1186.2 | 2527.4 | 0 | 0 |

## 6. 修复质量评分（3 轮）

评分流程对每个候选轮（`tools/xirang-chain-eval repair`，逐条命令见 §3.2）：

1. **基线**：pristine 副本上跑最小命令 `cargo test -p <站点所属 crate> --offline`（副本自带
   `CARGO_TARGET_DIR`），要求它绿；
2. **注入**（第 N 轮的构造变异），同一条命令再跑，要求它红；
3. **根因修复**：把注入的精确逆操作打回去（`--revert`），同一条命令要求它转绿；
4. **负对照**：不做修复，只给那个失败的测试加 `#[ignore]`，再跑一次。

四项判据（数字都由 `tools/xirang-chain-eval-repair.py` 从日志与文件算出来）：
① **绿恢复**＝退出码 0 **且** `failed=0` **且** `ignored` 没有变多（"不再跑的测试"不算"通过的测试"）；
② **改动最小性**＝相对 pristine 副本的 diff 行数、碰了几个文件、注入文件在不在其中；
③ **根因或症状**＝补丁落在注入文件的那一行（根因）、落在测试里（症状）、还是别处；
④ **附带损伤**＝注入文件之外被碰的文件，加上新增行里的绕过记号（`#[allow(`、`#[ignore`、`assert!(true)`、
`if false`、`todo!(` 等）。

候选池按深度从浅到深排队：第 1、4、10、7、13 轮，取满 3 个真的变红的。**第 10 轮与第 7 轮的注入没有
造出红灯**（`cargo test -p xirang-kernel --offline` 退出码 0），它们照原样留证、不获评分资格：
"函数体重写没有被测试覆盖"是关于测试套件的事实，不是把绿灯改写成红灯的理由。

### 第 1 轮（d=1，层次 `impl`）

| 项 | 值 |
|---|---|
| 最小命令 | `cargo test -p xirang-kernel --offline` |
| 注入 | `kernel/src/registry_core/source/calls.rs::direct_calls`，第 171 行 `||`→`&&` |
| ① 基线 | 退出码 0；passed=206 failed=0 ignored=0 |
| ① 注入后（红） | 退出码 101；passed=184 failed=3 ignored=0；失败测试：registry_core::source::calls::tests::a_declaration_macro_body_is_not_a_call_site, registry_core::source::calls::tests::direct_calls_are_sorted_distinct_and_exclude_the_function_itself, registry_core::source::source_tests::a_non_ascii_identifier_is_indexed_whole |
| ① 根因修复后 | 退出码 0；passed=206 failed=0 ignored=0 |
| ① 负对照（加 `#[ignore]`） | 退出码 0；passed=205 failed=0 ignored=1；`#[ignore]` 落在 `kernel/src/registry_core/source/calls.rs` 的 `registry_core::source::calls::tests::a_declaration_macro_body_is_not_a_call_site` |
| ② 注入补丁 | 改动 1 个文件、1 行增 / 1 行删；注入文件在改动集内：['kernel/src/registry_core/source/calls.rs'] |
| ② 根因修复补丁 | 改动 0 个文件、0 行增 / 0 行删（与 pristine 副本逐字节一致） |
| ② 负对照补丁 | 改动 1 个文件、1 行增 / 0 行删；改动集：['kernel/src/registry_core/source/calls.rs'] |
| ③ 根因或症状 | 根因：补丁就是注入的逆操作，落点即注入文件的那一行；对 pristine 副本的 diff 归零（0 个文件改动）。负对照落在注入文件里的测试属性上（第 171 行那条调用点原样留着） |
| ④ 附带损伤 | 根因修复：无关文件 []，绕过记号 []；负对照：无关文件 []，绕过记号 ['#[ignore'] |

四项判分：

| 判据 | 根因修复（注入的精确逆操作） | 负对照（加 `#[ignore]`） |
|---|---|---|
| ① 绿恢复 | ✓ 退出码 0、failed=0、ignored=0 | ✗ 退出码 0，但 ignored 从 0 变 1——测试是不跑了，不是通过了 |
| ② 改动最小性 | 改动 0 个文件、0 增 / 0 删（注入本身是 1 增 / 1 删，逆操作恰好吃掉它） | 改动 1 个文件、1 增 / 0 删 |
| ③ 根因或症状 | 根因：补丁落在注入文件、注入行上 | 症状：补丁落在测试属性上，注入的那个运算/调用原样留着 |
| ④ 附带损伤 | 无（无关文件 0 个；无绕过记号） | 有：绕过记号 ['#[ignore']；无关文件 [] |

红灯与绿灯的原文尾巴（截断处已标出）：

```text
$ cargo test -p xirang-kernel --offline   # 注入后
    registry_core::source::source_tests::a_non_ascii_identifier_is_indexed_whole

test result: FAILED. 184 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

error: test failed, to rerun pass `-p xirang-kernel --lib`
```

```text
$ cargo test -p xirang-kernel --offline   # 根因修复后
test kernel/src/registry_core/declaration/contract.rs - registry_core::declaration::contract::assert_contract (line 56) - compile fail ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

all doctests ran in 0.32s; merged doctests compilation took 0.27s
```

```text
$ cargo test -p xirang-kernel --offline   # 负对照：只加 #[ignore]
test kernel/src/registry_core/declaration/contract.rs - registry_core::declaration::contract::assert_contract (line 56) - compile fail ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

all doctests ran in 0.37s; merged doctests compilation took 0.32s
```

### 第 4 轮（d=2，层次 `impl`）

| 项 | 值 |
|---|---|
| 最小命令 | `cargo test -p xirang-kernel --offline` |
| 注入 | `kernel/src/registry_core/source/items.rs::item_symbols`，第 68 行 `true`→`false` |
| ① 基线 | 退出码 0；passed=206 failed=0 ignored=0 |
| ① 注入后（红） | 退出码 101；passed=198 failed=1 ignored=0；失败测试：qualified_functions_are_functions |
| ① 根因修复后 | 退出码 0；passed=206 failed=0 ignored=0 |
| ① 负对照（加 `#[ignore]`） | 退出码 0；passed=205 failed=0 ignored=1；`#[ignore]` 落在 `kernel/tests/b4_item_symbols.rs` 的 `qualified_functions_are_functions` |
| ② 注入补丁 | 改动 1 个文件、1 行增 / 1 行删；注入文件在改动集内：['kernel/src/registry_core/source/items.rs'] |
| ② 根因修复补丁 | 改动 0 个文件、0 行增 / 0 行删（与 pristine 副本逐字节一致） |
| ② 负对照补丁 | 改动 1 个文件、1 行增 / 0 行删；改动集：['kernel/tests/b4_item_symbols.rs'] |
| ③ 根因或症状 | 根因：补丁就是注入的逆操作，落点即注入文件的那一行；对 pristine 副本的 diff 归零（0 个文件改动）。负对照落在注入文件之外的测试文件 `kernel/tests/b4_item_symbols.rs` 上（注入的那行 `true→false` 原样留着） |
| ④ 附带损伤 | 根因修复：无关文件 []，绕过记号 []；负对照：无关文件 ['kernel/tests/b4_item_symbols.rs']，绕过记号 ['#[ignore'] |

四项判分：

| 判据 | 根因修复（注入的精确逆操作） | 负对照（加 `#[ignore]`） |
|---|---|---|
| ① 绿恢复 | ✓ 退出码 0、failed=0、ignored=0 | ✗ 退出码 0，但 ignored 从 0 变 1——测试是不跑了，不是通过了 |
| ② 改动最小性 | 改动 0 个文件、0 增 / 0 删（注入本身是 1 增 / 1 删，逆操作恰好吃掉它） | 改动 1 个文件、1 增 / 0 删 |
| ③ 根因或症状 | 根因：补丁落在注入文件、注入行上 | 症状：补丁落在测试属性上，注入的那个运算/调用原样留着 |
| ④ 附带损伤 | 无（无关文件 0 个；无绕过记号） | 有：绕过记号 ['#[ignore']；无关文件 ['kernel/tests/b4_item_symbols.rs'] |

红灯与绿灯的原文尾巴（截断处已标出）：

```text
$ cargo test -p xirang-kernel --offline   # 注入后
    qualified_functions_are_functions

test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p xirang-kernel --test b4_item_symbols`
```

```text
$ cargo test -p xirang-kernel --offline   # 根因修复后
test kernel/src/registry_core/declaration/contract.rs - registry_core::declaration::contract::assert_contract (line 56) - compile fail ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

all doctests ran in 0.40s; merged doctests compilation took 0.32s
```

```text
$ cargo test -p xirang-kernel --offline   # 负对照：只加 #[ignore]
test kernel/src/registry_core/declaration/contract.rs - registry_core::declaration::contract::assert_contract (line 56) - compile fail ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

all doctests ran in 0.38s; merged doctests compilation took 0.32s
```

### 第 13 轮（d=5，层次 `impl`）

| 项 | 值 |
|---|---|
| 最小命令 | `cargo test -p xirang-kernel --offline` |
| 注入 | `kernel/src/registry_core/tree/graft_ops/overlay.rs::overlay_cuts`，第 130 行 `!=`→`==` |
| ① 基线 | 退出码 0；passed=206 failed=0 ignored=0 |
| ① 注入后（红） | 退出码 101；passed=170 failed=17 ignored=0；失败测试：registry_core::tree::graft_ops::overlay::tests::a_non_full_graft_does_not_install_a_rule_its_children_violate, registry_core::tree::graft_ops::overlay::tests::an_unknown_replacement_names_the_selector, registry_core::tree::graft_ops::overlay::tests::overlay_keeps_base_siblings_and_source_trees_untouched … |
| ① 根因修复后 | 退出码 0；passed=206 failed=0 ignored=0 |
| ① 负对照（加 `#[ignore]`） | 退出码 0；passed=205 failed=0 ignored=1；`#[ignore]` 落在 `kernel/src/registry_core/tree/graft_ops/overlay.rs` 的 `registry_core::tree::graft_ops::overlay::tests::a_non_full_graft_does_not_install_a_rule_its_children_violate` |
| ② 注入补丁 | 改动 1 个文件、1 行增 / 1 行删；注入文件在改动集内：['kernel/src/registry_core/tree/graft_ops/overlay.rs'] |
| ② 根因修复补丁 | 改动 0 个文件、0 行增 / 0 行删（与 pristine 副本逐字节一致） |
| ② 负对照补丁 | 改动 1 个文件、1 行增 / 0 行删；改动集：['kernel/src/registry_core/tree/graft_ops/overlay.rs'] |
| ③ 根因或症状 | 根因：补丁就是注入的逆操作，落点即注入文件的那一行；对 pristine 副本的 diff 归零（0 个文件改动）。负对照落在注入文件里的测试属性上（注入的那行 `!=→==` 原样留着） |
| ④ 附带损伤 | 根因修复：无关文件 []，绕过记号 []；负对照：无关文件 []，绕过记号 ['#[ignore'] |

四项判分：

| 判据 | 根因修复（注入的精确逆操作） | 负对照（加 `#[ignore]`） |
|---|---|---|
| ① 绿恢复 | ✓ 退出码 0、failed=0、ignored=0 | ✗ 退出码 0，但 ignored 从 0 变 1——测试是不跑了，不是通过了 |
| ② 改动最小性 | 改动 0 个文件、0 增 / 0 删（注入本身是 1 增 / 1 删，逆操作恰好吃掉它） | 改动 1 个文件、1 增 / 0 删 |
| ③ 根因或症状 | 根因：补丁落在注入文件、注入行上 | 症状：补丁落在测试属性上，注入的那个运算/调用原样留着 |
| ④ 附带损伤 | 无（无关文件 0 个；无绕过记号） | 有：绕过记号 ['#[ignore']；无关文件 [] |

红灯与绿灯的原文尾巴（截断处已标出）：

```text
$ cargo test -p xirang-kernel --offline   # 注入后
    registry_core::tree::graft_ops::resolution::resolution_tests::full_cut_inherits_external_subtree_without_moving_source_trees

test result: FAILED. 170 passed; 17 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

error: test failed, to rerun pass `-p xirang-kernel --lib`
```

```text
$ cargo test -p xirang-kernel --offline   # 根因修复后
test kernel/src/registry_core/declaration/contract.rs - registry_core::declaration::contract::assert_contract (line 56) - compile fail ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

all doctests ran in 0.41s; merged doctests compilation took 0.33s
```

```text
$ cargo test -p xirang-kernel --offline   # 负对照：只加 #[ignore]
test kernel/src/registry_core/declaration/contract.rs - registry_core::declaration::contract::assert_contract (line 56) - compile fail ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

all doctests ran in 0.38s; merged doctests compilation took 0.30s
```

### 6.1 这三轮说明了什么

1. **红灯是构造出来的，不是借来的**：三轮的基线都是 187+ 个测试全绿，注入后分别有 3、1、17 个测试
   失败，逆操作后回到全绿。评分表因此有了可比的起点与终点。
2. **"最小修复"与"症状掩盖"被同一张表分开**：逆操作补丁的 diff 恰好是注入的镜像（1 行），
   无关文件 0 个、绕过记号 0 个；`#[ignore]` 补丁虽然也让退出码变成 0，但 `ignored` 计数暴露了它，
   绕过记号与（第 4 轮）一个无关文件把它钉在症状那一侧。
3. **两个候选轮没有红灯**：第 10 轮（`register_snapshot_batch` 的函数体）与第 7 轮
   （`apply_overlay_face` 的函数体）的 `false`→`true` / `||`→`&&` 重写没有让 `xirang-kernel` 的
   任何测试失败。这说明"函数体变异 + 单 crate 测试"并不是稳定的红灯来源，评分轮必须先验证红灯再
   评分——本流程就是这么做的（红灯不成立就不进评分）。
## 7. 补充探针：两处"看不见"，与 CG 的 `explore`/`impact`

这一节不计入命中率。它解释第 11 与第 13 轮为什么输，并且把两边**工具形状**的差别记录在案
（CG 有函数级 `explore`/`impact`，我们的 MCP 的 `impact` 是对注册面的，不是对函数的——这是形状差别，
不该记成能力差距，所以它不进比分）。

### 7.1 我们的 `callgraph` 只看定义所属成员：跨 crate 的调用者一个都报不出来

最小复现（都在 pristine 副本上，命令逐字给出）。同一个问题问两边：

```sh
$ python3 - <<'PY'
# 我们的 MCP：以工作区根为 root，不加 path（桥为整个工作区作答的用法）
call("xirang.callgraph", {"function": "find_registry", "root": "<副本根>", "limit": 50})
PY
xirang.callgraph callers = 17
by top directory: ['src', 'tests']        # 全部是成员内相对路径，也就是 kernel 自己
cross-crate present: []
```

```sh
$ codegraph callers find_registry -p . --json -l 50 | python3 -c "…统计…"
codegraph callers for find_registry = 36
by top directory: {'kernel': 10, 'toolchain': 26}
  cross-crate example: toolchain/src/runtime/src/authoring/operations/create.rs :: create_module
```

**36 对 17，差的全在跨 crate 的那 26 条上。** 机制在源码里：`callgraph` 先 `load_sources(root)`
再把调用者算出来，而工作区根的分派把"谁调用它"这半问交给了**定义所属成员**（另外 5 个成员回答
`no static function match`）——于是定义在 `kernel` 里的函数，它的 `toolchain` 调用者永远不会出现在
答案里。这不是驱动用错了 `root`：成员作用域正是桥自己的文档化用法（`root` 给成工作区根也一样，
上面这条命令就是工作区根）。这一条直接解释第 13 轮 45 条漏边里的 26 条，也解释第 9、12 轮各 1 条。

代价还有一半：在工作区根上问一次要 **9.8 秒**（成员新鲜度校验要逐成员读遍源码树），而成员作用域下
同一次提问是 **~50 ms**。也就是说"能不能跨 crate"与"贵不贵"在这套形状里是同一个决定的两种后果。

### 7.2 我们的源码扫描器对一种 `fn` 声明是盲的

第 11 轮的观察点是 `toolchain/src/studio/studio/ui/forms.rs` 里的一个函数，我们的
`callgraph` 对它回 `no static function match`，整轮答案 37 字节。它的声明长这样（原文）：

```rust
fn button<'a>(label: &'a str, color: Color) -> Paragraph<'a> {
```

诊断探针（**不是**评测轮：这里的对象是"泛型声明"这个语法属性的全集，与种子挑选正相反，全文逐字）：

```python
#!/usr/bin/env python3
"""Diagnostic: how many generic function declarations can each side see?
诊断：两边各能看到多少个泛型函数声明？

Not an evaluation round: the sites here are the *union* of a syntactic property
(generic parameters), which is the opposite of seed-picked. It exists to explain a
round's failure, and its numbers are reported as a defect measurement.
不是评测轮：这里的对象是某个语法属性（泛型参数）的**全集**，与种子挑选正相反。它存在的目的是解释
某一轮的失败，其数字按缺陷测量报告。
"""
import json, re, sqlite3, subprocess

root = "/tmp/xirang-eval/base"
conn = sqlite3.connect(f"file:{root}/.codegraph/codegraph.db?mode=ro", uri=True)
members = ["kernel", "macro", "toolchain"]
candidates = []
for name, path, start in conn.execute(
    "SELECT name, file_path, start_line FROM nodes WHERE language='rust' AND kind IN ('function','method')"
):
    relative = path.replace(root + "/", "")
    if relative.startswith("toolchain/src/mcp/") or not relative.endswith(".rs"):
        continue
    pieces = relative.split("/")
    if "tests" in pieces or pieces[-1].endswith("_tests.rs") or pieces[-1] == "tests.rs":
        continue
    member = next((m for m in members if relative.startswith(m + "/")), None)
    if member is None:
        continue
    lines = open(f"{root}/{relative}", encoding="utf-8").read().splitlines()
    if start - 1 < len(lines) and re.search(r"\bfn\s+" + re.escape(name) + r"\s*<", lines[start - 1]):
        candidates.append((relative, name, member))

process = subprocess.Popen(
    [f"{root}/target/debug/xirang-mcp"], cwd=root,
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1,
)
def call(request):
    process.stdin.write(json.dumps(request) + "\n"); process.stdin.flush()
    return json.loads(process.stdout.readline())
call({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
blind, seen, examples = 0, 0, []
for relative, name, member in candidates:
    response = call({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
        "name": "xirang.callgraph",
        "arguments": {"function": name, "path": relative[len(member) + 1:], "root": f"{root}/{member}"}}})
    text = response["result"]["content"][0]["text"]
    if "no static function match" in text:
        blind += 1
        examples.append((relative, name))
    else:
        seen += 1
print(f"generic declarations probed: {len(candidates)}; xirang.callgraph sees {seen}, blind to {blind}")
for relative, name in examples[:6]:
    print("  blind:", relative, "::", name)
```

它在一份 pristine 副本上的原样输出：

```text
$ python3 /tmp/xirang-eval/probe-generics.py
generic declarations probed: 35; xirang.callgraph sees 34, blind to 1
  blind: toolchain/src/studio/studio/ui/forms.rs :: button
```

同一份副本上，CG 的索引对同一个文件给出的符号表里有这个函数（`codegraph node` 的文件模式）：

```text
$ codegraph node -f toolchain/src/studio/studio/ui/forms.rs -p . --symbols-only
**toolchain/src/studio/studio/ui/forms.rs** — 3 symbols, used by 4 files: toolchain/src/studio/studio/ui/forms/face.rs, toolchain/src/studio/studio/ui/forms/graft.rs, toolchain/src/studio/studio/ui/forms/plugin.rs, toolchain/src/studio/studio/ui/forms/project.rs

**Symbols**
- `FormAreas` (struct) — :36
- `draw_form_frame` (function) ( frame: &mut Frame<'_>, row: Rect, list: Rect, confirm_label: &str, cancel_label: &str, hint: &str, ) -> FormAreas — :60
- `button` (function) (label: &'a str, color: Color) -> Paragraph<'a> — :79

> Drop `symbolsOnly` (or pass `offset`/`limit`) to read the source, like Read.
```

而我们的 `inspect` 对同一个文件只列出另一个函数（`button` 不在其中）：

```text
$ xirang.inspect {"path": "src/studio/src/studio/ui/forms.rs", "root": "<副本>/toolchain"}
file src/studio/src/studio/ui/forms.rs
fn draw_form_frame lines 60-100 calls=[…, button, …]
```

注意这两行放在一起才说明问题：`draw_form_frame` 的**调用列表里有 `button`**（扫描器在调用点上认得出
这个记号），但**定义列表里没有它**——盲的是"声明解析"这一半，不是"调用识别"那一半。35 个泛型声明里
我们看不见 1 个；比例低，但一次"看不见"就足以让整条链从第 0 步断掉（第 11 轮）。

### 7.3 CG 的 `explore` / `impact`：形状差别，不进比分

同一份副本上，CG 的 `explore` 一次给出符号、调用者数、覆盖测试的有无与源码，`impact` 一次给出
带深度的传递影响面：

```text
$ codegraph explore mask_non_code -p . --max-files 3
**Exploration: mask_non_code**

Found 28 symbols across 3 files.

**Blast radius — what depends on these (update/verify before editing)**

- `mask_non_code` (kernel/src/registry_core/source/source.rs:271) — 6 callers in `kernel/src/registry_core/source/calls.rs`, …; ⚠️ no covering tests found
…

$ codegraph impact find_registry -p . -d 2 --json
{ "symbol": "find_registry", "depth": 2, "nodeCount": 88, "edgeCount": 109, "affected": [ … ] }
```

我们的 MCP 没有对应的**函数级** `explore`/`impact`：`xirang.impact` 的输入是注册面节点
（`node`、`depth`、`limit`），不是函数。因此这三条命令没有被塞进判分流程——把"工具形状不同"记成
"能力差距"会是这次评测自己的误报。
## 8. 敏感串扫描

公开的是证据，不是秘密：扫描命令与它的原样输出都留在这里。扫描对象是本次新增的六个脚本与本文档。

```text
$ grep -nE (sk-[A-Za-z0-9]{16,}|ghp_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{12,}|BEGIN [A-Z ]*PRIVATE KEY|password[[:space:]]*=|secret[[:space:]]*=|token[[:space:]]*=) tools/xirang-chain-eval tools/xirang-chain-eval-graph.py tools/xirang-chain-eval-mutate.py tools/xirang-chain-eval-ask.py tools/xirang-chain-eval-judge.py tools/xirang-chain-eval-repair.py docs/audit-2026-09-29/audit-chain-eval.md
scan exit 1 (1 = no credential-shaped string)
$ grep -nF "$HOME" tools/xirang-chain-eval tools/xirang-chain-eval-graph.py tools/xirang-chain-eval-mutate.py tools/xirang-chain-eval-ask.py tools/xirang-chain-eval-judge.py tools/xirang-chain-eval-repair.py docs/audit-2026-09-29/audit-chain-eval.md   # 家目录的具体路径不该写进脚本，扫的是它
scan exit 1 (1 = no machine-specific path)
```

两条都返回 1（grep 的"没有匹配"），也就是：新增文件里没有凭据形状的字符串，也没有把本机家目录写死
进任何脚本或文档（扫描命令本身用 `$HOME` 而不是它的取值——第一次运行时它把 `/home/nich` 这个字面量
写在了自己身上，扫描因此报出两行命中，改法就是这条命令措辞的由来）。
## 9. 本次评测的局限

按"会改变结论"的程度排序，每条都给出它影响哪一格。

**① 参考边集用的是 CG 自己的索引，因此"召回"和"精确率"两列都是不对等的。**
参考 = pristine 副本的 codegraph 索引（5,835 个节点 / 18,719 条边）。CG 在变异后重建自己的索引，
所以它的答案与参考几乎必然一致（第 13 轮 82/84、第 14 轮 50/50），而我们的 MCP 的召回其实是
"**与 CG 的图重合多少**"，不是绝对真值。这条影响 §5.2 的整张表与 §0 的结论一、结论四。
可以部分缓解的证据是文本复核：第 13 轮我们漏掉的 45 条边，**45 条**都能在被报调用者的文件正文里
找到被调方的名字（所以它们多半是真边，不是 CG 的假边）；但"名字出现"不等于"这里是调用点"。

**② "确定误报"这一桶几乎是空的，所以"误报"这个题目这轮没真正测到。**
参考偏向 CG + 我们采取"回到正文复核、找不到才叫误报"的保守规则，剩下的可疑边落进"无法核实"
（第 13 轮 CG 53 条、我们 8 条）。0 条误报的结论**不能**读成"两边都不误报"。
按维护者原话里"避免模型过拟合导致的误报"的要求，这条是本次最大的未完成项。

**③ `chain`/`both` 轮的"命中"是弱判据，而且 d=1 时是平凡的。**
被删掉的调用点在静态上不可达（站点在断口上方），所以那一层的判据退化成"没把被删的边报回来 + 点名
了那一跳的被调方"。d=1 时被调方就是问题里的观察点，点名是白送的。这条影响 §4 的第 2、3、5、6、8、
9、12 轮读数——它们的 ✓ 不该被读成"定位准"。

**④ 层次辨认（`impl`/`chain`/`both`）只有词频，没有判词。**
没有 LLM 裁判的要求下，我记录了两组关键词的出现次数，但没有把它当判词：`chain`/`both` 轮里
"调用链"这类词在任何答案里都可能出现。因此"工具是否答对是哪一层"这个问题**本次没有可信答案**，
只留下了可复核的原始计数（`verdict.json` 的 `layer_words`）。

**⑤ 上限与发散：第 14 轮量的是预算，不是能力。**
驱动对两边施加同样的上限（每节点最多 10 个调用者、每层最多 12 个节点、整轮最多 48 个节点 / 70 次
调用）。d=5 时两边的游走都发散（我们 40 次调用、CG 31 次），没查到那一跳的被调方。这一轮应该被当作
"没有测量到"，而不是"两边都找不到"。同样的上限也造成第 9、12 轮各 1 条"截断即漏"的边
（我们的 `callgraph` 另有 20 条调用者的硬上限，这是工具自身的决定，不是驱动的）。

**⑥ 一条"漏边"是驱动自己造的。** 驱动把与查询节点同名的调用者从结果里过滤掉（避免自环），于是
被查函数的递归自调用被算成"漏"（第 10 轮两边各 1 条、第 13 轮 CG 2 条）。把它记成工具缺陷会是
本次评测自己的误报，这里明确纠正。

**⑦ 变异算子的覆盖面窄。**
只用了比较/布尔运算符互换（类型不变，必然编译）与"删掉一条整行的调用语句"。没有覆盖：类型/泛型/
生命周期改动、`match` 臂的增删、提前返回、trait 分派与动态派发、方法调用的改写、多行调用链的删除。
因此**第 11 轮那种"看不见"的缺陷是被"站点恰好落在看不见的函数上"撞出来的**，不是被系统搜索出来的。

**⑧ 编译验证只在 3 个评分轮做过。**
其余 11 轮没有跑编译器：变异的安全性靠两条构造性论证（算子互换不改类型、被删的是丢弃返回值的语句）
加上"codegraph 重建索引后该文件没有解析错误"。**没有**做类型检查，因此"11 轮都编译得过"是一个
推断，不是实测。第 6 节里两次"注入没有红灯"也说明"测试覆盖得到函数体"并不普遍。

**⑨ 每个站点只注入一次、只问一次，没有重复试验。**
14 轮各一次，没有做种子方差或重试统计，因此数字之间的小差别（例如第 9 轮 22/23 与 23/23）不该被
当成稳定的量差。

**⑩ 名字唯一性把"同名歧义"整类题目排除了。**
观察点与链上每个节点都要求名字全树唯一（否则"点名"有歧义，量的就不是深度了）。于是 CG 的
"50 条无法核实"里那一类真实难题（常见名 `new` 有 142 个定义）在这套评测里没有被评。

**⑪ 副本里没有 `.xirang` 产物，所以我们的 MCP 全程走的是"源码推导"这条路。**
`git archive` 不含未跟踪的构建产物，成员状态因此全是 `not built`（答案原文：
`cannot read …/target/xirang/out/source_scope.tsv`）。这既让答案更啰嗦（每个成员一行状态），
也让"先读已发布记录"那条产品路径没有在这套评测里被走到。

**⑫ 评测的树是固定修订，不是评测时刻的 HEAD。**
工作树在本次评测期间前进了 3 次，其中**一次提交本身不可构建**（我在 §2.1、§3.4 记了这个事故）。
副本全部来自固定的 `8f8b3b7…`，因此评测内部自洽，但"这棵树"不等于"当时的检出"。要在它之上继续
比较，必须用同一个 sha 重跑。

**⑬ 敏感串扫描只覆盖本次新增的 6 个文件与本文档**（命令与结果见 §8），不覆盖历史记录。
