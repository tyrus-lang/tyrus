---
title: Keywords
description: Palavras-chave ativas, palavras reservadas e built-ins ambientes no Tyrus.
---

# Keywords

Palavras-chave são tokens reservados na gramática do Tyrus que possuem significados sintáticos específicos. Elas não podem ser usadas como identificadores de variáveis, constantes, funções, interfaces, tipos, parâmetros ou outros bindings.

## Active Keywords

Palavras-chave ativas são reconhecidas e analisadas pelo compilador e lexer do Tyrus. Elas estão organizadas em cinco categorias funcionais principais.

### Declarations and Modifiers

Estas palavras-chave introduzem declarações, visibilidade de escopo, imutabilidade e avaliações em tempo de compilação.

| Keyword | Description |
|---|---|
| `async` | Declara funções assíncronas que retornam promises |
| `comptime` | Avalia expressões ou blocos em tempo de compilação |
| `const` | Declara bindings profundamente imutáveis |
| `enum` | Define um tipo enumerado com variantes fixas |
| `export` | Publica um símbolo para uso em arquivos de todo o projeto |
| `from` | Especifica o módulo de origem em instruções import |
| `function` | Declara uma função nomeada ou anônima |
| `import` | Carrega declarações de outros arquivos ou módulos nativos |
| `interface` | Define uma forma de objeto estrutural ou contrato |
| `let` | Declara um binding mutável |
| `protect` | Restringe o alcance de export à pasta declarante e subpastas |
| `readonly` | Impõe imutabilidade profunda em propriedades e tipos |
| `type` | Declara um alias de tipo |
| `validator` | Declara lógica de validação customizada para decorators |

### Control Flow and Error Handling

Estas palavras-chave controlam ramificações, loops, retornos, limpezas diferidas e tratamento de exceções.

| Keyword | Description |
|---|---|
| `break` | Encerra a execução do loop ou switch mais próximo |
| `case` | Define um caso de correspondência em um switch |
| `catch` | Captura erros lançados dentro de um bloco try |
| `continue` | Avança a execução para a próxima iteração do loop |
| `default` | Ramo padrão de fallback em uma instrução switch |
| `defer` | Agenda limpeza incondicional ao sair do escopo atual |
| `do` | Inicia o bloco de um loop do-while |
| `else` | Ramo alternativo em estruturas condicionais |
| `errdefer` | Agenda limpeza executada apenas se o escopo sair com erro |
| `finally` | Executa limpeza independentemente de sucesso ou exceção |
| `for` | Itera sobre coleções, intervalos ou sequências numéricas |
| `if` | Ramo de execução condicional |
| `loop` | Construto de loop infinito incondicional |
| `match` | Expressão de pattern matching com desestruturação |
| `onfail` | Anexa um manipulador de recuperação em falha de bloco |
| `return` | Retorna um valor e encerra a função atual |
| `switch` | Despacha controle com base em valores literais ou enums |
| `throw` | Lança um erro ou exceção |
| `throws` | Declara contratos de erros recuperáveis em funções |
| `try` | Executa código com captura de exceções |
| `while` | Executa repetição enquanto a condição for verdadeira |

### Expressions and Operators

Estas palavras-chave atuam como operadores, conversões de tipo ou cláusulas em expressões.

| Keyword | Description |
|---|---|
| `and` | Operador de conjunção lógica |
| `as` | Cláusula de asserção de tipo ou alias em import |
| `at` | Cláusula de indexação posicional ou alvo de decorator |
| `await` | Pausa a execução aguardando conclusão assíncrona |
| `extends` | Cláusula de restrição de tipo genérico |
| `in` | Teste de pertinência em loops e expressões |
| `is` | Operador de checagem de tipos |
| `not` | Operador de negação lógica |
| `of` | Cláusula de iteração de valores em loops for-of |
| `or` | Operador de disjunção lógica |
| `typeof` | Inspeciona metadados de tipo em compilação ou runtime |

### Primitive Types

Estas palavras-chave representam os tipos escalares e numéricos primitivos embutidos na linguagem.

