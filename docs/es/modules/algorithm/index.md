---
title: Algorithm
description: "Primitivas de algoritmos nativos: conjuntos disjuntos, colas de prioridad, heaps binomiales, contadores de frecuencia, colas de doble extremo, árboles de Fenwick, árboles de segmentos, tablas dispersas, conjuntos de bits, filtros de Bloom, árboles de prefijo, suffix arrays, suffix trees y listas enlazadas."
---

# Algorithm

Importe estructuras de datos, funciones directas y resultados de estudio de `tyrus:algorithm`. El módulo proporciona implementaciones nativas y de alto rendimiento de estructuras de datos clásicas, además de métricas deterministas de algoritmos. Todos los tipos de handle son genéricos excepto `FenwickTree`, `SegmentTree`, `SparseTable`, `SegmentTreeLazy`, `BitSet`, `BloomFilter` y `Trie`. Los handles nativos se liberan automáticamente cuando la variable sale del ámbito.

| Algoritmo | Descripción | Página |
|-----------|-------------|--------|
| `UnionFind<T>` | Conjunto disjunto (union-find) con compresión de camino y unión por rango | [UnionFind](./union-find) |
| `BinaryHeap<T>` | Cola de prioridad max-heap binaria | [BinaryHeap](./binary-heap) |
| `BinomialHeap<T>` | Cola de prioridad max-heap binomial con meld O(log n) | [BinomialHeap](./binomial-heap) |
| `Counter<T>` | Contador de frecuencia con preservación de orden de inserción | [Counter](./counter) |
| `Deque<T>` | Cola de doble extremo | [Deque](./deque) |
| `FenwickTree` | Árbol de Fenwick (binary indexed tree) para sumas de prefijo | [FenwickTree](./fenwick-tree) |
| `SegmentTree` | Árbol de segmentos para consultas por intervalo y actualizaciones puntuales | [SegmentTree](./segment-tree) |
| `SparseTable` | Tabla RMQ inmutable para consultas de mínimo por intervalo | [SparseTable](./sparse-table) |
| `SegmentTreeLazy` | Árbol de segmentos para consultas y sumas por intervalo | [SegmentTreeLazy](./segment-tree-lazy) |
| `BitSet` | Conjunto de bits compacto con operaciones de conjunto | [BitSet](./bit-set) |
| `BloomFilter` | Pertenencia probabilística a conjuntos con tasa de falso positivo configurable | [BloomFilter](./bloom-filter) |
| `Graph<N, W>` | Grafo dirigido ponderado con creación automática de nodos y renderizado DOT | [Graph](./graph) |
| `bfs` / `bfsInfo` | Recorrido en anchura o su resultado de estudio | [BFS](./bfs) |
| `dfs` / `dfsInfo` | Recorrido en profundidad iterativa o su resultado de estudio | [DFS](./dfs) |
| `iddfs` / `iddfsInfo` | Recorrido con profundización iterativa o su resultado de estudio | [IDDFS](./iddfs) |
| `dijkstra` / `dijkstraInfo` | Ruta más corta con pesos no negativos o su resultado de estudio | [Dijkstra](./dijkstra) |
| `aStar` / `aStarInfo` | Ruta más corta con heurística o su resultado de estudio | [A*](./a-star) |
| `bellmanFord` / `bellmanFordInfo` | Rutas más cortas con pesos negativos o su resultado de estudio | [Bellman-Ford](./bellman-ford) |
| `floydWarshall` / `floydWarshallInfo` | Rutas más cortas densas entre todos los pares o su resultado de estudio | [Floyd-Warshall](./floyd-warshall) |
| `spfa` / `spfaInfo` | Rutas más cortas basadas en cola o su resultado de estudio | [SPFA](./spfa) |
| `johnson` / `johnsonInfo` | Rutas más cortas dispersas entre todos los pares o su resultado de estudio | [Johnson](./johnson) |
| `kruskal` | Bosque de expansión mínima con conjuntos disjuntos | [kruskal](./kruskal) |
| `prim` | Árbol de expansión mínima con cola de prioridad | [prim](./prim) |
| `boruvka` | Bosque de expansión mínima para grafos densos | [boruvka](./boruvka) |
| `steinerTree` | Árbol de conexión ponderado de terminales | [steinerTree](./steiner-tree) |
| `edmondsKarp` | Flujo máximo con caminos aumentantes en anchura | [edmondsKarp](./edmonds-karp) |
| `dinic` | Flujo máximo con grafos de niveles | [dinic](./dinic) |
| `pushRelabel` | Flujo máximo con alturas de vértices | [pushRelabel](./push-relabel) |
| `minCut` | Corte mínimo obtenido del flujo máximo | [minCut](./min-cut) |
| `minCostMaxFlow` | Flujo máximo de coste mínimo | [minCostMaxFlow](./min-cost-max-flow) |
| `topologicalSortKahn` | Ordenación topológica por grados de entrada | [topologicalSortKahn](./topological-sort-kahn) |
| `topologicalSortDfs` | Ordenación topológica por DFS | [topologicalSortDfs](./topological-sort-dfs) |
| `tarjanScc` | Componentes fuertemente conexos con Tarjan | [tarjanScc](./tarjan-scc) |
| `kosarajuScc` | Componentes fuertemente conexos con Kosaraju | [kosarajuScc](./kosaraju-scc) |
| `fleuryEulerian` | Camino Euleriano con Fleury | [fleuryEulerian](./fleury-eulerian) |
| `hierholzerEulerian` | Camino Euleriano con Hierholzer | [hierholzerEulerian](./hierholzer-eulerian) |
| `findBridges` | Detección de puentes | [findBridges](./find-bridges) |
| `findCutVertices` | Detección de puntos de articulación | [findCutVertices](./find-cut-vertices) |
| `biconnectedComponents` | Descomposición en componentes biconexos | [biconnectedComponents](./biconnected-components) |
| `edmondsBlossom` | Emparejamiento máximo en grafo general | [edmondsBlossom](./edmonds-blossom) |
| `hungarian` | Asignación de coste mínimo | [hungarian](./hungarian) |
| `mergeSort` / `mergeSortInfo` | Ordenación merge sort estable y su resultado de estudio | [mergeSort](./merge-sort) |
| `quickSort` / `quickSortInfo` | Quick sort promedio o su resultado de estudio | [quickSort](./quick-sort) |
| `heapSort` / `heapSortInfo` | Heap sort o su resultado de estudio | [heapSort](./heap-sort) |
| `countingSort` / `countingSortInfo` | Counting sort o su resultado de estudio | [countingSort](./counting-sort) |
| `radixSort` / `radixSortInfo` | Radix sort o su resultado de estudio | [radixSort](./radix-sort) |
| `bucketSort` / `bucketSortInfo` | Bucket sort o su resultado de estudio | [bucketSort](./bucket-sort) |
| `timSort` / `timSortInfo` | Ordenación híbrida o su resultado de estudio | [timSort](./tim-sort) |
| `insertionSort` / `insertionSortInfo` | Insertion sort o su resultado de estudio | [insertionSort](./insertion-sort) |
| `shellSort` / `shellSortInfo` | Shell sort o su resultado de estudio | [shellSort](./shell-sort) |
| `selectionSort` / `selectionSortInfo` | Selection sort o su resultado de estudio | [selectionSort](./selection-sort) |
| `bubbleSort` / `bubbleSortInfo` | Bubble sort inmediato o resultado de estudio con métricas semánticas | [Bubble sort](./bubble-sort) |
| `shakerSort` / `shakerSortInfo` | Bubble sort bidireccional o su resultado de estudio | [shakerSort](./shaker-sort) |
| `quickselect` | Selecciona el k-ésimo menor valor sin ordenar todo el array | [quickselect](./quickselect) |
| `binarySearch` | Búsqueda binaria en un array ascendente de enteros | [binarySearch](./binary-search) |
| `interpolationSearch` | Búsqueda sensible a la distribución en un array de enteros uniformemente distribuido | [interpolationSearch](./interpolation-search) |
| `exponentialSearch` | Búsqueda en un array ascendente expandiendo el intervalo exponencialmente | [exponentialSearch](./exponential-search) |
| `kmpSearch` | Búsqueda exacta de subcadenas con preprocesamiento de prefijos | [kmpSearch](./kmp-search) |
| `zAlgorithm` | Preprocesamiento de coincidencias de prefijos de una cadena | [zAlgorithm](./z-algorithm) |
| `rabinKarp` | Búsqueda de subcadenas con rolling hash | [rabinKarp](./rabin-karp) |
| `boyerMoore` | Búsqueda de subcadenas con la regla del mal carácter | [boyerMoore](./boyer-moore) |
| `ahoCorasick` | Búsqueda de múltiples patrones devolviendo coincidencias | [ahoCorasick](./aho-corasick) |
| `levenshtein` | Distancia de edición entre dos cadenas | [levenshtein](./levenshtein) |
| `Trie` | Árbol de prefijo para palabras de string | [Trie](./trie) |
| `suffixArray` | Construcción de suffix array lexicográfico para strings | [suffixArray](./suffix-array) |
| `SuffixTree` | Suffix tree construido con el algoritmo de Ukkonen, con extracción de suffix array lexicográfico | [suffixTreeUkkonen](./suffix-tree) |
| `Lca` | Tablas de menor ancestro común sobre un árbol con consultas en O(log n) | [lcaBinaryLifting](./lca) |
| `Hld` | Descomposición heavy-light con consultas de ruta en cadenas O(log n) | [heavyLightDecomposition](./hld) |
| `Centroid` | Descomposición centroid con profundidad O(log n) en el árbol centroid | [centroidDecomposition](./centroid) |
| `LinkedList<T>` | Lista simplemente enlazada | [LinkedList](./linked-list) |
| `DoubleLinkedList<T>` | Lista doblemente enlazada | [DoubleLinkedList](./double-linked-list) |
| `AvlTree<T>` | Árbol AVL autoequilibrado (conjunto ordenado) | [AvlTree](./avl-tree) |
| `RedBlackTree<T>` | Árbol rojo-negro autoequilibrado (conjunto ordenado) | [RedBlackTree](./red-black-tree) |
| `SplayTree<T>` | Árbol de splay autoadaptativo (conjunto ordenado) | [SplayTree](./splay-tree) |
| `Treap<T>` | Treap aleatorio (conjunto ordenado) | [Treap](./treap) |
| `SortedSet<T>` | Conjunto ordenado sobre un árbol binario de búsqueda balanceado | [SortedSet](./sorted-set) |
| `SkipList<T>` | Skip list aleatoria (conjunto ordenado) | [SkipList](./skip-list) |
| `SortedMap<K, V>` | Mapa ordenado de clave → valor sobre un árbol binario de búsqueda balanceado | [SortedMap](./sorted-map) |
