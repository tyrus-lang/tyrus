---
title: Tipos
description: Tipos primitivos, numéricos, opcionais, objeto, array, tupla, union e função em tyrus.
---

# Tipos

Tyrus usa anotações de tipo e semântica nativa aplicada pelo compilador. Valores geralmente podem ser inferidos a partir dos inicializadores, mas anotações explícitas são recomendadas em fronteiras de API e quando a largura numérica importa.

::: tip Inferência é local e previsível
`const answer = 42;` é inferido a partir do literal. Use `const answer: i64 = 42;` quando a largura faz parte do contrato.
:::

::: warning Um literal de objeto nomeia sua interface
Interfaces são nominais, então um literal declara a interface que constrói em vez de ter uma adivinhada pela forma: `const user: User = { id: 1 };`. Um literal de objeto sem anotação é erro, e o `help:` nomeia toda interface de forma compatível, oferecida no editor como Quick Fix. Adivinhar pela forma faria o significado de um arquivo depender de todas as interfaces do programa.
:::

## Tipos primitivos

| Tipo | Use para | Exemplo |
|------|----------|---------|
| `boolean` | Valores de verdade e condições de branch. | `const enabled: boolean = true;` |
| `i8` | Bytes e valores assinados pequenos, com um byte de largura. | `const delta: i8 = -8;` |
| `i16` | Valores assinados pequenos, com dois bytes de largura. | `const offset: i16 = -1024;` |
| `i32` | Inteiro assinado default para contadores, IDs e a maioria da aritmética. | `const count: i32 = 42;` |
| `i64` | Inteiros assinados grandes, timestamps e identificadores persistidos. | `const micros: i64 = 1720000000000;` |
| `i128` | Cálculos inteiros assinados muito grandes. | `const ledger: i128 = 9000000000000000000;` |
| `isize` | Índices assinados pointer-sized e valores de tamanho nativo. | `const offset: isize = -1;` |
| `u8` | Bytes, códigos de protocolo e canais de cor. | `const red: u8 = 255;` |
| `u16` | Portas, code points e campos de protocolo de dois bytes. | `const port: u16 = 8080;` |
| `u32` | Contadores e tamanhos unsigned que cabem em 32 bits. | `const size: u32 = 4096;` |
| `u64` | Contadores e identificadores unsigned grandes. | `const sequence: u64 = 10000000000;` |
| `u128` | Cálculos inteiros unsigned muito grandes. | `const mask: u128 = 340282366920938463463374607431768211455;` |
| `usize` | Índices, comprimentos e capacidades unsigned pointer-sized. | `const length: usize = names.length;` |
| `f32` | Valores de ponto flutuante compactos quando a troca de precisão é aceitável. | `const ratio: f32 = 0.75;` |
| `f64` | Cálculos de ponto flutuante default. | `const total: f64 = 19.99;` |
| `f128` | Cálculos de ponto flutuante estendidos e constantes nativas min/max. | `const largest: f128 = f128.MAX;` |
| `decimal` | Aritmética exata de base dez com 34 algarismos significativos. | `const total: decimal = 0.1d + 0.2d;` |
| `char` | Um único caractere Unicode, para percorrer texto caractere a caractere. | `const letter: char = 'A';` |
| `string` | Texto UTF-8 com ownership. | `const label: string = "tyrus";` |
| `void` | Funções que não retornam valor. | `function log(): void { print("ok"); }` |
| `unknown` | Valores que precisam de narrowing antes do uso. | `const payload: unknown = readInput();` |
| `never` | Caminhos de código que não retornam normalmente. | `function fail(message: string): never { throw message; }` |

## Tipos numéricos

Use inteiros assinados (`i*`) quando valores podem ser negativos. Use inteiros unsigned (`u*`) para bytes, tamanhos, campos de protocolo e valores que nunca podem ser negativos. Use `f32`/`f64` para valores fracionários, não para dinheiro ou contadores exatos.

<!-- @include: ../../.fragments/language/types/numeric-types.md -->

## Métodos algébricos de ponto flutuante

