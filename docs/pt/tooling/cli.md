---
title: CLI
description: Interface de linha de comando do tyrus para scaffold, check, build, run, formatter, linter e integração com editor.
---

# CLI

A CLI Tyrus compila, executa, formata, analisa e cria scaffolds de projetos Tyrus.

## Criar um projeto

<!-- @include: ../../.fragments/tooling/cli-create-project.md -->

Execute `tyrus new` sem argumentos. A CLI pergunta o nome do projeto e, em seguida, o target.

<!-- @include: ../../.fragments/tooling/cli-target-flags.md -->

Para o target API, a CLI pergunta se deve configurar nenhum banco, SQLite, PostgreSQL, MySQL ou MongoDB. Ao escolher SQLite, o comando grava as opções recomendadas em `database.sqlite` e cria a pasta `data/`. Os backends servidor usam `DATABASE_URL` em `connectionEnv`; o scaffold existente de `.env` e `.env.example` inclui essa variável.

O `tyrus new` também escreve `__tests__/` com um teste que passa, então o `tyrus test` reporta um resultado já na primeira execução.

::: tip Projetos API
Ao escolher o target API nas perguntas de `tyrus new`, a CLI cria uma estrutura orientada a API e defaults de lint para camadas route, controller, service e repository. A CLI também pergunta qual SGBD configurar; escolher nenhum banco deixa essa configuração fora do `tyrus.json`.
:::

## Comandos comuns

<!-- @include: ../../.fragments/tooling/cli-common-commands.md -->

## Comandos de projeto

| Comando | Finalidade |
|---------|------------|
| `tyrus new` | Cria um projeto por meio de perguntas sobre o nome e o target. |
| `tyrus --init` | Adiciona chaves default ausentes em `tyrus.json` preservando valores existentes. |
| `tyrus --default` | Restaura os valores padrão gerados para o projeto em `tyrus.json`, removendo scripts e configurações personalizados. |
| `tyrus info` | Imprime metadados do projeto. |
| `tyrus clean [--cache] [--dry-run] [--force]` | Remove o diretório `target/` do projeto e, opcionalmente, o cache global do Tyrus. |
| `tyrus check` | Faz parse, type-check e borrow-check sem gerar binário nativo. |
| `tyrus build [--release] [--rotate-seal-key] [--pgo-generate\|--pgo-use]` | Compila o projeto. |
| `tyrus pgo-merge` | Funde os perfis que uma execução instrumentada escreveu num arquivo só. |
| `tyrus run [<path>\|<script>] [--release] [--rotate-seal-key] [-- <args>...]` | Executa um script de `tyrus.json` pelo nome, ou compila e executa `src/main.tyr` ou um arquivo fonte independente; argumentos depois de `--` são repassados ao script ou programa. |
| `tyrus test [<path>]` | Executa todos os arquivos `*.test.tyr` e reporta os resultados. |
| `tyrus watch [--run\|--test\|--command <cmd>] [--debounce <ms>] [--clear]` | Compila de novo sempre que um arquivo `.tyr` ou o `tyrus.json` muda; `--run` inicia o programa depois de cada build bem-sucedido, `--test` executa os testes e `--command` executa um comando de shell no lugar. |
| `tyrus dev [--debounce <ms>] [--clear]` | Compila de novo e reinicia o projeto depois de cada mudança de fonte usando o mesmo watcher e a mesma substituição graciosa de processo de `tyrus watch --run`. |
| `tyrus db introspect` | Lê o banco SQLite, PostgreSQL ou MySQL configurado em modo somente leitura e grava atomicamente `tyrus-db-schema.json`. |
| `tyrus db check` | Compara o schema SQLite, PostgreSQL ou MySQL atual com o snapshot sem alterar o banco. |
| `tyrus db diff` | Mostra diferenças entre o snapshot do schema e o banco SQL ativo sem gravar em nenhum schema; também compara os Models do projeto com o snapshot nos três backends SQL. |
| `tyrus db explain-sql [SQL]` | Valida uma instrução SQL com o snapshot local e exibe o SQL normalizado, os tipos de parâmetros inferidos e os campos de resultado com valores ocultos. Passe SQL entre aspas como argumento ou pelo stdin. |
| `tyrus db migrate create <name>` | Cria o próximo arquivo SQL numerado com um comentário inicial específico para SQLite, PostgreSQL ou MySQL. |
| `tyrus db migrate generate <name>` | Gera migrations SQLite a partir de declarações `@model` ou a migration inicial PostgreSQL/MySQL a partir de campos Model aceitos. Revise o SQL antes de aplicá-la. |
| `tyrus db migrate status` | Lista migrations aplicadas e pendentes em SQLite, PostgreSQL ou MySQL sem alterar o banco. |
| `tyrus db migrate apply` | Aplica migrations pendentes em transação SQLite/PostgreSQL ou com marcador de recuperação no MySQL, e registra checksums. |
| `tyrus db seed status [name]` | Lista programas de seed Tyrus aplicados e pendentes sem alterar o banco. |
| `tyrus db apply <name.tyr>` | Executa um programa de seed de `db/seeds/` e registra seu checksum. |

