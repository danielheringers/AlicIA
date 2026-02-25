# Alicia Desktop (Tauri + Next.js)

Aplicativo desktop Alicia com frontend Next.js e backend Tauri/Rust.

## Estrutura

- `frontend/`: UI Next.js
- `backend/`: runtime Tauri (com integracao Codex + Neuro)
- `docs/`: runbooks operacionais

## Dependencia de repositorio irmao (`codex`)

O backend usa dependencias `path` para crates em `../codex/codex-rs/*`.
Por isso, a estrutura esperada e:

```text
Neuromancer/
  codex/
  alicia/
```

Se `codex` nao estiver nesse caminho relativo, o build do backend falha.

## Pre-requisitos

- Node.js 20+
- pnpm 9+
- Rust toolchain (`rustup`, `cargo`)
- Dependencias de build do Tauri para seu SO

## 1) Build do `codex` para uso da Alicia

No diretorio `codex/codex-rs`:

```powershell
cargo build -p codex-core -p codex-protocol -p codex-app-server-protocol -p codex-rmcp-client -p neuro-engine -p neuro-types -p neuro-adt-core -p neuro-adt-ws
```

## 2) Rodar Alicia em desenvolvimento

No diretorio `alicia/`:

```powershell
pnpm run setup
pnpm run dev
```

Isso inicia:

- Next.js em `http://localhost:3000`
- runtime Rust/Tauri do `backend/`
- janela desktop Alicia conectada via Tauri IPC

## 3) Build da Alicia (desktop)

No diretorio `alicia/`:

```powershell
pnpm run build
```

## Validacoes uteis

No diretorio `alicia/backend`:

```powershell
cargo check
```

No diretorio `codex/codex-rs`:

```powershell
cargo test -p neuro-types -p neuro-engine
```

## Neuro runbook

Para setup de ambiente, execucao e diagnostico do runtime Neuro:

- [`docs/neuro-runbook.md`](./docs/neuro-runbook.md)