`f32` e `f64` fornecem `algebraicAdd`, `algebraicSub`, `algebraicMul`, `algebraicDiv` e `algebraicRem`. Eles correspondem, respectivamente, à adição, subtração, multiplicação, divisão e resto, permitindo explicitamente que o compilador reassocie o cálculo de ponto flutuante selecionado para otimização. Os resultados podem variar entre builds e targets, inclusive para `NaN`, infinito e zero com sinal. Os operadores aritméticos normais permanecem estritos.

<!-- @include: ../../.fragments/language/types/algebraic-float-methods.md -->

## Valores especiais de ponto flutuante

`Infinity` e `NaN` são literais reservados de ponto flutuante disponíveis sem importação. `-Infinity` é o `-` unário aplicado a `Infinity`. Eles inferem `f64` e podem receber contexto como `f32`, `f64` ou `f128`. A aritmética de ponto flutuante segue IEEE 754, inclusive para resultados não finitos; divisão inteira por zero continua sendo erro de runtime.

<!-- @include: ../../.fragments/language/types/special-floating-point-values.md -->

`isNaN(value)` aceita um valor de ponto flutuante e retorna `boolean`. Igualdade não identifica `NaN`: toda comparação de igualdade com ele é falsa, enquanto toda comparação de desigualdade é verdadeira.

::: warning Largura numérica faz parte do tipo
Passar um `i64` onde um `i32` é esperado exige conversão explícita de narrowing. O compilador rejeita narrowing implícito porque pode perder dados.
:::

::: danger error
<!-- @include: ../../.fragments/language/types/narrowing-example.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/types/narrowing-error.md -->
:::

## Constantes numéricas

Tipos numéricos expõem constantes `MIN` e `MAX` por sintaxe de propriedade no nome do tipo.

<!-- @include: ../../.fragments/language/types/numeric-constants.md -->

## Sintaxe de literais numéricos

O Tyrus aceita literais inteiros hexadecimais, binários, octais e decimais, com underscores opcionais como separadores visuais. Prefixos e dígitos hexadecimais não diferenciam maiúsculas de minúsculas. Os separadores de inteiros decimais devem agrupar os dígitos em conjuntos de três. Literais inteiros positivos acima de `i128.MAX` exigem um contexto `u128` explícito e não podem exceder `u128.MAX`; o limite negativo com sinal é `i128.MIN`. A notação científica é aceita em literais decimais de ponto flutuante, e partes inteiras grandes não são ampliadas implicitamente para `f128`.

<!-- @include: ../../.fragments/language/types/numeric-literals.md -->

## Parsing decimal

`parseDecimal(text)` interpreta uma string decimal completa de base dez e retorna `decimal?`. Texto inválido, sufixo `d` ou `D`, valores fora da faixa decimal e separadores malformados produzem ausência em vez de uma aproximação de ponto flutuante binário. Casts de inteiro para decimal são exatos; um cast de decimal para inteiro exige valor integral e faixa suficiente no destino.

<!-- @include: ../../.fragments/language/types/decimal-parsing.md -->

`MIN` de inteiro unsigned é sempre zero. `isize` e `usize` têm a largura de um ponteiro na máquina para a qual o programa é compilado, então seus limites não são fixados pela linguagem: eles não expõem `MIN` nem `MAX`, e um tipo de largura fixa deve ser usado quando o limite emsi importa.


::: danger Um literal inteiro precisa caber no seu tipo
`const byte: u8 = 300;` é erro de compilação (`E0706`) ancorado no literal, nunca um valor que dá a volta silenciosamente. O editor oferece um quick fix que alarga o tipo para o menor que comporta o valor.
:::
Literais numéricos podem carregar um sufixo explícito de tipo inteiro ou de ponto flutuante. O sufixo fixa o tipo do literal, participa da inferência e é diferente de uma conversão explícita com `as`.

<!-- @include: ../../.fragments/language/types/typed-numeric-literals.md -->
## Caracteres

Um `char` guarda exatamente um caractere Unicode e nunca é inferido: ele chega por anotação, e o valor é escrito como um literal de um caractere. Texto de qualquer outro tamanho é `string`.

<!-- @include: ../../.fragments/language/types/char.md -->

