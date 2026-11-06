use crate::ast::*;
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;
use crate::span::Span;

#[path = "builtins/builtins.rs"]
mod builtins;

pub(crate) use builtins::algorithm_builtins;

pub const UNION_FIND_NEW_TARGET: &str = "__tyrus_algorithm_union_find_new";
pub const UNION_FIND_FIND_TARGET: &str = "__tyrus_algorithm_union_find_find";
pub const UNION_FIND_UNION_TARGET: &str = "__tyrus_algorithm_union_find_union";
pub const UNION_FIND_CONNECTED_TARGET: &str = "__tyrus_algorithm_union_find_connected";
pub const UNION_FIND_SET_COUNT_TARGET: &str = "__tyrus_algorithm_union_find_set_count";

pub const BINARY_HEAP_NEW_TARGET: &str = "__tyrus_algorithm_binary_heap_new";
pub const BINARY_HEAP_PUSH_TARGET: &str = "__tyrus_algorithm_binary_heap_push";
pub const BINARY_HEAP_POP_TARGET: &str = "__tyrus_algorithm_binary_heap_pop";
pub const BINARY_HEAP_PEEK_TARGET: &str = "__tyrus_algorithm_binary_heap_peek";
pub const BINARY_HEAP_LEN_TARGET: &str = "__tyrus_algorithm_binary_heap_len";
pub const BINARY_HEAP_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_binary_heap_is_empty";
pub const BINARY_HEAP_CLEAR_TARGET: &str = "__tyrus_algorithm_binary_heap_clear";

pub const BINOMIAL_HEAP_NEW_TARGET: &str = "__tyrus_algorithm_binomial_heap_new";
pub const BINOMIAL_HEAP_PUSH_TARGET: &str = "__tyrus_algorithm_binomial_heap_push";
pub const BINOMIAL_HEAP_POP_TARGET: &str = "__tyrus_algorithm_binomial_heap_pop";
pub const BINOMIAL_HEAP_PEEK_TARGET: &str = "__tyrus_algorithm_binomial_heap_peek";
pub const BINOMIAL_HEAP_LEN_TARGET: &str = "__tyrus_algorithm_binomial_heap_len";
pub const BINOMIAL_HEAP_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_binomial_heap_is_empty";
pub const BINOMIAL_HEAP_CLEAR_TARGET: &str = "__tyrus_algorithm_binomial_heap_clear";

pub const COUNTER_NEW_TARGET: &str = "__tyrus_algorithm_counter_new";
pub const COUNTER_FROM_ARRAY_TARGET: &str = "__tyrus_algorithm_counter_from_array";
pub const COUNTER_ADD_TARGET: &str = "__tyrus_algorithm_counter_add";
pub const COUNTER_REMOVE_TARGET: &str = "__tyrus_algorithm_counter_remove";
pub const COUNTER_COUNT_TARGET: &str = "__tyrus_algorithm_counter_count";
pub const COUNTER_MOST_COMMON_TARGET: &str = "__tyrus_algorithm_counter_most_common";
pub const COUNTER_TOTAL_TARGET: &str = "__tyrus_algorithm_counter_total";
pub const COUNTER_UNIQUE_TARGET: &str = "__tyrus_algorithm_counter_unique";
pub const COUNTER_CLEAR_TARGET: &str = "__tyrus_algorithm_counter_clear";
pub const COUNTER_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_counter_is_empty";

pub const DEQUE_NEW_TARGET: &str = "__tyrus_algorithm_deque_new";
pub const DEQUE_PUSH_FRONT_TARGET: &str = "__tyrus_algorithm_deque_push_front";
pub const DEQUE_PUSH_BACK_TARGET: &str = "__tyrus_algorithm_deque_push_back";
pub const DEQUE_POP_FRONT_TARGET: &str = "__tyrus_algorithm_deque_pop_front";
pub const DEQUE_POP_BACK_TARGET: &str = "__tyrus_algorithm_deque_pop_back";
pub const DEQUE_PEEK_FRONT_TARGET: &str = "__tyrus_algorithm_deque_peek_front";
pub const DEQUE_PEEK_BACK_TARGET: &str = "__tyrus_algorithm_deque_peek_back";
pub const DEQUE_LEN_TARGET: &str = "__tyrus_algorithm_deque_len";
pub const DEQUE_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_deque_is_empty";
pub const DEQUE_CLEAR_TARGET: &str = "__tyrus_algorithm_deque_clear";

pub const FENWICK_TREE_NEW_TARGET: &str = "__tyrus_algorithm_fenwick_tree_new";
pub const FENWICK_TREE_ADD_TARGET: &str = "__tyrus_algorithm_fenwick_tree_add";
pub const FENWICK_TREE_SUM_TARGET: &str = "__tyrus_algorithm_fenwick_tree_sum";
pub const FENWICK_TREE_LEN_TARGET: &str = "__tyrus_algorithm_fenwick_tree_len";
pub const FENWICK_TREE_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_fenwick_tree_is_empty";

pub const SEGMENT_TREE_NEW_TARGET: &str = "__tyrus_algorithm_segment_tree_new";
pub const SEGMENT_TREE_UPDATE_TARGET: &str = "__tyrus_algorithm_segment_tree_update";
pub const SEGMENT_TREE_QUERY_TARGET: &str = "__tyrus_algorithm_segment_tree_query";
pub const SEGMENT_TREE_LEN_TARGET: &str = "__tyrus_algorithm_segment_tree_len";
pub const SEGMENT_TREE_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_segment_tree_is_empty";

pub const SPARSE_TABLE_NEW_TARGET: &str = "__tyrus_algorithm_sparse_table_new";
pub const SPARSE_TABLE_QUERY_TARGET: &str = "__tyrus_algorithm_sparse_table_query";
pub const SPARSE_TABLE_LEN_TARGET: &str = "__tyrus_algorithm_sparse_table_len";
pub const SPARSE_TABLE_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_sparse_table_is_empty";

pub const SEGMENT_TREE_LAZY_NEW_TARGET: &str = "__tyrus_algorithm_segment_tree_lazy_new";
pub const SEGMENT_TREE_LAZY_UPDATE_RANGE_TARGET: &str =
    "__tyrus_algorithm_segment_tree_lazy_update_range";
pub const SEGMENT_TREE_LAZY_QUERY_TARGET: &str = "__tyrus_algorithm_segment_tree_lazy_query";
pub const SEGMENT_TREE_LAZY_LEN_TARGET: &str = "__tyrus_algorithm_segment_tree_lazy_len";
pub const SEGMENT_TREE_LAZY_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_segment_tree_lazy_is_empty";

pub const BIT_SET_NEW_TARGET: &str = "__tyrus_algorithm_bit_set_new";
pub const BIT_SET_FROM_ARRAY_TARGET: &str = "__tyrus_algorithm_bit_set_from_array";
pub const BIT_SET_SET_TARGET: &str = "__tyrus_algorithm_bit_set_set";
pub const BIT_SET_FLIP_TARGET: &str = "__tyrus_algorithm_bit_set_flip";
pub const BIT_SET_CLEAR_TARGET: &str = "__tyrus_algorithm_bit_set_clear";
pub const BIT_SET_GET_TARGET: &str = "__tyrus_algorithm_bit_set_get";
pub const BIT_SET_COUNT_TARGET: &str = "__tyrus_algorithm_bit_set_count";
pub const BIT_SET_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_bit_set_is_empty";
pub const BIT_SET_IS_SUBSET_TARGET: &str = "__tyrus_algorithm_bit_set_is_subset";
pub const BIT_SET_IS_SUPERSET_TARGET: &str = "__tyrus_algorithm_bit_set_is_superset";
pub const BIT_SET_UNION_TARGET: &str = "__tyrus_algorithm_bit_set_union";
pub const BIT_SET_INTERSECTION_TARGET: &str = "__tyrus_algorithm_bit_set_intersection";
pub const BIT_SET_DIFFERENCE_TARGET: &str = "__tyrus_algorithm_bit_set_difference";
pub const BIT_SET_COMPLEMENT_TARGET: &str = "__tyrus_algorithm_bit_set_complement";

