---
title: Variáveis de Ambiente
description: Arquivos .env tipados, validação, diagnósticos, artefatos sealed e acesso por process.env.
---

# Variáveis de Ambiente

Quando `tyrus.json` define `security.envFile`, o compilador lê esse arquivo `.env` e expõe variáveis por `process.env` com tipos estáticos. O binário não embute secrets. O compilador e o Language Server compartilham a mesma análise.

## Configuração

<!-- @include: ../../.fragments/docs/config.md -->

## .env Tipado

<!-- @include: ../../.fragments/docs/typed-env.md -->

Validator Decorations em arquivos `.env` podem ser escritas sem `#` ou com `#` no início. O Tyrus interpreta as duas formas. Mantenha o `#` quando outro loader de `.env`, como o Docker, também ler o arquivo; esse loader verá um comentário e o Tyrus ainda aplicará os validators.

```dotenv
# @type(u16)
# @port()
PORT=8080
```

O parser separa no primeiro `=`, exige valor, remove aspas opcionais, suporta interpolação `${VAR}`, ignora comentários e linhas em branco, aceita `export KEY=value` e trata `#` inline como parte do valor. Toda variável deve ter `@type`, raw ou comentado como `# @type(...)` para compatibilidade com outros loaders de `.env`.

## Acesso

<!-- @include: ../../.fragments/docs/access.md -->

Member access e destructuring são tipados a partir da declaração do `.env`. Acesso indexado como `process.env["PORT"]` é rejeitado para que o compilador preserve garantias estáticas.

## Diagnósticos

Diagnósticos comuns incluem `E3147`, `E3148`, `E3149`, `E3150`, `E3151`, `E3152`, `E3144`, `E3146`, `E3142`, `E3143`, `E3141`, `E3140`, `E3145` e `E3083`.

Um diagnóstico de `.env` nunca imprime o valor que ele rejeita. Ele nomeia a variável e a regra que falhou, e seu code frame mostra o valor como `<redacted>`, e não uma máscara do mesmo tamanho, que ainda publicaria quantos caracteres o segredo tem. Um build que falha escreve seus diagnósticos direto num log de CI, e o code frame imprime a linha para a qual aponta, então uma mensagem cuidadosa sozinha não bastaria.

## Resolução em Runtime

Valores em runtime resolvem nesta ordem: o ambiente real do processo vence, e o arquivo `.env` é fallback. A inicialização valida presença, `@type` e Validator Decorations compatíveis antes do código do usuário rodar. Falhas imprimem em stderr e encerram com status diferente de zero.

## Variáveis Reservadas

`TYRUS_WORKER_THREADS` é lida pelo próprio runtime, não pelo seu programa. Ela define quantas threads de trabalho o runtime assíncrono pode usar e sobrepõe `runtime.workerThreads` do `tyrus.json`, um deploy consegue encolher um programa para a máquina em que ele caiu sem recompilá-lo. Quando nenhuma das duas está definida, o runtime usa uma worker por CPU lógica.

Um valor que não seja um inteiro positivo é ignorado com um aviso em stderr em vez de ser fatal, e uma contagem bem acima do número detectado de CPUs lógicas é respeitada, mas também reportada, já que sobrecarregar a máquina é ocasionalmente deliberado.

`TYRUS_REPORT_PENDING_TASKS=1` pede ao runtime que reporte, em stderr, quantas tarefas assíncronas ainda estavam pendentes quando o `main` terminou. Ela vem desligada porque um programa correto pode terminar com tarefas pendentes de propósito; ligue-a quando uma tarefa parecer não ter rodado nada.

## Env Sealed

Defina `security.envSealed` como `true` para criptografar a origem do env em `target/<profile>/.env.sealed` com `age`. O binário gerado passa a exigir `.env.sealed` ao lado do executável e rejeita fingerprints ou chaves incompatíveis. Use `tyrus build --rotate-seal-key` para gerar um novo par de chaves e reseal.

## .env.example

Tyrus mantém automaticamente um espelho seguro `<envFile>.example` ao lado do arquivo de env. Sempre que você adiciona, remove ou renomeia uma variável no `.env`, Tyrus espelha a mudança no `.env.example`, preservando ordem e decorators enquanto limpa valores após `=`. Esse arquivo deve ser commitado para que a equipe saiba quais variáveis são necessárias sem expor seus valores.
