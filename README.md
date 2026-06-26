# reason-map

本地论证地图工具:把不铁证如山的推理链摊到画布上,让 Claude 帮你**推进**(前向推演 / 缺口检测)和**对抗**(红队攻击),判定权始终在你手里。设计见 [`SPEC.md`](./SPEC.md)。

## 技术栈

- **Tauri 2**(Rust 内核 + 系统 webview)— 原生桌面 app
- **React + TypeScript + React Flow** — 画布
- **SQLite**(Rust 侧 `sqlx`,WAL + FTS5 + sqlite-vec)— 本地真源
- **官方 Anthropic API**(`claude-opus-4-8`,streaming + adaptive thinking)— API key 存 OS keychain
- 本地 embedding(fastembed/ONNX)在 `local-embeddings` feature 后

## 开发

```bash
npm install
npm run build              # tsc --noEmit + vite build(前端自检)
cd src-tauri && cargo test # 后端单测 + repo round-trip
npm run tauri dev          # 跑起来(需先装 Tauri 系统依赖)
```

首次启动:在设置(⚙)里填 Anthropic API key;会自动载入一张示例论证图。

## 结构

```
src/                     前端(React/TS)
  types/domain.ts        与 Rust serde 对齐的类型
  lib/ipc.ts             Tauri 命令的类型化封装
  state/store.ts         zustand store
  components/            canvas / outline / chat / challenges / inspector / staging
src-tauri/src/           后端(Rust)
  domain/                领域模型(enum + struct)
  db/  migrations/       SQLite 连接 + schema
  repo/                  持久化(每实体一文件)
  analysis/              结构分析(承重 / 最弱环节 / 循环论证)— 派生,不入库
  llm/                   Anthropic 客户端 + prompts(前向/缺口/对抗/对话)
  embeddings/            Embedder trait(本地模型在 feature 后)
  secrets.rs             keychain
  commands/              Tauri 命令面
```