pub const LINKED_LIST_NEW_TARGET: &str = "__tyrus_algorithm_linked_list_new";
pub const LINKED_LIST_FROM_ARRAY_TARGET: &str = "__tyrus_algorithm_linked_list_from_array";
pub const LINKED_LIST_PUSH_FRONT_TARGET: &str = "__tyrus_algorithm_linked_list_push_front";
pub const LINKED_LIST_PUSH_BACK_TARGET: &str = "__tyrus_algorithm_linked_list_push_back";
pub const LINKED_LIST_POP_FRONT_TARGET: &str = "__tyrus_algorithm_linked_list_pop_front";
pub const LINKED_LIST_POP_BACK_TARGET: &str = "__tyrus_algorithm_linked_list_pop_back";
pub const LINKED_LIST_FRONT_TARGET: &str = "__tyrus_algorithm_linked_list_front";
pub const LINKED_LIST_BACK_TARGET: &str = "__tyrus_algorithm_linked_list_back";
pub const LINKED_LIST_LEN_TARGET: &str = "__tyrus_algorithm_linked_list_len";
pub const LINKED_LIST_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_linked_list_is_empty";
pub const LINKED_LIST_CLEAR_TARGET: &str = "__tyrus_algorithm_linked_list_clear";

pub const DOUBLE_LINKED_LIST_NEW_TARGET: &str = "__tyrus_algorithm_double_linked_list_new";
pub const DOUBLE_LINKED_LIST_FROM_ARRAY_TARGET: &str =
    "__tyrus_algorithm_double_linked_list_from_array";
pub const DOUBLE_LINKED_LIST_PUSH_FRONT_TARGET: &str =
    "__tyrus_algorithm_double_linked_list_push_front";
pub const DOUBLE_LINKED_LIST_PUSH_BACK_TARGET: &str =
    "__tyrus_algorithm_double_linked_list_push_back";
pub const DOUBLE_LINKED_LIST_POP_FRONT_TARGET: &str =
    "__tyrus_algorithm_double_linked_list_pop_front";
pub const DOUBLE_LINKED_LIST_POP_BACK_TARGET: &str =
    "__tyrus_algorithm_double_linked_list_pop_back";
pub const DOUBLE_LINKED_LIST_FRONT_TARGET: &str = "__tyrus_algorithm_double_linked_list_front";
pub const DOUBLE_LINKED_LIST_BACK_TARGET: &str = "__tyrus_algorithm_double_linked_list_back";
pub const DOUBLE_LINKED_LIST_LEN_TARGET: &str = "__tyrus_algorithm_double_linked_list_len";
pub const DOUBLE_LINKED_LIST_IS_EMPTY_TARGET: &str =
    "__tyrus_algorithm_double_linked_list_is_empty";
pub const DOUBLE_LINKED_LIST_CLEAR_TARGET: &str = "__tyrus_algorithm_double_linked_list_clear";

pub const AVL_TREE_NEW_TARGET: &str = "__tyrus_algorithm_avl_tree_new";
pub const AVL_TREE_INSERT_TARGET: &str = "__tyrus_algorithm_avl_tree_insert";
pub const AVL_TREE_CONTAINS_TARGET: &str = "__tyrus_algorithm_avl_tree_contains";
pub const AVL_TREE_LEN_TARGET: &str = "__tyrus_algorithm_avl_tree_len";
pub const AVL_TREE_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_avl_tree_is_empty";

pub const RED_BLACK_TREE_NEW_TARGET: &str = "__tyrus_algorithm_red_black_tree_new";
pub const RED_BLACK_TREE_INSERT_TARGET: &str = "__tyrus_algorithm_red_black_tree_insert";
pub const RED_BLACK_TREE_CONTAINS_TARGET: &str = "__tyrus_algorithm_red_black_tree_contains";
pub const RED_BLACK_TREE_LEN_TARGET: &str = "__tyrus_algorithm_red_black_tree_len";
pub const RED_BLACK_TREE_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_red_black_tree_is_empty";

pub const SPLAY_TREE_NEW_TARGET: &str = "__tyrus_algorithm_splay_tree_new";
pub const SPLAY_TREE_INSERT_TARGET: &str = "__tyrus_algorithm_splay_tree_insert";
pub const SPLAY_TREE_CONTAINS_TARGET: &str = "__tyrus_algorithm_splay_tree_contains";
pub const SPLAY_TREE_LEN_TARGET: &str = "__tyrus_algorithm_splay_tree_len";
pub const SPLAY_TREE_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_splay_tree_is_empty";

pub const TREAP_NEW_TARGET: &str = "__tyrus_algorithm_treap_new";
pub const TREAP_INSERT_TARGET: &str = "__tyrus_algorithm_treap_insert";
pub const TREAP_CONTAINS_TARGET: &str = "__tyrus_algorithm_treap_contains";
pub const TREAP_LEN_TARGET: &str = "__tyrus_algorithm_treap_len";
pub const TREAP_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_treap_is_empty";

pub const SORTED_SET_NEW_TARGET: &str = "__tyrus_algorithm_sorted_set_new";
pub const SORTED_SET_INSERT_TARGET: &str = "__tyrus_algorithm_sorted_set_insert";
pub const SORTED_SET_REMOVE_TARGET: &str = "__tyrus_algorithm_sorted_set_remove";
pub const SORTED_SET_CONTAINS_TARGET: &str = "__tyrus_algorithm_sorted_set_contains";
pub const SORTED_SET_MIN_TARGET: &str = "__tyrus_algorithm_sorted_set_min";
pub const SORTED_SET_MAX_TARGET: &str = "__tyrus_algorithm_sorted_set_max";
pub const SORTED_SET_LEN_TARGET: &str = "__tyrus_algorithm_sorted_set_len";
pub const SORTED_SET_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_sorted_set_is_empty";
pub const SORTED_SET_CLEAR_TARGET: &str = "__tyrus_algorithm_sorted_set_clear";

pub const SORTED_MAP_NEW_TARGET: &str = "__tyrus_algorithm_sorted_map_new";
pub const SORTED_MAP_SET_TARGET: &str = "__tyrus_algorithm_sorted_map_set";
pub const SORTED_MAP_GET_TARGET: &str = "__tyrus_algorithm_sorted_map_get";
pub const SORTED_MAP_HAS_TARGET: &str = "__tyrus_algorithm_sorted_map_has";
pub const SORTED_MAP_DELETE_TARGET: &str = "__tyrus_algorithm_sorted_map_delete";
pub const SORTED_MAP_MIN_TARGET: &str = "__tyrus_algorithm_sorted_map_min";
pub const SORTED_MAP_MAX_TARGET: &str = "__tyrus_algorithm_sorted_map_max";
pub const SORTED_MAP_LEN_TARGET: &str = "__tyrus_algorithm_sorted_map_len";
pub const SORTED_MAP_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_sorted_map_is_empty";
pub const SORTED_MAP_CLEAR_TARGET: &str = "__tyrus_algorithm_sorted_map_clear";

pub const GRAPH_NEW_TARGET: &str = "__tyrus_algorithm_graph_new";
pub const GRAPH_ADD_NODE_TARGET: &str = "__tyrus_algorithm_graph_add_node";
pub const GRAPH_ADD_EDGE_TARGET: &str = "__tyrus_algorithm_graph_add_edge";
pub const GRAPH_REMOVE_NODE_TARGET: &str = "__tyrus_algorithm_graph_remove_node";
pub const GRAPH_REMOVE_EDGE_TARGET: &str = "__tyrus_algorithm_graph_remove_edge";
pub const GRAPH_HAS_NODE_TARGET: &str = "__tyrus_algorithm_graph_has_node";
pub const GRAPH_HAS_EDGE_TARGET: &str = "__tyrus_algorithm_graph_has_edge";
pub const GRAPH_EDGE_TARGET: &str = "__tyrus_algorithm_graph_edge";
pub const GRAPH_NEIGHBORS_TARGET: &str = "__tyrus_algorithm_graph_neighbors";
pub const GRAPH_DEGREE_TARGET: &str = "__tyrus_algorithm_graph_degree";
pub const GRAPH_NODE_COUNT_TARGET: &str = "__tyrus_algorithm_graph_node_count";
pub const GRAPH_EDGE_COUNT_TARGET: &str = "__tyrus_algorithm_graph_edge_count";
pub const GRAPH_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_graph_is_empty";
pub const GRAPH_NODES_TARGET: &str = "__tyrus_algorithm_graph_nodes";
pub const GRAPH_EDGES_SOURCES_TARGET: &str = "__tyrus_algorithm_graph_edges_sources";
pub const GRAPH_EDGES_TARGETS_TARGET: &str = "__tyrus_algorithm_graph_edges_targets";
pub const GRAPH_EDGES_WEIGHTS_TARGET: &str = "__tyrus_algorithm_graph_edges_weights";
pub const GRAPH_REVERSE_TARGET: &str = "__tyrus_algorithm_graph_reverse";
pub const GRAPH_TO_DOT_TARGET: &str = "__tyrus_algorithm_graph_to_dot";
pub const GRAPH_TO_STRING_TARGET: &str = "__tyrus_algorithm_graph_to_string";

