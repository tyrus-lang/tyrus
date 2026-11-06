---
title: Objetos e Interfaces
description: Interfaces, type aliases, campos opcionais e readonly, updates de objeto, unions e type utilities.
---

# Objetos e Interfaces

Tyrus usa sintaxe concisa para objetos, e interfaces nomeadas e aliases de objeto fazem lowering para layouts de struct nativos.

## Interfaces

<!-- @include: ../../.fragments/language/objects/interfaces.md -->

`readonly` impede reatribuição do campo. Campos opcionais leem como `T?` e devem ser tratados com narrowing ou `??`. Literais de objeto são checados para campos ausentes, extras e com tipo incorreto. Object spread cria updates imutáveis com o mesmo layout de interface do objeto de origem. Quando o valor de um campo é uma variável local com o mesmo nome, escreva o nome uma única vez, `{ active }` é açúcar sintático para `{ active: active }`, e combina livremente com campos explícitos `key: value` e com um spread update.

## Métodos automáticos de dados

Toda interface de dados nomeada recebe métodos automáticos de dados sem anotações nem declarações. Os quatro primeiros são chamadas de instância comuns, e `fromJson` é uma chamada estática no nome concreto da interface. Uma interface de comportamento pode definir um método de instância com o mesmo nome e tem precedência sobre o método automático.

### `copy()`

Cria uma cópia estrutural independente.

`copy()` não é só para objetos: existe em todo valor que possui um buffer, ou seja, todo tipo que não é `Copy`. Isso cobre `string`, arrays, tuplas e os opcionais, results e uniões construídos a partir deles. Num escalar `Copy` como `i32` o método não existe, porque `let a = b` já duplica e um método sugeriria que outra coisa acontece.

O resultado é um valor comum do próprio tipo. Uma decoration pertence ao campo de onde o valor foi lido, não ao valor, então nada da declaração de origem acompanha a cópia.

Ler um campo para um binding do mesmo tipo o move: `const items = order.items;` entrega o campo a `items`, `order` continua utilizável pelos outros campos, e ler `order.items` ou `order` inteiro depois é recusado (`E0382`) até o campo ser atribuído de novo. Onde o move não pode acontecer, o compilador sugere `copy()`: quando os campos do dono têm validators, cujas regras descrevem o valor inteiro, quando o campo é alcançado por uma referência, ou quando o binding é declarado com um tipo diferente do campo, como um `string` lido de um `string?` estreitado (`E3081`). `order.items.copy()` diz o que faz e mantém o custo visível, o que uma cópia inserida em silêncio não faria.

<!-- @include: ../../.fragments/language/objects/automatic-data-copy.md -->

### `equals(other: Type)`

Compara campos elegíveis estruturalmente, inclusive campos opcionais ausentes e presentes. Arrays não são elegíveis porque sua igualdade é por endereço. O argumento precisa ter a mesma interface nominal.

<!-- @include: ../../.fragments/language/objects/automatic-data-equals.md -->

Uma interface com campos correspondentes ainda não pode ser passada para `equals()` quando tiver outro nome:

::: danger error
<!-- @include: ../../.fragments/language/objects/automatic-data-methods-equals-error.md -->
:::

### `toString()`

Produz uma representação legível na ordem declarada dos campos, coloca aspas e escapes nas strings, mostra campos opcionais ausentes como `None` e renderiza valores aninhados recursivamente.

<!-- @include: ../../.fragments/language/objects/automatic-data-to-string.md -->

### `toJson(options?: { pretty: boolean })`

Emite JSON canônico com espaço após cada dois-pontos. Passe `{ pretty: true }` para JSON em múltiplas linhas com indentação de quatro espaços. As chaves saem em ordem alfabética, não na ordem de declaração, para que o mesmo dado produza sempre o mesmo texto: acrescentar um campo depois não move os que já estavam lá. Um `decimal` é escrito em notação comum, o mesmo texto com que ele é impresso: `12.50d` sai como `12.5`, com todos os dígitos preservados e sem arredondamento.

<!-- @include: ../../.fragments/language/objects/automatic-data-to-json.md -->

Um campo que carrega uma união discriminada é escrito como o objeto que a variante viva já é: não há envelope, e o campo discriminante é o que diz ao leitor qual variante ele recebeu. O `fromJson` lê de volta do mesmo jeito, escolhendo a variante pelo discriminante; um discriminante que não nomeia nenhuma variante declarada é recusado com `ERR_JSON_UNKNOWN_VARIANT` em vez de adivinhado, que é justamente para isso que ele é escrito.

