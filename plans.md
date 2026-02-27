# Plano de Implementação - Incremento 1 (Monaco ABAP no Alicia)

## Escopo do incremento
1. Criar `plans.md` no root `alicia` com estratégia de implementação.
2. Inserir painel Monaco fixo entre Sidebar e Conversation em `frontend/app/page.tsx`.
3. Criar componente editor ABAP usando comandos existentes `neuroGetSource`/`neuroUpdateSource`.
4. Integrar abertura de arquivo a partir de `fileChanges`/`diffs` já existentes.
5. Definir fallback mobile via `drawer`/`tabs`.

## Contexto atual (baseline)
- Layout desktop atual em `frontend/app/page.tsx` usa 2 colunas: `Sidebar` + área principal (Conversation + Terminal).
- Fluxo de diffs já existe em:
  - `Sidebar` (lista resumida de `fileChanges`)
  - `ReviewMode` (lista de arquivos + diff)
  - `ConversationPane`/`TerminalMessage` via `DiffViewer`
- Bridge Tauri já expõe:
  - `neuroGetSource(objectUri)`
  - `neuroUpdateSource({ objectUri, source, etag })`
  - `neuroSearchObjects(query, maxResults?)` (útil para fallback de resolução)
- Não existe `alicia/codex-bridge` neste checkout (somente `frontend` e `backend`).

## Plano por subprojeto

### 1) Frontend (implementação principal)

#### 1.1. Estruturar estado e contrato do editor no `app/page.tsx`
- Adicionar estado de editor no container principal:
  - arquivo ativo (`objectUri`, `displayName`)
  - conteúdo (`source`), `etag`, `dirty`, `loading`, `saving`, `error`
  - visibilidade (`desktop panel open`, `mobile editor open/tab`)
- Criar handlers de alto nível:
  - `handleOpenSourceFromRef(ref: string)`
  - `handleSaveSource()`
  - `handleCloseEditor()`
- Estratégia de resolução de referência (`ref -> objectUri`):
  - caminho 1: se já vier URI ADT (`/sap/bc/adt/...`) usar direto
  - caminho 2: heurística via nome/arquivo + `neuroSearchObjects`
  - caminho 3: erro explícito quando não resolvido (sem abrir editor)

#### 1.2. Inserir painel Monaco fixo entre Sidebar e Conversation (desktop)
- Reestruturar `ResizablePanelGroup` horizontal para 3 painéis:
  - Painel A: `Sidebar`
  - Painel B: novo `SourceEditorPanel` (fixo, redimensionável)
  - Painel C: área atual (Conversation + Terminal em vertical)
- Manter proporções iniciais conservadoras (ex.: `20 / 28 / 52`) e `minSize` para preservar usabilidade.
- Manter comportamento existente de sessão/review intacto.

#### 1.3. Criar componente do editor ABAP
- Novo componente dedicado (ex.: `components/alicia/source-editor-panel.tsx`) com:
  - header: nome do objeto, status (`loading/saved/dirty/error`)
  - ações: salvar, recarregar, fechar
  - área Monaco com linguagem ABAP (`language="abap"`)
- Usar import dinâmico de Monaco para evitar problemas SSR/Tauri (`next/dynamic`, `ssr: false`).
- Controle de dirty state:
  - `dirty=true` ao editar
  - `dirty=false` após save bem-sucedido
- Save com controle de concorrência por `etag`:
  - enviar `etag` atual em `neuroUpdateSource`
  - atualizar `etag` retornado
  - tratar conflito/erro de safety com mensagem clara

#### 1.4. Integrar abertura a partir de `fileChanges`/`diffs`
- Introduzir callback único de abertura (ex.: `onOpenInEditor(pathOrUri: string)`) propagado para:
  - `Sidebar` (itens de `fileChanges`)
  - `ReviewMode` (lista de arquivos e arquivo selecionado)
  - `DiffViewer` (header `filename` clicável)
  - `ConversationPane`/`TerminalMessage` (apenas encaminhamento do callback para `DiffViewer`)
- Regra funcional: qualquer origem (file change ou diff) dispara mesma função de resolução/abertura.

#### 1.5. Fallback mobile (drawer/tab)
- Reutilizar `useIsMobile` já existente.
- Mobile: não renderizar painel fixo entre Sidebar e Conversation.
- Expor editor via fallback:
  - opção recomendada: `Tabs` (`Chat`, `Editor`, `Terminal`) no conteúdo principal
  - opção complementar: `Drawer` para abrir editor sobreposto quando acionado por arquivo/diff
- Garantir comportamento consistente em teclado virtual (sem esconder ações primárias de salvar/fechar).

### 2) Backend (`alicia/backend`)
- Incremento 1: sem alteração obrigatória (comandos necessários já existem e estão registrados em `main.rs`).
- Contrato consumido pelo frontend:
  - `neuro_get_source` retorna `{ objectUri, source, etag? }`
  - `neuro_update_source` recebe `{ objectUri, source, etag? }` e retorna novo `etag?`
- Observação de risco: se resolução `path -> objectUri` via frontend ficar frágil, considerar incremento 2 com comando backend dedicado para resolução canônica.

