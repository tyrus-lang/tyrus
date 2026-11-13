---
title: Arrays
description: Literais de array, indexação, métodos funcionais e comportamento nativo de arrays.
---

# Arrays

<!-- @include: ../../.fragments/language/collections/arrays-overview.md -->

Ambas as formas `T[]` e `Array<T>` são aceitas. Leituras e escritas indexadas realizam checagens de limite em tempo de execução. Métodos mutantes exigem um receiver `let`. `length` informa a quantidade de elementos, enquanto `capacity` é uma extensão do tyrus somente leitura que expõe o armazenamento nativo reservado.

### Literais de array e valores repetidos

O tipo `[T; N]` declara um array com exatamente `N` elementos. Inicialize-o com uma lista contendo exatamente `N` valores ou com `[valor; N]` para repetir um valor. Literais de array sem anotação de tamanho fixo, como `T[]`, continuam tendo tamanho dinâmico.

<!-- @include: ../../.fragments/language/collections/array-literals.md -->

### Arrays construídos por uma sequência de `push`

Um array local que começa vazio e é preenchido por uma sequência direta de `push` é compilado como o literal de array equivalente, o que permite que ele viva na pilha em vez do heap:

<!-- @include: ../../.fragments/language/collections/push-run.md -->

A reescrita é exata, mesmos elementos, mesma ordem, mesmo tamanho, e o array continua crescendo depois. Ela só se aplica enquanto nada puder ter observado o array no meio: a sequência para no primeiro statement que não é um `push` naquele array, um elemento que lê o array em construção nunca é movido para antes dele, e um elemento com spread desqualifica a sequência por contribuir com um número desconhecido de elementos. Os tipos de elemento se limitam aos copiados por valor.

Nada disso é visível no comportamento, apenas em quanto o programa aloca. Crescer um array de outro modo aloca até em build de release, já que o crescimento é uma chamada dentro da qual o otimizador não consegue olhar.

### Checagens de limite que um laço já provou

Toda leitura e escrita indexada é checada em tempo de execução, exceto onde um laço em volta já provou que a checagem não pode falhar. No formato de contagem abaixo o corpo só é alcançado quando `i < values.length`, e nada entre esse teste e o acesso ao elemento pode mudar qualquer um dos dois lados, então a comparação que o acesso emitiria é a condição do laço escrita uma segunda vez:

<!-- @include: ../../.fragments/language/collections/counting-loop.md -->

A prova é deliberadamente estreita, porque errar nela remove a única coisa entre um índice ruim e uma leitura fora do buffer. Um laço só se qualifica quando o contador é declarado pelo `init` dele mesmo a partir de um literal não negativo, a condição é exatamente `contador < array.length`, a atualização apenas aumenta o contador por uma constante, o array é um local, um parâmetro ou um global `const` que nada pode fazer crescer e, dentro do corpo, o array nunca é usado para nada além de indexação e `length`, enquanto o contador nunca é escrito. Qualquer outra coisa mantém a checagem: um `push` no corpo, um índice diferente do contador, uma redeclaração de qualquer um dos dois nomes ou uma menção a qualquer um deles de dentro de uma closure.

Uma checagem mantida se comporta exatamente como sempre se comportou, nomeando o índice ofensor e o tamanho do array em stderr antes de encerrar com status diferente de zero.

### Checagens de limite que uma tabela fixa já provou

Um laço não é a única coisa que pode resolver um índice. Quando um binding é declarado `const` a partir de um literal de array, a contagem de elementos dele está escrita no código, e um índice literal abaixo dessa contagem está dentro do intervalo sem laço nenhum envolvido:

<!-- @include: ../../.fragments/language/collections/fixed-table.md -->

A checagem de `PRIMES[2]` é removida, e um índice além do fim é erro de compilação em vez de erro de execução, `PRIMES[8]` é rejeitado com `E0606`, nomeando o intervalo válido. O que fixa o tamanho é o `const`, não a anotação: o binding não pode ser reatribuído, e um método mutante sobre ele é rejeitado. `readonly const` é como uma tabela fixa normalmente se escreve, mas um literal de array `const` simples prova a mesma coisa.

