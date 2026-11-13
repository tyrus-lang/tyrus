---
title: Graph
description: "Estrutura de dados de grafo direcionado com pesos de tyrus:algorithm."
---

# Graph

`Graph<N, W>` representa um grafo direcionado com pesos. Os nós são identificados por valores do tipo `N` e as arestas carregam um peso do tipo `W`. Os nós são criados automaticamente quando uma aresta é adicionada, então construir um grafo nunca exige registro separado.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `Graph<N, W>()` | Cria um novo grafo direcionado vazio com tipo de nó `N` e tipo de peso `W` |

Os tipos de nó e peso suportados são `string`, `bool` e os tipos numéricos `i8`–`i64`, `u8`–`u64`, `f32` e `f64`. Os dois parâmetros de tipo devem ser declarados explicitamente; eles não podem ser inferidos.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `addEdge` | `(source: N, target: N, weight: W) => void` | `void` | Adiciona uma aresta; cria os nós se ausentes, substitui o peso se o par ordenado já tiver aresta |
| `addNode` | `(node: N) => void` | `void` | Adiciona um nó isolado |
| `removeNode` | `(node: N) => boolean` | `boolean` | Remove um nó e todas as suas arestas incidentes; retorna `true` se o nó existia |
| `removeEdge` | `(source: N, target: N) => boolean` | `boolean` | Remove a aresta entre `source` e `target`; os nós permanecem |
| `hasNode` | `(node: N) => boolean` | `boolean` | Se o nó existe |
| `hasEdge` | `(source: N, target: N) => boolean` | `boolean` | Se o par ordenado tem uma aresta |
| `edge` | `(source: N, target: N) => W?` | `W?` | O peso da aresta, ou vazio quando não há aresta |
| `neighbors` | `(node: N) => N[]` | `N[]` | Os vizinhos de saída de um nó, em ordem de inserção |
| `degree` | `(node: N) => number` | `number` | O grau de saída de um nó |
| `nodeCount` | `() => number` | `number` | O número total de nós |
| `edgeCount` | `() => number` | `number` | O número total de arestas |
| `isEmpty` | `() => boolean` | `boolean` | Se o grafo não tem nenhum nó |
| `nodes` | `() => N[]` | `N[]` | Todos os nós, em ordem de inserção |
| `edges` | `() => [N, N, W][]` | `[N, N, W][]` | Todas as arestas como tuplas `(source, target, weight)` |
| `reverse` | `() => Graph<N, W>` | `Graph<N, W>` | Um novo grafo com a direção de todas as arestas invertida |
| `mapEdges` | `((weight: W) => W') => Graph<N, W'>` | `Graph<N, W'>` | Um novo grafo com pesos transformados pelo callback |
| `mapNodes` | `((node: N) => N') => Graph<N', W>` | `Graph<N', W>` | Um novo grafo com valores de nó transformados pelo callback |
| `toDot` | `() => string` | `string` | O grafo renderizado como DOT do Graphviz |
| `toString` | `() => string` | `string` | Uma representação compacta de adjacência em uma linha |

`neighbors`, `nodes` e `edges` retornam arrays recém-alocados pertencentes ao chamador. O handle nativo do grafo é liberado automaticamente quando a variável sai de escopo.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `addEdge` / `addNode` | O(1) amortizado, O(1) esperado |
| `hasNode` / `edge` / `hasEdge` | O(1) esperado |
| `removeNode` | O(V + E) |
| `removeEdge` | O(grau(source)) |
| `neighbors` / `degree` | O(grau(node)) |
| `nodes` / `nodeCount` / `isEmpty` | O(V) / O(1) |
| `edges` / `edgeCount` | O(E) / O(1) |
| `reverse` / `mapEdges` / `mapNodes` | O(V + E) |
| `toDot` / `toString` | O(V + E) |

## Exemplos

### Construindo um grafo

<!-- @include: ../../../.fragments/modules/algorithm/graph/building-a-graph.md -->

### Consultas

<!-- @include: ../../../.fragments/modules/algorithm/graph/queries.md -->

### Iteração

<!-- @include: ../../../.fragments/modules/algorithm/graph/iteration.md -->

### Mutação

<!-- @include: ../../../.fragments/modules/algorithm/graph/mutation.md -->

### Transformações

<!-- @include: ../../../.fragments/modules/algorithm/graph/transformations.md -->

### Renderização

<!-- @include: ../../../.fragments/modules/algorithm/graph/rendering.md -->

### Nós numéricos e pesos float

<!-- @include: ../../../.fragments/modules/algorithm/graph/numeric-nodes-and-float-weights.md -->
