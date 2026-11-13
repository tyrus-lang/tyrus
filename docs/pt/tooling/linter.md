---
title: Linter
description: Referência completa de comandos, configuração, regras e fixes do tyrus-lint.
---

# Linter

`tyrus lint` reporta diagnósticos de estilo, correção, segurança, performance e arquitetura separados dos erros do compilador. O linter lê `tyrus-lint.json` na raiz do projeto quando ele existe; regras omitidas usam a severidade default interna.

## Comandos

<!-- @include: ../../.fragments/tooling/commands.md -->

`--fix` aplica fixes seguros e determinísticos e depois roda lint novamente. `--format json` emite relatórios legíveis por máquina. `--max-warnings 0` é útil para CI. `--explain <rule>` imprime a documentação da regra no terminal. `--init` cria ou atualiza `tyrus-lint.json` sem alterar severidades configuradas. `--default` redefine o arquivo para os defaults internos.

## Configuração

O arquivo é um objeto JSON plano que mapeia nomes de regras para severidades. Não existe `extends`, `presets` nem wrapper `rules`.

<!-- @include: ../../.fragments/tooling/config.md -->

| Severidade | Significado |
|------------|-------------|
| `off` | Regra desativada. |
| `info` | Diagnóstico informativo. |
| `warn` | Warning; não falha CI por default. |
| `error` | Erro; encerra com código `1`. |

## Regras

| Regra | Default | Categoria | Autofix | Resumo |
|-------|---------|-----------|---------|--------|
| `api-no-verb-in-path` | `warn` | style | no | Avisa quando paths de rota usam verbos de ação em vez de paths orientados a recurso. |
| `api-prefer-resource-nouns` | `warn` | style | no | Avisa quando paths de rota não parecem substantivos de recurso. |
| `controller-must-send-response` | `error` | correctness | no | Exige que todo caminho de sucesso do controller envie uma resposta. |
| `controller-no-db-access` | `error` | safety | no | Proíbe acesso direto ao banco em controllers. |
| `controller-no-try-catch` | `warn` | safety | no | Avisa quando controllers capturam erros esperados em vez de delegar o mapeamento. |
| `db-explicit-select-columns` | `info` | style | yes | Prefere projeções explícitas em `SELECT` e expande campos da interface de resultado local. |
| `db-no-extra-select-columns` | `info` | performance | no | Avisa quando uma consulta tipada seleciona colunas que não estão declaradas na interface de resultado. |
| `exhaustive-match` | `error` | correctness | no | Exige que `match` sobre tipos fechados cubra todos os casos. |
| `http-no-raw-status-code` | `warn` | safety | no | Sugere `HttpStatus` em vez de códigos numéricos crus em respostas. |
| `no-duplicate-case` | `error` | correctness | no | Detecta cases duplicados em `switch` ou `match`. |
| `no-duplicate-imports` | `error` | correctness | no | Detecta múltiplas declarações de import do mesmo módulo. |
| `no-empty-block` | `warn` | style | no | Detecta blocos vazios sem comentário explicativo. |
| `no-empty-catch` | `warn` | safety | no | Detecta blocos `catch` vazios sem comentário explicativo. |
| `no-floating-enum` | `error` | correctness | no | Proíbe valores `f64`/`f128` em enums. |
| `no-force-unwrap` | `error` | safety | no | Proíbe unwrap forçado de optional com `expr!`. |
| `no-heterogeneous-enum` | `error` | correctness | no | Proíbe misturar tipos primitivos de valor no mesmo enum. |
| `no-implicit-enum-primitive-cast` | `error` | safety | no | Exige casts `as` explícitos de enum para primitivos. |
| `no-implicit-heavy-move` | `warn` | performance | no | Avisa sobre moves surpreendentes de valores grandes ou non-Copy. |
| `no-layer-skip` | `warn` | safety | no | Avisa quando código API pula a camada route-controller-service-repository. |
| `no-null-literal` | `error` | correctness | no | Proíbe a palavra-chave `null`; use `T?`, campos opcionais e `??`. |
| `no-shadow` | `warn` | style | no | Detecta bindings que sombreiam bindings externos. |
| `no-string-concat-in-loop` | `warn` | performance | no | Detecta concatenação repetida de string dentro de loops. |
| `no-uninitialized-variable` | `warn` | correctness | no | Expõe diagnósticos de definite assignment para `let x: T;`. |
| `no-unnecessary-allocation` | `info` | performance | no | Detecta alocações claramente desnecessárias. |
| `no-unnecessary-await` | `warn` | style | yes | Detecta e corrige delegação redundante com `return await promise`. |
| `no-unnecessary-clone` | `warn` | performance | no | Detecta clones que poderiam ser substituídos por borrows. |
| `no-upward-imports` | `error` | safety | no | Proíbe camadas inferiores da API importarem camadas superiores. |
| `no-unreachable-code` | `error` | correctness | no | Detecta código após `return`, `break`, `continue` ou `throw`. |
| `no-unsafe-finally` | `error` | safety | no | Detecta mudanças de fluxo dentro de `finally`. |
| `no-useless-null-check` | `warn` | correctness | no | Detecta checagens de null em valores que nunca podem ser null. |
| `no-unused-functions` | `warn` | correctness | no | Detecta funções não exportadas que nunca são chamadas. |
| `no-unused-imports` | `warn` | correctness | yes | Detecta e pode remover specifiers de import não usados. |
| `no-unused-vars` | `warn` | correctness | no | Detecta variáveis não usadas; prefixe com `_` quando intencional. |
| `prefer-borrow` | `info` | performance | no | Sugere passar referências quando ownership não é necessário. |
| `prefer-const` | `warn` | style | yes | Sugere `const` para bindings que nunca são reatribuídos. |
| `prefer-nullish-coalescing` | `info` | style | no | Sugere `??` para padrões de fallback nullable. |
| `repository-no-http-error` | `warn` | safety | no | Avisa quando repositories dependem de `HttpReply`. |
| `repository-only-db-access` | `error` | safety | no | Restringe acesso direto ao banco a módulos repository. |
| `route-no-business-logic` | `warn` | complexity | no | Avisa quando módulos de rota contêm lógica de negócio. |
| `service-no-reply` | `error` | safety | no | Proíbe services de dependerem de `HttpReply`. |
| `use-caught-error` | `warn` | safety | no | Detecta erros capturados que nunca são usados. |