`tyrus clean` remove somente o diretório `target/` do projeto. O comando pede confirmação em um terminal interativo; use `--force` para ignorar a pergunta, e execuções sem terminal exigem essa opção antes de apagar qualquer coisa. Adicione `--dry-run` para estimar o espaço que seria liberado sem alterar arquivos. Adicione `--cache` para incluir `~/.tyrus/cache/`; o Tyrus avisa que esse cache é compartilhado entre projetos e pede confirmação separadamente.

Passe um caminho de fonte para `tyrus run` para executar um arquivo separado sem alterar `src/main.tyr`. Isso é útil para experimentos rápidos e exemplos que contêm statements de topo:

<!-- @include: ../../.fragments/tooling/cli/project-commands.md -->

Sem um caminho, `tyrus run` continua compilando e executando `src/main.tyr`.

Projetos gerados incluem as entradas `fmt`, `lint` e `test` em `scripts` no `tyrus.json`. Declare scripts adicionais do projeto como valores string e execute um pelo nome com `tyrus run <name>`, por exemplo `tyrus run example`; argumentos para o comando configurado vêm depois de `--`. `tyrus --init` preserva scripts personalizados ao adicionar defaults ausentes, e `tyrus --default` substitui o manifesto pelos defaults gerados para o projeto.

`tyrus watch` observa `src/` (e `tests/` com `--test`) e espera `--debounce` milissegundos, 300 por padrão, para que um save que toca vários arquivos inicie um único ciclo. Um build que falha imprime seus diagnósticos e continua observando. Com `--run`, o programa do ciclo anterior é encerrado antes do próximo build, então um servidor libera sua porta. `target/` e `.git` são sempre ignorados; liste outros caminhos num arquivo `.tyrusignore` na raiz do projeto, um por linha, onde `src/generated` ignora esse diretório e um nome solto como `fixtures` ignora qualquer diretório com esse nome. As linhas de status vão para o stderr, então a saída do programa ou do comando continua no stdout.

`tyrus dev` é o atalho focado em desenvolvimento para `tyrus watch --run`: aceita apenas `--debounce` e `--clear`, recompila depois de mudanças em `src/` ou no `tyrus.json`, encerra o programa anterior antes de iniciar a substituição e mantém o status do watcher no stderr.

## Comandos de schema do banco

Configure um caminho SQLite relativo ao projeto em `tyrus.json`. A configuração não possui campos de credenciais, e os caminhos do banco e do snapshot precisam permanecer dentro do diretório do projeto.

<!-- @include: ../../.fragments/tooling/cli/db-config.md -->