| Keyword | Description |
|---|---|
| `boolean` | Tipo lógico verdadeiro ou falso |
| `char` | Caractere escalar Unicode de 32 bits |
| `decimal` | Tipo aritmético decimal de precisão fixa exata |
| `f128` | Ponto flutuante IEEE 754 de 128 bits |
| `f32` | Ponto flutuante de precisão simples de 32 bits |
| `f64` | Ponto flutuante de precisão dupla de 64 bits |
| `i128` | Inteiro com sinal de 128 bits |
| `i16` | Inteiro com sinal de 16 bits |
| `i32` | Inteiro com sinal de 32 bits |
| `i64` | Inteiro com sinal de 64 bits |
| `i8` | Inteiro com sinal de 8 bits |
| `isize` | Inteiro com sinal do tamanho de ponteiro da arquitetura |
| `never` | Tipo inferior representando valores inalcançáveis |
| `string` | Tipo string imutável codificado em UTF-8 |
| `u128` | Inteiro sem sinal de 128 bits |
| `u16` | Inteiro sem sinal de 16 bits |
| `u32` | Inteiro sem sinal de 32 bits |
| `u64` | Inteiro sem sinal de 64 bits |
| `u8` | Inteiro sem sinal de 8 bits |
| `unknown` | Tipo topo seguro exigindo narrowing antes do uso |
| `usize` | Inteiro sem sinal do tamanho de ponteiro da arquitetura |
| `void` | Tipo de retorno para funções sem valor |

### Literal Values

Estas palavras-chave denotam literais primitivos e constantes de ausência.

| Keyword | Description |
|---|---|
| `false` | Literal booleano falso |
| `Infinity` | Literal de infinito positivo em ponto flutuante |
| `NaN` | Literal de valor numérico inválido (Not-a-Number) |
| `None` | Sentinela de ausência opcional em pattern matching |
| `null` | Literal de ausência reservado (proibido como valor em runtime) |
| `true` | Literal booleano verdadeiro |

## Reserved for Future Use

Estas palavras-chave são reservadas para evolução futura da linguagem e não podem ser usadas como identificadores.

| Category | Keywords | Description |
|---|---|---|
| Classes e POO | `class`, `constructor`, `implements`, `instanceof`, `new`, `override`, `super`, `this` | Reservadas para possíveis construtos orientados a objetos e classes |
| Visibilidade e Modificadores | `abstract`, `private`, `protected`, `public`, `static` | Reservadas para visibilidade de membros e qualificadores de escopo |
| Módulos e Escopos | `declare`, `module`, `namespace`, `package`, `require`, `var` | Reservadas para declarações ambientes e extensões de módulos |
| Sistema de Tipos | `asserts`, `infer`, `intrinsic`, `keyof`, `satisfies`, `unique` | Reservadas para operadores avançados de tipos e narrowing |
| Acessores e Parâmetros | `get`, `out`, `set` | Reservadas para acessores de propriedades e modos de passagem |
| Execução e Controle | `debugger`, `delete`, `when`, `with`, `yield` | Reservadas para depuração, geradores e condições de padrões |

## Ambient Built-ins

Estes identificadores embutidos são disponibilizados no escopo global sem necessidade de import explícito.

| Identifier | Description |
|---|---|
| `assert` | Helper de asserção em runtime verificando condição booleana |
| `assertEqual` | Helper de verificação de igualdade em runtime entre dois valores |
| `assertThrows` | Helper verificando se uma função lança o erro esperado |
| `console` | Namespace padrão para logging no console |
| `isNaN` | Verifica se um valor numérico é NaN |
| `Map` | Coleção de mapa associativo chave-valor embutida |
| `panic` | Aborta a execução imediatamente com mensagem de erro |
| `print` | Função de saída padrão no stdout |
| `printCR` | Helper de carriage return para progresso no terminal |
| `printError` | Função de escrita no stream padrão de erro stderr |
| `printRaw` | Função de escrita bruta sem formatação adicional |
| `printWarn` | Função de aviso no stream padrão de erro stderr |
| `Set` | Coleção de conjunto de valores únicos embutida |

