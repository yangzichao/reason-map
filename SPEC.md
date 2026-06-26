# reason-map — 产品 / 技术规格

> 一个本地 app,用来**梳理逻辑**:把脑子里半成型、不铁证如山的论证链摊到画布上,
> 让 LLM 帮你**推进**和**对抗**,但**判定权始终在人手里**。

---

## 1. 它是什么 / 不是什么

**是**:论证地图(argument map)。节点 = 命题,边 = 带方向和性质的推理关系。
工具主动帮你 (a) 往下推、(b) 找缺口、(c) 红队攻击。

**不是**:思维导图。思维导图的连线只表示"有关系";这里的边有**方向、性质、(可选)强度**。

**核心立场**:这些论证本来就**不铁证如山**——里头有假设、有数据、有赌。
所以工具不追求"算出"对错,而是帮你**看清自己在赌什么、哪里最虚**。

---

## 2. 数据模型(核心对象)

### 节点 = 命题(claim)
带元数据,不是裸文字框:

- `status`:`fact`(硬事实) / `assumption`(假设) / `bet`(赌) / `evidenced`(有证据) / `open`(开放问题)
- `text`:命题内容
- `origin`:`user` / `ai_suggested` / `ai_accepted`(见 §5 provenance)
- 画布坐标 `x, y`

### 边 = 推理关系
自身可带(可选)强度:

- `type`:`support`(支持) / `rebut`(反驳) / `premise_of`(是…前提) / `depends_on`(依赖)
- 打在**边**上 = 针对"这一步推理"本身

### 证据 / 引用
挂在节点上:`kind` = `url | quote | data | file`。

---

## 3. 不用置信度数字 —— 用"对抗 + 人判"代替

**决定:不强制数字置信度。** 给自然语言论证套 0.6/0.7 是假精度。

一个节点稳不稳,不靠猜分,而是看它**扛过了多少次攻击**:

- **"我在赌什么"** = 挑出所有 `status = bet / assumption` 的节点(标签即信号)。
- **"赌错了崩多少"** = 选节点 → 高亮所有依赖它的**下游**(图可达性,不算概率)。
- **"哪个点最要命"** = 找**承重节点 / 关节点**(articulation point,结构上所有路都过它)。

> 强度概念是**可选**的:默认纯标签;哪天某张图真想量化再手动填,不填工具照常转。
> (粗粒度"强/弱/存疑"三档为未来可加项,见 §8。)

---

## 4. LLM 能力(嵌进画布的推理助手)

所有能力的产物都先进 **staging**,人点了才落进真源(§5)。

1. **前向推演**:选节点 → "从这些还能推出什么",给 2–3 个候选下游节点。
2. **缺口检测**:选两个节点 → "A 到 B 中间缺了什么",补隐含前提(enthymeme)成中间节点。
3. **对抗 / 红队(核心)**:一个 button,让 LLM **攻击**选中的节点或边,人**手动判定**。详见 §4.1。
4. **上下文感知 chat**:右侧 panel 知道当前选了哪些节点、整张图长啥样;回答能一键变节点。

### 4.1 对抗按钮(置信度的诚实替代品)

攻击是一等对象,但**不是图里的节点**——它活在 staging 层。

攻击种类 `kind`:
- `rebuttal`(反驳:命题为假)
- `counterexample`(反例)
- `hidden_assumption`(隐藏假设:偷偷依赖了 X)
- `alternative`(替代解释:你的证据其实支持别的结论)
- `non_sequitur`(跳步:A 推不出 B)—— 通常打在**边**上

**人判定的三种结果,且每种都带后果**(不然就是嘴炮):
- `conceded`(成立,我认)→ 攻击可一键**晋升**进真源,变成图里的 rebut 节点/边;或把被打节点降级为 `bet / open`。
- `rebutted`(我能驳)→ 写下反驳理由;**反驳本身是资产**,让节点更硬,也能晋升成节点。
- `deferred`(待定)→ 挂节点上当未结悬念。

**payoff**:每个节点攒出**战绩(litigation history)**——被丢过什么、你怎么接的。
于是"最弱环节"不用算,就是**还挂着 pending / conceded 攻击的节点**;附带"我当初为什么信"的完整记录。

---

## 5. 数据层(一开始就要对的接缝)

### 真源:SQLite(单个 `.db` 文件)
- **状态表当真源**(state tables as source of truth),change log 只记历史(见下)。
- 不用图数据库:数据量极小(一张图几百节点、几十 KB),图算法内存里跑是瞬时的。

