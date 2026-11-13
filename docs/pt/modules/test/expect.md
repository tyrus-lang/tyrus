---
title: expect
description: Verifica um valor com um matcher.
---

# expect

<SignatureBlock label="Assinatura">expect&lt;T&gt;(value: T): Assertion</SignatureBlock>

`expect(value)` avalia `value` exatamente uma vez e retorna uma asserção. Um matcher chamado sobre a asserção compara o valor e reprova o teste quando a comparação não se sustenta. `.not` antes do matcher o inverte.

| Matcher | Passa quando |
| --- | --- |
| `toBe(expected)`, `toEqual(expected)`, `toStrictEqual(expected)` | os valores são iguais: um escalar, um objeto campo a campo, ou um array elemento a elemento, até o fim |
| `toBeTruthy()`, `toBeFalsy()` | o valor é truthy ou falsy |
| `toBeNull()`, `toBeUndefined()`, `toBeDefined()` | um valor opcional está ausente ou presente |
| `toBeGreaterThan(n)`, `toBeGreaterThanOrEqual(n)` | o número é maior que `n` (ou igual a ele) |
| `toBeLessThan(n)`, `toBeLessThanOrEqual(n)` | o número é menor que `n` (ou igual a ele) |
| `toBeCloseTo(n)` | o float está a no máximo `0.005` de `n` |
| `toHaveLength(n)` | a string ou o array tem comprimento `n` |
| `toContain(value)` | a string contém a substring, ou o array contém o elemento |
| `toMatch(text)` | a string contém `text` |
| `toThrow()`, `toThrow(message)` | chamar a função lança (e a `string` lançada é igual a `message`); a função precisa declarar `throws` e não receber argumentos |

Um matcher que falha cita os valores comparados, com strings entre aspas: `expected "abc" to equal "abd"`.

Dois matchers que outros frameworks têm ficam de fora de propósito, e escrever um deles é erro de compilação que informa o que usar no lugar. `toHaveProperty` pergunta algo que o compilador já decidiu: um campo é resolvido em tempo de compilação, então a asserção só pode ser sempre verdadeira; para saber se um campo opcional tem valor, escreva `expect(user.email).toBeDefined()`. `toBeInstanceOf` não tem o que consultar, pois não existem instâncias de classe; estreite o valor com `match` ou uma checagem de opcional e afirme sobre o que o valor estreitado guarda.

`.rejects` antes de `toThrow` aguarda uma promise e afirma que ela falhou: `expect(load()).rejects.toThrow("not found")`, com a mensagem opcional e `.not` disponível. A espera acontece pelo runtime em vez de suspender, então funciona também em um teste síncrono. A promise precisa ser uma cuja falha seja uma rejeição, como `Promise.reject(...)` ou uma função `async` que retorne uma; uma função que declara `throws` tem esse efeito tratado no local da chamada, antes de o matcher vê-la. `.resolves` resolve uma promise do mesmo jeito e entrega ao matcher seguinte o valor obtido, de modo que todos os matchers funcionam depois dele: `expect(load()).resolves.toBe(42)`, `expect(load()).resolves.toEqual({ id: 1 })`. Uma promise que falhou reprova a asserção, informando com o que ela foi rejeitada. Os dois são exclusivos, pois uma promise não pode ser afirmada como falha e sucesso ao mesmo tempo, e `toThrow` pertence a `.rejects`: depois de `.resolves` o matcher já tem o valor, então não sobra falha alguma a afirmar. `toBe`, `toEqual` e `toStrictEqual` comparam valores escalares, objetos campo a campo (a mesma comparação que o `equals` automático faz) e arrays elemento a elemento. Um array de escalares usa a mesma igualdade de elemento que `includes` e `indexOf` usam; um array de objetos, ou de arrays, é comparado até o fim, parando na primeira diferença.

<!-- @include: ../../../.fragments/modules/test/expect.md -->