As aspas não têm significado, quem faz o valor ser um `char` é a anotação, mas o `tyrus fmt` escreve um `char` com aspas simples e uma `string` com o estilo de aspas configurado, então os dois se distinguem à primeira vista.

"Um caractere Unicode" significa um único valor escalar Unicode, um único ponto de código, não um grafema, que é o que o leitor percebe como um símbolo. Alguns emoji são um único valor escalar e cabem em um `char`; muitos são escritos com vários pontos de código e precisam ser `string`:

| Emoji | Pontos de código | Quantidade | `char` |
|-------|------------------|------------|--------|
| 😀 | `U+1F600` | 1 | ✅ |
| 🇧🇷 | dois indicadores regionais (`U+1F1E7` `U+1F1F7`) | 2 | ❌ |
| ❤️ | coração mais seletor de variação (`U+2764` `U+FE0F`) | 2 | ❌ |
| 👍🏽 | polegar mais modificador de tom de pele | 2 | ❌ |
| 👨‍👩‍👧‍👦 | família, uma sequência ZWJ | 7 | ❌ |

Assim `const emoji: char = '😀';` é aceito, enquanto um emoji composto como `'🇧🇷'` falha em tempo de compilação, `a char holds exactly one character, but 🇧🇷 has 2`, e o diagnóstico aponta para `string`.

O `char` oferece `toUpperCase`, `toLowerCase`, `isDigit`, `isAlphabetic`, `isAlphanumeric`, `isWhitespace` e `toString`. Caixa e classificação seguem as regras do Unicode, não as do ASCII, então `'ç'.toUpperCase()` é `'Ç'`. Converter um inteiro em `char` é checado: um valor que não é um caractere Unicode aborta, e `as char?` devolve ausência.

## Valores unknown

`unknown` guarda um valor cujo tipo só é conhecido em tempo de execução. Ele aceita uma `string`, um `boolean`, um `char`, qualquer tipo numérico ou um `decimal`, inclusive uma union deles, e nada pode ser feito com o valor até que uma verificação `typeof` comprove o que ele guarda. No ramo em que `typeof value === "string"` é verdadeiro, `value` é uma `string`. A comparação pode ser escrita em qualquer ordem, e o ramo `else` de `typeof value !== "string"` é estreitado da mesma forma.

<!-- @include: ../../.fragments/language/types/unknown-narrowing.md -->

Fora do ramo comprovado o valor continua `unknown`. Uma verificação de `"object"` ou `"function"`, ou o ramo em que a verificação falhou, não comprova nenhum tipo único e mantém o valor como `unknown`. Comparar `typeof` com um nome que ele nunca produz, como `"number"` ou `"String"`, é um erro de compilação (`E0795`) ancorado no literal, e o editor oferece um quick fix quando exatamente uma categoria é a pretendida.

Arrays, objetos, interfaces, opcionais e funções não podem ser guardados em `unknown` (`E0794`). Dê ao valor um tipo que o descreva.

::: danger error
<!-- @include: ../../.fragments/language/types/unknown-composite-error.md -->
:::

## Tipos opcionais

`T?` representa um valor que pode estar ausente. tyrus não tem `undefined` nem valor `null`: ausência não é um valor, e sim o estado de um `T?` que não tem valor.

<!-- @include: ../../.fragments/language/types/optional-basic.md -->

Como ausência não é um valor, você a produz de quatro formas: omitir um campo ou parâmetro opcional, escrever um `return;` vazio numa função cujo retorno é `T?`, declarar um binding opcional sem inicializador (`let x: T?;` começa vazio) ou chamar um método nativo que não encontra nada (`array.find`). Não há literal `null` para escrever.

<!-- @include: ../../.fragments/language/types/optional-absence.md -->

Leia membros de um valor opcional com `?.`. O acesso faz short-circuit quando o receiver está ausente, e o resultado continua opcional: `name?.length` é `usize?`, não `usize`.

<!-- @include: ../../.fragments/language/types/optional-chaining.md -->

::: danger Trate valores opcionais antes do uso
Unwrap forçado com `value!` é rejeitado pelo compilador (`E0716`) porque pode transformar ausência em falha de runtime. Faça narrowing do valor com `if (value)` ou forneça um fallback com `??`.
:::