### 3) Codex-bridge (`alicia/codex-bridge`)
- Não aplicável neste repositório: diretório não existe no checkout atual.
- Ação: registrar explicitamente como “sem impacto neste incremento” para evitar trabalho fantasma.

## Arquivos prováveis a alterar

### Frontend (alta probabilidade)
- `frontend/app/page.tsx`
- `frontend/components/alicia/diff-viewer.tsx`
- `frontend/components/alicia/sidebar.tsx`
- `frontend/components/alicia/review-mode.tsx`
- `frontend/components/alicia/conversation-pane.tsx`
- `frontend/components/alicia/terminal-message.tsx`
- `frontend/package.json` (dependências Monaco)

### Frontend (novos arquivos prováveis)
- `frontend/components/alicia/source-editor-panel.tsx`
- `frontend/components/alicia/source-editor-mobile.tsx` (se separar fallback)
- `frontend/lib/abap-source-resolver.ts` (resolver `path/diff -> objectUri`)

### Backend
- Sem mudanças previstas no incremento 1.

### Codex-bridge
- Sem mudanças (subprojeto ausente).

## Decisões técnicas propostas
1. **Um único ponto de verdade para abertura**: toda abertura por `fileChanges`/`diffs` converge em `handleOpenSourceFromRef` no `page.tsx`.
2. **Editor controlado por container**: estado de carregamento/salvamento no `page.tsx`, componente visual stateless onde possível.
3. **Monaco com carregamento dinâmico**: evita SSR mismatch e reduz risco em Tauri/Next.
4. **Persistência otimista com ETag**: sempre salvar com `etag` quando disponível para evitar sobrescrita cega.
5. **Fallback mobile explícito**: desktop com painel fixo; mobile com `Tabs` (e opcional `Drawer`) para preservar ergonomia.
6. **Sem alteração backend no incremento 1**: aproveitar contratos já existentes, postergar nova API de resolução para incremento 2 se necessário.

## Riscos técnicos e de regressão
1. **Resolução ambígua de referência para `objectUri`**
   - Sintoma: arquivo/diff abre objeto errado ou não abre.
   - Mitigação: heurística conservadora + exigir match único; quando ambíguo, bloquear abertura com erro orientativo.
2. **Conflito de atualização por ETag/safety policy**
   - Sintoma: `neuroUpdateSource` falha em save.
   - Mitigação: mensagens explícitas; opção de recarregar fonte antes de salvar novamente.
3. **Peso do Monaco e impacto de performance**
   - Sintoma: tempo de abertura da UI maior.
   - Mitigação: import dinâmico/lazy + render sob demanda.
4. **Regressão de layout/resizable no desktop**
   - Sintoma: Conversation/Terminal ficam comprimidos ou quebram min-height.
   - Mitigação: ajustar `defaultSize/minSize` e validar resize extremo.
5. **Regressão mobile**
   - Sintoma: ações de chat ficam inacessíveis com editor aberto.
   - Mitigação: tabs claras e botão de retorno/fechar sempre visível.
6. **Quebra de compatibilidade de props em componentes existentes**
   - Sintoma: erro TS ao propagar callbacks novos (`onOpenInEditor`).
   - Mitigação: introduzir props opcionais com fallback no-op.

## Checklist de validação objetiva

### Build/Lint
- [ ] `cd alicia/frontend && pnpm lint` sem erros.
- [ ] `cd alicia/frontend && pnpm build` sem erros de tipo/SSR.

### Desktop UX
- [ ] Painel do editor aparece fixo entre Sidebar e Conversation no desktop.
- [ ] Redimensionamento horizontal funciona sem colapsar Conversation/Terminal.
- [ ] Abrir arquivo por item de `fileChanges` funciona.
- [ ] Abrir arquivo por `DiffViewer` (conversation/review) funciona.
- [ ] `neuroGetSource` carrega fonte e `etag` corretamente.
- [ ] Editar + salvar chama `neuroUpdateSource` e limpa estado dirty.
- [ ] Erro de save (ex.: conflito/credencial) é exibido de forma clara.

### Mobile UX
- [ ] Em viewport `<768px`, painel fixo não é renderizado.
- [ ] Fallback via `tabs`/`drawer` permite abrir, editar e salvar.
- [ ] Navegação entre chat/editor/terminal permanece funcional com teclado virtual.

### Regressão funcional
- [ ] Fluxo de sessão (start/stop/resume/fork) continua intacto.
- [ ] `ReviewMode` continua executando `/review` e `/review-file` normalmente.
- [ ] Terminal tabs e envio de comandos continuam sem regressão.

## Ordem de execução recomendada (incremento 1)
1. Implementar estado/handlers no `page.tsx`.
2. Criar componente `SourceEditorPanel` + integração Monaco.
3. Reestruturar layout desktop (3 painéis).
4. Integrar callback de abertura em `Sidebar`, `ReviewMode`, `DiffViewer`, `ConversationPane`, `TerminalMessage`.
5. Implementar fallback mobile (`Tabs`/`Drawer`).
6. Validar com checklist e ajustar sizing/ergonomia.

## Fora do escopo deste incremento
- Navegador ABAP completo (árvore/pacotes) dentro do Alicia.
- Resolução backend canônica `path -> objectUri` (apenas se heurística front falhar no incremento 2).
- Merge/compare avançado de versões no editor ABAP.
