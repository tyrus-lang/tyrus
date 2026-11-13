---
title: Aliases de caminho
description: Aliases estáticos de pastas para imports internos.
---

# Aliases de caminho

Declare aliases estáticos para pastas internas de código fonte no objeto `imports` de `tyrus.json`.

<!-- @include: ../../.fragments/docs/path-aliases.md -->

Toda chave de alias começa com `#` e mapeia uma pasta relativa à raiz do projeto. Use `#` para uma pasta `src/` convencional e importe seus filhos com `#/pasta`.

Use `#config` para importar `src/config/index.tyr`, ou `#config/mongodb` para importar um módulo filho. Um alias nunca aponta diretamente para um arquivo fonte, expande outro alias, sai da raiz do projeto ou é resolvido em runtime.

Quando mais de um alias se aplica, o prefixo mais longo vence. Um specifier `#` não resolvido é erro de compilação e diagnóstico no editor.