Tipos opcionais se compõem com coleções e retornos de função:

<!-- @include: ../../.fragments/language/types/optional-composition.md -->

## Valores Result

`Result<T, E>` representa um payload de sucesso exatamente `T` ou um payload de erro exatamente `E`. Todo valor retornado por uma função `Result` precisa ser envolvido por `Ok(value)` ou `Err(value)`, portanto a variante escolhida fica explícita mesmo quando `T` e `E` diferem. O postfix `?` extrai o payload de sucesso e retorna o erro pela função atual quando necessário.

<!-- @include: ../../.fragments/language/types/result-values.md -->

Os tipos dos payloads devem ser exatos, e payloads opcionais como `Result<User?, string>` são rejeitados. `Result<void, E>` retorna sucesso com `Ok()`. Um retorno vazio é rejeitado com Quick Fix para usar `Ok()`; um `return value;` sem construtor é rejeitado com Quick Fix que envolve o valor no único construtor compatível, ou oferece ambos os construtores quando os dois tipos de payload coincidem. Um result é estreitado com padrões explícitos `Ok(binding)` e `Err(binding)` no `match`, e ambas as variantes são exigidas salvo quando um braço `_` explícito cobre o caso restante. `Ok()` está disponível quando o payload de sucesso é `void`. O operador `?` é a alternativa explícita quando o erro deve ser propagado ao chamador.

`?` é um operador pós-fixo, então o valor que ele desembrulha pode participar de qualquer expressão: `load()? + 1`, `load()? === 40`, ou um ramo de ternário como `wanted ? load()? : 0`. Ele compartilha o token com o ternário, e os dois se distinguem pelo que vem depois: um `?` cuja expressão seguinte é seguida de `:` abre um ternário, então `flag ? -1 : 2` continua sendo ternário enquanto `load()? - 1` propaga.

<!-- @include: ../../.fragments/language/types/try-in-expressions.md -->

Numa função `async`, o `await` abrange o operando até o primeiro `?`, então `await loadUser(id)?` espera a promise e depois propaga o `Result`: `const user = await loadUser(id)?;` ou `return Ok(await load()? + 1);`. Parênteses mantêm o significado, então `await (load()?)` aplica o `?` à própria promise, o que é recusado com um `help` que aponta para `await load()?`. Um `await` também pode ser o sujeito de um `match` ou de um `if`, ou o argumento de uma chamada, inclusive dentro de um laço.

<!-- @include: ../../.fragments/language/types/result-in-async.md -->

## Tratamento inline de falhas

`onfail` é uma guarda terminal anexada ao inicializador de um `const` ou `let` simples. Ela trata `Result<T, E>`, uma `Promise<Result<T, E>>` aguardada, uma chamada direta com um único efeito `throws E` declarado ou uma rejeição aguardada. O nome declarado é o erro dentro do handler e o valor de sucesso depois dele. Um handler nunca continua com um valor substituto: ele sai com `return`, `throw`, `continue` ou `break`, e chegar ao seu fim sai por ele, retornando de uma função `void`, retornando `Err(name)` de uma função que retorna `Result<U, E>` com o mesmo tipo de erro ou propagando a exceção permitida. Numa função que retorna qualquer outro tipo, o handler precisa sair explicitamente (`E0277`).

<!-- @include: ../../.fragments/language/types/inline-failure-handling.md -->

Sobre um `Result` aguardado, o `Err` é separado depois do `await`, então a mesma guarda trata as falhas dos módulos nativos. O handler em si executa sem suspender, e um `await` dentro dele é recusado (`E3002`).

<!-- @include: ../../.fragments/language/types/inline-failure-handling-async.md -->

Use `try`/`catch` para várias operações protegidas, recuperação com valor substituto ou um bloco `finally`. Um `finally` externo ainda executa exatamente uma vez quando um handler de `onfail` termina.

## Tipos de objeto

<!-- @include: ../../.fragments/language/types/object-types.md -->

Literais de objeto são checados estruturalmente contra declarações de interface. Campos opcionais são acessados como valores opcionais.

## Arrays e tuplas

<!-- @include: ../../.fragments/language/types/arrays-tuples.md -->

