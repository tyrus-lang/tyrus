---
title: Primeiros Passos
description: Instale o Tyrus e crie um projeto com a CLI interativa.
---

# Primeiros Passos

Instale o Tyrus com o comando do seu sistema operacional e crie um projeto com a CLI interativa.

## Instale o Tyrus

### macOS, Linux e WSL

```sh
curl -fsSL https://tyruslang.org/install.sh | sh
```

### Windows PowerShell

```powershell
irm https://tyruslang.org/install.ps1 | iex
```

## Crie um Projeto

Execute:

```sh
tyrus new
```

A CLI pergunta o nome e o target do projeto. Para um projeto de API, também pergunta se deve configurar um banco de dados. Responda às perguntas para gerar o projeto.

No diretório do projeto gerado, execute:

```sh
tyrus run
```

## Próximos Passos

Leia o [**guia da CLI**](../tooling/cli.md) para conhecer os comandos disponíveis ou continue com [**Configuração do Projeto**](./configuration.md).