pub const BFS_TARGET: &str = "__tyrus_algorithm_bfs";
pub const BFS_INFO_TARGET: &str = "__tyrus_algorithm_bfs_info";
pub const DFS_TARGET: &str = "__tyrus_algorithm_dfs";
pub const DFS_INFO_TARGET: &str = "__tyrus_algorithm_dfs_info";
pub const IDDFS_TARGET: &str = "__tyrus_algorithm_iddfs";
pub const IDDFS_INFO_TARGET: &str = "__tyrus_algorithm_iddfs_info";
pub const DIJKSTRA_TARGET: &str = "__tyrus_algorithm_dijkstra";
pub const DIJKSTRA_INFO_TARGET: &str = "__tyrus_algorithm_dijkstra_info";
pub const A_STAR_TARGET: &str = "__tyrus_algorithm_a_star";
pub const A_STAR_INFO_TARGET: &str = "__tyrus_algorithm_a_star_info";
pub const BELLMAN_FORD_TARGET: &str = "__tyrus_algorithm_bellman_ford";
pub const BELLMAN_FORD_INFO_TARGET: &str = "__tyrus_algorithm_bellman_ford_info";
pub const FLOYD_WARSHALL_TARGET: &str = "__tyrus_algorithm_floyd_warshall";
pub const FLOYD_WARSHALL_INFO_TARGET: &str = "__tyrus_algorithm_floyd_warshall_info";
pub const SPFA_TARGET: &str = "__tyrus_algorithm_spfa";
pub const SPFA_INFO_TARGET: &str = "__tyrus_algorithm_spfa_info";
pub const JOHNSON_TARGET: &str = "__tyrus_algorithm_johnson";
pub const JOHNSON_INFO_TARGET: &str = "__tyrus_algorithm_johnson_info";
pub const KRUSKAL_TARGET: &str = "__tyrus_algorithm_kruskal";
pub const KRUSKAL_INFO_TARGET: &str = "__tyrus_algorithm_kruskal_info";
pub const PRIM_TARGET: &str = "__tyrus_algorithm_prim";
pub const PRIM_INFO_TARGET: &str = "__tyrus_algorithm_prim_info";
pub const BORUVKA_TARGET: &str = "__tyrus_algorithm_boruvka";
pub const BORUVKA_INFO_TARGET: &str = "__tyrus_algorithm_boruvka_info";
pub const STEINER_TREE_TARGET: &str = "__tyrus_algorithm_steiner_tree";
pub const STEINER_TREE_INFO_TARGET: &str = "__tyrus_algorithm_steiner_tree_info";
pub const EDMONDS_KARP_TARGET: &str = "__tyrus_algorithm_edmonds_karp";
pub const DINIC_TARGET: &str = "__tyrus_algorithm_dinic";
pub const PUSH_RELABEL_TARGET: &str = "__tyrus_algorithm_push_relabel";
pub const MIN_CUT_TARGET: &str = "__tyrus_algorithm_min_cut";
pub const MIN_COST_MAX_FLOW_TARGET: &str = "__tyrus_algorithm_min_cost_max_flow";
pub const EDMONDS_KARP_INFO_TARGET: &str = "__tyrus_algorithm_edmonds_karp_info";
pub const DINIC_INFO_TARGET: &str = "__tyrus_algorithm_dinic_info";
pub const PUSH_RELABEL_INFO_TARGET: &str = "__tyrus_algorithm_push_relabel_info";
pub const MIN_CUT_INFO_TARGET: &str = "__tyrus_algorithm_min_cut_info";
pub const MIN_COST_MAX_FLOW_INFO_TARGET: &str = "__tyrus_algorithm_min_cost_max_flow_info";
pub const TOPOLOGICAL_KAHN_TARGET: &str = "__tyrus_algorithm_topological_sort_kahn";
pub const TOPOLOGICAL_DFS_TARGET: &str = "__tyrus_algorithm_topological_sort_dfs";
pub const TOPOLOGICAL_KAHN_INFO_TARGET: &str = "__tyrus_algorithm_topological_sort_kahn_info";
pub const TOPOLOGICAL_DFS_INFO_TARGET: &str = "__tyrus_algorithm_topological_sort_dfs_info";
pub const TARJAN_SCC_TARGET: &str = "__tyrus_algorithm_tarjan_scc";
pub const KOSARAJU_SCC_TARGET: &str = "__tyrus_algorithm_kosaraju_scc";
pub const TARJAN_SCC_INFO_TARGET: &str = "__tyrus_algorithm_tarjan_scc_info";
pub const KOSARAJU_SCC_INFO_TARGET: &str = "__tyrus_algorithm_kosaraju_scc_info";
pub const FLEURY_EULERIAN_TARGET: &str = "__tyrus_algorithm_fleury_eulerian";
pub const HIERHOLZER_EULERIAN_TARGET: &str = "__tyrus_algorithm_hierholzer_eulerian";
pub const FLEURY_EULERIAN_INFO_TARGET: &str = "__tyrus_algorithm_fleury_eulerian_info";
pub const HIERHOLZER_EULERIAN_INFO_TARGET: &str = "__tyrus_algorithm_hierholzer_eulerian_info";
pub const FIND_BRIDGES_INFO_TARGET: &str = "__tyrus_algorithm_find_bridges_info";
pub const FIND_CUT_VERTICES_INFO_TARGET: &str = "__tyrus_algorithm_find_cut_vertices_info";
pub const BICONNECTED_COMPONENTS_INFO_TARGET: &str =
    "__tyrus_algorithm_biconnected_components_info";