### 必须第一天就打进去的(以后补要剜肉)
- **ID 策略**:ULID / UUIDv7(客户端生成、可排序)。**不用自增整数**——以后想同步/合并/分享必撞。
- **Migration 框架**:`user_version` pragma 或 migrations 表;schema 一定会演化。
- **断言 vs 派生分离**:节点存的是**断言**(你说它是 `bet`);**派生**的(它是承重节点)只在内存里算,不存真源。
- **AI provenance + staging**:LLM 产物先待在 staging(`origin = ai_suggested`),人接受才落真源。
- **全文搜索**:FTS5 over `nodes.text`,第一天就建。
- **语义检索**:`sqlite-vec`,每节点存 embedding + `embedding_model / dim / embedded_at` + dirty flag(文字改了要重算)。
  这是比传统论证软件强一档的地方,且最痛回溯(要 backfill),必须现在埋。

### 完整性
- FK + `ON DELETE` 级联(边引用节点、证据挂节点)。
- `chat.context_node_ids` 是 JSON,FK 管不到 → 悬挂引用要自己清。
- **环检测**:留着,但只为揪**循环论证**这个毛病(不再是怕传播算炸)。

### 变更历史(已选:状态表当真源 + log 只记历史)
- `events(id, map_id, ts, op, payload JSON)`,追加式。
- 每条 event 存够信息支持 **undo / redo**(before/after 或正反操作)。
- 白拿:跨会话撤销、回看、审计。

### 配套
- **应用状态与文档分离**:窗口位置、选的模型、auth、上次打开哪张图 → 单独 kv 表,不混进文档表。
- **备份 / durability**:WAL 模式 + 自动定时快照(单 `.db` = 单点故障)。
- **软删除**:`deleted_at`,误删可恢复。

### 分享 / git
- JSON 降级为**导出格式**:一张图一键 export 成 `.argmap.json`,用于分享 / git 版本管理 / diff。
- 真源仍在 DB,可读快照按需吐出。

### 表草图
```sql
maps(id, title, created_at, updated_at, meta JSON)

nodes(id, map_id, text, status, origin, x, y, created_at, updated_at, deleted_at)
  -- status: fact | assumption | bet | evidenced | open
  -- origin: user | ai_suggested | ai_accepted

edges(id, map_id, from_node, to_node, type, strength NULLABLE)
  -- type: support | rebut | premise_of | depends_on

evidence(id, node_id, kind, payload JSON, created_at)
  -- kind: url | quote | data | file

challenges(id, target_kind, target_id, kind, content,
           status, verdict, user_note, created_at)
  -- target_kind: node | edge
  -- kind: rebuttal | counterexample | hidden_assumption | alternative | non_sequitur
  -- status/verdict: pending | conceded | rebutted | deferred

chat_messages(id, map_id, role, content, context_node_ids JSON, created_at)

events(id, map_id, ts, op, payload JSON)   -- 历史 / undo

settings(key, value)                        -- 应用状态,与文档分离

-- FTS5 虚拟表 over nodes.text
-- sqlite-vec 表存 node embeddings
```

---

## 6. 技术栈(从第一天就按最佳本地体验定死,不退而求其次)

每一项都是**最优解**,不是省事的替代品。理由写在后面,免得日后被当成妥协砍掉。

- **形态**:原生桌面 app,**Tauri 2**(Rust 内核 + 系统 webview)。
  - 为什么不是 Electron:Electron 才是退而求其次(臃肿、吃内存、包大)。Tauri 给原生窗口/菜单/全局快捷键、包体小、冷启动快、内存省,Rust 侧直接握住 SQLite 和密钥。
  - 为什么不是纯 SwiftUI 原生:画布/图编辑的成熟生态全在 web 侧;手搓原生 canvas 反而**更差**的交互。Tauri 让我们拿到最好的 canvas 库,同时保留原生外壳。

- **画布 / 图渲染**:**React + TypeScript + React Flow (@xyflow)**。
  - 在这个规模(几百节点)它给的交互最好:节点是**可内联编辑的富组件**、平移/缩放顺滑、定制度高、a11y 好。
  - WebGL(PixiJS 等)在这反而更差——富文本编辑、节点内嵌内容都难搞。所以 React Flow 是**最佳而非妥协**。

- **本地真源**:**SQLite**,Rust 侧用 `sqlx` 直连(不走 JS 插件,少一层)。WAL + FTS5 + `sqlite-vec`。工业级标配。

- **语义检索 / embedding**:**本地 embedding 模型**(`fastembed` / ONNX,跑在 Rust 侧)。
  - 离线、零外部依赖、数据不出本机——这才是最佳本地体验。
  - Anthropic 无 embedding 端点;**绝不为了 embedding 引入任何云依赖**。

