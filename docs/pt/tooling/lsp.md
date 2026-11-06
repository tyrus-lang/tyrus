---
title: Language Server Protocol
description: Visão geral do suporte de editor do Tyrus por meio do Language Server Protocol.
---

# Language Server Protocol

O Language Server do Tyrus fornece inteligência compartilhada de editor para integrações com VS Code, Neovim, Zed, Kate e IntelliJ/WebStorm.

## Recursos

- Diagnósticos
- Hover
- Completion
- Signature help
- Code actions
- Semantic tokens

Dentro de um arquivo de teste, completion e hover cobrem a superfície do `tyrus:test`: os matchers depois de `expect(...)`, os modificadores `.not`, `.rejects` e `.resolves`, e os modificadores depois de `test.`, `it.` ou `describe.`.

Dentro de um literal de string SQL de `db.query`, completion sugere tabelas e colunas do snapshot local do schema. O hover em uma tabela ou coluna mostra informações do schema, inclusive a opcionalidade criada por um join externo. O compilador e o Language Server usam o mesmo snapshot para os diagnósticos SQL.

## Inicie o Server

<!-- @include: ../../.fragments/tooling/lsp-start-server.md -->

O server se comunica por stdio. Integrações de editor devem iniciar `tyrus lsp` e depender dos diagnósticos compartilhados do compilador em vez de duplicar regras da linguagem em cada cliente de editor.

## Diagnósticos e Fixes

Diagnósticos do compilador e do linter devem preservar códigos estáveis, ranges precisos, informações relacionadas e ajuda concreta. Quando o compilador ou linter tiver uma edição determinística, o Language Server deve expô-la como code action.

## Navegação e Quick Fixes

Use o comando Go to Definition do editor sobre um identificador para navegar até declarações locais ou importadas. Para aplicar uma correção determinística, invoque Quick Fix sobre um diagnóstico pelo comando de code action ou pelo ícone de lâmpada do editor. O server publica as edições por code actions do LSP, enquanto os atalhos dependem do editor.

## Integrações de Editores

### VS Code

Instale a [extensão do VS Code](./vscode.md). Ela inicia o servidor compartilhado `tyrus lsp` para arquivos `.tyr`.

### Neovim

Configure o cliente LSP do Neovim para executar `tyrus lsp` no filetype `tyrus`. A integração do Neovim no repositório fornece detecção de arquivos e syntax highlighting.

### Zed

Instale a extensão de desenvolvimento do Tyrus e configure-a para iniciar `tyrus lsp`. A integração atual do Zed é um scaffold de desenvolvimento.

### IntelliJ e WebStorm

Instale o LSP4IJ, registre `tyrus lsp` como language server e associe arquivos `*.tyr` a ele. A grammar TextMate do VS Code pode fornecer syntax highlighting.

### Kate

Habilite o LSP Client integrado do Kate e registre `tyrus lsp`. A configuração detalhada do Kate está abaixo.

## Kate Configuration

O Kate suporta o Language Server Protocol por meio do plugin LSP Client integrado. O Tyrus se integra ao Kate pelo mesmo server `tyrus lsp` usado por todos os outros editores.

### Pré-requisitos

- Tyrus instalado e `tyrus` acessível no `PATH`.
- Kate com o plugin LSP Client (incluído por padrão na maioria das distribuições).

### Configuração do LSP

Abra o Kate e habilite **Settings > Configure Kate > Plugins > LSP Client**. Depois vá em **Settings > Configure Kate > LSP Client > User Server Settings** e defina **Settings File** com o arquivo de configuração. Apenas criar o arquivo JSON não faz o Kate carregá-lo.

Use o caminho correspondente à sua instalação:

| Instalação | Arquivo de configuração |
|---|---|
| Linux (nativo) | `$HOME/.config/kate/lspclient/settings.json` |
| Snap (`classic`) | `$HOME/.config/kate/lspclient/settings.json` |
| Snap (confinamento estrito) | `$HOME/snap/kate/current/.config/kate/lspclient/settings.json` |

Execute `snap list kate` e verifique a coluna **Notes** para distinguir os dois modos do Snap. O Snap oficial do Kate atualmente usa `classic`, portanto lê os mesmos diretórios do usuário que uma instalação nativa.

Adicione uma entrada para o Tyrus:

<!-- @include: ../../.fragments/tooling/lsp-kate-settings.md -->

| Campo | Finalidade |
|---|---|
| `command` | Binário e subcomando que iniciam o LSP. |
| `path` | Caminhos adicionais para procurar executáveis. Isso permite que o Kate iniciado pela interface encontre o binário do Tyrus mesmo quando o `PATH` herdado não inclui o diretório de instalação. |
| `highlightingModeRegex` | Deve corresponder ao nome da definição de syntax highlighting do Tyrus instalada no Kate. |
| `rootIndicationFileNames` | Arquivos que sinalizam a raiz do projeto. O Kate percorre a árvore de diretórios procurando por um deles. |

### Syntax Highlighting

O Kate usa definições de syntax baseadas em XML. A definição do Tyrus é gerada pelo crate xtask a partir da mesma fonte de keywords dos outros editores. Em uma instalação Linux nativa, execute o helper do repositório para copiar o arquivo gerado:

<!-- @include: ../../.fragments/tooling/kate-install-syntax.md -->

Defina `KATE_SYNTAX_DIR` ao usar outro diretório de dados do Kate. Os caminhos compatíveis são:

| Plataforma | Caminho |
|---|---|
| Linux (nativo) | `$HOME/.local/share/org.kde.syntax-highlighting/syntax/` |
| Flatpak | `$HOME/.var/app/*package-name*/data/org.kde.syntax-highlighting/syntax/` |
| Snap (`classic`) | `$HOME/.local/share/org.kde.syntax-highlighting/syntax/` |
| Snap (confinamento estrito) | `$HOME/snap/kate/current/.local/share/org.kde.syntax-highlighting/syntax/` |
| Windows | `%USERPROFILE%\AppData\Local\org.kde.syntax-highlighting\syntax\` |
| macOS | `$HOME/Library/Application Support/org.kde.syntax-highlighting/syntax/` |

Reabra o Kate após instalar ou atualizar o arquivo. Se um arquivo `.tyr` já estiver aberto, vá em **Tools > Editing Tools > Refresh Syntax Highlighting** e confirme que o modo de highlighting é **Tyrus**. A entrada `highlightingModeRegex` ativa o server apenas para documentos que usam exatamente esse modo.

### Verificação

Abra um arquivo `.tyr` dentro de um diretório que contenha `tyrus.json`. O LSP inicia automaticamente e fornece diagnósticos, completion, hover e outros recursos. O Kate não mostra um indicador persistente de conexão, e um arquivo válido pode deixar o painel de diagnósticos vazio. Teste a conexão com hover ou completion e use **View > Tool Views > Diagnostics** e **Symbol Outline** para resultados visíveis. Após alterar o JSON, use **Tools > LSP Client > Restart LSP Server** ou reinicie o Kate.