O binding também precisa ser apenas lido e nada mais. Passá-lo para uma função, capturá-lo numa closure ou chamar qualquer método sobre ele encerra a prova e a checagem permanece, a mesma regra estreita que o formato de laço segue, e pelo mesmo motivo.


::: warning Métodos mutáveis exigem `let`
Um array `const` é profundamente imutável. Chamar `push`, `sort`, `reverse` ou outro método mutável em um array `const` gera um erro de compilação.
:::

::: danger error
<!-- @include: ../../.fragments/language/collections/const-mutate-error.md -->
:::

## Propriedades

| Membro | Tipo | Comportamento |
| --- | --- | --- |
| `length` | `usize` | Quantidade atual de elementos. |
| `capacity` | `usize` | Armazenamento nativo reservado; esta é uma extensão do Tyrus. |

## Métodos mutáveis

Esses métodos alteram o array e, por isso, exigem um binding `let` mutável. `pop()` e `shift()` retornam `T?`, então um array vazio produz ausência.

| Método | Assinatura | Comportamento |
| --- | --- | --- |
| `push` | `push(...items: T[]): usize` | Adiciona valores ao final e retorna o novo tamanho. |
| `pop` | `pop(): T?` | Remove e retorna o último valor, ou ausência. |
| `unshift` | `unshift(...items: T[]): usize` | Adiciona valores no início e retorna o novo tamanho. |
| `shift` | `shift(): T?` | Remove e retorna o primeiro valor, ou ausência. |
| `splice` | `splice(start: i64, deleteCount?: i64, ...items: T[]): T[]` | Remove, substitui ou insere valores e retorna os valores removidos. |