pub const EDMONDS_BLOSSOM_INFO_TARGET: &str = "__tyrus_algorithm_edmonds_blossom_info";
pub const HUNGARIAN_INFO_TARGET: &str = "__tyrus_algorithm_hungarian_info";
pub const FIND_BRIDGES_TARGET: &str = "__tyrus_algorithm_find_bridges";
pub const FIND_CUT_VERTICES_TARGET: &str = "__tyrus_algorithm_find_cut_vertices";
pub const BICONNECTED_COMPONENTS_TARGET: &str = "__tyrus_algorithm_biconnected_components";
pub const EDMONDS_BLOSSOM_TARGET: &str = "__tyrus_algorithm_edmonds_blossom";
pub const HUNGARIAN_TARGET: &str = "__tyrus_algorithm_hungarian";
pub const MERGE_SORT_TARGET: &str = "__tyrus_algorithm_merge_sort";
pub const QUICK_SORT_TARGET: &str = "__tyrus_algorithm_quick_sort";
pub const HEAP_SORT_TARGET: &str = "__tyrus_algorithm_heap_sort";
pub const COUNTING_SORT_TARGET: &str = "__tyrus_algorithm_counting_sort";
pub const RADIX_SORT_TARGET: &str = "__tyrus_algorithm_radix_sort";
pub const BUCKET_SORT_TARGET: &str = "__tyrus_algorithm_bucket_sort";
pub const TIM_SORT_TARGET: &str = "__tyrus_algorithm_tim_sort";
pub const INSERTION_SORT_TARGET: &str = "__tyrus_algorithm_insertion_sort";
pub const SHELL_SORT_TARGET: &str = "__tyrus_algorithm_shell_sort";
pub const SELECTION_SORT_TARGET: &str = "__tyrus_algorithm_selection_sort";
pub const BUBBLE_SORT_TARGET: &str = "__tyrus_algorithm_bubble_sort";
pub const BUBBLE_SORT_INFO_TARGET: &str = "__tyrus_algorithm_bubble_sort_info";
pub const MERGE_SORT_INFO_TARGET: &str = "__tyrus_algorithm_merge_sort_info";
pub const QUICK_SORT_INFO_TARGET: &str = "__tyrus_algorithm_quick_sort_info";
pub const HEAP_SORT_INFO_TARGET: &str = "__tyrus_algorithm_heap_sort_info";
pub const COUNTING_SORT_INFO_TARGET: &str = "__tyrus_algorithm_counting_sort_info";
pub const RADIX_SORT_INFO_TARGET: &str = "__tyrus_algorithm_radix_sort_info";
pub const BUCKET_SORT_INFO_TARGET: &str = "__tyrus_algorithm_bucket_sort_info";
pub const TIM_SORT_INFO_TARGET: &str = "__tyrus_algorithm_tim_sort_info";
pub const INSERTION_SORT_INFO_TARGET: &str = "__tyrus_algorithm_insertion_sort_info";
pub const SHELL_SORT_INFO_TARGET: &str = "__tyrus_algorithm_shell_sort_info";
pub const SELECTION_SORT_INFO_TARGET: &str = "__tyrus_algorithm_selection_sort_info";
pub const SHAKER_SORT_INFO_TARGET: &str = "__tyrus_algorithm_shaker_sort_info";
pub const SHAKER_SORT_TARGET: &str = "__tyrus_algorithm_shaker_sort";
pub const BUBBLE_SORT_NEW_TARGET: &str = "__tyrus_algorithm_bubble_sort_new";
pub const BUBBLE_SORT_STEP_TARGET: &str = "__tyrus_algorithm_bubble_sort_step";
pub const BUBBLE_SORT_RUN_TARGET: &str = "__tyrus_algorithm_bubble_sort_run";
pub const BUBBLE_SORT_VALUES_TARGET: &str = "__tyrus_algorithm_bubble_sort_values";
pub const BUBBLE_SORT_DONE_TARGET: &str = "__tyrus_algorithm_bubble_sort_done";
pub const BUBBLE_SORT_LEFT_TARGET: &str = "__tyrus_algorithm_bubble_sort_left";
pub const BUBBLE_SORT_RIGHT_TARGET: &str = "__tyrus_algorithm_bubble_sort_right";
pub const BUBBLE_SORT_SWAPPED_TARGET: &str = "__tyrus_algorithm_bubble_sort_swapped";
pub const BUBBLE_SORT_PASS_TARGET: &str = "__tyrus_algorithm_bubble_sort_pass";
pub const BUBBLE_SORT_COMPARISONS_TARGET: &str = "__tyrus_algorithm_bubble_sort_comparisons";
pub const BUBBLE_SORT_SWAPS_TARGET: &str = "__tyrus_algorithm_bubble_sort_swaps";
pub const BUBBLE_SORT_RESET_TARGET: &str = "__tyrus_algorithm_bubble_sort_reset";
pub const QUICKSELECT_TARGET: &str = "__tyrus_algorithm_quickselect";
pub const QUICKSELECT_INFO_TARGET: &str = "__tyrus_algorithm_quickselect_info";
pub const BINARY_SEARCH_TARGET: &str = "__tyrus_algorithm_binary_search";
pub const BINARY_SEARCH_INFO_TARGET: &str = "__tyrus_algorithm_binary_search_info";
pub const INTERPOLATION_SEARCH_TARGET: &str = "__tyrus_algorithm_interpolation_search";
pub const INTERPOLATION_SEARCH_INFO_TARGET: &str = "__tyrus_algorithm_interpolation_search_info";
pub const EXPONENTIAL_SEARCH_TARGET: &str = "__tyrus_algorithm_exponential_search";
pub const EXPONENTIAL_SEARCH_INFO_TARGET: &str = "__tyrus_algorithm_exponential_search_info";
pub const KMP_SEARCH_TARGET: &str = "__tyrus_algorithm_kmp_search";
pub const KMP_SEARCH_INFO_TARGET: &str = "__tyrus_algorithm_kmp_search_info";
pub const Z_ALGORITHM_TARGET: &str = "__tyrus_algorithm_z_algorithm";
pub const Z_ALGORITHM_INFO_TARGET: &str = "__tyrus_algorithm_z_algorithm_info";
pub const RABIN_KARP_TARGET: &str = "__tyrus_algorithm_rabin_karp";
pub const RABIN_KARP_INFO_TARGET: &str = "__tyrus_algorithm_rabin_karp_info";
pub const BOYER_MOORE_TARGET: &str = "__tyrus_algorithm_boyer_moore";
pub const BOYER_MOORE_INFO_TARGET: &str = "__tyrus_algorithm_boyer_moore_info";
pub const AHO_CORASICK_TARGET: &str = "__tyrus_algorithm_aho_corasick";
pub const AHO_CORASICK_INFO_TARGET: &str = "__tyrus_algorithm_aho_corasick_info";
pub const LEVENSHTEIN_TARGET: &str = "__tyrus_algorithm_levenshtein";
pub const LEVENSHTEIN_INFO_TARGET: &str = "__tyrus_algorithm_levenshtein_info";

/// Value-kind tags passed to `tyrus_graph_new`, shared with the runtime
/// (`crates/tyrus-runtime/src/graph/graph.rs`). Nodes and weights are erased at the C
/// ABI boundary; the tags drive identity and rendering of the raw bytes.
pub const GRAPH_TAG_SIGNED: u64 = 1;
pub const GRAPH_TAG_UNSIGNED: u64 = 2;
pub const GRAPH_TAG_FLOAT: u64 = 3;
pub const GRAPH_TAG_BOOL: u64 = 4;
pub const GRAPH_TAG_STRING: u64 = 5;

pub const TRIE_NEW_TARGET: &str = "__tyrus_algorithm_trie_new";
pub const TRIE_INSERT_TARGET: &str = "__tyrus_algorithm_trie_insert";
pub const TRIE_SEARCH_TARGET: &str = "__tyrus_algorithm_trie_search";
pub const TRIE_STARTS_WITH_TARGET: &str = "__tyrus_algorithm_trie_starts_with";
pub const TRIE_LEN_TARGET: &str = "__tyrus_algorithm_trie_len";
pub const TRIE_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_trie_is_empty";
pub const TRIE_CLEAR_TARGET: &str = "__tyrus_algorithm_trie_clear";

pub const SUFFIX_ARRAY_TARGET: &str = "__tyrus_algorithm_suffix_array";
pub const SUFFIX_ARRAY_INFO_TARGET: &str = "__tyrus_algorithm_suffix_array_info";

pub const SUFFIX_TREE_NEW_TARGET: &str = "__tyrus_algorithm_suffix_tree_new";

pub const SUFFIX_TREE_GET_SUFFIX_ARRAY_TARGET: &str =
    "__tyrus_algorithm_suffix_tree_get_suffix_array";

pub const LCA_NEW_TARGET: &str = "__tyrus_algorithm_lca_new";

pub const LCA_QUERY_TARGET: &str = "__tyrus_algorithm_lca_query";

pub const HLD_NEW_TARGET: &str = "__tyrus_algorithm_hld_new";

pub const HLD_QUERY_PATH_TARGET: &str = "__tyrus_algorithm_hld_query_path";

pub const HLD_POSITION_TARGET: &str = "__tyrus_algorithm_hld_position";

pub const HLD_HEAD_TARGET: &str = "__tyrus_algorithm_hld_head";

pub const CENTROID_NEW_TARGET: &str = "__tyrus_algorithm_centroid_new";

pub const CENTROID_PARENT_OF_TARGET: &str = "__tyrus_algorithm_centroid_parent_of";

pub const CENTROID_DEPTH_TARGET: &str = "__tyrus_algorithm_centroid_depth";

pub const BLOOM_FILTER_NEW_TARGET: &str = "__tyrus_algorithm_bloom_filter_new";

pub const BLOOM_FILTER_INSERT_TARGET: &str = "__tyrus_algorithm_bloom_filter_insert";

pub const BLOOM_FILTER_CONTAINS_TARGET: &str = "__tyrus_algorithm_bloom_filter_contains";

pub const SKIP_LIST_NEW_TARGET: &str = "__tyrus_algorithm_skip_list_new";

pub const SKIP_LIST_INSERT_TARGET: &str = "__tyrus_algorithm_skip_list_insert";

