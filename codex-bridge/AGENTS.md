# AlicIA Codex Bridge - AGENTS

## Escopo
- Este subprojeto centraliza o contrato entre `alicia/frontend` e `alicia/backend` para runtime Codex.
- Edicoes manuais devem ficar em `alicia/codex-bridge/**`.
- Arquivos gerados sao derivados do schema unico em `schema/runtime-contract.json`.

## Fluxo de trabalho
1. Atualize `schema/runtime-contract.json` preservando compatibilidade com contratos existentes.
2. Rode o gerador para atualizar artefatos gerados.
3. Rode o checker para validar unicidade e consistencia basica.

## Build/Test minimo
- Gerar artefatos:
  - `node alicia/codex-bridge/generators/generate-runtime-contract.mjs --write-external`
- Validar contrato:
  - `node alicia/codex-bridge/generators/check-runtime-contract.mjs`

## Regras de compatibilidade
- Nao remover metodos/comandos/canais sem plano de migracao.
- Alteracoes de contrato devem explicitar impacto em:
  - metodos de runtime (`runtimeMethods`)
  - comandos Tauri (`tauriCommands`)
  - canais de evento (`tauriEventChannels`)
- Alias de metodos devem permanecer documentados no schema.
