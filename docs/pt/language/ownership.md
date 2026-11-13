---
title: Ownership e Borrowing
description: Semântica de move, valores Copy, borrows imutáveis e mutáveis, lifetimes, valores nullable e inicialização adiada.
---

# Ownership e Borrowing

Tyrus aplica regras de ownership em tempo de compilação, mantendo uma sintaxe concisa.

## Move e Copy

Valores non-Copy como `string` movem em atribuição. O binding original fica inválido depois do move.

::: danger error
<!-- @include: ../../.fragments/language/ownership/move-copy.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/ownership/moved-value-diagnostic.md -->
:::

Valores primitivos como `i32`, `i64`, `f32`, `f64` e `boolean` são Copy e continuam utilizáveis depois da atribuição.

<!-- @include: ../../.fragments/language/ownership/copy-primitives.md -->

Chamadas de função passam argumentos read-only sem consumir bindings comuns non-Copy do chamador, mantendo APIs usuais concisas. Referências explícitas estão disponíveis quando uma API precisa expor borrowing.

## Borrowing

<!-- @include: ../../.fragments/language/ownership/borrowing.md -->


Um empréstimo pode tomar uma parte do valor: `&basket.label` empresta o campo. A referência lê através de `basket`, então é `basket` que fica emprestado, e atribuir a ele enquanto a referência está viva é recusado. Emprestar dois campos diferentes ao mesmo tempo é recusado pelo mesmo motivo, o que é mais restrito do que precisaria ser e nunca está errado.

<!-- @include: ../../.fragments/language/ownership/borrowing-a-field.md -->
Você pode ter vários borrows imutáveis ou um borrow mutável, nunca ambos ao mesmo tempo. Borrows temporários terminam no fim do statement. Um borrow armazenado termina no último ponto em que seu binding é mencionado, não na chave de fechamento: assim que nada mais lê a referência, o valor apontado fica livre para ser atribuído, emprestado de forma mutável ou movido, mesmo com o binding ainda em escopo. Uma menção dentro do corpo de um laço ou capturada por uma closure conta, e mantém o borrow vivo enquanto aquela construção puder executar. Uma menção dentro de um corpo de `defer` ou `errdefer` mantém o borrow vivo até a chave de fechamento, onde quer que o finalizador esteja escrito, porque é ali que ele roda. Referências não podem viver mais que o valor para o qual apontam.

Dentro de uma função `async` um empréstimo se comporta do mesmo jeito, com uma regra a mais: ele não pode continuar vivo num `await`. Segurar um através de uma suspensão é rejeitado, "borrow `b` of `a` crosses `await`", e é justamente isso que torna empréstimos baratos ali, já que a máquina de estados guarda os bytes emprestados no frame dela e não libera nenhum deles.

## Non-Lexical Lifetimes (NLL)

O Tyrus implementa Non-Lexical Lifetimes (NLL). Em vez de manter os empréstimos ativos até a chave de fechamento do bloco léxico (`}`), o borrow checker constrói um Grafo de Fluxo de Controle (CFG) e utiliza análise de liveness retroativa (backward) para liberar empréstimos imediatamente após seu último uso real.

### Empréstimo Mutável Seguido de Mutação

Um empréstimo mutável libera a variável original assim que a referência não é mais lida:

<!-- @include: ../../.fragments/language/ownership/nll-mutable-borrow.md -->

Sem NLL, `r` permaneceria ativo até o final do bloco léxico, rejeitando `x += 1`.

### Empréstimos Dentro de Branches Condicionais

Empréstimos realizados dentro de um bloco `if` expiram quando o branch termina ou cessa de utilizá-los, liberando a variável no escopo externo:

<!-- @include: ../../.fragments/language/ownership/nll-conditional-branches.md -->

### Move Após Empréstimo

Assim que uma referência emprestada não for mais acessada, o valor original pode ser movido ou consumido normalmente:

<!-- @include: ../../.fragments/language/ownership/nll-move-after-borrow.md -->

### Reatribuição de Referências

Reatribuir uma referência mutável a um novo alvo libera imediatamente o empréstimo sobre o alvo anterior:

<!-- @include: ../../.fragments/language/ownership/nll-reassignment.md -->

### Prevenção de Escape de Referências

O NLL encurta tempos de vida onde é seguro; ele nunca estende o tempo de vida além do valor original. Retornar ou armazenar referências a variáveis locais cujo escopo encerra continua estritamente proibido:

::: danger error
<!-- @include: ../../.fragments/language/ownership/nll-escape-prevention.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/ownership/nll-escape-prevention-diagnostic.md -->
:::

## Valores Nullable

tyrus não tem `undefined`, e código de usuário não deve usar `null` como inicializador normal. Valores nullable usam `T?`, campos opcionais, parâmetros opcionais, fronteiras externas, narrowing e `??`.

<!-- @include: ../../.fragments/language/ownership/nullable-values.md -->

Forced unwrap (`value!`) é intencionalmente desencorajado e diagnosticado pelo tooling da linguagem; prefira narrowing explícito ou valores fallback.

## Inicialização Adiada

<!-- @include: ../../.fragments/language/ownership/deferred-init.md -->

O compilador roda definite-assignment analysis e rejeita leitura de um binding `let x: T;` antes de todo caminho atribuí-lo. Bindings `const` devem ser inicializados onde são declarados.