pub const SKIP_LIST_REMOVE_TARGET: &str = "__tyrus_algorithm_skip_list_remove";

pub const SKIP_LIST_CONTAINS_TARGET: &str = "__tyrus_algorithm_skip_list_contains";

pub const SKIP_LIST_MIN_TARGET: &str = "__tyrus_algorithm_skip_list_min";

pub const SKIP_LIST_MAX_TARGET: &str = "__tyrus_algorithm_skip_list_max";

pub const SKIP_LIST_LEN_TARGET: &str = "__tyrus_algorithm_skip_list_len";

pub const SKIP_LIST_IS_EMPTY_TARGET: &str = "__tyrus_algorithm_skip_list_is_empty";

pub const SKIP_LIST_CLEAR_TARGET: &str = "__tyrus_algorithm_skip_list_clear";

fn algorithm_field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

pub fn union_find_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "find",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "union",
            Type::Function {
                params: vec![Type::Usize, Type::Usize],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "connected",
            Type::Function {
                params: vec![Type::Usize, Type::Usize],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "setCount",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
    ]
}

// ─── BinaryHeap interface fields ───
//
// BinaryHeap<T> is generic; the element type is erased at the C ABI boundary.
// All methods operate on opaque pointers. The type parameter T is tracked only
// at the type-checker level.

pub fn binary_heap_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "push",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "pop",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "peek",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

// ─── BinomialHeap interface fields ───
//
// BinomialHeap<T> is generic; the element type is erased at the C ABI boundary.
// All methods operate on opaque pointers. The type parameter T is tracked only
// at the type-checker level.

pub fn binomial_heap_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "push",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "pop",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "peek",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

// ─── Counter interface fields ───
//
// Counter<T> is generic; the element type is erased at the C ABI boundary.
// All methods operate on opaque pointers. The type parameter T is tracked only
// at the type-checker level.

pub fn counter_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "add",
            Type::Function {
                params: vec![Type::Unknown, Type::I32],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "remove",
            Type::Function {
                params: vec![Type::Unknown, Type::I32],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "count",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::I32),
            },
        ),
        algorithm_field(
            "mostCommon",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Array(Box::new(Type::Tuple(vec![
                    Type::Unknown,
                    Type::I32,
                ])))),
            },
        ),
        algorithm_field(
            "total",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::I32),
            },
        ),
        algorithm_field(
            "unique",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

// ─── Deque interface fields ───
//
// Deque<T> is generic; the element type is erased at the C ABI boundary.
// All methods operate on opaque pointers. The type parameter T is tracked only
// at the type-checker level.

pub fn deque_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "pushFront",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "pushBack",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "popFront",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "popBack",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "peekFront",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "peekBack",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

pub fn fenwick_tree_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "add",
            Type::Function {
                params: vec![Type::Usize, Type::I32],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "sum",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::I32),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
    ]
}

pub fn segment_tree_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "update",
            Type::Function {
                params: vec![Type::Usize, Type::I32],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "query",
            Type::Function {
                params: vec![Type::Usize, Type::Usize],
                ret: Box::new(Type::I32),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
    ]
}

pub fn segment_tree_lazy_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "updateRange",
            Type::Function {
                params: vec![Type::Usize, Type::Usize, Type::I32],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "query",
            Type::Function {
                params: vec![Type::Usize, Type::Usize],
                ret: Box::new(Type::I32),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
    ]
}

pub fn sparse_table_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "query",
            Type::Function {
                params: vec![Type::Usize, Type::Usize],
                ret: Box::new(Type::I32),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
    ]
}

pub fn bit_set_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "set",
            Type::Function {
                params: vec![Type::Usize, Type::Bool],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "flip",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "get",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "count",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "isSubset",
            Type::Function {
                params: vec![Type::BitSet],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "isSuperset",
            Type::Function {
                params: vec![Type::BitSet],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "union",
            Type::Function {
                params: vec![Type::BitSet],
                ret: Box::new(Type::BitSet),
            },
        ),
        algorithm_field(
            "intersection",
            Type::Function {
                params: vec![Type::BitSet],
                ret: Box::new(Type::BitSet),
            },
        ),
        algorithm_field(
            "difference",
            Type::Function {
                params: vec![Type::BitSet],
                ret: Box::new(Type::BitSet),
            },
        ),
        algorithm_field(
            "complement",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::BitSet),
            },
        ),
    ]
}

// ─── LinkedList interface fields ───
//
// LinkedList<T> is generic; the element type is erased at the C ABI boundary.
// All methods operate on opaque pointers. The type parameter T is tracked only
// at the type-checker level.

pub fn linked_list_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "pushFront",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "pushBack",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "popFront",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "popBack",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "front",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "back",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

// ─── DoubleLinkedList interface fields ───
//
// DoubleLinkedList<T> is generic; the element type is erased at the C ABI
// boundary. All methods operate on opaque pointers. The type parameter T is
// tracked only at the type-checker level.

pub fn double_linked_list_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "pushFront",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "pushBack",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "popFront",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "popBack",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "front",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "back",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

// ─── Ordered tree interface fields ───
//
// AvlTree<T>, RedBlackTree<T>, SplayTree<T> and Treap<T> share the same
// surface: an ordered set backed by a balanced BST. The element type is erased
// at the C ABI boundary; methods operate on opaque pointers and the type
// parameter T is tracked only at the type-checker level.

fn ordered_tree_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "insert",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "contains",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
    ]
}

pub fn avl_tree_interface_fields() -> Vec<Field> {
    ordered_tree_interface_fields()
}

pub fn red_black_tree_interface_fields() -> Vec<Field> {
    ordered_tree_interface_fields()
}

pub fn splay_tree_interface_fields() -> Vec<Field> {
    ordered_tree_interface_fields()
}

pub fn treap_interface_fields() -> Vec<Field> {
    ordered_tree_interface_fields()
}

// ─── SortedSet interface fields ───
//
// SortedSet<T> is an ordered set backed by a balanced BST: `insert` reports
// whether a fresh element was added and `remove`/`min`/`max` complete the
// ordered surface. The element type is erased at the C ABI boundary; the type
// parameter T is tracked only at the type-checker level.

pub fn sorted_set_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "insert",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "remove",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "contains",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "min",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "max",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

// ─── SkipList interface fields ───
//
// SkipList<T> is an ordered set backed by a probabilistic skip list: `insert`
// reports whether a fresh element was added and `remove`/`min`/`max` complete
// the ordered surface. The element type is erased at the C ABI boundary; the
// type parameter T is tracked only at the type-checker level.

pub fn skip_list_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "insert",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "remove",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "contains",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "min",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "max",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

// ─── SortedMap interface fields ───
//
// SortedMap<K, V> is an ordered key → value map backed by a balanced BST. The
// key and value types are erased at the C ABI boundary; the type parameters K
// and V are tracked only at the type-checker level.

pub fn sorted_map_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "set",
            Type::Function {
                params: vec![Type::Unknown, Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "get",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "has",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "delete",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "min",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "max",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

// ─── Graph interface fields ───
//
// Graph<N, W> is a directed, weighted graph. The node and weight types are
// erased at the C ABI boundary; the type parameters N and W are tracked only
// at the type-checker level. Type-parameterized returns are `Unknown`.

pub fn graph_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "addEdge",
            Type::Function {
                params: vec![Type::Unknown, Type::Unknown, Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "addNode",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "removeNode",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "removeEdge",
            Type::Function {
                params: vec![Type::Unknown, Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "hasNode",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "hasEdge",
            Type::Function {
                params: vec![Type::Unknown, Type::Unknown],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "edge",
            Type::Function {
                params: vec![Type::Unknown, Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "neighbors",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "degree",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "nodeCount",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "edgeCount",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "nodes",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "edges",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "reverse",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "mapEdges",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "mapNodes",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
        ),
        algorithm_field(
            "toDot",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Str),
            },
        ),
        algorithm_field(
            "toString",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Str),
            },
        ),
    ]
}

pub fn trie_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "insert",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "search",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "startsWith",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "len",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "isEmpty",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Bool),
            },
        ),
        algorithm_field(
            "clear",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

pub fn suffix_tree_interface_fields() -> Vec<Field> {
    vec![algorithm_field(
        "getSuffixArray",
        Type::Function {
            params: vec![],
            ret: Box::new(Type::Array(Box::new(Type::Usize))),
        },
    )]
}

pub fn lca_interface_fields() -> Vec<Field> {
    vec![algorithm_field(
        "query",
        Type::Function {
            params: vec![Type::Usize, Type::Usize],
            ret: Box::new(Type::Usize),
        },
    )]
}

pub fn hld_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "queryPath",
            Type::Function {
                params: vec![Type::Usize, Type::Usize],
                ret: Box::new(Type::Array(Box::new(Type::Tuple(vec![
                    Type::Usize,
                    Type::Usize,
                ])))),
            },
        ),
        algorithm_field(
            "position",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Usize),
            },
        ),
        algorithm_field(
            "head",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Usize),
            },
        ),
    ]
}