Arrays guardam um tipo de elemento repetido. Tuplas guardam uma forma fixa com um tipo conhecido por posição.

Use `[valor; tamanho]` para criar um array onde todos os elementos são iguais ao valor informado. O tamanho deve ser um inteiro constante conhecido em tempo de compilação. Tipos Copy (inteiros, booleanos) e não-Copy (strings, arrays aninhados) são suportados; elementos não-Copy são clonados profundamente para cada posição.

Use `Array.withCapacity(n)` para criar um array vazio com buffer pré-alocado. O tipo do elemento é inferido pela anotação na variável. Isso evita realocações repetidas quando você sabe aproximadamente quantos elementos vai inserir.

## Construtores de tipos genéricos

Use brackets angulares para fornecer argumentos de tipo a construtores de tipo nomeados. `Array<T>` equivale a `T[]`, e `Tuple<T, ...>` equivale a `[T, ...]`; escolha a forma que deixa o tipo ao redor mais fácil de ler.

| Forma | Argumentos | Significado |
| --- | --- | --- |
| `Array<T>` | um | Uma sequência mutável ou imutável de `T`; equivale a `T[]`. |
| <code style="white-space: nowrap">Tuple&lt;T, ...&gt;</code> | <span style="white-space: nowrap">zero ou mais</span> | Uma sequência de tamanho fixo com um tipo por posição; equivale a `[T, ...]`. |
| `Optional<T>` | um | Um valor que pode estar ausente; equivale a `T?`. |
| `Range<T>` | um | Um range numérico produzido por `a..b` ou `a..=b`. |
| `Set<T>` | um | Uma coleção de valores únicos de `T`. |
| `Map<K, V>` | dois | Uma coleção indexada de `K` para `V`. |
| `Record<K, V>` | dois | Um objeto com chaves `K` e valores `V`; `Record<string, V>` é um map dinâmico. |
| `Promise<T>` | um | O resultado de trabalho assíncrono que resolve para `T`. |

Os utilitários de tipo de objetos também são genéricos: `Partial<T>`, `Required<T>`, `Readonly<T>`, `DeepPartial<T>`, `Pick<T, K>`, `Omit<T, K>`, `PickByType<T, Type>`, `OmitByType<T, Type>`, `CamelCaseKeys<T>`, `SnakeCaseKeys<T>`, `Require<T, K>`, `AtLeastOne<T, K>`, `DataOnly<T>`, `Exclude<T, U>`, `Extract<T, U>`, `ReturnType<T>` e `Parameters<T>`. Consulte [Coleções](./collections), [Objetos e interfaces](./objects) e [Async e Await](./async-await) para operações e restrições.

## Tipos de função

<!-- @include: ../../.fragments/language/types/function-types.md -->

Assinaturas de função incluem tipos de parâmetros e retorno. O compilador usa essas assinaturas para checagem de chamadas, completion, hover e signature help.

Tipos de função se compõem com outros construtos de tipo:

<!-- @include: ../../.fragments/language/types/function-composition.md -->

Funções podem ser armazenadas em arrays e maps, habilitando registries de callbacks e sistemas de eventos:

<!-- @include: ../../.fragments/language/types/function-registries.md -->

Parênteses agrupam um tipo, então um array de tipo função pode ser escrito na forma inline em vez de só através de um alias:

<!-- @include: ../../.fragments/language/types/parenthesized-types.md -->

O que separa um grupo de uma lista de parâmetros é a seta, e ela só é legível depois do parêntese de fechamento: `(n: i32) => i32` é um tipo função, `(i32)[]` é um array de `i32`. Um grupo contém um tipo só, sem nome e sem `...`; qualquer outra coisa é lista de parâmetros e continua exigindo a seta.


Um tipo de função pode declarar o que a função pode lançar: `(string) => i32 throws string`. Uma chamada através de um valor desse tipo precisa capturar ou propagar esses erros, como uma chamada direta, e uma função que lança não pode ser usada onde o tipo de função não os declara (`E3298`). Uma função `async` reporta seus erros como rejeição, então cabe num tipo de função sem `throws`.

<!-- @include: ../../.fragments/language/types/function-throws.md -->