No SQLite, `database.sqlite.journalMode` aceita `wal` ou `delete`, `synchronous` aceita `full` ou `normal`, e `busyTimeoutMs` aceita valores de 1 a 120000. Os padrões são WAL, FULL e 5000 ms; `:memory:` sempre usa o journal de memória do SQLite. `database.sqlite` é rejeitado nos outros backends. WAL exige suporte compatível a memória compartilhada e bloqueios e não funciona em sistemas de arquivos de rede; o Tyrus confirma que o SQLite aplicou o modo solicitado, mas não consegue verificar os bloqueios do sistema de arquivos nem as garantias de sincronização do armazenamento. No modo WAL, FULL sincroniza cada confirmação de transação, enquanto NORMAL sincroniza com menos frequência e pode perder confirmações recentes após uma queda de energia, embora preserve a consistência do banco. O SQLite faz checkpoints automaticamente durante a operação normal e quando a última conexão é fechada. Não copie apenas o arquivo principal do banco enquanto ele estiver aberto; use uma operação de backup consistente do SQLite. Consulte a [documentação do WAL do SQLite](https://www.sqlite.org/wal.html) e a [documentação da pragma synchronous](https://www.sqlite.org/pragma.html#pragma_synchronous) para conhecer as garantias e compensações de armazenamento.

Execute `tyrus db introspect` quando o schema mudar para criar ou atualizar o snapshot versionado. O comando aceita SQLite, PostgreSQL e MySQL e abre o banco configurado em modo somente leitura. Builds e o Language Server permanecem offline e não se conectam ao banco. Execute `tyrus db check` na CI ou antes de um build para detectar snapshot ausente ou desatualizado. `tyrus db diff` compara o banco ativo com o snapshot e compara os Models do projeto com o snapshot para SQLite, PostgreSQL e MySQL.

Use `tyrus db diff` para revisar diferenças entre os Models do projeto, o snapshot local e o banco atual antes de alterar qualquer um deles. O comando lista mudanças nas tabelas e colunas dos Models, na versão do schema SQLite, nos tipos lógico e nativo, na nulabilidade, na expressão default e no engine da tabela, além de alterações em chaves primárias, constraints unique, foreign keys e índices. A comparação com Models é somente leitura; a geração de migrations destrutivas ou in-place continua exclusiva do SQLite. `tyrus db check` continua sendo o comando de CI que falha quando há drift no banco.

Passe uma instrução SQL entre aspas para `tyrus db explain-sql` ou redirecione-a pelo stdin para inspecionar o SQL normalizado, os tipos de parâmetros inferidos e os campos de resultado sem abrir uma conexão com o banco. Para `SELECT`, cada campo de resultado inclui tipo e opcionalidade; mutações mostram `void`. Literais string e numéricos são ocultados no SQL exibido, e os valores vinculados nunca são exibidos.

<!-- @include: ../../.fragments/tooling/cli/db-explain.md -->

## Migrations SQLite

Crie uma migration, edite seu SQL, consulte a lista de pendências e aplique-a explicitamente. Use `tyrus db migrate generate <name>` para criar instruções iniciais `CREATE TABLE` quando o banco configurado ainda não existir. Para um banco existente, ele pode criar tabelas para Models novos, instruções aditivas seguras `ALTER TABLE ... ADD COLUMN` para campos opcionais ou com valores padrão literais e reconstruir uma tabela quando um Model remove colunas ou altera a chave primária ou constraints `@unique`. A reconstrução copia as colunas mantidas e restaura as definições de índices personalizados a partir do catálogo SQLite. Ela é recusada se uma coluna removida for usada por um índice ou se chaves estrangeiras, views, triggers ou adições simultâneas impedirem a preservação segura. A aplicação pode falhar se os registros existentes violarem uma nova constraint de chave; a transação SQLite será revertida. A geração incremental exige um snapshot atual do schema. Ela recusa divergências no schema e alterações em definições de colunas existentes ou constraints de chaves estrangeiras; escreva essas migrations manualmente. Tipos sem mapeamento sem perda para SQLite são recusados. A geração cria um arquivo para revisão e nunca altera o banco. Atualize o snapshot do schema depois de aplicar uma migration de schema.

<!-- @include: ../../.fragments/tooling/cli/db-migrate.md -->

Para PostgreSQL e MySQL, `tyrus db migrate generate <name>` grava schemas iniciais sem se conectar ao servidor. Depois disso, conecta-se ao servidor configurado, verifica o schema ativo contra o snapshot e gera novas tabelas de Model ou colunas aditivas seguras; novas colunas únicas devem ser opcionais e não ter default, e PostgreSQL e MySQL podem adicionar colunas opcionais com chave estrangeira quando a tabela de destino, a chave única e o tipo compatível constarem no snapshot; o MySQL também exige InnoDB nas duas tabelas e tipos inteiros assinados correspondentes registrados pela introspecção. Outras mudanças de chave, alterações em definições existentes e operações destrutivas exigem SQL escrito manualmente. Colunas adicionadas devem ser opcionais ou ter um default literal, e Models com defaults existentes ou metadados de chave estrangeira são recusados preventivamente na geração incremental. O comando recusa `@timestamps` em Models existentes e tipos de campo sem suporte. `status` e `apply` também aceitam os dois backends de servidor; PostgreSQL usa uma transação e lock advisory, enquanto MySQL registra um marcador de recuperação porque DDL pode confirmar implicitamente.

Os arquivos usam nomes como `0001_add_users.sql`. Cada arquivo pode conter várias instruções SQLite. A CLI registra um checksum SHA-256 em `_tyrus_migrations`, recusa arquivos aplicados alterados ou ausentes e versões pendentes anteriores a uma versão aplicada, além de serializar aplicações concorrentes com um lock de escrita do SQLite. Uma falha reverte todo o lote pendente. `status` abre um banco existente em modo somente leitura; nem `status` nem um build aplica migrations. Os arquivos devem ser arquivos regulares UTF-8 de até 4 MiB. Migrations gerenciadas não aceitam comandos de transação, `PRAGMA`, `ATTACH`, `DETACH` ou `VACUUM`; o controle de transação pertence à CLI. Migrations exigem um caminho SQLite persistente e não têm downgrade automático.

Crie programas de seed manualmente como arquivos `.tyr` em `db/seeds/`. O histórico de seeds é compatível com SQLite, PostgreSQL e MySQL; o MongoDB ainda não oferece histórico de seeds gerenciado pela CLI. Uma seed pode usar a API de Model ou `db.query`; execute um arquivo com `tyrus db apply default_roles.tyr` depois de aplicar as migrations. `tyrus db seed status [name]` lista os estados das seeds. A CLI registra um checksum SHA-256 em `_tyrus_seeds` e recusa executar novamente um arquivo aplicado, aceitar alterações ou arquivos aplicados ausentes. O programa de seed roda em um processo Tyrus separado, portanto a CLI não pode envolver operações arbitrárias em uma única transação do banco; use a API de transação do banco quando várias alterações precisarem ser atômicas. Um programa que falhar ou for interrompido permanece como `started` e bloqueia novas tentativas. Inspecione o banco, corrija ou reverta alterações parciais e então remova a linha correspondente em `_tyrus_seeds` antes de tentar novamente; crie um novo arquivo de seed para mudanças posteriores nos dados.

## Comandos de tooling

| Comando | Finalidade |
|---------|------------|
| `tyrus fmt [--check] [--stdout] [--init] [--default] [<path>...]` | Formata arquivos fonte. |
| `tyrus lint [--fix] [--format json] [--max-warnings <n>] [--explain <rule>] [--init] [--default] [<path>...]` | Reporta diagnósticos de lint. |
| `tyrus lsp` | Inicia o Language Server por stdio. |
| `tyrus mcp install` | Baixa a documentação do Tyrus e configura automaticamente os clientes MCP detectados. |
| `tyrus mcp update` | Atualiza o cache de documentação MCP e as entradas gerenciadas nos clientes. |
| `tyrus mcp status` | Mostra o estado do cache MCP e das configurações dos clientes. |
| `tyrus mcp uninstall` | Remove entradas Tyrus MCP inalteradas e o cache local da documentação. |
| `tyrus mcp serve` | Inicia o servidor MCP stdio; clientes configurados executam este comando. |
| `tyrus repl` | Inicia o console interativo REPL. |

## Convenções de saída

Saída normal do programa pertence ao stdout. Saída de compilador, linter, falha de runtime, prompt e progresso pertence ao stderr para que programas gerados continuem amigáveis a pipes.

::: details Exemplo de CI
<!-- @include: ../../.fragments/tooling/cli-ci-example.md -->
:::

### Otimização guiada por perfil

O `-O3` adivinha quais desvios são tomados; um perfil sabe. Compilar contra um deles deixa o otimizador juntar os blocos quentes e afastar os frios, um caminho de erro, um desvio raro, que é a única coisa que o código-fonte sozinho não diz.

São três passos, porque um perfil precisa vir de uma execução real:

<!-- @include: ../../.fragments/tooling/cli/pgo-workflow.md -->

O ganho é de layout, então aparece onde há desvios a prever e não onde há contas a fazer: medido em **1,20x** num laço de validação com cinco desvios desiguais e caminhos de erro frios, e **1,03x** num laço aritmético simples. As duas flags exigem `--release`; um build instrumentado em `-O0` mede código que o otimizador nunca vê.

O `llvm-profdata` precisa vir do mesmo toolchain do driver do compilador, porque o perfil bruto carrega uma versão de formato ligada a ele. O correto é encontrado automaticamente; `TYRUS_LLVM_PROFDATA` sobrescreve a escolha.