<!-- @include: ../../.fragments/language/objects/automatic-data-to-json-union.md -->

### `Type.fromJson(text: string)`

Valida sintaxe, campos, tipos e faixas numéricas, retornando `JsonError` com `code` estável, `message` legível e `path` do campo em vez de lançar.

<!-- @include: ../../.fragments/language/objects/json-error.md -->

<!-- @include: ../../.fragments/language/objects/automatic-data-from-json.md -->

Use `onfail` para imprimir um `JsonError` e encerrar a função `void` atual quando a decodificação falhar:

::: danger error
<!-- @include: ../../.fragments/language/objects/automatic-data-from-json-onfail.md -->
:::


## Interfaces de comportamento com `at`

Interfaces de comportamento anexam métodos executáveis a um ou mais tipos receptores com `at`. O receptor é o primeiro parâmetro explícito, então o método continua sendo comum e a chamada usa a sintaxe `value.method()`. Quando sua anotação é omitida para múltiplos alvos, o compilador especializa e verifica a implementação separadamente para cada alvo. Uma interface de comportamento contém métodos ou shorthand bindings, não campos de dados.

<!-- @include: ../../.fragments/language/objects/behavior-interfaces.md -->

Shorthand bindings nomeiam funções declaradas no mesmo módulo ou importadas de outro módulo. As assinaturas são checadas quando o comportamento é declarado, e a assinatura importada é preservada entre módulos para a checagem de tipos e a geração de código nativo. Interfaces de comportamento também podem usar `extend` para herdar métodos de outras interfaces de comportamento.

O receptor de `at` pode ser uma interface nomeada ou um tipo primitivo. A mesma sintaxe de chamada por membro funciona para valores numéricos, strings, booleanos e chars.

## Acesso a propriedade por nome

Um campo é lido e escrito com a forma de ponto e com um literal de string entre colchetes, os dois são o mesmo acesso: a chave está escrita no código, então ela nomeia um campo que o compilador conhece, e ambos compilam para o mesmo offset fixo.

<!-- @include: ../../.fragments/language/objects/literal-property-access.md -->

Só um literal se qualifica. `person[chave]`, com `chave` sendo uma variável, é erro de compilação (`E0608`): uma interface tem um conjunto fixo de campos e nenhuma busca em tempo de execução para onde recorrer. Um literal que não nomeia campo nenhum é `E0609`, reportado com o nome. Para chaves decididas em tempo de execução, use `Record<string, T>`, que é um hash map de verdade, `lookup["a"]` ali é uma busca que pode falhar, então devolve `T?`.

## Atribuição de campo

Um campo pode ser atribuído no lugar com `object.field = value`, incluindo as formas compostas como `+=`, através de receptores aninhados e em elementos indexados.

<!-- @include: ../../.fragments/language/objects/field-assignment.md -->

A atribuição no lugar exige um binding mutável: um binding `const` é profundamente imutável, e um campo `readonly` ou um valor de um tipo `readonly` rejeita a escrita. A atribuição no lugar hoje funciona para campos de número, booleano e char; para trocar um campo `string`, um array ou um objeto aninhado, construa um novo valor com object spread.

## Herança de Interface

Uma interface pode `extend` uma ou mais outras interfaces para herdar seus campos. A interface filha exige todos os campos herdados mais os próprios, então um valor do tipo filho precisa fornecer todos eles. A herança só faz merge de campos, ela não torna a filha atribuível à pai (veja o layout nominal abaixo).

<!-- @include: ../../.fragments/language/objects/interface-inheritance.md -->

## Interseção de Tipos

Um alias `type` combina tipos de objeto com `&`. O alias exige todos os campos de todos os operandos, exatamente como a herança faz, e cada operando é uma interface nomeada ou um objeto escrito inline. O `&` é a forma do alias para o que o `extend` faz numa interface: `interface` herda com `extend`, `type` combina com `&`.

<!-- @include: ../../.fragments/language/objects/type-intersection.md -->

Quando dois operandos declaram o mesmo campo com tipos diferentes, o alias é recusado onde está escrito (`E3214`) em vez de produzir um campo que nenhum valor consegue satisfazer.