pub fn centroid_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "parentOf",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Optional(Box::new(Type::Usize))),
            },
        ),
        algorithm_field(
            "depth",
            Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Usize),
            },
        ),
    ]
}

pub fn bloom_filter_interface_fields() -> Vec<Field> {
    vec![
        algorithm_field(
            "insert",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(Type::Void),
            },
        ),
        algorithm_field(
            "contains",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(Type::Bool),
            },
        ),
    ]
}

pub fn algorithm_error_fields() -> Vec<Field> {
    vec![
        algorithm_field("code", Type::Str),
        algorithm_field("message", Type::Str),
    ]
}

fn unweighted_adjacency() -> Type {
    Type::Array(Box::new(Type::Array(Box::new(Type::Usize))))
}

fn weighted_adjacency() -> Type {
    Type::Array(Box::new(Type::Array(Box::new(Type::Tuple(vec![
        Type::Usize,
        Type::I32,
    ])))))
}

fn weight_matrix() -> Type {
    Type::Array(Box::new(Type::Array(Box::new(Type::Optional(Box::new(
        Type::I32,
    ))))))
}

fn capacity_matrix() -> Type {
    Type::Array(Box::new(Type::Array(Box::new(Type::I32))))
}

fn any_weighted_graph() -> Type {
    Type::Graph(Box::new(Type::Infer), Box::new(Type::I32))
}

fn path_input(plain: Type) -> Type {
    Type::Union(vec![plain, any_weighted_graph()])
}

fn usize_array() -> Type {
    Type::Array(Box::new(Type::Usize))
}

fn i32_array() -> Type {
    Type::Array(Box::new(Type::I32))
}

fn edge_array() -> Type {
    Type::Array(Box::new(Type::Tuple(vec![
        Type::Usize,
        Type::Usize,
        Type::I32,
    ])))
}

fn cut_edge_array() -> Type {
    Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize])))
}

fn flow_result() -> Type {
    Type::Tuple(vec![Type::I32, edge_array()])
}

fn cut_result() -> Type {
    Type::Tuple(vec![Type::I32, cut_edge_array()])
}

fn min_cost_result() -> Type {
    Type::Tuple(vec![Type::I32, Type::I32])
}

fn pair_array() -> Type {
    Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize])))
}
fn nested_pair_array() -> Type {
    Type::Array(Box::new(pair_array()))
}
fn nested_usize_array() -> Type {
    Type::Array(Box::new(Type::Array(Box::new(Type::Usize))))
}
fn optional_usize_array() -> Type {
    Type::Optional(Box::new(usize_array()))
}

fn path_with_distance() -> Type {
    Type::Optional(Box::new(Type::Tuple(vec![usize_array(), Type::I32])))
}

fn distances_with_previous() -> Type {
    Type::Tuple(vec![
        i32_array(),
        Type::Array(Box::new(Type::Optional(Box::new(Type::Usize)))),
    ])
}

fn algorithm_error() -> Type {
    Type::Named("AlgorithmError".to_string())
}

fn path_algorithm_export(target: &str, params: Vec<Type>, ret: Type, throws: bool) -> ExportInfo {
    ExportInfo {
        target: target.to_string(),
        ty: Type::Function {
            params,
            ret: Box::new(if throws {
                Type::Result {
                    success: Box::new(ret),
                    error: Box::new(algorithm_error()),
                }
            } else {
                ret
            }),
        },
        interface_fields: None,
        behavior_info: None,
        type_alias_ty: None,
        enum_info: None,
        throws: vec![],
        db_model: None,
    }
}

/// The export table of `tyrus:algorithm`, built once. The two lookups below run
/// per call expression during code generation, and each used to rebuild the
/// whole map, every `ExportInfo`, every `Type` and every field list, to answer
/// one question about one target.
fn algorithm_builtins_cached() -> &'static HashMap<String, ExportInfo> {
    static CACHE: std::sync::OnceLock<HashMap<String, ExportInfo>> = std::sync::OnceLock::new();
    CACHE.get_or_init(algorithm_builtins)
}

pub fn algorithm_target_return_type(target: &str) -> Option<Type> {
    algorithm_builtins_cached().values().find_map(|info| {
        if info.target != target {
            return None;
        }
        match &info.ty {
            Type::Function { ret, .. } => Some((**ret).clone()),
            _ => None,
        }
    })
}

pub fn is_collection_constructor_target(target: &str) -> bool {
    algorithm_builtins_cached().iter().any(|(name, info)| {
        info.target == target
            && matches!(
                &info.ty,
                Type::Function { ret, .. }
                    if matches!(&**ret, Type::Named(returned) if returned == name)
            )
    })
}

pub fn algorithm_interface_fields() -> Vec<(String, Vec<Field>)> {
    vec![
        ("UnionFind".to_string(), union_find_interface_fields()),
        ("BinaryHeap".to_string(), binary_heap_interface_fields()),
        ("BinomialHeap".to_string(), binomial_heap_interface_fields()),
        ("Counter".to_string(), counter_interface_fields()),
        ("Deque".to_string(), deque_interface_fields()),
        ("FenwickTree".to_string(), fenwick_tree_interface_fields()),
        ("SegmentTree".to_string(), segment_tree_interface_fields()),
        ("SparseTable".to_string(), sparse_table_interface_fields()),
        (
            "SegmentTreeLazy".to_string(),
            segment_tree_lazy_interface_fields(),
        ),
        ("BitSet".to_string(), bit_set_interface_fields()),
        ("LinkedList".to_string(), linked_list_interface_fields()),
        (
            "DoubleLinkedList".to_string(),
            double_linked_list_interface_fields(),
        ),
        ("AvlTree".to_string(), avl_tree_interface_fields()),
        (
            "RedBlackTree".to_string(),
            red_black_tree_interface_fields(),
        ),
        ("SplayTree".to_string(), splay_tree_interface_fields()),
        ("Treap".to_string(), treap_interface_fields()),
        ("SortedSet".to_string(), sorted_set_interface_fields()),
        ("SkipList".to_string(), skip_list_interface_fields()),
        ("SortedMap".to_string(), sorted_map_interface_fields()),
        ("Graph".to_string(), graph_interface_fields()),
        ("Trie".to_string(), trie_interface_fields()),
        ("SuffixTree".to_string(), suffix_tree_interface_fields()),
        ("Lca".to_string(), lca_interface_fields()),
        ("Hld".to_string(), hld_interface_fields()),
        ("Centroid".to_string(), centroid_interface_fields()),
        ("BloomFilter".to_string(), bloom_filter_interface_fields()),
        ("AlgorithmError".to_string(), algorithm_error_fields()),
    ]
}

