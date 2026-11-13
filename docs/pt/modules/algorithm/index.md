---
title: Algorithm
description: "Primitivas de algoritmos nativos: conjuntos disjuntos, filas de prioridade, heaps binomiais, contadores de frequência, filas de duas pontas, árvores de Fenwick, árvores de segmentos, tabelas esparsas, conjuntos de bits, filtros de Bloom, árvores de prefixo, suffix arrays, suffix trees e listas encadeadas."
---

# Algorithm

Importe estruturas de dados, funções diretas e resultados de estudo de `tyrus:algorithm`. O módulo fornece implementações nativas e de alto desempenho de estruturas de dados clássicas, além de métricas determinísticas de algoritmos. Todos os tipos de handle são genéricos exceto `FenwickTree`, `SegmentTree`, `SparseTable`, `SegmentTreeLazy`, `BitSet`, `BloomFilter` e `Trie`. Handles nativos são liberados automaticamente quando a variável sai de escopo.

| Algoritmo | Descrição | Página |
|-----------|-----------|--------|
| `UnionFind<T>` | Conjunto disjunto (union-find) com compressão de caminho e união por rank | [UnionFind](./union-find) |
| `BinaryHeap<T>` | Fila de prioridade max-heap binária | [BinaryHeap](./binary-heap) |
| `BinomialHeap<T>` | Fila de prioridade max-heap binomial com meld O(log n) | [BinomialHeap](./binomial-heap) |
| `Counter<T>` | Contador de frequência com preservação de ordem de inserção | [Counter](./counter) |
| `Deque<T>` | Fila de duas pontas | [Deque](./deque) |
| `FenwickTree` | Árvore de Fenwick (binary indexed tree) para somas de prefixo | [FenwickTree](./fenwick-tree) |
| `SegmentTree` | Árvore de segmentos para consultas por intervalo e atualizações pontuais | [SegmentTree](./segment-tree) |
| `SparseTable` | Tabela RMQ imutável para consultas de mínimo por intervalo | [SparseTable](./sparse-table) |
| `SegmentTreeLazy` | Árvore de segmentos para consultas e adições por intervalo | [SegmentTreeLazy](./segment-tree-lazy) |
| `BitSet` | Conjunto de bits compacto com operações de conjunto | [BitSet](./bit-set) |
| `BloomFilter` | Pertinência probabilística em conjuntos com taxa de falso positivo configurável | [BloomFilter](./bloom-filter) |
| `Graph<N, W>` | Grafo direcionado com pesos, criação automática de nós e renderização DOT | [Graph](./graph) |
| `bfs` / `bfsInfo` | Travessia em largura ou seu resultado de estudo | [BFS](./bfs) |
| `dfs` / `dfsInfo` | Travessia em profundidade iterativa ou seu resultado de estudo | [DFS](./dfs) |
| `iddfs` / `iddfsInfo` | Travessia com aprofundamento iterativo ou seu resultado de estudo | [IDDFS](./iddfs) |
| `dijkstra` / `dijkstraInfo` | Menor caminho com pesos não negativos ou seu resultado de estudo | [Dijkstra](./dijkstra) |
| `aStar` / `aStarInfo` | Menor caminho com heurística ou seu resultado de estudo | [A*](./a-star) |
| `bellmanFord` / `bellmanFordInfo` | Menores caminhos com pesos negativos ou seu resultado de estudo | [Bellman-Ford](./bellman-ford) |
| `floydWarshall` / `floydWarshallInfo` | Menores caminhos entre todos os pares densos ou seu resultado de estudo | [Floyd-Warshall](./floyd-warshall) |
| `spfa` / `spfaInfo` | Menores caminhos baseados em fila ou seu resultado de estudo | [SPFA](./spfa) |
| `johnson` / `johnsonInfo` | Menores caminhos entre todos os pares esparsos ou seu resultado de estudo | [Johnson](./johnson) |
| `kruskal` | Floresta geradora mínima com conjuntos disjuntos | [kruskal](./kruskal) |
| `prim` | Árvore geradora mínima com fila de prioridade | [prim](./prim) |
| `boruvka` | Floresta geradora mínima para grafos densos | [boruvka](./boruvka) |
| `steinerTree` | Árvore de conexão ponderada de terminais | [steinerTree](./steiner-tree) |
| `edmondsKarp` | Fluxo máximo com caminhos aumentantes em largura | [edmondsKarp](./edmonds-karp) |
| `dinic` | Fluxo máximo com grafos de níveis | [dinic](./dinic) |
| `pushRelabel` | Fluxo máximo com alturas de vértices | [pushRelabel](./push-relabel) |
| `minCut` | Corte mínimo obtido do fluxo máximo | [minCut](./min-cut) |
| `minCostMaxFlow` | Fluxo máximo de custo mínimo | [minCostMaxFlow](./min-cost-max-flow) |
| `topologicalSortKahn` | Ordenação topológica por graus de entrada | [topologicalSortKahn](./topological-sort-kahn) |
| `topologicalSortDfs` | Ordenação topológica por DFS | [topologicalSortDfs](./topological-sort-dfs) |
| `tarjanScc` | Componentes fortemente conexos com Tarjan | [tarjanScc](./tarjan-scc) |
| `kosarajuScc` | Componentes fortemente conexos com Kosaraju | [kosarajuScc](./kosaraju-scc) |
| `fleuryEulerian` | Caminho Euleriano com Fleury | [fleuryEulerian](./fleury-eulerian) |
| `hierholzerEulerian` | Caminho Euleriano com Hierholzer | [hierholzerEulerian](./hierholzer-eulerian) |
| `findBridges` | Detecção de pontes | [findBridges](./find-bridges) |
| `findCutVertices` | Detecção de pontos de articulação | [findCutVertices](./find-cut-vertices) |
| `biconnectedComponents` | Decomposição em componentes biconexos | [biconnectedComponents](./biconnected-components) |
| `edmondsBlossom` | Emparelhamento máximo em grafo geral | [edmondsBlossom](./edmonds-blossom) |
| `hungarian` | Atribuição de custo mínimo | [hungarian](./hungarian) |
| `mergeSort` / `mergeSortInfo` | Ordenação merge sort estável e seu resultado de estudo | [mergeSort](./merge-sort) |
| `quickSort` / `quickSortInfo` | Quick sort médio ou seu resultado de estudo | [quickSort](./quick-sort) |
| `heapSort` / `heapSortInfo` | Heap sort ou seu resultado de estudo | [heapSort](./heap-sort) |
| `countingSort` / `countingSortInfo` | Counting sort ou seu resultado de estudo | [countingSort](./counting-sort) |
| `radixSort` / `radixSortInfo` | Radix sort ou seu resultado de estudo | [radixSort](./radix-sort) |
| `bucketSort` / `bucketSortInfo` | Bucket sort ou seu resultado de estudo | [bucketSort](./bucket-sort) |
| `timSort` / `timSortInfo` | Ordenação híbrida ou seu resultado de estudo | [timSort](./tim-sort) |
| `insertionSort` / `insertionSortInfo` | Insertion sort ou seu resultado de estudo | [insertionSort](./insertion-sort) |
| `shellSort` / `shellSortInfo` | Shell sort ou seu resultado de estudo | [shellSort](./shell-sort) |
| `selectionSort` / `selectionSortInfo` | Selection sort ou seu resultado de estudo | [selectionSort](./selection-sort) |
| `bubbleSort` / `bubbleSortInfo` | Bubble sort imediato ou resultado de estudo com métricas semânticas | [Bubble sort](./bubble-sort) |
| `shakerSort` / `shakerSortInfo` | Bubble sort bidirecional ou seu resultado de estudo | [shakerSort](./shaker-sort) |
| `quickselect` | Seleciona o k-ésimo menor valor sem ordenar o array inteiro | [quickselect](./quickselect) |
| `binarySearch` | Busca binária em um array crescente de inteiros | [binarySearch](./binary-search) |
| `interpolationSearch` | Busca sensível à distribuição em um array de inteiros uniformemente distribuído | [interpolationSearch](./interpolation-search) |
| `exponentialSearch` | Busca em um array crescente expandindo o intervalo exponencialmente | [exponentialSearch](./exponential-search) |
| `kmpSearch` | Busca exata de substring com pré-processamento de prefixos | [kmpSearch](./kmp-search) |
| `zAlgorithm` | Pré-processamento de correspondência de prefixos de uma string | [zAlgorithm](./z-algorithm) |
| `rabinKarp` | Busca de substring com rolling hash | [rabinKarp](./rabin-karp) |
| `boyerMoore` | Busca de substring com a regra do mau caractere | [boyerMoore](./boyer-moore) |
| `ahoCorasick` | Busca de múltiplos padrões retornando as correspondências | [ahoCorasick](./aho-corasick) |
| `levenshtein` | Distância de edição entre duas strings | [levenshtein](./levenshtein) |
| `Trie` | Árvore de prefixo para palavras de string | [Trie](./trie) |
| `suffixArray` | Construção de suffix array lexicográfico para strings | [suffixArray](./suffix-array) |
| `SuffixTree` | Suffix tree construída com o algoritmo de Ukkonen, com extração de suffix array lexicográfico | [suffixTreeUkkonen](./suffix-tree) |
| `Lca` | Tabelas de menor ancestral comum sobre uma árvore com consultas em O(log n) | [lcaBinaryLifting](./lca) |
| `Hld` | Decomposição heavy-light com consultas de caminho em cadeias O(log n) | [heavyLightDecomposition](./hld) |
| `Centroid` | Decomposição centroid com profundidade O(log n) na árvore centroid | [centroidDecomposition](./centroid) |
| `LinkedList<T>` | Lista simplesmente encadeada | [LinkedList](./linked-list) |
| `DoubleLinkedList<T>` | Lista duplamente encadeada | [DoubleLinkedList](./double-linked-list) |
| `AvlTree<T>` | Árvore AVL autobalanceada (conjunto ordenado) | [AvlTree](./avl-tree) |
| `RedBlackTree<T>` | Árvore rubro-negra autobalanceada (conjunto ordenado) | [RedBlackTree](./red-black-tree) |
| `SplayTree<T>` | Árvore de splay autoadjustável (conjunto ordenado) | [SplayTree](./splay-tree) |
| `Treap<T>` | Treap aleatório (conjunto ordenado) | [Treap](./treap) |
| `SortedSet<T>` | Conjunto ordenado sobre uma árvore binária de busca balanceada | [SortedSet](./sorted-set) |
| `SkipList<T>` | Skip list aleatória (conjunto ordenado) | [SkipList](./skip-list) |
| `SortedMap<K, V>` | Mapa ordenado de chave → valor sobre uma árvore binária de busca balanceada | [SortedMap](./sorted-map) |
