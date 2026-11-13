---
title: Algorithm
description: "Native algorithm primitives: disjoint sets, priority queues, binomial heaps, frequency counters, double-ended queues, Fenwick trees, segment trees, sparse tables, bit sets, Bloom filters, prefix trees, suffix arrays, suffix trees, and linked lists."
---

# Algorithm

Import algorithm data structures, direct functions, and study results from `tyrus:algorithm`. The module provides native, high-performance implementations of classic data structures together with deterministic algorithm metrics. All handle types are generic except `FenwickTree`, `SegmentTree`, `SparseTable`, `SegmentTreeLazy`, `BitSet`, `BloomFilter`, and `Trie`. Native handles are released automatically when their binding goes out of scope.

| Algorithm | Description | Page |
|-----------|-------------|------|
| `UnionFind<T>` | Disjoint set (union-find) with path compression and union by rank | [UnionFind](./union-find) |
| `BinaryHeap<T>` | Binary max-heap priority queue | [BinaryHeap](./binary-heap) |
| `BinomialHeap<T>` | Binomial max-heap priority queue with O(log n) meld | [BinomialHeap](./binomial-heap) |
| `Counter<T>` | Frequency counter with insertion-order preservation | [Counter](./counter) |
| `Deque<T>` | Double-ended queue | [Deque](./deque) |
| `FenwickTree` | Fenwick tree (binary indexed tree) for prefix sums | [FenwickTree](./fenwick-tree) |
| `SegmentTree` | Segment tree for range queries and point updates | [SegmentTree](./segment-tree) |
| `SparseTable` | Immutable RMQ table for range minimum queries | [SparseTable](./sparse-table) |
| `SegmentTreeLazy` | Segment tree for range queries and range additions | [SegmentTreeLazy](./segment-tree-lazy) |
| `BitSet` | Compact bit set with set-theoretic operations | [BitSet](./bit-set) |
| `BloomFilter` | Probabilistic set membership with a configurable false-positive rate | [BloomFilter](./bloom-filter) |
| `Graph<N, W>` | Directed weighted graph with automatic node creation and DOT rendering | [Graph](./graph) |
| `bfs` / `bfsInfo` | Breadth-first traversal or its study result | [BFS](./bfs) |
| `dfs` / `dfsInfo` | Iterative depth-first traversal or its study result | [DFS](./dfs) |
| `iddfs` / `iddfsInfo` | Iterative-deepening traversal or its study result | [IDDFS](./iddfs) |
| `dijkstra` / `dijkstraInfo` | Non-negative shortest path or its study result | [Dijkstra](./dijkstra) |
| `aStar` / `aStarInfo` | Heuristic shortest path or its study result | [A*](./a-star) |
| `bellmanFord` / `bellmanFordInfo` | Negative-weight shortest paths or their study result | [Bellman-Ford](./bellman-ford) |
| `floydWarshall` / `floydWarshallInfo` | Dense all-pairs shortest paths or their study result | [Floyd-Warshall](./floyd-warshall) |
| `spfa` / `spfaInfo` | Queue-based shortest paths or their study result | [SPFA](./spfa) |
| `johnson` / `johnsonInfo` | Sparse all-pairs shortest paths or their study result | [Johnson](./johnson) |
| `kruskal` | Minimum spanning forest with disjoint sets | [kruskal](./kruskal) |
| `prim` | Minimum spanning tree with a priority queue | [prim](./prim) |
| `boruvka` | Minimum spanning forest for dense graphs | [boruvka](./boruvka) |
| `steinerTree` | Weighted terminal connection tree | [steinerTree](./steiner-tree) |
| `edmondsKarp` | Maximum flow with breadth-first augmenting paths | [edmondsKarp](./edmonds-karp) |
| `dinic` | Maximum flow with level graphs | [dinic](./dinic) |
| `pushRelabel` | Maximum flow with vertex heights | [pushRelabel](./push-relabel) |
| `minCut` | Minimum cut from maximum flow | [minCut](./min-cut) |
| `minCostMaxFlow` | Minimum-cost maximum flow | [minCostMaxFlow](./min-cost-max-flow) |
| `topologicalSortKahn` | Topological ordering with indegrees | [topologicalSortKahn](./topological-sort-kahn) |
| `topologicalSortDfs` | Topological ordering with DFS | [topologicalSortDfs](./topological-sort-dfs) |
| `tarjanScc` | Strongly connected components with Tarjan | [tarjanScc](./tarjan-scc) |
| `kosarajuScc` | Strongly connected components with Kosaraju | [kosarajuScc](./kosaraju-scc) |
| `fleuryEulerian` | Eulerian path with Fleury | [fleuryEulerian](./fleury-eulerian) |
| `hierholzerEulerian` | Eulerian path with Hierholzer | [hierholzerEulerian](./hierholzer-eulerian) |
| `findBridges` | Bridge detection | [findBridges](./find-bridges) |
| `findCutVertices` | Articulation point detection | [findCutVertices](./find-cut-vertices) |
| `biconnectedComponents` | Biconnected component decomposition | [biconnectedComponents](./biconnected-components) |
| `edmondsBlossom` | Maximum general-graph matching | [edmondsBlossom](./edmonds-blossom) |
| `hungarian` | Minimum-cost assignment | [hungarian](./hungarian) |
| `mergeSort` / `mergeSortInfo` | Stable merge sort and its study result | [mergeSort](./merge-sort) |
| `quickSort` / `quickSortInfo` | Average-case quick sort or its study result | [quickSort](./quick-sort) |
| `heapSort` / `heapSortInfo` | Heap sort or its study result | [heapSort](./heap-sort) |
| `countingSort` / `countingSortInfo` | Counting sort or its study result | [countingSort](./counting-sort) |
| `radixSort` / `radixSortInfo` | Radix sort or its study result | [radixSort](./radix-sort) |
| `bucketSort` / `bucketSortInfo` | Bucket sort or its study result | [bucketSort](./bucket-sort) |
| `timSort` / `timSortInfo` | Hybrid comparison sort or its study result | [timSort](./tim-sort) |
| `insertionSort` / `insertionSortInfo` | Insertion sort or its study result | [insertionSort](./insertion-sort) |
| `shellSort` / `shellSortInfo` | Shell sort or its study result | [shellSort](./shell-sort) |
| `selectionSort` / `selectionSortInfo` | Selection sort or its study result | [selectionSort](./selection-sort) |
| `bubbleSort` / `bubbleSortInfo` | Immediate bubble sort or study result with semantic metrics | [Bubble sort](./bubble-sort) |
| `shakerSort` / `shakerSortInfo` | Bidirectional bubble sort or its study result | [shakerSort](./shaker-sort) |
| `quickselect` | Select the k-th smallest value without fully sorting the array | [quickselect](./quickselect) |
| `binarySearch` | Binary search over an ascending integer array | [binarySearch](./binary-search) |
| `interpolationSearch` | Distribution-aware search over a uniformly distributed integer array | [interpolationSearch](./interpolation-search) |
| `exponentialSearch` | Search an ascending integer array by exponentially expanding its range | [exponentialSearch](./exponential-search) |
| `kmpSearch` | Exact substring search with prefix preprocessing | [kmpSearch](./kmp-search) |
| `zAlgorithm` | Prefix-match preprocessing for a string | [zAlgorithm](./z-algorithm) |
| `rabinKarp` | Substring search with a rolling hash | [rabinKarp](./rabin-karp) |
| `boyerMoore` | Substring search with the bad-character rule | [boyerMoore](./boyer-moore) |
| `ahoCorasick` | Multiple-pattern search returning pattern matches | [ahoCorasick](./aho-corasick) |
| `levenshtein` | Edit distance between two strings | [levenshtein](./levenshtein) |
| `Trie` | Prefix tree for string words | [Trie](./trie) |
| `suffixArray` | Lexicographic suffix array construction for strings | [suffixArray](./suffix-array) |
| `SuffixTree` | Suffix tree built with Ukkonen's algorithm, with lexicographic suffix array extraction | [suffixTreeUkkonen](./suffix-tree) |
| `Lca` | Lowest common ancestor tables over a tree with O(log n) queries | [lcaBinaryLifting](./lca) |
| `Hld` | Heavy-light decomposition with O(log n) chain path queries | [heavyLightDecomposition](./hld) |
| `Centroid` | Centroid decomposition with O(log n) centroid-tree depth | [centroidDecomposition](./centroid) |
| `LinkedList<T>` | Singly linked list | [LinkedList](./linked-list) |
| `DoubleLinkedList<T>` | Doubly linked list | [DoubleLinkedList](./double-linked-list) |
| `AvlTree<T>` | Self-balancing AVL tree (ordered set) | [AvlTree](./avl-tree) |
| `RedBlackTree<T>` | Self-balancing red-black tree (ordered set) | [RedBlackTree](./red-black-tree) |
| `SplayTree<T>` | Self-adjusting splay tree (ordered set) | [SplayTree](./splay-tree) |
| `Treap<T>` | Randomized treap (ordered set) | [Treap](./treap) |
| `SortedSet<T>` | Ordered set backed by a balanced binary search tree | [SortedSet](./sorted-set) |
| `SkipList<T>` | Randomized skip list (ordered set) | [SkipList](./skip-list) |
| `SortedMap<K, V>` | Ordered key → value map backed by a balanced binary search tree | [SortedMap](./sorted-map) |