pub fn algorithm_method_target(type_name: &str, method: &str) -> Option<&'static str> {
    match (type_name, method) {
        ("UnionFind", "find") => Some(UNION_FIND_FIND_TARGET),
        ("UnionFind", "union") => Some(UNION_FIND_UNION_TARGET),
        ("UnionFind", "connected") => Some(UNION_FIND_CONNECTED_TARGET),
        ("UnionFind", "setCount") => Some(UNION_FIND_SET_COUNT_TARGET),
        ("BinaryHeap", "push") => Some(BINARY_HEAP_PUSH_TARGET),
        ("BinaryHeap", "pop") => Some(BINARY_HEAP_POP_TARGET),
        ("BinaryHeap", "peek") => Some(BINARY_HEAP_PEEK_TARGET),
        ("BinaryHeap", "len") => Some(BINARY_HEAP_LEN_TARGET),
        ("BinaryHeap", "isEmpty") => Some(BINARY_HEAP_IS_EMPTY_TARGET),
        ("BinaryHeap", "clear") => Some(BINARY_HEAP_CLEAR_TARGET),
        ("BinomialHeap", "push") => Some(BINOMIAL_HEAP_PUSH_TARGET),
        ("BinomialHeap", "pop") => Some(BINOMIAL_HEAP_POP_TARGET),
        ("BinomialHeap", "peek") => Some(BINOMIAL_HEAP_PEEK_TARGET),
        ("BinomialHeap", "len") => Some(BINOMIAL_HEAP_LEN_TARGET),
        ("BinomialHeap", "isEmpty") => Some(BINOMIAL_HEAP_IS_EMPTY_TARGET),
        ("BinomialHeap", "clear") => Some(BINOMIAL_HEAP_CLEAR_TARGET),
        ("Counter", "add") => Some(COUNTER_ADD_TARGET),
        ("Counter", "remove") => Some(COUNTER_REMOVE_TARGET),
        ("Counter", "count") => Some(COUNTER_COUNT_TARGET),
        ("Counter", "mostCommon") => Some(COUNTER_MOST_COMMON_TARGET),
        ("Counter", "total") => Some(COUNTER_TOTAL_TARGET),
        ("Counter", "unique") => Some(COUNTER_UNIQUE_TARGET),
        ("Counter", "isEmpty") => Some(COUNTER_IS_EMPTY_TARGET),
        ("Counter", "clear") => Some(COUNTER_CLEAR_TARGET),
        ("Deque", "pushFront") => Some(DEQUE_PUSH_FRONT_TARGET),
        ("Deque", "pushBack") => Some(DEQUE_PUSH_BACK_TARGET),
        ("Deque", "popFront") => Some(DEQUE_POP_FRONT_TARGET),
        ("Deque", "popBack") => Some(DEQUE_POP_BACK_TARGET),
        ("Deque", "peekFront") => Some(DEQUE_PEEK_FRONT_TARGET),
        ("Deque", "peekBack") => Some(DEQUE_PEEK_BACK_TARGET),
        ("Deque", "len") => Some(DEQUE_LEN_TARGET),
        ("Deque", "isEmpty") => Some(DEQUE_IS_EMPTY_TARGET),
        ("Deque", "clear") => Some(DEQUE_CLEAR_TARGET),
        ("FenwickTree", "add") => Some(FENWICK_TREE_ADD_TARGET),
        ("FenwickTree", "sum") => Some(FENWICK_TREE_SUM_TARGET),
        ("FenwickTree", "len") => Some(FENWICK_TREE_LEN_TARGET),
        ("FenwickTree", "isEmpty") => Some(FENWICK_TREE_IS_EMPTY_TARGET),
        ("BubbleSort", "step") => Some(BUBBLE_SORT_STEP_TARGET),
        ("BubbleSort", "run") => Some(BUBBLE_SORT_RUN_TARGET),
        ("BubbleSort", "values") => Some(BUBBLE_SORT_VALUES_TARGET),
        ("BubbleSort", "done") => Some(BUBBLE_SORT_DONE_TARGET),
        ("BubbleSort", "left") => Some(BUBBLE_SORT_LEFT_TARGET),
        ("BubbleSort", "right") => Some(BUBBLE_SORT_RIGHT_TARGET),
        ("BubbleSort", "swapped") => Some(BUBBLE_SORT_SWAPPED_TARGET),
        ("BubbleSort", "pass") => Some(BUBBLE_SORT_PASS_TARGET),
        ("BubbleSort", "comparisons") => Some(BUBBLE_SORT_COMPARISONS_TARGET),
        ("BubbleSort", "swaps") => Some(BUBBLE_SORT_SWAPS_TARGET),
        ("BubbleSort", "reset") => Some(BUBBLE_SORT_RESET_TARGET),
        ("SegmentTree", "update") => Some(SEGMENT_TREE_UPDATE_TARGET),
        ("SegmentTree", "query") => Some(SEGMENT_TREE_QUERY_TARGET),
        ("SegmentTree", "len") => Some(SEGMENT_TREE_LEN_TARGET),
        ("SegmentTree", "isEmpty") => Some(SEGMENT_TREE_IS_EMPTY_TARGET),
        ("SparseTable", "query") => Some(SPARSE_TABLE_QUERY_TARGET),
        ("SparseTable", "len") => Some(SPARSE_TABLE_LEN_TARGET),
        ("SparseTable", "isEmpty") => Some(SPARSE_TABLE_IS_EMPTY_TARGET),
        ("SegmentTreeLazy", "updateRange") => Some(SEGMENT_TREE_LAZY_UPDATE_RANGE_TARGET),
        ("SegmentTreeLazy", "query") => Some(SEGMENT_TREE_LAZY_QUERY_TARGET),
        ("SegmentTreeLazy", "len") => Some(SEGMENT_TREE_LAZY_LEN_TARGET),
        ("SegmentTreeLazy", "isEmpty") => Some(SEGMENT_TREE_LAZY_IS_EMPTY_TARGET),
        ("BitSet", "set") => Some(BIT_SET_SET_TARGET),
        ("BitSet", "flip") => Some(BIT_SET_FLIP_TARGET),
        ("BitSet", "clear") => Some(BIT_SET_CLEAR_TARGET),
        ("BitSet", "get") => Some(BIT_SET_GET_TARGET),
        ("BitSet", "count") => Some(BIT_SET_COUNT_TARGET),
        ("BitSet", "isEmpty") => Some(BIT_SET_IS_EMPTY_TARGET),
        ("BitSet", "isSubset") => Some(BIT_SET_IS_SUBSET_TARGET),
        ("BitSet", "isSuperset") => Some(BIT_SET_IS_SUPERSET_TARGET),
        ("BitSet", "union") => Some(BIT_SET_UNION_TARGET),
        ("BitSet", "intersection") => Some(BIT_SET_INTERSECTION_TARGET),
        ("BitSet", "difference") => Some(BIT_SET_DIFFERENCE_TARGET),
        ("BitSet", "complement") => Some(BIT_SET_COMPLEMENT_TARGET),
        ("LinkedList", "pushFront") => Some(LINKED_LIST_PUSH_FRONT_TARGET),
        ("LinkedList", "pushBack") => Some(LINKED_LIST_PUSH_BACK_TARGET),
        ("LinkedList", "popFront") => Some(LINKED_LIST_POP_FRONT_TARGET),
        ("LinkedList", "popBack") => Some(LINKED_LIST_POP_BACK_TARGET),
        ("LinkedList", "front") => Some(LINKED_LIST_FRONT_TARGET),
        ("LinkedList", "back") => Some(LINKED_LIST_BACK_TARGET),
        ("LinkedList", "len") => Some(LINKED_LIST_LEN_TARGET),
        ("LinkedList", "isEmpty") => Some(LINKED_LIST_IS_EMPTY_TARGET),
        ("LinkedList", "clear") => Some(LINKED_LIST_CLEAR_TARGET),
        ("DoubleLinkedList", "pushFront") => Some(DOUBLE_LINKED_LIST_PUSH_FRONT_TARGET),
        ("DoubleLinkedList", "pushBack") => Some(DOUBLE_LINKED_LIST_PUSH_BACK_TARGET),
        ("DoubleLinkedList", "popFront") => Some(DOUBLE_LINKED_LIST_POP_FRONT_TARGET),
        ("DoubleLinkedList", "popBack") => Some(DOUBLE_LINKED_LIST_POP_BACK_TARGET),
        ("DoubleLinkedList", "front") => Some(DOUBLE_LINKED_LIST_FRONT_TARGET),
        ("DoubleLinkedList", "back") => Some(DOUBLE_LINKED_LIST_BACK_TARGET),
        ("DoubleLinkedList", "len") => Some(DOUBLE_LINKED_LIST_LEN_TARGET),
        ("DoubleLinkedList", "isEmpty") => Some(DOUBLE_LINKED_LIST_IS_EMPTY_TARGET),
        ("DoubleLinkedList", "clear") => Some(DOUBLE_LINKED_LIST_CLEAR_TARGET),
        ("AvlTree", "insert") => Some(AVL_TREE_INSERT_TARGET),
        ("AvlTree", "contains") => Some(AVL_TREE_CONTAINS_TARGET),
        ("AvlTree", "len") => Some(AVL_TREE_LEN_TARGET),
        ("AvlTree", "isEmpty") => Some(AVL_TREE_IS_EMPTY_TARGET),
        ("RedBlackTree", "insert") => Some(RED_BLACK_TREE_INSERT_TARGET),
        ("RedBlackTree", "contains") => Some(RED_BLACK_TREE_CONTAINS_TARGET),
        ("RedBlackTree", "len") => Some(RED_BLACK_TREE_LEN_TARGET),
        ("RedBlackTree", "isEmpty") => Some(RED_BLACK_TREE_IS_EMPTY_TARGET),
        ("SplayTree", "insert") => Some(SPLAY_TREE_INSERT_TARGET),
        ("SplayTree", "contains") => Some(SPLAY_TREE_CONTAINS_TARGET),
        ("SplayTree", "len") => Some(SPLAY_TREE_LEN_TARGET),
        ("SplayTree", "isEmpty") => Some(SPLAY_TREE_IS_EMPTY_TARGET),
        ("Treap", "insert") => Some(TREAP_INSERT_TARGET),
        ("Treap", "contains") => Some(TREAP_CONTAINS_TARGET),
        ("Treap", "len") => Some(TREAP_LEN_TARGET),
        ("Treap", "isEmpty") => Some(TREAP_IS_EMPTY_TARGET),
        ("SortedSet", "insert") => Some(SORTED_SET_INSERT_TARGET),
        ("SortedSet", "remove") => Some(SORTED_SET_REMOVE_TARGET),
        ("SortedSet", "contains") => Some(SORTED_SET_CONTAINS_TARGET),
        ("SortedSet", "min") => Some(SORTED_SET_MIN_TARGET),
        ("SortedSet", "max") => Some(SORTED_SET_MAX_TARGET),
        ("SortedSet", "len") => Some(SORTED_SET_LEN_TARGET),
        ("SortedSet", "isEmpty") => Some(SORTED_SET_IS_EMPTY_TARGET),
        ("SortedSet", "clear") => Some(SORTED_SET_CLEAR_TARGET),
        ("SkipList", "insert") => Some(SKIP_LIST_INSERT_TARGET),
        ("SkipList", "remove") => Some(SKIP_LIST_REMOVE_TARGET),
        ("SkipList", "contains") => Some(SKIP_LIST_CONTAINS_TARGET),
        ("SkipList", "min") => Some(SKIP_LIST_MIN_TARGET),
        ("SkipList", "max") => Some(SKIP_LIST_MAX_TARGET),
        ("SkipList", "len") => Some(SKIP_LIST_LEN_TARGET),
        ("SkipList", "isEmpty") => Some(SKIP_LIST_IS_EMPTY_TARGET),
        ("SkipList", "clear") => Some(SKIP_LIST_CLEAR_TARGET),
        ("SortedMap", "set") => Some(SORTED_MAP_SET_TARGET),
        ("SortedMap", "get") => Some(SORTED_MAP_GET_TARGET),
        ("SortedMap", "has") => Some(SORTED_MAP_HAS_TARGET),
        ("SortedMap", "delete") => Some(SORTED_MAP_DELETE_TARGET),
        ("SortedMap", "min") => Some(SORTED_MAP_MIN_TARGET),
        ("SortedMap", "max") => Some(SORTED_MAP_MAX_TARGET),
        ("SortedMap", "len") => Some(SORTED_MAP_LEN_TARGET),
        ("SortedMap", "isEmpty") => Some(SORTED_MAP_IS_EMPTY_TARGET),
        ("SortedMap", "clear") => Some(SORTED_MAP_CLEAR_TARGET),
        ("Graph", "addEdge") => Some(GRAPH_ADD_EDGE_TARGET),
        ("Graph", "addNode") => Some(GRAPH_ADD_NODE_TARGET),
        ("Graph", "removeNode") => Some(GRAPH_REMOVE_NODE_TARGET),
        ("Graph", "removeEdge") => Some(GRAPH_REMOVE_EDGE_TARGET),
        ("Graph", "hasNode") => Some(GRAPH_HAS_NODE_TARGET),
        ("Graph", "hasEdge") => Some(GRAPH_HAS_EDGE_TARGET),
        ("Graph", "edge") => Some(GRAPH_EDGE_TARGET),
        ("Graph", "neighbors") => Some(GRAPH_NEIGHBORS_TARGET),
        ("Graph", "degree") => Some(GRAPH_DEGREE_TARGET),
        ("Graph", "nodeCount") => Some(GRAPH_NODE_COUNT_TARGET),
        ("Graph", "edgeCount") => Some(GRAPH_EDGE_COUNT_TARGET),
        ("Graph", "isEmpty") => Some(GRAPH_IS_EMPTY_TARGET),
        ("Graph", "nodes") => Some(GRAPH_NODES_TARGET),
        ("Graph", "edgesSources") => Some(GRAPH_EDGES_SOURCES_TARGET),
        ("Graph", "edgesTargets") => Some(GRAPH_EDGES_TARGETS_TARGET),
        ("Graph", "edgesWeights") => Some(GRAPH_EDGES_WEIGHTS_TARGET),
        ("Graph", "reverse") => Some(GRAPH_REVERSE_TARGET),
        ("Graph", "toDot") => Some(GRAPH_TO_DOT_TARGET),
        ("Graph", "toString") => Some(GRAPH_TO_STRING_TARGET),
        ("Trie", "insert") => Some(TRIE_INSERT_TARGET),
        ("Trie", "search") => Some(TRIE_SEARCH_TARGET),
        ("Trie", "startsWith") => Some(TRIE_STARTS_WITH_TARGET),
        ("Trie", "len") => Some(TRIE_LEN_TARGET),
        ("Trie", "isEmpty") => Some(TRIE_IS_EMPTY_TARGET),
        ("Trie", "clear") => Some(TRIE_CLEAR_TARGET),
        ("SuffixTree", "getSuffixArray") => Some(SUFFIX_TREE_GET_SUFFIX_ARRAY_TARGET),
        ("Lca", "query") => Some(LCA_QUERY_TARGET),
        ("Hld", "queryPath") => Some(HLD_QUERY_PATH_TARGET),
        ("Hld", "position") => Some(HLD_POSITION_TARGET),
        ("Hld", "head") => Some(HLD_HEAD_TARGET),
        ("Centroid", "parentOf") => Some(CENTROID_PARENT_OF_TARGET),
        ("Centroid", "depth") => Some(CENTROID_DEPTH_TARGET),
        ("BloomFilter", "insert") => Some(BLOOM_FILTER_INSERT_TARGET),
        ("BloomFilter", "contains") => Some(BLOOM_FILTER_CONTAINS_TARGET),
        _ => None,
    }
}

