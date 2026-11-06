---
title: Playground
description: Escreva, compile e execute código tyrus diretamente no navegador.
---

# Playground

O Tyrus Playground é um editor web para escrever, compilar e executar código tyrus sem precisar instalar nada localmente. Ele compila seu código para WebAssembly no servidor e o executa em um Web Worker sandboxed no navegador.

## Acesso

Abra [playground.tyruslang.org](https://playground.tyruslang.org) em qualquer navegador moderno.

## Funcionalidades

- **Monaco Editor** com syntax highlighting customizado para Tyrus.
- **Temas escuro e claro** via botão de alternância.
- **Exemplos pré-carregados** (Hello World, Fibonacci, Match, Loops, Const, Enums).
- **Ctrl+Enter** para compilar e executar.
- **Painel de saída** mostrando stdout, stderr e erros de compilação.
- **Links de compartilhamento** via URLs curtas (`/s/{shortId}`).

## Como Funciona

1. Você escreve código no editor.
2. Clicar em **Run** envia o código para o backend.
3. O backend compila para WebAssembly usando `tyrus build --compile-target wasm`.
4. O binário WASM é carregado em um Web Worker sandboxed.
5. O Worker executa o código e captura stdout/stderr.
6. A saída é exibida no painel de saída.

## O que roda no sandbox

O playground executa seu programa no navegador, então tudo que depende de um sistema operacional por trás não funciona ali. O restante funciona, a linguagem em si, a biblioteca padrão e as partes do runtime que são computação pura.

Disponível: strings e expressões regulares, arrays e seus métodos funcionais, `Map` e `Set`, `decimal`, JSON, matemática, validadores e `tyrus:temporal`, incluindo conversão de fuso horário.

::: warning `tyrus:cli`, `tyrus:crypto`, `tyrus:fs`, `tyrus:http`, `tyrus:net`, `tyrus:os`, `tyrus:process`, `tyrus:stream` e `tyrus:task` não rodam no playground
Cada um desses precisa de acesso real ao sistema operacional que o sandbox do navegador não tem, um pool de threads para o paralelismo de `tyrus:task`, o sistema de arquivos para `tyrus:fs`, sockets para `tyrus:net`/`tyrus:http`, e assim por diante. Importar qualquer um deles é recusado na compilação do programa, com o módulo nomeado (`module `tyrus:task` is not available in a sandboxed build`, por exemplo). O namespace ambiente `process` não tem import a recusar, então se declara indisponível quando o programa o acessa. A mesma recusa vale para qualquer build que tenha o WebAssembly como alvo, não só a do playground, porque quem não tem o acesso é o alvo e não o sandbox; sem ela o programa chegava ao linker e falhava lá com um símbolo de runtime indefinido em vez de uma mensagem nomeando o import. Rode exemplos que usem qualquer um desses localmente com `tyrus run`.
:::

Acessar uma API ambiente que o sandbox não fornece não derruba o programa em silêncio. Dependendo da API, ela lança um erro capturável cujo `code` segue o formato `ERR_<MÓDULO>_UNAVAILABLE`, ou interrompe o programa com uma mensagem nomeando o que foi acessado. Nos dois casos o motivo aparece no painel de saída.

<!-- @include: ../../.fragments/tooling/playground/what-runs-in-the-sandbox.md -->

<!-- @include: ../../.fragments/tooling/playground/what-runs-in-the-sandbox-2.md -->

::: warning `async`/`await` não roda no playground
Código assíncrono compila, mas não há o que o conduza: o sandbox não tem executor de tarefas, e `main()` roda até o fim de forma síncrona. Um programa que usa `await` não vai produzir o resultado esperado. Rode localmente com `tyrus run`.

Isto é uma propriedade do sandbox, não uma lacuna da linguagem, código assíncrono funciona normalmente fora do playground.
:::

## Limitações

- O código é limitado a 8 KB.
- A execução é limitada a 5 segundos.
- A memória WASM é limitada a 8 MB.
- Módulos de sistema de arquivos, rede, SO e a execução de `async`/`await` estão indisponíveis no sandbox.
- A compilação tem limite de 5 requisições por minuto por IP.
