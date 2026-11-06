---
title: clipboardWrite
description: Escreva texto na área de transferência do sistema através do módulo nativo tyrus:os.
---

# clipboardWrite

`clipboardWrite` solicita a gravação de `text` na área de transferência do sistema, substituindo o conteúdo anterior, e retorna um erro estruturado se a área de transferência não puder aceitá-lo.

<SignatureBlock label="Assinatura">clipboardWrite(text: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-clipboard-write.md -->
Retorna `ERR_OS_INVALID_ARGUMENT` para uma string nula, `ERR_OS_CLIPBOARD_UNAVAILABLE` se não for possível abrir a área de transferência do sistema ou `ERR_OS_CLIPBOARD_WRITE_FAILED` se ela recusar o texto. Trate o `Result` retornado antes de presumir que a área de transferência foi atualizada.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Um identificador de erro estável e legível por máquina. |
| `message` | `string` | Uma descrição legível da falha. |

::: warning X11: a escrita não é necessariamente durável ainda quando esta chamada retorna
No Linux sob X11 (diferente de Wayland, macOS ou Windows), a área de transferência não é armazenada centralmente pelo sistema operacional, o próprio programa que escreveu continua servindo o conteúdo sob demanda, repassando essa responsabilidade a um processo em segundo plano de vida curta assim que sai. Ler de volta imediatamente depois, no mesmo programa, pode transitoriamente ver como ausente antes dessa transição terminar. Uma colagem real (um humano apertando uma tecla um instante depois) nunca é rápida o bastante pra cair nisso; só uma escrita-depois-leitura automatizada no mesmo instante é.
:::
