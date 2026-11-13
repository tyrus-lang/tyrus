---
title: REPL
description: Console interativo para explorar, prototipar e aprender tyrus.
---

# REPL

REPL significa **Read-Eval-Print Loop**: um programa que lê uma entrada, avalia, imprime o resultado e repete.

`tyrus repl` inicia um console interativo onde você escreve e executa código tyrus linha por linha. É útil para explorar APIs, testar expressões e aprender a linguagem sem criar um projeto.

## Inicie o REPL

<!-- @include: ../../.fragments/tooling/repl-start.md -->

A transcrição acima mostra o banner atual de inicialização e a saída exata de `:help`.

O REPL exige um terminal interativo. Ele se recusa a executar em ambiente não TTY, como um pipe.

## Recursos

### Realce de sintaxe

O código é realçado em tempo real enquanto você digita, espelhando o esquema de cores clássico do Monokai: keywords de controle, operadores e a maioria das outras keywords em rosa/vermelho, keywords de armazenamento/tipo (`const`, `let`, tipos primitivos, `interface`/`enum`/`type`, `=>`) em ciano, strings e template literals em amarelo, números e booleanos em roxo, chamadas de função em verde, identificadores em branco-acinzentado e comentários em cinza.

### Auto-completion

Pressione Tab para completar comandos, keywords, tipos primitivos e símbolos definidos na sessão atual. Comandos são completados após digitar `:` (ex.: `:lo` → `:load `). Caminhos de arquivo são completados para `:load` e `:restore`; `:restore` também completa os nomes de sessões salvas anteriormente. Em acesso a membros, os membros são completados após `.` ou `?.`: digitar `console.` sugere `log`, `error` e `warn`, e o nome de um enum da sessão ou de um import nativo (ex.: `HttpStatus.`) sugere seus membros.

### Entrada multi-linha

O REPL detecta entrada incompleta com o parser. Quando uma linha termina no meio de uma construção (por exemplo um `{`, `[` ou `(` sem fechamento), o prompt muda para `.... ` para continuação. Um erro de parse real, como um `;` solto dentro de uma chamada não fechada, é reportado imediatamente, sem esperar por mais linhas. Pressione Enter em uma linha vazia para avaliar a entrada acumulada.

### Histórico de comandos

Comandos anteriores são salvos em `~/.tyrus_repl_history` (até 5000 entradas) e ficam disponíveis com as setas Up/Down. O arquivo de histórico persiste entre sessões.

## Comandos

Todo meta-comando começa com `:`.

| Comando | Finalidade |
|---------|------------|
| `:help` | Mostra comandos disponíveis |
| `:clear` | Limpa a tela e o buffer da sessão |
| `:load <file>` | Carrega e executa um arquivo `.tyr` |
| `:list` | Lista sessões salvas via `:save` |
| `:type <expr>` | Mostra o tipo inferido de uma expressão |
| `:doc <symbol>` | Mostra documentação de um símbolo |
| `:edit` | Edita o buffer da sessão no editor externo (`$EDITOR`) |
| `:save <name.tyr>` | Salva o buffer da sessão em `~/.tyrus/repl/<name>.tyr` |
| `:restore <name\|file> [--merge\|-m]` | Restaura uma sessão salva ou um arquivo, reiniciando a sessão antes (valida o código) |
| `:delete <name>` | Exclui uma sessão salva de `~/.tyrus/repl` |
| `:reset` | Reinicia a sessão inteira |
| `:exit` | Sai do REPL |
| `:quit` | Alias para `:exit` |
| `:q` | Alias para `:exit` |

## Gerenciamento de sessão

O REPL preserva o estado da sessão entre comandos. Variáveis, funções e aliases de tipo definidos durante a sessão permanecem disponíveis conforme você insere novo código.

Sessões são arquivos `.tyr` armazenados em `~/.tyrus/repl/`, um por sessão salva, de modo que sobrevivem entre invocações do REPL e independem do diretório atual. `:save <name.tyr>` grava o buffer da sessão em `~/.tyrus/repl/<name>.tyr` e exige a extensão `.tyr`; `:restore <name>` recarrega uma sessão salva (a extensão é opcional, e um caminho de arquivo existente também funciona) e a reproduz como foi digitada: cada declaração é ecoada como uma transcrição `tyrus> ` com syntax highlighting e depois executada, de modo que cada saída do programa aparece logo abaixo da linha que a produziu. Por padrão `:restore` reinicia a sessão antes, de modo que o código restaurado sempre roda como estava no momento do `:save`; passe `--merge` (ou `-m`) para manter o buffer atual e mesclar o código restaurado nele. Com `--merge`, um nome já definido em qualquer um dos lados aborta o restore com um erro antes que qualquer coisa seja executada, deixando a sessão inalterada, resolva o conflito com `:reset` e tente novamente. `:delete <name>` remove uma sessão salva (a extensão também é opcional). `:list` mostra as sessões salvas com quantidade de linhas, tamanho e quando foram salvas, da mais recente para a mais antiga. Se uma unidade restaurada contiver código inválido, o restore é rejeitado e a sessão não é alterada.

`:load <file>` reproduz um arquivo do projeto na sessão atual do mesmo modo que `:restore`: cada declaração é ecoada como uma transcrição `tyrus> ` e depois executada, em vez de apenas relatar uma quantidade de linhas.

`:reset` limpa o buffer e retorna ao estado inicial. `:clear` limpa a tela sem descartar o buffer.

## Inspeção de tipo

O comando `:type` mostra o tipo inferido de qualquer expressão sem executá-la:

<!-- @include: ../../.fragments/tooling/repl/type-inspection.md -->

## Consulta de documentação

O comando `:doc` exibe documentação de símbolos definidos na sessão, incluindo nomes de parâmetros, tipos de retorno, anotações `@param`, `@returns` e `@deprecated`, e exemplos de uso:

<!-- @include: ../../.fragments/tooling/repl/documentation-lookup.md -->

## Editor externo

O comando `:edit` abre o buffer da sessão no editor especificado por `$EDITOR` ou `$VISUAL` (padrão: `nano`). Após o editor sair, o buffer modificado é compilado, se houver erros, a sessão reverte ao estado anterior.

## Compilação

Cada entrada é compilada do zero usando o compilador JIT LLVM em processo, o que torna a avaliação instantânea. Quando o JIT não está disponível (sem runtime LLVM), o REPL cai para o modo compilado via clang.

O buffer da sessão é envolvido em uma função `main` (`const main = (): void => { ... };`) para que declarações de função e bindings de variáveis coexistam no mesmo escopo.