- **LLM 后端**:**本机 Claude Code 登录态**(`claude-opus-4-8`),通过官方 `claude` CLI 的
  headless 模式驱动。**(2026-06 决策反转 —— 见下方「为何反转」)**
  - Rust 内核侧 spawn `claude -p`(子进程):system prompt 走 argv、user prompt 走 stdin、
    `--output-format json`(单次)或 `stream-json --include-partial-messages`(流式),
    用 `--disallowedTools` 关掉所有工具,使其退化为纯单轮推理引擎。见 `src-tauri/src/llm/cli.rs`。
  - **不读取、不复制 OAuth token**:认证完全由官方 `claude` 进程自己持有/管理。这是 Anthropic
    支持的 headless / Agent-SDK 路径,不是「抠 keychain 里的订阅凭证」的灰色 hack。
  - **无需 API key**:用你的 Claude Code 订阅登录;前端只展示后端是否就绪(`ai_backend_status`),
    不再输入/存储任何 key。
  - **默认 streaming + 高 effort**(`--effort high` 映射原 adaptive thinking),回答边生成边经
    Tauri channel 流进画布。
  - **代价(诚实记录)**:① 受**订阅额度**约束(5 小时 / 周窗口),密集分析可能触发限流,
    调用会返回限流提示;② 依赖本机装了 Claude Code 且已 `claude login`,换机/分发需各自登录;
    ③ 比直连 API 多一层子进程开销。
  - **为何反转**:原 spec 定的是「官方 Anthropic API + 真实 API key,不蹭订阅 OAuth」。用户在
    2026-06 明确要求改用本机订阅登录态。落地方式刻意选了 CLI 子进程(而非抠 token),以在满足
    「用订阅、不要 key」的同时,避开凭证提取这一真正的灰色操作。
  - **彻底无 key**:旧的 keychain 存 key 链路(`secrets.rs`、`has/set/clear_api_key`)及原 HTTP
    传输依赖(`reqwest`/`futures-util`)已一并删除,代码里不再有任何 API-key 概念。

---

## 7. UX 设计原则(本项目的体验主张——优先级等同技术栈)

这些是会被实现细节侵蚀、所以必须写死的体验底线。每条都有"为什么",别当成可选项。

1. **文字优先,结构按需 —— 画布不是主输入口。**
   这类工具(Rationale / Argdown)死于"逼你先建结构再思考"。人脑比手快,
   "加节点→打字→加边→选类型"会打断 flow。所以:
   - 有一个**大纲 / 自由文本输入模式**:回车=新命题,Tab 缩进=依赖关系,可粘整段让 LLM 拆成草图。
   - 画布用于**精修和看清**,不是录入。两种视图(文本 ⇄ 图)实时双向同步。

2. **LLM 环境化,不弹窗化。**
   - 对抗 / 推演 / 缺口的产物 = **虚影卡片(ghost card)贴在相关节点旁**,流式生成,
     像 Copilot ghost text。接受变实,一键打发。
   - 绝不用阻塞式模态框打断思考。

3. **判定循环是全 app 最快的动作。**
   - 攻击出现 → 判定 = 纯键盘:`1` 认 / `2` 驳(内联写理由)/ `3` 待定。
   - **攻击收件箱(challenge inbox)**:像扫邮件一样批量 triage 所有 pending 攻击。

4. **可读性主动设计(图必然变面条)。**
   - **focus 模式**:选节点 → 其余变暗,只留它的论证邻域(祖先 + 后代)。
   - **稳定的分层自动布局**(dagre / elk,沿推理方向):加一个节点**不重排全图**(重排很晕)。
   - 子树**折叠 / 展开**。
   - **脆弱点一眼可见**:`bet` 发光、挂未结攻击的节点带红角标——眯眼就看出风险在哪。

5. **空间语法一致 —— 论证有方向。**
   - 前提 → 结论有固定的空间朝向(如自下而上 / 自左向右),读图不用每次重新学。
   - 反驳视觉上明显有别(红色、从侧面攻入)。

6. **AI 永不悄悄动真源,且"看得见"地不动。**
   - AI 产物永远视觉有别(虚线 / 灰),非一个明确手势不落真源;落后留淡来源标记。
   - 信任是梳理逻辑工具的命根子:用户必须始终相信"图 = 我的思考"。