## Match exaustivo

`exhaustive-match` exige que um `match` sobre um tipo fechado, uma tagged union, uma union literal ou um booleano, cubra todos os casos. Ele é aplicado pelo compilador como `E0004`, então reporta mesmo com a regra de lint desabilitada.

<!-- @include: ../../.fragments/tooling/exhaustive-match.md -->

## Código nullable

tyrus não usa o literal `null` em código normal. Expresse ausência com `T?`, campos opcionais, parâmetros opcionais e `??`.

<!-- @include: ../../.fragments/tooling/nullable-code.md -->

## Arquitetura de API

Projetos criados ao escolher o target API nas perguntas de `tyrus new` recebem defaults de lint orientados a API. Módulos de rota declaram rotas, controllers traduzem fronteiras HTTP, services contêm comportamento de negócio e repositories concentram acesso direto ao banco.

<!-- @include: ../../.fragments/tooling/api-architecture.md -->

## Quick Fixes

Regras expõem Quick Fixes quando a edição é determinística. `prefer-const` pode reescrever `let` para `const`; `no-unused-imports` pode remover specifiers de import não usados. Regras que exigem uma decisão de design, como renomear uma variável sombreada, reportam ajuda mas não inventam uma edição.

<!-- @include: ../../.fragments/tooling/quick-fixes.md -->

::: warning Regras de arquitetura intencionalmente não têm autofix
Regras como `controller-no-db-access` e `service-no-reply` identificam problemas de design. Elas exigem mover código para a camada correta, não uma edição mecânica.
:::