`sort`, `reverse`, `rotate`, `fill` e `copyWithin` também alteram o receiver. O comportamento deles está em [Métodos funcionais](#metodos-funcionais).

## Métodos estáticos

| Método | Assinatura | Comportamento |
| --- | --- | --- |
| `Array.of` | `Array.of(...items: T[]): T[]` | Cria um array com os argumentos recebidos. |
| `Array.from` | `Array.from(items: T[]): T[]` | Copia um array para um array de tamanho dinâmico. |
| `Array.withCapacity` | `Array.withCapacity(capacity: usize): T[]` | Cria um array vazio com capacidade reservada. |
| `Array.isArray` | `Array.isArray(value: unknown): boolean` | Verifica se um valor é um array. |

## Métodos Funcionais

| Categoria | Métodos |
| --- | --- |
| Iteração & Transformação | [`map()`](#map), [`filter()`](#filter), [`partition()`](#partition), [`reduce()`](#reduce), [`reduceRight()`](#reduceright), [`forEach()`](#foreach) |
| Janelas e Matrizes | [`windows()`](#windows), [`scan()`](#scan), [`cartesianProduct()`](#cartesianproduct), [`zip()`](#zip), [`transpose()`](#transpose) |
| Deduplicação e Agrupamento | [`unique()`](#unique), [`uniqueBy()`](#uniqueby), [`groupBy()`](#groupby) |
| Toggle | [`toggle()`](#toggle), [`toggleAll()`](#toggleall) |
| Busca & Teste | [`find()`](#find), [`findLast()`](#findlast), [`findIndex()`](#findindex), [`findLastIndex()`](#findlastindex), [`some()`](#some), [`every()`](#every), [`includes()`](#includes), [`equals()`](#equals), [`indexOf()`](#indexof), [`lastIndexOf()`](#lastindexof) |
| Flatten & Reordenação | [`flat()`](#flat), [`flatMap()`](#flatmap), [`sort()`](#sort), [`toSorted()`](#tosorted), [`sortBy()`](#sortby), [`sortedBy()`](#sortedby), [`reverse()`](#reverse), [`rotate()`](#rotate), [`toReversed()`](#toreversed), [`random()`](#random), [`shuffle()`](#shuffle), [`sample()`](#sample) |
| Fatiamento | [`slice()`](#slice), [`splitAt()`](#splitat) |
| Agregação | [`sum()`](#sum), [`product()`](#product), [`avg()`](#avg), [`sumBy()`](#sumby), [`avgBy()`](#avgby), [`count()`](#count), [`concat()`](#concat), [`join()`](#join) |
| Mutação | [`splice()`](#splice), [`fill()`](#fill), [`copyWithin()`](#copywithin) |
| Acesso & Iteração | [`first()`](#first), [`last()`](#last), [`at()`](#at), [`keys()`](#keys), [`values()`](#values), [`entries()`](#entries) |
| Cópia Imutável | [`with()`](#with) |

### Iteração & Transformação

Os métodos baseados em predicado (`filter`, `partition`, `some`, `every`, `find`, `findLast`, `findIndex`, `findLastIndex`, `uniqueBy` e `groupBy`) aceitam uma callback arrow explícita ou uma expressão implícita como `value > 2`; o tipo do elemento é inferido a partir do array receptor.

#### `map()`

Cria um novo array aplicando uma transformação a cada elemento.

<SignatureBlock label="Assinatura">map&lt;U&gt;(callback: (element: T, index?: i64) =&gt; U): U[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-map.md -->

#### `filter()`

Seleciona elementos que satisfazem um predicado.

<SignatureBlock label="Assinatura">filter(predicate: (element: T) =&gt; boolean): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-filter.md -->

#### `partition()`

Separa os elementos em dois arrays estáveis de acordo com um predicado. O primeiro array contém os elementos que correspondem; o segundo contém os demais. O receiver não é alterado.

<SignatureBlock label="Assinatura">partition(predicate: (element: T) =&gt; boolean): [T[], T[]]</SignatureBlock>

<SignatureBlock label="Assinatura">partition(field: string): [T[], T[]]</SignatureBlock>

<SignatureBlock label="Assinatura">partition(field: string, ...values: string[]): [T[], T[]]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/partition.md -->

Arrays de objetos podem usar diretamente o nome de um campo booleano, ou um campo string seguido de um ou mais valores correspondentes. Os nomes dos campos são verificados contra o tipo do elemento em tempo de compilação.

<!-- @include: ../../.fragments/language/collections/functional-partition-properties.md -->

Um campo desconhecido é rejeitado em tempo de compilação.

::: danger error
<!-- @include: ../../.fragments/language/collections/functional-partition-error.md -->
:::

Para predicados mais complexos, use uma arrow function explícita. Ela pode receber o elemento, seu índice e o array original:

<!-- @include: ../../.fragments/language/collections/partition-2.md -->

`partition()` avalia o predicado uma vez por elemento e preserva a ordem original dentro dos dois arrays de saída. Assim como em `count()`, uma expressão comum com exatamente um identificador livre é tratada como predicado implícito por elemento; uma arrow function explícita continua disponível para casos mais complexos. Por segurança de memória, `T` precisa ser `Copy`, `string` ou um objeto cujos campos são `Copy` ou `string`.

#### `reduce()`

Acumula elementos em um único valor, da esquerda para a direita.

<SignatureBlock label="Assinatura">reduce&lt;U&gt;(callback: (accumulator: U, element: T) =&gt; U, initial: U): U</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-reduce.md -->

#### `reduceRight()`

Acumula elementos em um único valor, da direita para a esquerda.

<SignatureBlock label="Assinatura">reduceRight&lt;U&gt;(callback: (accumulator: U, element: T) =&gt; U, initial: U): U</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-reduceRight.md -->

#### `forEach()`

Executa um callback para cada elemento. O valor de retorno é descartado.

<SignatureBlock label="Assinatura">forEach(callback: (element: T) =&gt; void): void</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-forEach.md -->

### Busca & Teste

#### `find()`

Retorna o primeiro elemento que satisfaz um predicado, ou ausência se nenhum satisfizer.

<SignatureBlock label="Assinatura">find(predicate: (element: T) =&gt; boolean): T?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-find.md -->

#### `findLast()`

Retorna o último elemento que satisfaz um predicado, ou ausência se nenhum satisfizer.

<SignatureBlock label="Assinatura">findLast(predicate: (element: T) =&gt; boolean): T?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-findLast.md -->

#### `findIndex()`

Retorna o índice do primeiro elemento que satisfaz o predicado, ou `-1`.

<SignatureBlock label="Assinatura">findIndex(predicate: (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-findIndex.md -->

#### `findLastIndex()`

Retorna o índice do último elemento que satisfaz o predicado, ou `-1`.

<SignatureBlock label="Assinatura">findLastIndex(predicate: (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-findLastIndex.md -->

#### `some()`

Verifica se ao menos um elemento satisfaz uma condição.

<SignatureBlock label="Assinatura">some(predicate: (element: T) =&gt; boolean): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-some.md -->

#### `every()`

Verifica se todos os elementos satisfazem uma condição.

<SignatureBlock label="Assinatura">every(predicate: (element: T) =&gt; boolean): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-every.md -->

#### `includes()`

Verifica se um valor específico existe no array.

<SignatureBlock label="Assinatura">includes(value: T): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-includes.md -->

#### `equals()`

Compara dois arrays elemento a elemento: mesmo comprimento, mesmos elementos, mesma ordem. `===` e `!==` não comparam arrays; o compilador reporta `E0369` e aponta para cá.

<SignatureBlock label="Assinatura">equals(other: T[]): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-equals.md -->

#### `indexOf()`

Retorna o primeiro índice de um valor, ou `-1` se não encontrado.

<SignatureBlock label="Assinatura">indexOf(value: T, fromIndex?: i64): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-indexOf.md -->

#### `lastIndexOf()`

Retorna o último índice de um valor, ou `-1` se não encontrado.

<SignatureBlock label="Assinatura">lastIndexOf(value: T, fromIndex?: i64): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-lastIndexOf.md -->

### Janelas, Acumulação e Matrizes

#### `windows()`

`windows(size)` retorna cópias sobrepostas de `size` elementos consecutivos. O receiver não muda; `windows(0)` é rejeitado e um tamanho maior que o array retorna um array vazio.

<!-- @include: ../../.fragments/language/collections/windows.md -->

#### `scan()`

`scan(initial, callback)` retorna o acumulador inicial seguido de cada valor acumulado. A callback recebe o acumulador e o elemento atual, e o receiver não muda.

<!-- @include: ../../.fragments/language/collections/scan.md -->

#### `cartesianProduct()`

`cartesianProduct(other)` retorna pares na ordem lexicográfica do receiver e do argumento. Qualquer entrada vazia produz um resultado vazio, e o receiver não muda.

<!-- @include: ../../.fragments/language/collections/cartesianproduct.md -->

#### `zip()`

`zip(other)` intercala o receiver e o argumento em uma array de pares `(T, U)[]`, em que `T` é o tipo elementar do receiver e `U` é o tipo elementar do argumento. O comprimento do resultado é igual ao das duas entradas, que precisam ser iguais; as arrays fonte não mudam.

<!-- @include: ../../.fragments/language/collections/zip.md -->

Comprimentos diferentes lançam um `ZipError` recuperável (code `ERR_ARRAY_ZIP_MISMATCH`) em vez de abortar, então a chamada precisa ser coberta por um `catch` ou por uma cláusula `throws ZipError` na função que a contém:

<!-- @include: ../../.fragments/language/collections/zip-2.md -->

#### `transpose()`

`transpose()` transforma uma matriz retangular em colunas. Matrizes vazias e matrizes com linhas de largura zero retornam arrays vazios; linhas irregulares falham antes de produzir um resultado parcial.

<!-- @include: ../../.fragments/language/collections/transpose.md -->

#### `unique()`

`unique()` retorna a primeira ocorrência de cada valor distinto na ordem de entrada. Ele não altera o receiver e usa a política nativa de igualdade para strings e números.

<!-- @include: ../../.fragments/language/collections/unique.md -->

#### `uniqueBy()`

`uniqueBy(selector)` mantém o primeiro elemento para cada chave hashable distinta. O seletor pode ser o nome de um campo para elementos objeto, uma callback explícita ou uma expressão implícita, na qual o tipo do elemento é inferido automaticamente.

<!-- @include: ../../.fragments/language/collections/uniqueby.md -->

#### `groupBy()`

`groupBy(selector)` cria um mapa de cada chave hashable para um array de elementos na ordem original. O seletor pode ser o nome de um campo para elementos objeto, uma callback explícita ou uma expressão implícita, na qual o tipo do elemento é inferido automaticamente. A primeira implementação exige elementos Copy para que cada grupo mantenha seus valores com segurança.

<!-- @include: ../../.fragments/language/collections/groupby.md -->

### Flatten & Reordenação

#### `flat()`

Achata arrays aninhados em um nível.

<SignatureBlock label="Assinatura">flat(): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-flat.md -->

#### `flatMap()`

Mapeia cada elemento para um array e achata o resultado em um nível.

<SignatureBlock label="Assinatura">flatMap&lt;U&gt;(callback: (element: T) =&gt; U[]): U[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-flatMap.md -->

#### `sort()`

Ordena o array in place usando uma função de comparação.

<SignatureBlock label="Assinatura">sort(compare?: (left: T, right: T) =&gt; i32): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-sort.md -->

#### `toSorted()`

Retorna uma nova cópia ordenada sem modificar o original.

<SignatureBlock label="Assinatura">toSorted(compare?: (left: T, right: T) =&gt; i32): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-toSorted.md -->

#### `sortBy()`

Ordena um array in place por um campo resolvido estaticamente. A chave deve ser um literal string com o nome de um campo numérico, booleano, de caractere ou string. O segundo argumento é opcional: a ordem padrão é crescente e `"desc"` seleciona a ordem decrescente.

<SignatureBlock label="Assinatura">sortBy(key: keyof T, order?: "asc" | "desc"): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sortby.md -->

O campo é resolvido em compile-time, então o loop de merge sort estável gerado lê o campo diretamente sem reflexão em runtime. `sortBy()` exige um receiver mutável em `let` e executa em `O(n log n)`, com `O(n)` de espaço auxiliar.

#### `sortedBy()`

Retorna uma cópia ordenada por um campo resolvido estaticamente e mantém o receiver inalterado. O segundo argumento é opcional: a ordem padrão é crescente e `"desc"` seleciona a ordem decrescente. A primeira versão exige que os campos do elemento sejam `Copy`, evitando uma cópia rasa insegura de valores possuídos. Ela usa merge sort estável `O(n log n)`, com `O(n)` de espaço auxiliar.

<SignatureBlock label="Assinatura">sortedBy(key: keyof T, order?: "asc" | "desc"): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sortedby.md -->

#### `reverse()`

Inverte a ordem dos elementos in place.

<SignatureBlock label="Assinatura">reverse(): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-reverse.md -->

#### `rotate()`

Rotaciona o array in place. Valores positivos rotacionam para a esquerda; valores negativos rotacionam para a direita. O valor é normalizado pelo comprimento do array, e arrays vazios ou com um único elemento não mudam.

<SignatureBlock label="Assinatura">rotate(amount: i64): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/rotate.md -->

`rotate()` exige um receiver mutável em `let` e executa em `O(n)` com espaço auxiliar `O(1)`, sem alocar um segundo array.

#### `toReversed()`

Retorna uma nova cópia com ordem invertida sem modificar o original.

<SignatureBlock label="Assinatura">toReversed(): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-toReversed.md -->

#### `random()`

Retorna um `T` selecionado uniformemente. Um array vazio aborta em runtime.

<SignatureBlock label="Assinatura">random(): T</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/random.md -->

::: danger error
<!-- @include: ../../.fragments/language/collections/random-2.md -->

<!-- @include: ../../.fragments/language/collections/random-3.md -->
:::

#### `shuffle()`

Retorna um novo array embaralhado uniformemente sem modificar o receptor. Arrays com menos de dois elementos abortam em runtime.

<SignatureBlock label="Assinatura">shuffle(): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/shuffle.md -->

::: danger error
<!-- @include: ../../.fragments/language/collections/shuffle-2.md -->

<!-- @include: ../../.fragments/language/collections/shuffle-3.md -->
:::

#### `sample()`

Retorna um novo array com `count` elementos selecionados uniformemente. `count` deve estar entre `1` e o tamanho do receptor, inclusive. Sem opções, posições de origem não se repetem. `{ repeat: true }` permite repetições de posições, mas não as garante.

<SignatureBlock label="Assinatura">sample(count: usize, options?: { repeat?: boolean }): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sample.md -->

::: danger error
<!-- @include: ../../.fragments/language/collections/sample-2.md -->

<!-- @include: ../../.fragments/language/collections/sample-3.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/collections/sample-4.md -->

<!-- @include: ../../.fragments/language/collections/sample-5.md -->
:::

### Fatiamento

#### `slice()`

Extrai uma cópia de um subarray contíguo.

<SignatureBlock label="Assinatura">slice(start?: i64, end?: i64): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-slice.md -->

Indexar com um `Range` é açúcar para `.slice()`: `arr[a..b]` é `arr.slice(a, b)`, e toda forma aberta também funciona, `arr[a..]`, `arr[..b]`, `arr[..=b]` e o range completo `arr[..]`. Assim como `.slice()`, isso sempre copia para um array novo; arrays de tyrus ainda não têm representação de fatia emprestada sem cópia.

<!-- @include: ../../.fragments/language/collections/range-slicing.md -->

#### `splitAt()`

Divide o array em dois arrays em um índice dado.

<SignatureBlock label="Assinatura">splitAt(index: i64): [T[], T[]]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-splitAt.md -->

`splitAt` sempre retorna dois arrays comuns de tamanho dinâmico (`[T[], T[]]`), inclusive quando chamado em um array de tamanho fixo. `index` pode ser igual ao comprimento do array, produzindo uma segunda metade vazia, mas um índice maior é erro de compilação quando o comprimento do array é conhecido estaticamente, e abort em tempo de execução caso contrário.

### Agregação

#### `count()`

Retorna o comprimento, ou conta ocorrências de um valor ou predicado.

<SignatureBlock label="Assinatura">count(value?: T | (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-count.md -->

`count()` retorna o comprimento; `count(valor)` conta elementos iguais a `valor`; `count(condição)`, uma expressão comum referenciando exatamente um identificador ainda não declarado no escopo atual, vira uma condição implícita por elemento, açúcar para `count((implicitParam): boolean => condição)` sem precisar escrever nenhuma arrow function. Um identificador que já resolve no escopo não é livre, então a expressão é checada como uma chamada comum `count(valor)`, comparar seu resultado (constante) `boolean` contra o tipo do elemento é erro de compilação citando o identificador que sombreou o parâmetro implícito. Uma expressão referenciando mais de um identificador não declarado também é erro de compilação, já que o açúcar só introduz um parâmetro implícito.

#### `sum()`

Retorna a soma de todos os elementos numéricos usando o tipo dos elementos do array. Um array numérico vazio retorna o valor zero desse tipo.

<SignatureBlock label="Assinatura">sum(): T</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sum.md -->

`sum()` é rejeitado para tipos de elemento não numéricos e é compilado como um loop nativo direto, sem callback ou despacho ao runtime por elemento.

#### `product()`

Retorna o produto de todos os elementos numéricos usando o tipo dos elementos do array. Um array numérico vazio retorna o valor um desse tipo.

<SignatureBlock label="Assinatura">product(): T</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/product.md -->

`product()` segue as mesmas regras de tipo numérico e loop nativo de `sum()`.

#### `avg()`

Retorna a média aritmética dos elementos numéricos. Arrays de inteiros, `f32` e `f64` retornam `f64?`; arrays de `f128` retornam `f128?`. Um array vazio produz ausência.

<SignatureBlock label="Assinatura">avg(): f64? | f128?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/avg.md -->

O resultado opcional deve ser tratado com `??`, narrowing ou outra operação válida de opcionais antes do uso numérico direto.

#### `sumBy()`

Retorna a soma de um campo numérico em todos os elementos. A chave deve ser um literal de string nomeando um campo numérico.

<SignatureBlock label="Assinatura">sumBy(key: string): T[K]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sumby.md -->

#### `avgBy()`

Retorna a média aritmética de um campo numérico em todos os elementos. A chave deve ser um literal de string nomeando um campo numérico. Sempre retorna `f64?` (ou `f128?` para campos `f128`).

<SignatureBlock label="Assinatura">avgBy(key: string): f64? | f128?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/avgby.md -->

#### `concat()`

Mescla dois arrays em um novo.

<SignatureBlock label="Assinatura">concat(...items: (T | T[])[]): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-concat.md -->

#### `join()`

Junta todos os elementos em uma única string usando um separador.

<SignatureBlock label="Assinatura">join(separator: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-join.md -->

### Mutação

#### `splice()`

Remove, substitui ou insere elementos in place.

<SignatureBlock label="Assinatura">splice(start: i64, deleteCount?: i64, ...items: T[]): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-splice.md -->

`splice(start, deleteCount?, ...items)` modifica o receptor e retorna um novo array com os elementos removidos. Índices iniciais negativos contam a partir do fim, a remoção é limitada ao intervalo disponível e os valores inseridos devem corresponder ao tipo dos elementos do array.

#### `fill()`

Substitui todos os elementos por um valor fixo.

<SignatureBlock label="Assinatura">fill(value: T): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-fill.md -->

#### `copyWithin()`

Copia uma seção contígua do array para outra posição no mesmo array.

<SignatureBlock label="Assinatura">copyWithin(target: i64, start: i64, end?: i64): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-copyWithin.md -->

### Acesso & Iteração

#### `first()`

Retorna o primeiro elemento como `T?`. Um array vazio produz ausência. A anotação de tipo é opcional: quando `values` é `i32[]`, o tyrus infere `i32?` automaticamente. `Optional<i32>` é a forma explícita equivalente.

<SignatureBlock label="Assinatura">first(): T?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/first.md -->

`T?` é intencional: `first()` retorna um valor, não uma coleção, e continua `O(1)` sem alocar um array vazio como resultado. O uso direto como `i32` é rejeitado até que um `if`, `??`, ternário, optional chaining ou outra operação válida de narrowing trate a ausência.

#### `last()`

Retorna o último elemento como `T?`. Um array vazio produz ausência, e o tyrus infere automaticamente o tipo opcional do elemento quando a anotação é omitida.

<SignatureBlock label="Assinatura">last(): T?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/last.md -->

`last()` segue as mesmas regras de ausência e narrowing de `first()`, lendo o elemento final em `O(1)` sem alocar um array de resultado.

#### `at()`

Retorna o elemento em um índice, suportando indexação negativa a partir do fim.

<SignatureBlock label="Assinatura">at(index: i64): T</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-at.md -->

#### `keys()`

Retorna um iterador sobre os índices do array.

<SignatureBlock label="Assinatura">keys(): Iterable&lt;i64&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-keys.md -->

#### `values()`

Retorna um iterador sobre os valores do array.

<SignatureBlock label="Assinatura">values(): Iterable&lt;T&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-values.md -->

#### `entries()`

Retorna um iterador sobre pares índice-valor.

<SignatureBlock label="Assinatura">entries(): Iterable&lt;[i64, T]&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-entries.md -->

### Cópia Imutável

#### `with()`

Retorna um novo array com um único elemento substituído em um índice dado.

<SignatureBlock label="Assinatura">with(index: i64, value: T): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-with.md -->

### Toggle

#### `toggle()`

Adiciona um elemento ao array se ele não estiver presente e o remove se estiver presente, retornando um novo array e nunca mutando o receiver.

<SignatureBlock label="Assinatura">toggle(item: T): T[]</SignatureBlock>

<SignatureBlock label="Assinatura">toggle(item: T, key: string): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/toggle.md -->

Quando um nome de campo `key` é informado, a comparação é feita por esse campo em vez do elemento como um todo. O campo deve existir no tipo do elemento.

<!-- @include: ../../.fragments/language/collections/toggle-2.md -->

#### `toggleAll()`

Aplica o mesmo comportamento de adicionar ou remover a cada elemento do array argumento, preservando a ordem. Não muta nada.

<SignatureBlock label="Assinatura">toggleAll(items: T[]): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/toggleall.md -->

### Encadeamento

Métodos funcionais se encadeiam em pipelines de dados legíveis:

<!-- @include: ../../.fragments/language/collections/functional-chaining.md -->
