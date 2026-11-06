---
title: Configuração do Projeto
description: Estrutura do projeto, tyrus.json, scripts de documentação e fluxo de release.
---

# Configuração do Projeto

Um projeto tyrus gerado tem um manifesto pequeno, um ponto de entrada, arquivos de ambiente tipados opcionais e um diretório `target/` para artefatos gerados.

## Estrutura do Projeto

<!-- @include: ../../.fragments/docs/configuration-project-structure.md -->

Este é o projeto `script` padrão. O template `cli` adiciona `src/prompts/setup.tyr` e usa `__tests__/example.test.tyr` no lugar do teste padrão. O template `api` adiciona `src/controllers/user.tyr`, `src/models/user.tyr`, `src/repositories/user.tyr`, `src/routes/user.tyr`, `src/schemas/user.tyr` e `src/services/user.tyr`, e também usa `__tests__/example.test.tyr`. O template `api` cria o diretório `data/` quando SQLite é selecionado.

`src/main.tyr` é o arquivo de entrada obrigatório e deve declarar `const main = (): void => { ... };`. O caminho e a assinatura da função são fixos. Os artefatos de build são gravados em `target/debug/` por padrão e em `target/release/` com `tyrus build --release`. `tyrus dev` usa `target/debug/`.

## tyrus.json

<!-- @include: ../../.fragments/docs/configuration-tyrus-json.md -->

`project` descreve metadados do pacote. `build.optimize` seleciona o modo padrão de otimização. `development.https` faz um servidor iniciado sem `tls` servir HTTPS em `localhost` enquanto o programa roda com `TYRUS_ENV=development`, e não faz nada em qualquer outro ambiente; veja [createRouter](../modules/http/create-router.md). `runtime.workerThreads` limita quantas threads de trabalho o runtime assíncrono pode usar e, quando omitido, assume o número de CPUs lógicas. `security.envFile` habilita análise tipada de ambiente. `security.envSealed` troca o carregamento de ambiente em runtime por um artefato sealed criptografado.

`scripts` mapeia nomes de scripts do projeto para comandos de shell. Manifestos gerados incluem as entradas `fmt`, `lint` e `test`. Adicione um script personalizado como valor string e execute-o pelo nome com `tyrus run <name>`. Por exemplo, esta entrada faz `tyrus run example` executar `tyrus run example.tyr` no diretório do projeto:

```json
{
  "scripts": {
    "example": "tyrus run example.tyr"
  }
}
```

Os argumentos para o comando configurado vêm depois de `--`, como em `tyrus run example -- --verbose`. `tyrus --init` preserva scripts personalizados ao adicionar defaults ausentes; `tyrus --default` restaura o manifesto aos defaults gerados para o projeto e remove scripts personalizados.

## Site de Documentação

A documentação do repositório fica em `docs/` e é buildada com VitePress:

<!-- @include: ../../.fragments/docs/configuration-docs-commands.md -->

A documentação atual é publicada em `/`. Releases arquivados são gerados a partir de Git tags em caminhos como `/v0.1.0/`. Metadados de versão ficam em `docs/.vitepress/versions.ts`.