::: danger error
<!-- @include: ../../.fragments/language/objects/type-intersection-conflict.md -->
:::

O `&` pertence a um alias `type` e a mais nenhum lugar. Ele não é aceito numa anotação de tipo (`E3215`): nomeie a interseção primeiro e depois use o nome.

::: danger error
<!-- @include: ../../.fragments/language/objects/type-intersection-annotation.md -->
:::

Todo operando precisa ser um tipo de objeto. Um operando que seja um primitivo, um array, uma união ou uma função é recusado com `E3213`, e um alias que se refere a si mesmo pela cadeia é recusado com `E3216`. Misturar `&` e `|` no mesmo alias é recusado com `E3217`, um diagnóstico próprio porque o motivo é a precedência: o `|` liga mais forte, então `type A = B & C | D;` seria lido como `B & (C | D)`. Só parênteses não resolvem, já que uma união nunca é um operando válido. Nomeie a interseção e use esse nome na união: `type BC = B & C;` e depois `type A = BC | D;`.


## Layout Nativo Nominal

No tyrus não trata todo formato de objeto compatível como intercambiável. Interfaces são tipos produto nativos nomeados, então passar um `CompleteUser` onde `SummaryUser` é esperado exige um objeto de conversão explícito.

<!-- @include: ../../.fragments/language/objects/nominal-native-layout.md -->

Para guardar valores de várias formas numa mesma coleção, use uma tagged union com um campo discriminante literal e `match` em vez de uma interface base compartilhada, ela preserva os campos de cada variante e mantém a exaustividade:

<!-- @include: ../../.fragments/language/objects/tagged-union-narrowing.md -->

### Ordem de Armazenamento dos Campos

Os campos são armazenados na ordem que desperdiça menos espaço, que nem sempre é a ordem em que foram escritos: um campo largo é posto à frente de um estreito para que o vão que o estreito deixaria antes dele desapareça. A interface abaixo ocupa 40 bytes escrita assim e 24 bytes armazenada, e guardar um milhão delas é a diferença entre 40 MB e 24 MB.

<!-- @include: ../../.fragments/language/objects/field-storage-order.md -->

Nada muda no uso da interface: os campos são lidos, escritos, espalhados e desestruturados por nome, um literal de objeto continua avaliando seus campos na ordem em que os lista, e imprimir um valor continua mostrando os campos na ordem em que a interface os declara. A ordem é uma decisão de armazenamento e nunca é observável a partir do programa.

## Type Aliases

<!-- @include: ../../.fragments/language/objects/type-aliases.md -->

Para formatos de objeto, `type` e `interface` são equivalentes no layout nativo. Um alias `type` também pode nomear unions, tuplas, tipos de função e utility types.

## Utilitários de Tipo