7. **零摩擦进入。**
   - 即时启动、自动打开上次的图、无登录墙(复用本机 Claude Code 登录态,无需在 app 里填 key)。
   - **自动保存**(SQLite,无"保存"按钮);离线可用(除 LLM 调用)。
   - 节点**双击就地编辑**;status 用 chip / 快捷键循环切换,不开侧栏表单。

8. **绝不给空白画布。**
   - 首启动载入一张**真实有趣的示例论证图**,让用户立刻能戳对抗按钮看它干啥。
   - 空白画布是这类工具的头号杀手。

9. **chat 与画布是一个世界,不是两个。**
   - 选中节点 → 自动进 chat 上下文(用 chip 显示当前喂了什么)。
   - chat 的回答可**拖到画布变节点** / 一键 add to map;chat 里引用的节点点一下在画布高亮。

10. **战绩可达但不喧宾夺主。**
    - 选中节点 → 侧栏显示它的 litigation history(被丢过什么、你怎么接的),平时不占画布。

### 交互主循环
1. 把半成型论证倒进画布(文本模式倒 / 画布精修)。
2. 标 status:哪些是事实、哪些是赌。
3. 用 LLM:前向推演补下游 / 缺口检测补中间 / **对抗按钮**红队(均为 ghost card)。
4. 对每个攻击**手动判定**(认 / 驳 / 待定,键盘),后果回写图。
5. 看脆弱点:发光的赌 + 带角标的未结攻击节点 + 承重节点。
6. 针对性补证据 / 改结构;战绩沉淀成"我为什么信"。

---

## 8. 待定 / 未来可加(open decisions)

- **对抗按钮的落点**:单个选中节点/边 vs 一键扫全图挑"最该挨打的几个点"。**[未定]**
- **多视角攻击**:是否多个独立 attacker 同时上(一个挑事实、一个挑跳步、一个挑隐藏假设),避免 LLM 老从一个角度钻。
- **辩论树**:驳完之后能否再按按钮让 LLM 攻你的反驳 → challenge 自引用(`parent_challenge_id`),递归。
- **粗粒度强度**:边/节点上"强/弱/存疑"三档(非数字,可排序)。
- **跨图能力**:全库语义搜索("我以前是不是论证过类似的")。

---

## 9. 完成度基线(不砍功能,这是依赖顺序而非取舍)

第一版就是**完整的好体验**,不做残废 MVP。按依赖顺序落地,但每一项都进:
画布(节点/边 CRUD + 顺滑交互 + undo/redo)→ SQLite 真源(全套接缝)→
对抗按钮 + 人判定回写 + 战绩 → 前向推演 → 缺口检测 → 上下文 chat → 本地语义检索。
"先这样以后再说"不在此项目的词典里。

---

## 10. 实现状态(诚实清单,经多 agent review 校准)

**已实现且自检过(cargo test 8 绿 / tsc / vite / cargo build):**
- SQLite 真源:全 schema + WAL + FK 级联 + ULID + 迁移(0001/0002)+ 软删除;`events` 全快照日志
- **多级 undo**(`undo` 命令 + ⌘Z;读最近事件取逆操作)—— redo 暂未做
- 结构分析:承重 / 最弱环节(关节点)/ 下游可达 / 循环论证检测 —— 派生不入库
- 对抗按钮(单点 + 多视角)+ pending 持久化 + 攻击收件箱(键盘 1/2/3,自动聚焦)+ 判定回写
- **晋升**:原子事务 + 仅对已判定(认/驳)+ 幂等(不重复生成节点)
- 前向推演 / 缺口检测(ghost card 暂存,接受才入真源)+ 上下文流式 chat(SSE 按字节解码,中文不乱)+ 每图历史恢复
- 节点删除会**解结其边并把相关 pending 攻击置为 deferred**(不留悬挂)
- 全文搜索改 **trigram** tokenizer(中文可搜);LLM 后端走本机 `claude` CLI(订阅登录,无 key);自动布局(dagre,⌘按钮"整理")
- 语义搜索:in-memory cosine 已接好,真正向量需开 `local-embeddings` feature(否则回退 FTS)
- CSP 收紧(非 null)

**已知简化 / 尚未做(不藏着):**
- **outline ⇄ graph 目前是单向**(文本→图的快速捕获);实时双向同步未做
- staging 的前向/缺口/对抗是**非流式**(一次性返回);ghost card 在侧栏而非贴在节点旁
- **导出 `.argmap.json`**、定时快照备份、子树折叠/展开:未做
- 辩论树(challenge 自引用递归)、粗粒度强度三档:未做(见 §8)
- 本地 embedding 真模型在 feature 后,默认构建不含(自检走 FTS)