pub(crate) fn is_algorithm_type_name(name: &str) -> bool {
    matches!(
        name,
        "UnionFind"
            | "BinaryHeap"
            | "BinomialHeap"
            | "Counter"
            | "Deque"
            | "FenwickTree"
            | "BubbleSort"
            | "SegmentTree"
            | "SparseTable"
            | "SegmentTreeLazy"
            | "BitSet"
            | "LinkedList"
            | "DoubleLinkedList"
            | "AvlTree"
            | "RedBlackTree"
            | "SplayTree"
            | "Treap"
            | "SortedSet"
            | "SkipList"
            | "SortedMap"
            | "Graph"
            | "Trie"
            | "BloomFilter"
            | "SuffixTree"
            | "Lca"
            | "Hld"
            | "Centroid"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_cycle_path_algorithms_return_result_without_throws() {
        for name in ["bellmanFord", "spfa", "spfaInfo", "johnson", "johnsonInfo"] {
            let export = algorithm_builtins_cached()
                .get(name)
                .unwrap_or_else(|| panic!("missing `tyrus:algorithm` export `{name}`"));
            assert!(export.throws.is_empty(), "`{name}` must not declare throws");
            assert!(
                matches!(&export.ty, Type::Function { ret, .. } if matches!(ret.as_ref(), Type::Result { error, .. } if error.as_ref() == &algorithm_error())),
                "`{name}` must return Result with AlgorithmError"
            );
        }
    }
}