| Categoria | Utilities |
| --- | --- |
| Transformação de objeto | [`Partial<T>`](#partial-t), [`Optional<T, K>`](#optional-t-k), [`Required<T>`](#required-t), [`Readonly<T>`](#readonly-t), [`DeepPartial<T>`](#deeppartial-t) |
| Seleção de chaves | [`Pick<T, K>`](#pick-t-k), [`Omit<T, K>`](#omit-t-k) |
| Seleção por tipo | [`PickByType<T, Type>`](#pickbytype-t-type), [`OmitByType<T, Type>`](#omitbytype-t-type) |
| Renomeação de chaves | [`CamelCaseKeys<T>`](#camelcasekeys-t), [`SnakeCaseKeys<T>`](#snakecasekeys-t) |
| Controle de campos | [`Require<T, K>`](#require-t-k), [`AtLeastOne<T, K>`](#atleastone-t-k) |
| Extração de dados | [`DataOnly<T>`](#dataonly-t) |
| Mapeamento chave/valor | [`keyof`](#keyof), [`Record<K, T>`](#record-k-t) |
| Tipos de função | [`ReturnType<T>`](#returntype-t), [`Parameters<T>`](#parameters-t) |
| Manipulação de union | [`Exclude<T, U>`](#exclude-t-u), [`Extract<T, U>`](#extract-t-u) |
| Tipos condicionais | [`T extends U ? X : Y`](#t-extends-u-x-y), [`infer R`](#infer-r) |

### Transformação de objeto

#### `Partial<T>`

Torna todo campo opcional, quem chama pode fornecer qualquer subconjunto.

<SignatureBlock label="Assinatura">Partial&lt;T&gt; = { [K in keyof T]?: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/partial.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/partial-error.md -->
:::

#### `Optional<T, K>`

Mantém todos os campos e torna opcionais somente as chaves selecionadas. A forma existente com um argumento, `Optional<T>`, continua sendo a grafia de `T?`.

<SignatureBlock label="Assinatura">Optional&lt;T, K extends keyof T&gt; = { [P in keyof T]: P extends K ? T[P]? : T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/optional.md -->

#### `Required<T>`

Torna todo campo opcional em obrigatório, todos os campos devem ser fornecidos.

<SignatureBlock label="Assinatura">Required&lt;T&gt; = { [K in keyof T]-?: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/required.md -->

#### `Readonly<T>`

Impede a reatribuição de campos, o objeto inteiro se torna profundamente imutável. É um dos seis lugares em que a palavra-chave aparece; a página [Readonly](/pt/language/readonly) cobre todos eles.

<SignatureBlock label="Assinatura">Readonly&lt;T&gt; = { readonly [K in keyof T]: Readonly&lt;T[K]&gt; }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/readonly.md -->

#### `DeepPartial<T>`

Torna todo campo opcional recursivamente, até 5 níveis de profundidade. Ideal para payloads de atualização parcial (PATCH).

<SignatureBlock label="Assinatura">DeepPartial&lt;T&gt; = { [K in keyof T]?: DeepPartial&lt;T[K]&gt; }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/deep-partial.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/deep-partial-error.md -->
:::

### Seleção de chaves

#### `Pick<T, K>`

Seleciona um subconjunto de campos por chave, o resultado tem apenas as chaves listadas.

<SignatureBlock label="Assinatura">Pick&lt;T, K extends keyof T&gt; = { [P in K]: T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/pick.md -->

#### `Omit<T, K>`

Remove um subconjunto de campos por chave, o resultado tem tudo exceto as chaves listadas.

<SignatureBlock label="Assinatura">Omit&lt;T, K extends keyof T&gt; = { [P in Exclude&lt;keyof T, K&gt;]: T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/omit.md -->

### Projeções de valor

`value.pick(keys)` e `value.omit(keys)` criam um novo objeto com os campos selecionados ou restantes. As chaves precisam ser conhecidas em compilação, em um array literal ou em uma variável com tipo finito de chaves literais. O receptor não é mutado, a ordem dos campos segue o tipo de origem e não há busca de chave em runtime.

<!-- @include: ../../.fragments/language/objects/type-utilities/optional.md -->

### Seleção por tipo

#### `PickByType<T, Type>`

Seleciona campos pelo tipo do valor, mantém apenas os campos cujo tipo corresponde a `Type`.

<SignatureBlock label="Assinatura">PickByType&lt;T, Type&gt; = { [K in keyof T as T[K] extends Type ? K : never]: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/pick-by-type.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/pick-by-type-error.md -->
:::

#### `OmitByType<T, Type>`

Remove campos pelo tipo do valor, exclui todos os campos cujo tipo corresponde a `Type`.

<SignatureBlock label="Assinatura">OmitByType&lt;T, Type&gt; = { [K in keyof T as T[K] extends Type ? never : K]: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/omit-by-type.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/omit-by-type-error.md -->
:::

### Mapeamento chave/valor

#### `keyof`

Extrai as chaves de um tipo como uma union literal de string.

<SignatureBlock label="Assinatura">keyof T = string literal union of T's keys</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/keyof.md -->

#### `Record<K, T>`

`Record<K, T>` é um tipo utilitário embutido, como `Pick<T, K>` e `Omit<T, K>`. O compilador o resolve e ele não cria uma classe `Record` em runtime. Quando `K` é uma união finita de chaves string literais, o Tyrus produz um tipo objeto com um campo obrigatório por chave. Quando `K` é aberto, como `string`, o Tyrus o resolve para `Map<K, T>`, adequado a chaves dinâmicas.

<SignatureBlock label="Assinatura">Record&lt;K extends string, T&gt; = campos obrigatórios de objeto para K literal finito; Map&lt;K, T&gt; para K aberto</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/record.md -->

### Renomeação de chaves

#### `CamelCaseKeys<T>`

Transforma todos os nomes de campo de `snake_case` para `camelCase`. Essencial para integrações com APIs externas.

<SignatureBlock label="Assinatura">CamelCaseKeys&lt;T&gt; = { [K in camelCase&lt;keyof T&gt;]: T[snake_case&lt;K&gt;] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/camel-case-keys.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/camel-case-keys-error.md -->
:::

#### `SnakeCaseKeys<T>`

Transforma todos os nomes de campo de `camelCase` para `snake_case`. Útil para payloads de APIs legadas e bancos de dados.

<SignatureBlock label="Assinatura">SnakeCaseKeys&lt;T&gt; = { [K in snake_case&lt;keyof T&gt;]: T[camelCase&lt;K&gt;] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/snake-case-keys.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/snake-case-keys-error.md -->
:::

### Tipos de função

#### `ReturnType<T>`

Infere o tipo de retorno de um tipo função, extrai o que a função produz.

<SignatureBlock label="Assinatura">ReturnType&lt;T extends (...args: any) =&gt; any&gt; = T's return type</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/returntype.md -->

#### `Parameters<T>`

Infere os tipos dos parâmetros de um tipo função como tupla, extrai o que a função espera.

<SignatureBlock label="Assinatura">Parameters&lt;T extends (...args: any) =&gt; any&gt; = T's parameter tuple</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/parameters.md -->

### Controle de campos

#### `Require<T, K>`

Torna campos específicos obrigatórios. É um noop se o campo já for obrigatório.

<SignatureBlock label="Assinatura">Require&lt;T, K extends keyof T&gt; = T & { [P in K]-?: T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/require.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/require-error.md -->
:::

#### `AtLeastOne<T, K>`

Torna todos os campos especificados opcionais mas exige que pelo menos um deles esteja presente. Cada chave em `K` deve ser uma propriedade de `T`.

<SignatureBlock label="Assinatura">AtLeastOne&lt;T, K extends keyof T&gt; = { [P in keyof T]: P extends K ? T[P]? : T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/at-least-one.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/at-least-one-error.md -->
:::

### Extração de dados

#### `DataOnly<T>`

Remove todos os campos com tipo função, mantém apenas propriedades de dados puros. Ideal para payloads de API.

<SignatureBlock label="Assinatura">DataOnly&lt;T&gt; = { [K in keyof T as T[K] extends Function ? never : K]: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/data-only.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/data-only-error.md -->
:::

### Manipulação de união

#### `Exclude<T, U>`

Remove membros de `U` de `T`, mantém tudo em `T` que não está em `U`.

<SignatureBlock label="Assinatura">Exclude&lt;T, U&gt; = T extends U ? never : T</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/exclude.md -->

#### `Extract<T, U>`

Mantém apenas membros de `T` que também pertencem a `U`, calcula a interseção.

<SignatureBlock label="Assinatura">Extract&lt;T, U&gt; = T extends U ? T : never</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/extract.md -->

### Tipos condicionais

Tipos condicionais escolhem entre dois tipos em tempo de compilação. A restrição `extends` seleciona o ramo, e `infer` extrai um tipo de dentro da estrutura correspondente.

#### `T extends U ? X : Y`

Um tipo condicional resolve para `X` quando `T` satisfaz a restrição `U`, e para `Y` caso contrário.

<SignatureBlock label="Assinatura">T extends U ? Then : Else</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/conditional.md -->

#### `infer R`

Vincula uma variável de tipo dentro da restrição de um condicional, quando a restrição se mantém, o tipo inferido vira o ramo verdadeiro.

<SignatureBlock label="Assinatura">T extends (arg: i32) =&gt; infer R ? R : never</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/infer.md -->

## Union Types

<!-- @include: ../../.fragments/language/objects/union-types.md -->

Tagged unions fazem narrowing por checagens de discriminante literal. O compilador rastreia branches positivos e negativos para que cada branch exponha os campos disponíveis no conjunto de variantes narrow.

Unions também funcionam com enums para payloads de eventos tipados:

<!-- @include: ../../.fragments/language/objects/union-with-enums.md -->

## Enums algébricos

Enums algébricos modelam um conjunto fechado de variantes, onde cada variante pode não ter payload, ter campos posicionais ou campos nomeados. Um `match` sobre um enum algébrico deve ser exaustivo salvo se incluir `_`; padrões de tupla e struct vinculam diretamente o payload selecionado.

<!-- @include: ../../.fragments/language/objects/algebraic-enums.md -->
