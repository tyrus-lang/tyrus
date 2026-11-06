//! Native end-to-end coverage for `tyrus:algorithm` operations.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct AlgorithmProject {
    root: PathBuf,
    name: String,
}

impl AlgorithmProject {
    fn new(source: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("algorithm-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "Algorithm runtime integration test"
  }},
  "security": {{
    "envFile": ".env",
    "envSealed": false
  }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(root.join(".env"), "").unwrap();
        fs::write(root.join("src/main.tyr"), source).unwrap();
        Self { root, name }
    }

    fn build(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("build")
            .current_dir(&self.root)
            .output()
            .unwrap()
    }

    fn run(&self) -> Output {
        Command::new(self.root.join("target/debug").join(&self.name))
            .output()
            .unwrap()
    }
}

impl Drop for AlgorithmProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn graph_traversals_support_arrays_and_graphs() {
    let project = AlgorithmProject::new(
        r#"import { Graph, bfs, dfs } from "tyrus:algorithm";

function runChecks(): void {
    const adjacency: usize[][] = [[1, 2], [3], [3], []];
    const breadth: usize[] = bfs(adjacency, 0);
    console.log(breadth[0], breadth[1], breadth[2], breadth[3]);
    console.log("assert:", 0, 1, 2, 3);

    const depth: usize[] = dfs(adjacency, 0);
    console.log(depth[0], depth[1], depth[2], depth[3]);
    console.log("assert:", 0, 1, 3, 2);

    const graph = Graph<i32, i32>();
    graph.addEdge(0, 1, 1);
    graph.addEdge(0, 2, 1);
    graph.addEdge(1, 3, 1);
    const graphBreadth: usize[] = bfs(graph, 0);
    console.log(graphBreadth[0], graphBreadth[1], graphBreadth[2], graphBreadth[3]);
    console.log("assert:", 0, 1, 2, 3);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0 1 2 3\nassert: 0 1 2 3\n0 1 3 2\nassert: 0 1 3 2\n0 1 2 3\nassert: 0 1 2 3\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suffix_array_info_supports_alias_namespace_and_edge_cases() {
    let project = AlgorithmProject::new(
        r#"import { SuffixArrayInfo as Study, suffixArrayInfo as inspect } from "tyrus:algorithm";
import * as alg from "tyrus:algorithm";

function runChecks(): void {
    const banana: Study = inspect("banana");
    console.log(banana.text);
    console.log("assert:", "banana");
    console.log(banana.suffixArray);
    console.log(banana.suffixArray.length);
    console.log("assert:", 6);
    console.log(banana.suffixArray[0]);
    console.log("assert:", 5);
    console.log(banana.suffixArray[5]);
    console.log("assert:", 2);
    console.log(banana.sortRoundCount);
    console.log("assert:", 3);
    console.log(banana.comparisonCount);
    console.log("assert:", 11);
    console.log(banana.comparedByteCount);
    console.log("assert:", 14);

    const empty: Study = alg.suffixArrayInfo("");
    console.log(empty.suffixArray.length);
    console.log("assert:", 0);
    console.log(empty.sortRoundCount);
    console.log("assert:", 0);
    console.log(empty.comparisonCount);
    console.log("assert:", 0);
    console.log(empty.comparedByteCount);
    console.log("assert:", 0);

    const single: Study = inspect("a");
    console.log(single.suffixArray);
    console.log(single.suffixArray[0]);
    console.log("assert:", 0);
    console.log(single.sortRoundCount);
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "banana\nassert: banana\n[5, 3, 1, 0, 4, 2]\n6\nassert: 6\n5\nassert: 5\n2\nassert: 2\n\
         3\nassert: 3\n11\nassert: 11\n14\nassert: 14\n\
         0\nassert: 0\n0\nassert: 0\n0\nassert: 0\n0\nassert: 0\n\
         [0]\n0\nassert: 0\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn sorting_algorithms_return_ordered_arrays() {
    let project = AlgorithmProject::new(
        r#"import { mergeSort, quickSort, heapSort, countingSort, radixSort, bucketSort, timSort, insertionSort, shellSort, selectionSort, bubbleSort, shakerSort } from "tyrus:algorithm";

function check(values: i32[]): void {
    console.log(values[0], values[1], values[2], values[3], values[4], values[5]);
    console.log("assert:", -1, 0, 2, 2, 3, 4);
}

const main = (): void => {
    const values: i32[] = [4, -1, 3, 0, 2, 2];
    check(mergeSort(values));
    check(quickSort(values));
    check(heapSort(values));
    const counted: i32[] = countingSort([4, 1, 3, 0, 2, 2], 4);
    console.log(counted[0], counted[1], counted[2], counted[3], counted[4], counted[5]);
    console.log("assert:", 0, 1, 2, 2, 3, 4);
    check(radixSort(values));
    check(bucketSort(values));
    check(timSort(values));
    check(insertionSort(values));
    check(shellSort(values));
    check(selectionSort(values));
    check(bubbleSort(values));
    check(shakerSort(values));
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches("assert:").count(), 12);
    assert!(stdout.contains("assert: -1 0 2 2 3 4"));
    assert!(stdout.contains("assert: 0 1 2 2 3 4"));
    assert!(output.stderr.is_empty());
}

#[test]
fn bubble_sort_info_exposes_independent_snapshots_and_metrics() {
    let project = AlgorithmProject::new(
        r#"import { BubbleSortInfo, bubbleSortInfo } from "tyrus:algorithm";

const main = (): void => {
    const input: i32[] = [4, 1, 3, 2];
    const info: BubbleSortInfo = bubbleSortInfo(input);
    // Copied, not aliased: reading the field into a binding would hand a second
    // owner the same buffer, and writing through either would be visible in the
    // other. The spread is what makes the snapshot the name promises.
    let original: i32[] = [...info.originalValues];
    const sorted: i32[] = [...info.sortedValues];
    original[0] = 8;
    console.log(input[0], input[1], input[2], input[3]);
    console.log(original[0], original[1], original[2], original[3]);
    console.log(sorted[0], sorted[1], sorted[2], sorted[3]);
    console.log(info.comparisonCount, info.swapCount, info.passCount);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!("4 1 3 2\n", "8 1 3 2\n", "1 2 3 4\n", "6 4 3\n",)
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn bubble_sort_info_supports_aliased_and_namespace_imports() {
    for source in [
        r#"import { BubbleSortInfo as Result, bubbleSortInfo as inspect } from "tyrus:algorithm";
const main = (): void => {
    const info: Result = inspect([3, 2, 1]);
    console.log(info.comparisonCount, info.swapCount, info.passCount);
};
"#,
        r#"import * as algorithm from "tyrus:algorithm";
const main = (): void => {
    const info = algorithm.bubbleSortInfo([3, 2, 1]);
    console.log(info.comparisonCount, info.swapCount, info.passCount);
};
"#,
    ] {
        let project = AlgorithmProject::new(source);
        let build = project.build();
        assert!(
            build.status.success(),
            "build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        let output = project.run();
        assert!(
            output.status.success(),
            "program failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "3 3 2\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn sorting_study_results_return_snapshots_and_metrics() {
    let project = AlgorithmProject::new(
        r#"import { SortInfo, mergeSortInfo, quickSortInfo, heapSortInfo, countingSortInfo, radixSortInfo, bucketSortInfo, timSortInfo, insertionSortInfo, shellSortInfo, selectionSortInfo, shakerSortInfo } from "tyrus:algorithm";

function check(info: SortInfo): void {
    console.log(info.originalValues[0], info.originalValues[1], info.originalValues[2], info.originalValues[3]);
    console.log(info.sortedValues[0], info.sortedValues[1], info.sortedValues[2], info.sortedValues[3]);
}

const main = (): void => {
    const values: i32[] = [4, 1, 3, 2];
    check(mergeSortInfo(values));
    check(quickSortInfo(values));
    check(heapSortInfo(values));
    check(countingSortInfo([4, 1, 3, 2], 4));
    check(radixSortInfo(values));
    check(bucketSortInfo(values));
    check(timSortInfo(values));
    check(insertionSortInfo(values));
    check(shellSortInfo(values));
    check(selectionSortInfo(values));
    check(shakerSortInfo(values));
    const merge: SortInfo = mergeSortInfo(values);
    console.log(merge.comparisonCount, merge.swapCount);
    const insertion: SortInfo = insertionSortInfo(values);
    console.log(insertion.comparisonCount, insertion.swapCount);
    const selection: SortInfo = selectionSortInfo(values);
    console.log(selection.comparisonCount, selection.swapCount);
    const counting: SortInfo = countingSortInfo([4, 1, 3, 2], 4);
    console.log(counting.comparisonCount, counting.swapCount);
    const radix: SortInfo = radixSortInfo(values);
    console.log(radix.comparisonCount, radix.swapCount);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches("4 1 3 2").count(), 11);
    assert_eq!(stdout.matches("1 2 3 4").count(), 11);
    assert!(stdout.contains("5 0\n"), "unexpected output: {stdout}");
    assert!(stdout.contains("6 4\n"));
    assert!(stdout.contains("6 2\n"));
    assert_eq!(stdout.matches("0 0\n").count(), 2);
    assert!(output.stderr.is_empty());
}

#[test]
fn search_and_selection_algorithms_run_end_to_end() {
    let project = AlgorithmProject::new(
        r#"import { quickselect, binarySearch, interpolationSearch, exponentialSearch } from "tyrus:algorithm";

const main = (): void => {
    const values: i32[] = [1, 2, 4, 7, 9];
    const selected: i32? = quickselect(values, 2);
    console.log(selected ?? -1);
    const binary: usize? = binarySearch(values, 7);
    console.log(binary ?? 99);
    const interpolation: usize? = interpolationSearch(values, 4);
    console.log(interpolation ?? 99);
    const exponential: usize? = exponentialSearch(values, 9);
    console.log(exponential ?? 99);
    const missing: usize? = binarySearch(values, 6);
    if (missing) { console.log(99); } else { console.log(0); }
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "4\n3\n2\n4\n0\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn search_and_selection_info_results_expose_snapshots_and_metrics() {
    let project = AlgorithmProject::new(
        r#"import { SearchInfo, InterpolationSearchInfo, ExponentialSearchInfo, SelectionInfo, binarySearchInfo, interpolationSearchInfo, exponentialSearchInfo, quickselectInfo } from "tyrus:algorithm";
const main = (): void => {
    const values: i32[] = [1, 2, 4, 7, 9];
    const binary: SearchInfo = binarySearchInfo(values, 7);
    const interpolation: InterpolationSearchInfo = interpolationSearchInfo(values, 4);
    const exponential: ExponentialSearchInfo = exponentialSearchInfo(values, 9);
    const selection: SelectionInfo = quickselectInfo([7, 1, 4, 9, 2], 2);
    const missing: SearchInfo = binarySearchInfo(values, 6);
    const outside: SelectionInfo = quickselectInfo(values, 8);
    console.log(binary.values[0], binary.index ?? 99, binary.comparisonCount, binary.probeCount);
    console.log(interpolation.index ?? 99, interpolation.estimatedPositions[0]);
    console.log(exponential.index ?? 99, exponential.rangeExpansions, exponential.finalLow, exponential.finalHigh);
    console.log(selection.values[0], selection.selected ?? -1, selection.rank, selection.partitionCount);
    console.log(missing.index ?? 99, outside.selected ?? -1, outside.partitionCount);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1 3 3 3\n2 1\n4 2 2 5\n7 4 2 1\n99 -1 0\n"
    );
}

#[test]
fn traversal_info_results_expose_orders_and_metrics() {
    let project = AlgorithmProject::new(
        r#"import { Graph, TraversalInfo, IddfsInfo, bfsInfo, dfsInfo, iddfsInfo } from "tyrus:algorithm";
const main = (): void => {
    const graph: usize[][] = [[1, 2], [3], [3], []];
    const breadth: TraversalInfo = bfsInfo(graph, 0);
    const depth: TraversalInfo = dfsInfo(graph, 0);
    const deepening: IddfsInfo = iddfsInfo(graph, 0, 2);
    const handle = Graph<i32, i32>();
    handle.addEdge(0, 1, 1);
    handle.addEdge(0, 2, 1);
    handle.addEdge(1, 3, 1);
    const graphBreadth: TraversalInfo = bfsInfo(handle, 0);
    console.log(breadth.visitOrder, breadth.visitedCount, breadth.examinedEdgeCount, breadth.maxFrontierSize, breadth.maxDepth);
    console.log(depth.visitOrder, depth.visitedCount, depth.examinedEdgeCount, depth.maxFrontierSize, depth.maxDepth);
    console.log(deepening.visitOrder, deepening.depthLimitsTried, deepening.iterationCount);
    console.log(graphBreadth.visitOrder, graphBreadth.examinedEdgeCount);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn shortest_path_info_results_support_arrays_and_graphs() {
    let project = AlgorithmProject::new(
        r#"import { Graph, AlgorithmError, ShortestPathInfo, DijkstraInfo, AStarInfo, BellmanFordInfo, SpfaInfo, JohnsonInfo, FloydWarshallInfo, dijkstraInfo, aStarInfo, bellmanFordInfo, spfaInfo, johnsonInfo, floydWarshallInfo } from "tyrus:algorithm";
const run = (): Result<void, AlgorithmError> => {
    const adjacency: [usize, i32][][] = [[[1, 4], [2, 1]], [[3, 1]], [[1, 2], [3, 5]], []];
    const dijkstra: DijkstraInfo = dijkstraInfo(adjacency, 0, 3);
    const star: AStarInfo = aStarInfo(adjacency, 0, 3, [3, 1, 2, 0]);
    const bellman: BellmanFordInfo = bellmanFordInfo(adjacency, 0);
    const queue: SpfaInfo = spfaInfo(adjacency, 0)?;
    const johnson: JohnsonInfo = johnsonInfo(adjacency)?;
    const graph = Graph<i32, i32>();
    graph.addEdge(0, 1, 4);
    graph.addEdge(0, 2, 1);
    graph.addEdge(1, 3, 1);
    graph.addEdge(2, 1, 2);
    graph.addEdge(2, 3, 5);
    const floyd: FloydWarshallInfo = floydWarshallInfo(graph);
    const graphResult: DijkstraInfo = dijkstraInfo(graph, 0, 3);
    const cycle: [usize, i32][][] = [[[1, 1]], [[2, -2]], [[1, -2]]];
    const negative: BellmanFordInfo = bellmanFordInfo(cycle, 0);
    console.log(dijkstra.distances, dijkstra.relaxationCount, dijkstra.heapExtractionCount, dijkstra.finalizedVertexCount);
    console.log(star.finalCost ?? -1, star.heuristicEvaluationCount, star.openedVertexCount, star.closedVertexCount);
    console.log(bellman.distances, bellman.passCount, bellman.negativeCycleDetected, bellman.negativeCycleEdge ? 1 : 0);
    console.log(queue.queuePushCount, queue.queuePopCount, queue.maxQueueSize, queue.examinedEdgeCount);
    console.log(johnson.distances[0][0] ?? -1, johnson.distances[0][1] ?? -1, johnson.distances[0][2] ?? -1, johnson.distances[0][3] ?? -1, johnson.reweightingCount, johnson.bellmanFordRelaxationCount, johnson.dijkstraExecutionCount);
    console.log(floyd.distances[0][0] ?? -1, floyd.distances[0][1] ?? -1, floyd.distances[0][2] ?? -1, floyd.distances[0][3] ?? -1, floyd.kPassCount, floyd.distanceUpdateCount, floyd.negativeCycleDetected);
    console.log(graphResult.distances, dijkstra.processedVertexCount);
    console.log(negative.negativeCycleDetected);
    const cycleEdge = negative.negativeCycleEdge;
    if (cycleEdge) {
        console.log(cycleEdge[0], cycleEdge[1], cycleEdge[2]);
    }
    return Ok();
};
const main = (): void => {
    match (run()) {
        Ok() => {}
        Err(error) => { console.log(error.code); }
    }
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[0, 3, 1, 4] 5 5 4\n4 6 4 4\n[0, 3, 1, 4] 3 false 0\n6 6 2 6\n0 3 1 4 5 4 4\n0 3 1 4 4 4 false\n[0, 3, 1, 4] 4\ntrue\n1 2 -2\n"
    );
}

#[test]
fn string_search_algorithms_run_end_to_end() {
    let project = AlgorithmProject::new(
        r#"import { ahoCorasick, boyerMoore, kmpSearch, levenshtein, rabinKarp, zAlgorithm } from "tyrus:algorithm";

const main = (): void => {
    const text: string = "ababa";
    const kmp: usize[] = kmpSearch(text, "aba");
    console.log(kmp[0], kmp[1]);
    const rabin: usize[] = rabinKarp(text, "ba");
    console.log(rabin[0], rabin[1]);
    const boyer: usize[] = boyerMoore(text, "aba");
    console.log(boyer[0], boyer[1]);
    const z: usize[] = zAlgorithm("aabcaab");
    console.log(z[1], z[4]);
    const patterns: string[] = ["aba", "ba"];
    const matches: [usize, usize][] = ahoCorasick(text, patterns);
    console.log(matches[0][0], matches[0][1], matches[3][0], matches[3][1]);
    console.log(levenshtein("kitten", "sitting"));
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0 2\n1 3\n0 2\n1 3\n2 0 4 1\n3\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn graph_path_algorithms_run_end_to_end() {
    let project = AlgorithmProject::new(
        r#"import {
    Graph,
    iddfs,
    dijkstra,
    aStar,
    bellmanFord,
    floydWarshall,
    spfa,
    johnson,
    AlgorithmError,
} from "tyrus:algorithm";

function runChecks(): void {
    let unweighted: usize[][] = [[1], [2], [3], []];
    let limited: usize[] = iddfs(unweighted, 0, 2);
    console.log(limited.length, limited[0], limited[2]);
    console.log("assert:", 3, 0, 2);

    let weighted = Graph<i32, i32>();
    weighted.addEdge(0, 1, 4);
    weighted.addEdge(0, 2, 2);
    weighted.addEdge(1, 3, 5);
    weighted.addEdge(2, 3, 1);
    let shortest: [usize[], i32]? = dijkstra(weighted, 0, 3);
    if (shortest) {
        console.log(shortest[1]);
    } else {
        console.log(-1);
    }
    console.log("assert:", 3);
    let heuristic: i32[] = [3, 1, 1, 0];
    let aStarResult: [usize[], i32]? = aStar(weighted, 0, 3, heuristic);
    if (aStarResult) {
        console.log(aStarResult[1]);
    } else {
        console.log(-1);
    }
    console.log("assert:", 3);

    let negative = Graph<i32, i32>();
    negative.addEdge(0, 1, 2);
    negative.addEdge(1, 2, -1);
    match (bellmanFord(negative, 0)) {
        Ok(bellman) => {
            console.log(bellman[0][2]);
            console.log("assert:", 1);
        }
        Err(error) => { console.log(error.code); }
    }
    match (spfa(negative, 0)) {
        Ok(spfaResult) => {
            console.log(spfaResult[0][2]);
            console.log("assert:", 1);
        }
        Err(error) => { console.log(error.code); }
    }

    let allPairs: Array<Array<i32?>> = floydWarshall(weighted);
    console.log(allPairs[0][3] ?? -1, allPairs[3][0] ?? -1);
    console.log("assert:", 3, -1);
    match (johnson(weighted)) {
        Ok(johnsonResult) => {
            console.log(johnsonResult[0][3] ?? -1);
            console.log("assert:", 3);
        }
        Err(error) => { console.log(error.code); }
    }

    let graphPath: [usize[], i32]? = dijkstra(weighted, 0, 3);
    if (graphPath) {
        console.log(graphPath[1]);
    } else {
        console.log(-1);
    }
    console.log("assert:", 3);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3 0 2\nassert: 3 0 2\n3\nassert: 3\n3\nassert: 3\n1\nassert: 1\n1\nassert: 1\n3 -1\nassert: 3 -1\n3\nassert: 3\n3\nassert: 3\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn minimum_spanning_tree_algorithms_run_end_to_end() {
    let project = AlgorithmProject::new(
        r#"import { boruvka, kruskal, prim, steinerTree } from "tyrus:algorithm";

function runChecks(): void {
    const edges: [usize, usize, i32][] = [[0, 1, 4], [0, 2, 2], [1, 2, 1], [1, 3, 5], [2, 3, 8], [2, 4, 10], [3, 4, 2]];
    const kruskalTree: [usize, usize, i32][] = kruskal(edges, 5);
    console.log(kruskalTree.length, kruskalTree[0][2] + kruskalTree[1][2] + kruskalTree[2][2] + kruskalTree[3][2]);
    console.log("assert:", 4, 10);

    const boruvkaTree: [usize, usize, i32][] = boruvka(edges, 5);
    console.log(boruvkaTree.length, boruvkaTree[0][2] + boruvkaTree[1][2] + boruvkaTree[2][2] + boruvkaTree[3][2]);
    console.log("assert:", 4, 10);

    const adjacency: [usize, i32][][] = [[ [1, 4], [2, 2] ], [ [0, 4], [2, 1], [3, 5] ], [ [0, 2], [1, 1], [3, 8], [4, 10] ], [ [1, 5], [2, 8], [4, 2] ], [ [2, 10], [3, 2] ]];
    const primTree: [usize, usize, i32][] = prim(adjacency, 0);
    console.log(primTree.length, primTree[0][2] + primTree[1][2] + primTree[2][2] + primTree[3][2]);
    console.log("assert:", 4, 10);

    const terminals: usize[] = [0, 4];
    const steiner: [usize, usize, i32][] = steinerTree(edges, 5, terminals);
    console.log(steiner.length);
    console.log("assert:", 4);
}
const main = (): void => {
    runChecks();
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "4 10\nassert: 4 10\n4 10\nassert: 4 10\n4 10\nassert: 4 10\n4\nassert: 4\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn spanning_tree_info_results_expose_construction_metrics() {
    let project = AlgorithmProject::new(
        r#"import { Graph, SpanningTreeInfo, KruskalInfo, PrimInfo, BoruvkaInfo, SteinerTreeInfo, kruskalInfo, primInfo, boruvkaInfo, steinerTreeInfo } from "tyrus:algorithm";
const main = (): void => {
    const edges: [usize, usize, i32][] = [[0, 1, 4], [0, 2, 2], [1, 2, 1], [1, 3, 5], [2, 3, 8], [2, 4, 10], [3, 4, 2]];
    const kruskal: KruskalInfo = kruskalInfo(edges, 5);
    const adjacency: [usize, i32][][] = [[[1, 4], [2, 2]], [[0, 4], [2, 1], [3, 5]], [[0, 2], [1, 1], [3, 8], [4, 10]], [[1, 5], [2, 8], [4, 2]], [[2, 10], [3, 2]]];
    const prim: PrimInfo = primInfo(adjacency, 0);
    const boruvka: BoruvkaInfo = boruvkaInfo(edges, 5);
    const steiner: SteinerTreeInfo = steinerTreeInfo(edges, 5, [0, 4]);
    const graph = Graph<i32, i32>();
    graph.addEdge(0, 1, 4); graph.addEdge(1, 0, 4); graph.addEdge(0, 2, 2); graph.addEdge(2, 0, 2); graph.addEdge(1, 2, 1); graph.addEdge(2, 1, 1); graph.addEdge(1, 3, 5); graph.addEdge(3, 1, 5); graph.addEdge(2, 3, 8); graph.addEdge(3, 2, 8); graph.addEdge(2, 4, 10); graph.addEdge(4, 2, 10); graph.addEdge(3, 4, 2); graph.addEdge(4, 3, 2);
    const graphPrim: PrimInfo = primInfo(graph, 0);
    console.log(kruskal.edges.length, kruskal.totalWeight, kruskal.componentCount, kruskal.examinedEdgeCount, kruskal.acceptedEdgeCount, kruskal.unionAttemptCount, kruskal.successfulUnionCount, kruskal.sortedEdgeExaminationCount);
    console.log(prim.edges.length, prim.totalWeight, prim.componentCount, prim.examinedEdgeCount, prim.acceptedEdgeCount, prim.heapPushCount, prim.heapPopCount, prim.keyUpdateCount);
    console.log(boruvka.edges.length, boruvka.totalWeight, boruvka.componentCount, boruvka.examinedEdgeCount, boruvka.acceptedEdgeCount, boruvka.phaseCount, boruvka.componentCountsAfterPhase);
    console.log(steiner.edges.length, steiner.totalWeight, steiner.terminals, steiner.evaluatedSubsetCount, steiner.vertexCount, steiner.maxTerminalCount, steiner.withinTerminalLimit);
    console.log(graphPrim.edges.length, graphPrim.totalWeight);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "4 10 1 5 4 5 4 5\n4 10 1 12 4 7 7 7\n4 10 1 14 4 2 [2, 1]\n4 10 [0, 4] 3 5 20 true\n4 10\n"
    );
}

#[test]
fn flow_and_cut_algorithms_run_end_to_end() {
    let project = AlgorithmProject::new(
        r#"import { dinic, edmondsKarp, minCostMaxFlow, minCut, pushRelabel } from "tyrus:algorithm";

function runChecks(): void {
    const capacity: i32[][] = [[0, 3, 2, 0], [0, 0, 1, 2], [0, 0, 0, 3], [0, 0, 0, 0]];
    const ek: [i32, [usize, usize, i32][]] = edmondsKarp(capacity, 0, 3);
    console.log(ek[0]);
    console.log("assert:", 5);
    const di: [i32, [usize, usize, i32][]] = dinic(capacity, 0, 3);
    console.log(di[0]);
    console.log("assert:", 5);
    const pr: [i32, [usize, usize, i32][]] = pushRelabel(capacity, 0, 3);
    console.log(pr[0]);
    console.log("assert:", 5);
    const cut: [i32, [usize, usize][]] = minCut(capacity, 0, 3);
    console.log(cut[0], cut[1].length);
    console.log("assert:", 5, 2);

    const network: [usize, usize, i32, i32][] = [[0, 1, 2, 1], [0, 2, 1, 2], [1, 2, 1, 1], [1, 3, 1, 3], [2, 3, 2, 1]];
    const minCost: [i32, i32] = minCostMaxFlow(network, 4, 0, 3);
    console.log(minCost[0], minCost[1]);
    console.log("assert:", 3, 10);
}

const main = (): void => {
    runChecks();
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "5\nassert: 5\n5\nassert: 5\n5\nassert: 5\n5 2\nassert: 5 2\n3 10\nassert: 3 10\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn flow_info_results_expose_snapshots_and_metrics() {
    let project = AlgorithmProject::new(
        r#"import { FlowInfo, EdmondsKarpInfo, DinicInfo, PushRelabelInfo, MinCutInfo, MinCostMaxFlowInfo, edmondsKarpInfo, dinicInfo, pushRelabelInfo, minCutInfo, minCostMaxFlowInfo } from "tyrus:algorithm";
const main = (): void => {
    const capacity: i32[][] = [[0, 3, 2, 0], [0, 0, 1, 2], [0, 0, 0, 3], [0, 0, 0, 0]];
    const ek: EdmondsKarpInfo = edmondsKarpInfo(capacity, 0, 3);
    const di: DinicInfo = dinicInfo(capacity, 0, 3);
    const pr: PushRelabelInfo = pushRelabelInfo(capacity, 0, 3);
    const cut: MinCutInfo = minCutInfo(capacity, 0, 3);
    const cost: MinCostMaxFlowInfo = minCostMaxFlowInfo([[0, 1, 2, 1], [0, 2, 1, 2], [1, 2, 1, 1], [1, 3, 1, 3], [2, 3, 2, 1]], 4, 0, 3);
    console.log(ek.maxFlow, ek.flowEdges.length, ek.bfsRunCount, ek.augmentingPathLengths.length, ek.maxFrontierSize);
    console.log(di.maxFlow, di.flowEdges.length, di.levelGraphCount, di.blockingFlowCount, di.currentArcAdvanceCount);
    console.log(pr.maxFlow, pr.flowEdges.length, pr.pushCount, pr.relabelCount, pr.dischargeCount, pr.maxHeight);
    console.log(cut.cutValue, cut.sourcePartition, cut.sinkPartition, cut.cutEdges.length, cut.flowEdges.length);
    console.log(cost.maxFlow, cost.minimumCost, cost.augmentationCount, cost.costRelaxationCount, cost.cumulativeCosts);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "5 5 6 1 4\n5 5 6 5 16\n5 5 5 4 5 4\n5 [0] [1, 2, 3] 2 5\n3 10 5 20 [10]\n"
    );
}

#[test]
fn topological_sort_info_results_expose_metrics() {
    let project = AlgorithmProject::new(
        r#"import { TopologicalSortInfo, TopologicalSortKahnInfo, TopologicalSortDfsInfo, topologicalSortKahnInfo, topologicalSortDfsInfo } from "tyrus:algorithm";
const main = (): void => {
    const dag: usize[][] = [[1, 2], [3], [3], []];
    const kahn: TopologicalSortKahnInfo = topologicalSortKahnInfo(dag);
    const dfs: TopologicalSortDfsInfo = topologicalSortDfsInfo(dag);
    const cyclicGraph: usize[][] = [[1], [0]];
    const cycle: TopologicalSortDfsInfo = topologicalSortDfsInfo(cyclicGraph);
    console.log(kahn.order, kahn.processedVertexCount, kahn.examinedEdgeCount, kahn.cycleDetected, kahn.initialIndegrees, kahn.indegreeDecrementCount, kahn.maxQueueSize);
    console.log(dfs.order, dfs.processedVertexCount, dfs.examinedEdgeCount, dfs.cycleDetected, dfs.finalStates, dfs.maxDepth, dfs.backEdge ?? [9usize, 9usize]);
    console.log(cycle.order, cycle.processedVertexCount, cycle.examinedEdgeCount, cycle.cycleDetected, cycle.finalStates, cycle.maxDepth, cycle.backEdge ?? [9usize, 9usize]);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[0, 1, 2, 3] 4 4 false [0, 1, 1, 2] 4 2\n[0, 2, 1, 3] 4 4 false [2, 2, 2, 2] 3 [9, 9]\n[] 0 2 true [1, 1] 2 [1, 0]\n"
    );
}

#[test]
fn scc_info_results_expose_components_and_metrics() {
    let project = AlgorithmProject::new(
        r#"import { SccInfo, TarjanSccInfo, KosarajuSccInfo, tarjanSccInfo, kosarajuSccInfo } from "tyrus:algorithm";
const main = (): void => { const graph: usize[][] = [[1], [2], [0, 3], [4], [3]]; const tarjan: TarjanSccInfo = tarjanSccInfo(graph); const kosaraju: KosarajuSccInfo = kosarajuSccInfo(graph); console.log(tarjan.componentCount, tarjan.examinedEdgeCount, tarjan.stackPushCount, tarjan.stackPopCount); console.log(kosaraju.componentCount, kosaraju.examinedEdgeCount, kosaraju.firstTraversalEdgeCount, kosaraju.secondTraversalEdgeCount); };
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "2 6 5 5\n2 6 6 6\n"
    );
}

#[test]
fn eulerian_info_results_support_arrays_and_graphs() {
    let project = AlgorithmProject::new(
        r#"import { EulerianInfo, FleuryEulerianInfo, HierholzerEulerianInfo, Graph, fleuryEulerianInfo, hierholzerEulerianInfo } from "tyrus:algorithm";
const main = (): void => {
    const adjacency: usize[][] = [[1, 2], [0, 2], [0, 1]];
    const fleury: FleuryEulerianInfo = fleuryEulerianInfo(adjacency);
    const hierholzer: HierholzerEulerianInfo = hierholzerEulerianInfo(adjacency);
    console.log(fleury.path ?? [9usize], fleury.usedEdgeCount, fleury.examinedEdgeCount, fleury.isCircuit, fleury.bridgeCheckCount, fleury.avoidedBridgeChoiceCount);
    console.log(hierholzer.path ?? [9usize], hierholzer.usedEdgeCount, hierholzer.examinedEdgeCount, hierholzer.isCircuit, hierholzer.stackPushCount, hierholzer.stackPopCount, hierholzer.joinedCycleCount);

    const graph = Graph<i32, i32>();
    graph.addEdge(0, 1, 1); graph.addEdge(1, 0, 1);
    graph.addEdge(1, 2, 1); graph.addEdge(2, 1, 1);
    graph.addEdge(2, 0, 1); graph.addEdge(0, 2, 1);
    const graphFleury: FleuryEulerianInfo = fleuryEulerianInfo(graph);
    const graphHierholzer: HierholzerEulerianInfo = hierholzerEulerianInfo(graph);
    console.log(graphFleury.path ?? [9usize], graphFleury.usedEdgeCount, graphFleury.examinedEdgeCount, graphFleury.isCircuit, graphFleury.bridgeCheckCount, graphFleury.avoidedBridgeChoiceCount);
    console.log(graphHierholzer.path ?? [9usize], graphHierholzer.usedEdgeCount, graphHierholzer.examinedEdgeCount, graphHierholzer.isCircuit, graphHierholzer.stackPushCount, graphHierholzer.stackPopCount, graphHierholzer.joinedCycleCount);
    const bridgeGraph: usize[][] = [[3, 1, 2], [0, 2], [1, 0], [0]];
    const bridgeChoice: FleuryEulerianInfo = fleuryEulerianInfo(bridgeGraph);
    console.log(bridgeChoice.path ?? [9usize], bridgeChoice.usedEdgeCount, bridgeChoice.examinedEdgeCount, bridgeChoice.isCircuit, bridgeChoice.bridgeCheckCount, bridgeChoice.avoidedBridgeChoiceCount);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[0, 1, 2, 0] 3 3 true 1 0\n[0, 2, 1, 0] 3 6 true 7 7 1\n[0, 1, 2, 0] 3 3 true 1 0\n[0, 2, 1, 0] 3 6 true 7 7 1\n[0, 1, 2, 0, 3] 4 5 false 2 1\n"
    );
}

#[test]
fn connectivity_info_results_support_arrays_and_graphs() {
    let project = AlgorithmProject::new(
        r#"import { BiconnectedComponentsInfo, ConnectivityInfo, FindBridgesInfo, FindCutVerticesInfo, Graph, biconnectedComponentsInfo, findBridgesInfo, findCutVerticesInfo } from "tyrus:algorithm";
const main = (): void => {
    const adjacency: usize[][] = [[1, 2], [0, 2, 3], [0, 1], [1]];
    const bridges: FindBridgesInfo = findBridgesInfo(adjacency);
    const cuts: FindCutVerticesInfo = findCutVerticesInfo(adjacency);
    const components: BiconnectedComponentsInfo = biconnectedComponentsInfo(adjacency);
    console.log(bridges.bridges, bridges.examinedEdgeCount, bridges.discoveryIndices, bridges.lowLinks, bridges.resultCount);
    console.log(cuts.cutVertices, cuts.examinedEdgeCount, cuts.discoveryIndices, cuts.lowLinks, cuts.resultCount);
    console.log(components.components.length, components.examinedEdgeCount, components.discoveryIndices, components.lowLinks, components.resultCount);
    const graph = Graph<i32, i32>();
    graph.addEdge(0, 1, 1); graph.addEdge(1, 0, 1); graph.addEdge(1, 2, 1); graph.addEdge(2, 1, 1); graph.addEdge(2, 0, 1); graph.addEdge(0, 2, 1); graph.addEdge(1, 3, 1); graph.addEdge(3, 1, 1);
    const graphBridges: FindBridgesInfo = findBridgesInfo(graph);
    console.log(graphBridges.bridges, graphBridges.examinedEdgeCount, graphBridges.resultCount);
};"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[[1, 3]] 8 [0, 1, 2, 3] [0, 0, 0, 3] 1\n[1] 8 [0, 1, 2, 3] [0, 0, 0, 3] 1\n2 8 [0, 1, 2, 3] [0, 0, 0, 3] 2\n[[1, 3]] 8 1\n"
    );
}

#[test]
fn matching_info_results_support_blossom_and_hungarian() {
    let project = AlgorithmProject::new(
        r#"import { EdmondsBlossomInfo, Graph, HungarianInfo, MatchingInfo, edmondsBlossomInfo, hungarianInfo } from "tyrus:algorithm";
const main = (): void => {
    const graph: usize[][] = [[1], [0], [3], [2]];
    const blossom: EdmondsBlossomInfo = edmondsBlossomInfo(graph);
    const hungarian: HungarianInfo = hungarianInfo([[4, 1], [2, 3]]);
    console.log(blossom.pairs, blossom.cardinality, blossom.examinedEdgeCount, blossom.blossomContractionCount, blossom.augmentingPathSearchCount, blossom.augmentationCount);
    console.log(hungarian.assignments, hungarian.totalCost, hungarian.potentialAdjustmentCount, hungarian.matrixIterationCount);
    const graphInput = Graph<i32, i32>(); graphInput.addEdge(0, 1, 1); graphInput.addEdge(1, 0, 1); graphInput.addEdge(2, 3, 1); graphInput.addEdge(3, 2, 1);
    const graphBlossom: EdmondsBlossomInfo = edmondsBlossomInfo(graphInput);
    console.log(graphBlossom.pairs, graphBlossom.cardinality, graphBlossom.examinedEdgeCount);
};"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[[0, 1], [2, 3]] 2 4 0 4 2\n[[1, 0], [0, 1]] 3 4 2\n[[0, 1], [2, 3]] 2 4\n"
    );
}

#[test]
fn kmp_search_info_exposes_snapshots_and_metrics() {
    let project = AlgorithmProject::new(
        r#"import { KmpSearchInfo, StringSearchInfo, kmpSearchInfo } from "tyrus:algorithm";
const main = (): void => { const info: KmpSearchInfo = kmpSearchInfo("ababa", "aba"); console.log(info.text, info.pattern, info.matches, info.comparisonCount, info.shiftCount, info.prefixTable, info.prefixFallbackCount); };"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ababa aba [0, 2] 7 5 [0, 0, 1] 0\n"
    );
}

#[test]
fn z_algorithm_info_exposes_table_and_metrics() {
    let project = AlgorithmProject::new(
        r#"import { ZAlgorithmInfo, zAlgorithmInfo } from "tyrus:algorithm";
const main = (): void => { const info: ZAlgorithmInfo = zAlgorithmInfo("aabcaab"); console.log(info.text, info.zTable, info.comparisonCount, info.windowExtensionCount); };"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "aabcaab [0, 1, 0, 0, 3, 1, 0] 9 4\n"
    );
}

#[test]
fn rabin_karp_info_exposes_windows_and_verifications() {
    let project = AlgorithmProject::new(
        r#"import { RabinKarpInfo, StringSearchInfo, rabinKarpInfo } from "tyrus:algorithm";
const main = (): void => { const info: RabinKarpInfo = rabinKarpInfo("ababa", "ba"); console.log(info.text, info.pattern, info.matches, info.comparisonCount, info.shiftCount, info.windowCount, info.hashCollisionCount, info.textVerificationCount); };"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ababa ba [1, 3] 8 3 4 0 2\n"
    );
}

#[test]
fn graph_structure_and_matching_algorithms_run_end_to_end() {
    let project = AlgorithmProject::new(
        r#"import { Graph, biconnectedComponents, dinic, edmondsBlossom, findBridges, findCutVertices, fleuryEulerian, hungarian, hierholzerEulerian, kosarajuScc, tarjanScc, topologicalSortDfs, topologicalSortKahn } from "tyrus:algorithm";

function runChecks(): void {
    const dag: usize[][] = [[1, 2], [3], [3], []];
    const kahn: usize[]? = topologicalSortKahn(dag);
    const dfs: usize[]? = topologicalSortDfs(dag);
    let kahnPresent = false;
    let dfsPresent = false;
    if (kahn) { kahnPresent = true; }
    if (dfs) { dfsPresent = true; }
    console.log(kahnPresent, dfsPresent);
    console.log("assert:", true, true);

    const directed: usize[][] = [[1], [2], [0, 3], [4], [3]];
    const tarjan: usize[][] = tarjanScc(directed);
    const kosaraju: usize[][] = kosarajuScc(directed);
    console.log(tarjan.length, kosaraju.length);
    console.log("assert:", 2, 2);

    const undirected: usize[][] = [[1, 2], [0, 2, 3], [0, 1], [1]];
    const bridges: [usize, usize][] = findBridges(undirected);
    const cuts: usize[] = findCutVertices(undirected);
    const blocks: [usize, usize][][] = biconnectedComponents(undirected);
    console.log(bridges.length, cuts.length, blocks.length);
    console.log("assert:", 1, 1, 2);

    const cycle: usize[][] = [[1, 2], [0, 2], [0, 1]];
    const fleury: usize[]? = fleuryEulerian(cycle);
    const hierholzer: usize[]? = hierholzerEulerian(cycle);
    let fleuryPresent = false;
    let hierholzerPresent = false;
    if (fleury) { fleuryPresent = true; }
    if (hierholzer) { hierholzerPresent = true; }
    console.log(fleuryPresent, hierholzerPresent);
    console.log("assert:", true, true);

    const matchingGraph: usize[][] = [[1, 2], [0, 2], [0, 1, 3], [2]];
    const matching: [usize, usize][] = edmondsBlossom(matchingGraph);
    console.log(matching.length);
    console.log("assert:", 2);

    const assignment: [i32, [usize, usize][]] = hungarian([[4, 1], [2, 3]]);
    console.log(assignment[0], assignment[1].length);
    console.log("assert:", 3, 2);

    const graph = Graph<i32, i32>();
    graph.addEdge(0, 1, 1);
    graph.addEdge(1, 2, 1);
    graph.addEdge(2, 3, 1);
    const graphScc: usize[][] = tarjanScc(graph);
    console.log(graphScc.length);
    console.log("assert:", 4);
}

const main = (): void => {
    runChecks();
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true true\nassert: true true\n2 2\nassert: 2 2\n1 1 2\nassert: 1 1 2\ntrue true\nassert: true true\n2\nassert: 2\n3 2\nassert: 3 2\n4\nassert: 4\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── UnionFind ───

#[test]
fn union_find_basic_connectivity() {
    let project = AlgorithmProject::new(
        r#"import { UnionFind } from "tyrus:algorithm";

function runChecks(): void {
    const uf = UnionFind(10);
    console.log(uf.setCount());
    console.log("assert:", 10);

    console.log(uf.union(0, 1));
    console.log("assert:", true);
    console.log(uf.connected(0, 1));
    console.log("assert:", true);
    console.log(uf.connected(0, 2));
    console.log("assert:", false);

    console.log(uf.union(2, 3));
    console.log("assert:", true);
    console.log(uf.connected(2, 3));
    console.log("assert:", true);
    console.log(uf.connected(0, 2));
    console.log("assert:", false);

    console.log(uf.union(1, 3));
    console.log("assert:", true);
    console.log(uf.connected(0, 2));
    console.log("assert:", true);
    console.log(uf.connected(1, 3));
    console.log("assert:", true);

    console.log(uf.setCount());
    console.log("assert:", 7);

    console.log(uf.union(0, 2));
    console.log("assert:", false);
    console.log(uf.setCount());
    console.log("assert:", 7);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "10\nassert: 10\n\
         true\nassert: true\ntrue\nassert: true\nfalse\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\nfalse\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\ntrue\nassert: true\n\
         7\nassert: 7\nfalse\nassert: false\n7\nassert: 7\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn union_find_stress_repeated_union() {
    let project = AlgorithmProject::new(
        r#"import { UnionFind } from "tyrus:algorithm";

function runChecks(): void {
    const uf = UnionFind(100);
    for (let i: usize = 0; i < 99; i++) {
        uf.union(i, i + 1);
    }
    console.log(uf.setCount());
    console.log("assert:", 1);

    console.log(uf.connected(0, 99));
    console.log("assert:", true);
    console.log(uf.connected(50, 51));
    console.log("assert:", true);
    console.log(uf.find(0) === uf.find(99));
    console.log("assert:", true);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1\nassert: 1\n\
         true\nassert: true\ntrue\nassert: true\ntrue\nassert: true\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── FenwickTree ───

#[test]
fn fenwick_tree_prefix_sums() {
    let project = AlgorithmProject::new(
        r#"import { FenwickTree } from "tyrus:algorithm";

function runChecks(): void {
    const bit = FenwickTree(6);

    console.log(bit.isEmpty());
    console.log("assert:", false);
    console.log(bit.len());
    console.log("assert:", 6);
    console.log(bit.sum(0));
    console.log("assert:", 0);

    bit.add(0, 1);
    bit.add(1, 2);
    bit.add(2, 3);
    bit.add(3, 4);
    bit.add(4, 5);
    bit.add(5, 6);

    console.log(bit.sum(1));
    console.log("assert:", 1);
    console.log(bit.sum(2));
    console.log("assert:", 3);
    console.log(bit.sum(3));
    console.log("assert:", 6);
    console.log(bit.sum(6));
    console.log("assert:", 21);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "false\nassert: false\n6\nassert: 6\n0\nassert: 0\n1\nassert: 1\n\
         3\nassert: 3\n6\nassert: 6\n21\nassert: 21\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn avl_tree_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { AvlTree } from "tyrus:algorithm";

function runChecks(): void {
    const t = AvlTree<i32>();
    console.log(t.isEmpty());
    console.log("assert:", true);

    t.insert(5);
    t.insert(3);
    t.insert(8);
    t.insert(1);
    t.insert(4);
    t.insert(7);
    t.insert(9);
    console.log(t.len());
    console.log("assert:", 7);

    console.log(t.contains(4));
    console.log("assert:", true);
    console.log(t.contains(6));
    console.log("assert:", false);

    // Duplicates are ignored.
    t.insert(4);
    console.log(t.len());
    console.log("assert:", 7);
    console.log(t.contains(1));
    console.log("assert:", true);
    console.log(t.contains(9));
    console.log("assert:", true);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n7\nassert: 7\ntrue\nassert: true\nfalse\nassert: false\n\
         7\nassert: 7\ntrue\nassert: true\ntrue\nassert: true\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn avl_tree_string_elements() {
    let project = AlgorithmProject::new(
        r#"import { AvlTree } from "tyrus:algorithm";

function runChecks(): void {
    const t = AvlTree<string>();
    t.insert("banana");
    t.insert("apple");
    t.insert("cherry");
    t.insert("apple");
    console.log(t.len());
    console.log("assert:", 3);
    console.log(t.contains("apple"));
    console.log("assert:", true);
    console.log(t.contains("grape"));
    console.log("assert:", false);
    t.insert("grape");
    console.log(t.contains("grape"));
    console.log("assert:", true);
    console.log(t.len());
    console.log("assert:", 4);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\ntrue\nassert: true\nfalse\nassert: false\n\
         true\nassert: true\n4\nassert: 4\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn avl_tree_boolean_and_float_elements() {
    let project = AlgorithmProject::new(
        r#"import { AvlTree } from "tyrus:algorithm";

function runChecks(): void {
    const b = AvlTree<boolean>();
    b.insert(true);
    b.insert(false);
    b.insert(true);
    console.log(b.contains(false));
    console.log("assert:", true);
    console.log(b.len());
    console.log("assert:", 2);

    const f = AvlTree<f64>();
    f.insert(1.5);
    f.insert(2.5);
    f.insert(0.5);
    console.log(f.contains(2.5));
    console.log("assert:", true);
    console.log(f.len());
    console.log("assert:", 3);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n2\nassert: 2\n\
         true\nassert: true\n3\nassert: 3\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── SortedSet ───

#[test]
fn sorted_set_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { SortedSet } from "tyrus:algorithm";

function runChecks(): void {
    const s = SortedSet<i32>();
    console.log(s.isEmpty());
    console.log("assert:", true);

    s.insert(3);
    s.insert(1);
    s.insert(2);
    s.insert(2);
    console.log(s.len());
    console.log("assert:", 3);
    console.log(s.contains(2));
    console.log("assert:", true);
    console.log(s.contains(9));
    console.log("assert:", false);

    console.log(s.min() ?? 0);
    console.log("assert:", 1);
    console.log(s.max() ?? 0);
    console.log("assert:", 3);

    s.remove(2);
    console.log(s.contains(2));
    console.log("assert:", false);
    console.log(s.len());
    console.log("assert:", 2);

    s.clear();
    console.log(s.isEmpty());
    console.log("assert:", true);
    console.log(s.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n\
         3\nassert: 3\ntrue\nassert: true\nfalse\nassert: false\n\
         1\nassert: 1\n3\nassert: 3\n\
         false\nassert: false\n2\nassert: 2\n\
         true\nassert: true\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn sorted_set_string_elements() {
    let project = AlgorithmProject::new(
        r#"import { SortedSet } from "tyrus:algorithm";

function runChecks(): void {
    const s = SortedSet<string>();
    s.insert("banana");
    s.insert("apple");
    s.insert("cherry");
    s.insert("apple");
    console.log(s.len());
    console.log("assert:", 3);
    console.log(s.contains("apple"));
    console.log("assert:", true);
    console.log(s.min() ?? "none");
    console.log("assert:", "apple");
    console.log(s.max() ?? "none");
    console.log("assert:", "cherry");
    s.remove("apple");
    console.log(s.contains("apple"));
    console.log("assert:", false);
    console.log(s.len());
    console.log("assert:", 2);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\ntrue\nassert: true\n\
         apple\nassert: apple\ncherry\nassert: cherry\n\
         false\nassert: false\n2\nassert: 2\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── SortedMap ───

#[test]
fn sorted_map_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { SortedMap } from "tyrus:algorithm";

function runChecks(): void {
    const m = SortedMap<i32, string>();
    console.log(m.isEmpty());
    console.log("assert:", true);

    m.set(3, "three");
    m.set(1, "one");
    m.set(2, "two");
    console.log(m.get(2) ?? "none");
    console.log("assert:", "two");
    console.log(m.get(9) ?? "none");
    console.log("assert:", "none");
    console.log(m.has(1));
    console.log("assert:", true);
    console.log(m.min() ?? 0);
    console.log("assert:", 1);
    console.log(m.max() ?? 0);
    console.log("assert:", 3);

    m.delete(1);
    console.log(m.has(1));
    console.log("assert:", false);
    console.log(m.len());
    console.log("assert:", 2);

    m.clear();
    console.log(m.isEmpty());
    console.log("assert:", true);
    console.log(m.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n\
         two\nassert: two\nnone\nassert: none\n\
         true\nassert: true\n1\nassert: 1\n3\nassert: 3\n\
         false\nassert: false\n2\nassert: 2\n\
         true\nassert: true\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn sorted_map_string_keys_and_values() {
    let project = AlgorithmProject::new(
        r#"import { SortedMap } from "tyrus:algorithm";

function runChecks(): void {
    const m = SortedMap<string, i32>();
    m.set("x", 1);
    m.set("a", 2);
    m.set("m", 3);
    console.log(m.get("a") ?? 0);
    console.log("assert:", 2);
    console.log(m.has("z"));
    console.log("assert:", false);
    console.log(m.min() ?? "none");
    console.log("assert:", "a");
    console.log(m.max() ?? "none");
    console.log("assert:", "x");
    console.log(m.len());
    console.log("assert:", 3);
    m.set("a", 5);
    console.log(m.get("a") ?? 0);
    console.log("assert:", 5);
    console.log(m.len());
    console.log("assert:", 3);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "2\nassert: 2\nfalse\nassert: false\n\
         a\nassert: a\nx\nassert: x\n\
         3\nassert: 3\n5\nassert: 5\n3\nassert: 3\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn red_black_tree_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { RedBlackTree } from "tyrus:algorithm";

function runChecks(): void {
    const t = RedBlackTree<i32>();
    console.log(t.isEmpty());
    console.log("assert:", true);

    const vals = [5, 3, 8, 1, 4, 7, 9, 6, 2, 0];
    for (const v of vals) { t.insert(v); }
    t.insert(4);
    console.log(t.len());
    console.log("assert:", 10);
    console.log(t.contains(6));
    console.log("assert:", true);
    console.log(t.contains(11));
    console.log("assert:", false);
    console.log(t.contains(0));
    console.log("assert:", true);
    console.log(t.contains(9));
    console.log("assert:", true);

    const s = RedBlackTree<string>();
    s.insert("banana");
    s.insert("apple");
    s.insert("cherry");
    s.insert("apple");
    console.log(s.len());
    console.log("assert:", 3);
    console.log(s.contains("banana"));
    console.log("assert:", true);
    console.log(s.contains("grape"));
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n10\nassert: 10\ntrue\nassert: true\nfalse\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\n3\nassert: 3\ntrue\nassert: true\n\
         false\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn splay_tree_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { SplayTree } from "tyrus:algorithm";

function runChecks(): void {
    const t = SplayTree<i32>();
    console.log(t.isEmpty());
    console.log("assert:", true);

    const vals = [5, 3, 8, 1, 4, 7, 9, 6, 2, 0];
    for (const v of vals) { t.insert(v); }
    t.insert(4);
    console.log(t.len());
    console.log("assert:", 10);
    console.log(t.contains(6));
    console.log("assert:", true);
    console.log(t.contains(11));
    console.log("assert:", false);
    console.log(t.contains(0));
    console.log("assert:", true);
    console.log(t.contains(9));
    console.log("assert:", true);

    const s = SplayTree<string>();
    s.insert("banana");
    s.insert("apple");
    s.insert("cherry");
    s.insert("apple");
    console.log(s.len());
    console.log("assert:", 3);
    console.log(s.contains("banana"));
    console.log("assert:", true);
    console.log(s.contains("grape"));
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n10\nassert: 10\ntrue\nassert: true\nfalse\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\n3\nassert: 3\ntrue\nassert: true\n\
         false\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn treap_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { Treap } from "tyrus:algorithm";

function runChecks(): void {
    const t = Treap<i32>();
    console.log(t.isEmpty());
    console.log("assert:", true);

    const vals = [5, 3, 8, 1, 4, 7, 9, 6, 2, 0];
    for (const v of vals) { t.insert(v); }
    t.insert(4);
    console.log(t.len());
    console.log("assert:", 10);
    console.log(t.contains(6));
    console.log("assert:", true);
    console.log(t.contains(11));
    console.log("assert:", false);
    console.log(t.contains(0));
    console.log("assert:", true);
    console.log(t.contains(9));
    console.log("assert:", true);

    const s = Treap<string>();
    s.insert("banana");
    s.insert("apple");
    s.insert("cherry");
    s.insert("apple");
    console.log(s.len());
    console.log("assert:", 3);
    console.log(s.contains("banana"));
    console.log("assert:", true);
    console.log(s.contains("grape"));
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n10\nassert: 10\ntrue\nassert: true\nfalse\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\n3\nassert: 3\ntrue\nassert: true\n\
         false\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn ordered_trees_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { RedBlackTree, SplayTree, Treap } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: i32 = 0; i < 1024; i++) {
        const r = RedBlackTree<i32>();
        r.insert(i);
        r.insert(i + 1);
        r.insert(i - 1);
        r.contains(i);
        r.contains(i + 100);
        r.len();
        r.isEmpty();
        const s = SplayTree<i32>();
        s.insert(i);
        s.insert(i + 1);
        s.insert(i - 1);
        s.contains(i);
        s.contains(i + 100);
        s.len();
        s.isEmpty();
        const t = Treap<i32>();
        t.insert(i);
        t.insert(i + 1);
        t.insert(i - 1);
        t.contains(i);
        t.contains(i + 100);
        t.len();
        t.isEmpty();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn avl_tree_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { AvlTree } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: i32 = 0; i < 1024; i++) {
        const t = AvlTree<i32>();
        t.insert(i);
        t.insert(i + 1);
        t.insert(i - 1);
        t.contains(i);
        t.contains(i + 100);
        t.len();
        t.isEmpty();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── SparseTable ───

#[test]
fn sparse_table_range_minimums() {
    let project = AlgorithmProject::new(
        r#"import { SparseTable } from "tyrus:algorithm";

function runChecks(): void {
    const data: i32[] = [7, 2, 9, 1, 5, 8, 3];
    const st = SparseTable(data);

    console.log(st.isEmpty());
    console.log("assert:", false);
    console.log(st.len());
    console.log("assert:", 7);

    console.log(st.query(0, 7));
    console.log("assert:", 1);
    console.log(st.query(0, 3));
    console.log("assert:", 2);
    console.log(st.query(2, 5));
    console.log("assert:", 1);
    console.log(st.query(1, 2));
    console.log("assert:", 2);
    console.log(st.query(3, 4));
    console.log("assert:", 1);
    console.log(st.query(0, 2));
    console.log("assert:", 2);
    console.log(st.query(4, 7));
    console.log("assert:", 3);
    console.log(st.query(5, 6));
    console.log("assert:", 8);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "false\nassert: false\n7\nassert: 7\n1\nassert: 1\n2\nassert: 2\n1\nassert: 1\n2\nassert: 2\n1\nassert: 1\n2\nassert: 2\n3\nassert: 3\n8\nassert: 8\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn sparse_table_non_power_of_two_and_single_element() {
    let project = AlgorithmProject::new(
        r#"import { SparseTable } from "tyrus:algorithm";

function runChecks(): void {
    const data: i32[] = [4, -3, 6, 2, -1, 0];
    const st = SparseTable(data);

    console.log(st.query(0, 6));
    console.log("assert:", -3);
    console.log(st.query(1, 5));
    console.log("assert:", -3);
    console.log(st.query(3, 6));
    console.log("assert:", -1);
    console.log(st.query(0, 1));
    console.log("assert:", 4);
    console.log(st.query(4, 6));
    console.log("assert:", -1);
    console.log(st.query(2, 4));
    console.log("assert:", 2);

    const single = SparseTable([9]);
    console.log(single.len());
    console.log("assert:", 1);
    console.log(single.query(0, 1));
    console.log("assert:", 9);
    console.log(single.isEmpty());
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "-3\nassert: -3\n-3\nassert: -3\n-1\nassert: -1\n4\nassert: 4\n-1\nassert: -1\n2\nassert: 2\n1\nassert: 1\n9\nassert: 9\nfalse\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn sparse_table_empty_construction() {
    let project = AlgorithmProject::new(
        r#"import { SparseTable } from "tyrus:algorithm";

function runChecks(): void {
    const empty: i32[] = [];
    const st = SparseTable(empty);

    console.log(st.isEmpty());
    console.log("assert:", true);
    console.log(st.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn sparse_table_query_out_of_bounds_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SparseTable } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SparseTable(data);
    st.query(1, 4);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SparseTable range is out of bounds"));
}

#[test]
fn sparse_table_query_empty_range_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SparseTable } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SparseTable(data);
    st.query(2, 2);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SparseTable range is out of bounds"));
}

#[test]
fn sparse_table_query_reversed_range_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SparseTable } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SparseTable(data);
    st.query(3, 1);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SparseTable range is out of bounds"));
}

#[test]
fn sparse_table_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { SparseTable } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const st = SparseTable([7, 2, 9, 1, 5, 8, 3]);
        st.query(0, 7);
        st.query(2, 5);
        st.len();
        st.isEmpty();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn fenwick_tree_negative_and_update_deltas() {
    let project = AlgorithmProject::new(
        r#"import { FenwickTree } from "tyrus:algorithm";

function runChecks(): void {
    const bit = FenwickTree(4);
    bit.add(0, 10);
    bit.add(1, 20);
    bit.add(2, 30);
    bit.add(3, 40);

    console.log(bit.sum(4));
    console.log("assert:", 100);
    console.log(bit.sum(2));
    console.log("assert:", 30);

    bit.add(1, -5);
    console.log(bit.sum(2));
    console.log("assert:", 25);
    console.log(bit.sum(4));
    console.log("assert:", 95);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "100\nassert: 100\n30\nassert: 30\n25\nassert: 25\n95\nassert: 95\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn fenwick_tree_zero_size() {
    let project = AlgorithmProject::new(
        r#"import { FenwickTree } from "tyrus:algorithm";

function runChecks(): void {
    const bit = FenwickTree(0);

    console.log(bit.isEmpty());
    console.log("assert:", true);
    console.log(bit.len());
    console.log("assert:", 0);
    console.log(bit.sum(0));
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn fenwick_tree_add_out_of_bounds_aborts() {
    let project = AlgorithmProject::new(
        r#"import { FenwickTree } from "tyrus:algorithm";

const main = (): void => {
    const bit = FenwickTree(3);
    bit.add(3, 1);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("FenwickTree index is out of bounds"));
}

#[test]
fn fenwick_tree_sum_out_of_bounds_aborts() {
    let project = AlgorithmProject::new(
        r#"import { FenwickTree } from "tyrus:algorithm";

const main = (): void => {
    const bit = FenwickTree(3);
    bit.sum(4);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("FenwickTree index is out of bounds"));
}

#[test]
fn fenwick_tree_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { FenwickTree } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const bit = FenwickTree(16);
        bit.add(0, 1);
        bit.add(15, 2);
        bit.sum(16);
        bit.sum(8);
        bit.len();
        bit.isEmpty();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── SegmentTree ───

#[test]
fn segment_tree_range_sums() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTree } from "tyrus:algorithm";

function runChecks(): void {
    const data: i32[] = [1, 3, 5, 7, 9, 11];
    const st = SegmentTree(data);

    console.log(st.isEmpty());
    console.log("assert:", false);
    console.log(st.len());
    console.log("assert:", 6);

    console.log(st.query(0, 6));
    console.log("assert:", 36);
    console.log(st.query(0, 3));
    console.log("assert:", 9);
    console.log(st.query(2, 5));
    console.log("assert:", 21);
    console.log(st.query(1, 2));
    console.log("assert:", 3);
    console.log(st.query(0, 0));
    console.log("assert:", 0);
    console.log(st.query(6, 6));
    console.log("assert:", 0);
    console.log(st.query(3, 6));
    console.log("assert:", 27);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "false\nassert: false\n6\nassert: 6\n36\nassert: 36\n9\nassert: 9\n21\nassert: 21\n3\nassert: 3\n0\nassert: 0\n0\nassert: 0\n27\nassert: 27\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn segment_tree_negative_values_and_updates() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTree } from "tyrus:algorithm";

function runChecks(): void {
    const data: i32[] = [10, -4, 7, -2, 5];
    const st = SegmentTree(data);

    console.log(st.query(0, 5));
    console.log("assert:", 16);
    console.log(st.query(1, 4));
    console.log("assert:", 1);

    st.update(1, 100);
    console.log(st.query(0, 5));
    console.log("assert:", 120);
    console.log(st.query(0, 2));
    console.log("assert:", 110);
    console.log(st.query(1, 2));
    console.log("assert:", 100);

    st.update(4, -5);
    console.log(st.query(0, 5));
    console.log("assert:", 110);
    console.log(st.query(3, 5));
    console.log("assert:", -7);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "16\nassert: 16\n1\nassert: 1\n120\nassert: 120\n110\nassert: 110\n100\nassert: 100\n110\nassert: 110\n-7\nassert: -7\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn segment_tree_zero_length() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTree } from "tyrus:algorithm";

function runChecks(): void {
    const empty: i32[] = [];
    const st = SegmentTree(empty);

    console.log(st.isEmpty());
    console.log("assert:", true);
    console.log(st.len());
    console.log("assert:", 0);
    console.log(st.query(0, 0));
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn segment_tree_update_out_of_bounds_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTree } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SegmentTree(data);
    st.update(3, 5);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SegmentTree index is out of bounds"));
}

#[test]
fn segment_tree_query_out_of_bounds_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTree } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SegmentTree(data);
    st.query(1, 4);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SegmentTree range is out of bounds"));
}

#[test]
fn segment_tree_query_reversed_range_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTree } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SegmentTree(data);
    st.query(2, 1);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SegmentTree range is out of bounds"));
}

#[test]
fn segment_tree_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTree } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 512; i++) {
        const data: i32[] = [1, 2, 3, 4, 5, 6, 7, 8];
        const st = SegmentTree(data);
        st.query(0, 8);
        st.query(2, 6);
        st.update(3, 10);
        st.query(0, 8);
        st.len();
        st.isEmpty();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── SegmentTreeLazy ───

#[test]
fn segment_tree_lazy_range_updates() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTreeLazy } from "tyrus:algorithm";

function runChecks(): void {
    const data: i32[] = [1, 3, 5, 7, 9, 11];
    const st = SegmentTreeLazy(data);

    console.log(st.isEmpty());
    console.log("assert:", false);
    console.log(st.len());
    console.log("assert:", 6);

    console.log(st.query(0, 6));
    console.log("assert:", 36);
    console.log(st.query(0, 3));
    console.log("assert:", 9);
    console.log(st.query(2, 5));
    console.log("assert:", 21);

    st.updateRange(1, 4, 10);

    console.log(st.query(0, 6));
    console.log("assert:", 66);
    console.log(st.query(1, 4));
    console.log("assert:", 45);
    console.log(st.query(0, 3));
    console.log("assert:", 29);
    console.log(st.query(4, 6));
    console.log("assert:", 20);

    st.updateRange(0, 6, -5);

    console.log(st.query(0, 6));
    console.log("assert:", 36);
    console.log(st.query(0, 2));
    console.log("assert:", 4);
    console.log(st.query(3, 6));
    console.log("assert:", 22);

    st.updateRange(2, 2, 100);

    console.log(st.query(2, 3));
    console.log("assert:", 10);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "false\nassert: false\n6\nassert: 6\n36\nassert: 36\n9\nassert: 9\n21\nassert: 21\n66\nassert: 66\n45\nassert: 45\n29\nassert: 29\n20\nassert: 20\n36\nassert: 36\n4\nassert: 4\n22\nassert: 22\n10\nassert: 10\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn segment_tree_lazy_partial_overlaps() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTreeLazy } from "tyrus:algorithm";

function runChecks(): void {
    const data: i32[] = [1, 2, 3, 4, 5, 6, 7, 8];
    const st = SegmentTreeLazy(data);

    console.log(st.query(0, 8));
    console.log("assert:", 36);
    console.log(st.query(3, 6));
    console.log("assert:", 15);

    st.updateRange(2, 5, 100);

    console.log(st.query(2, 5));
    console.log("assert:", 312);
    console.log(st.query(0, 8));
    console.log("assert:", 336);
    console.log(st.query(1, 4));
    console.log("assert:", 209);

    st.updateRange(0, 8, -1);

    console.log(st.query(0, 8));
    console.log("assert:", 328);
    console.log(st.query(4, 8));
    console.log("assert:", 122);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "36\nassert: 36\n15\nassert: 15\n312\nassert: 312\n336\nassert: 336\n209\nassert: 209\n328\nassert: 328\n122\nassert: 122\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn segment_tree_lazy_zero_length() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTreeLazy } from "tyrus:algorithm";

const main = (): void => {
    const empty: i32[] = [];
    const st = SegmentTreeLazy(empty);

    console.log(st.isEmpty());
    console.log("assert:", true);
    console.log(st.len());
    console.log("assert:", 0);
    console.log(st.query(0, 0));
    console.log("assert:", 0);
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn segment_tree_lazy_update_range_out_of_bounds_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTreeLazy } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SegmentTreeLazy(data);
    st.updateRange(2, 4, 5);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("SegmentTreeLazy range is out of bounds")
    );
}

#[test]
fn segment_tree_lazy_update_reversed_range_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTreeLazy } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SegmentTreeLazy(data);
    st.updateRange(3, 1, 5);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("SegmentTreeLazy range is out of bounds")
    );
}

#[test]
fn segment_tree_lazy_query_out_of_bounds_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTreeLazy } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SegmentTreeLazy(data);
    st.query(0, 4);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("SegmentTreeLazy range is out of bounds")
    );
}

#[test]
fn segment_tree_lazy_query_reversed_range_aborts() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTreeLazy } from "tyrus:algorithm";

const main = (): void => {
    const data: i32[] = [1, 2, 3];
    const st = SegmentTreeLazy(data);
    st.query(2, 1);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("SegmentTreeLazy range is out of bounds")
    );
}

#[test]
fn segment_tree_lazy_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { SegmentTreeLazy } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 512; i++) {
        const data: i32[] = [1, 2, 3, 4, 5, 6, 7, 8];
        const st = SegmentTreeLazy(data);
        st.query(0, 8);
        st.updateRange(2, 6, 10);
        st.query(2, 6);
        st.updateRange(0, 8, -1);
        st.query(0, 8);
        st.len();
        st.isEmpty();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn union_find_all_methods_comprehensive() {
    let project = AlgorithmProject::new(
        r#"import { UnionFind } from "tyrus:algorithm";

function runChecks(): void {
    const uf = UnionFind<u32>(8);

    console.log(uf.setCount());
    console.log("assert:", 8);

    console.log(uf.find(0));
    console.log("assert:", 0);
    console.log(uf.find(7));
    console.log("assert:", 7);

    console.log(uf.connected(0, 1));
    console.log("assert:", false);

    console.log(uf.union(0, 1));
    console.log("assert:", true);
    console.log(uf.connected(0, 1));
    console.log("assert:", true);

    console.log(uf.union(2, 3));
    console.log("assert:", true);
    console.log(uf.union(4, 5));
    console.log("assert:", true);
    console.log(uf.union(6, 7));
    console.log("assert:", true);

    console.log(uf.setCount());
    console.log("assert:", 4);

    console.log(uf.union(0, 2));
    console.log("assert:", true);
    console.log(uf.union(4, 6));
    console.log("assert:", true);

    console.log(uf.setCount());
    console.log("assert:", 2);

    console.log(uf.connected(0, 3));
    console.log("assert:", true);
    console.log(uf.connected(0, 5));
    console.log("assert:", false);
    console.log(uf.connected(4, 7));
    console.log("assert:", true);

    console.log(uf.union(0, 4));
    console.log("assert:", true);
    console.log(uf.setCount());
    console.log("assert:", 1);
    console.log(uf.connected(1, 6));
    console.log("assert:", true);
    console.log(uf.connected(0, 7));
    console.log("assert:", true);

    console.log(uf.union(0, 7));
    console.log("assert:", false);
    console.log(uf.setCount());
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "8\nassert: 8\n\
         0\nassert: 0\n7\nassert: 7\n\
         false\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\n\
         true\nassert: true\ntrue\nassert: true\ntrue\nassert: true\n\
         4\nassert: 4\n\
         true\nassert: true\ntrue\nassert: true\n\
         2\nassert: 2\n\
         true\nassert: true\nfalse\nassert: false\ntrue\nassert: true\n\
         true\nassert: true\n1\nassert: 1\ntrue\nassert: true\ntrue\nassert: true\n\
         false\nassert: false\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn union_find_stress_repeated_union_chain() {
    let project = AlgorithmProject::new(
        r#"import { UnionFind } from "tyrus:algorithm";

function runChecks(): void {
    const uf = UnionFind<u32>(200);

    for (let i: usize = 0; i < 199; i++) {
        uf.union(i as u32, (i + 1) as u32);
    }

    console.log(uf.setCount());
    console.log("assert:", 1);

    for (let i: usize = 0; i < 200; i++) {
        console.log(uf.connected(i as u32, 199));
        console.log("assert:", true);
    }
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed with status {:?}:\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("1\nassert: 1\n"));
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 402);
    assert!(output.stderr.is_empty());
}

#[test]
fn union_find_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { UnionFind } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const uf = UnionFind(64);
        uf.union(0, 1);
        uf.union(2, 3);
        uf.connected(0, 1);
        uf.setCount();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── BinaryHeap ───

#[test]
fn binary_heap_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { BinaryHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinaryHeap<i32>();

    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);

    bh.push(10);
    bh.push(3);
    bh.push(7);
    bh.push(1);

    console.log(bh.len());
    console.log("assert:", 4);
    console.log(bh.isEmpty());
    console.log("assert:", false);

    const p1: i32? = bh.peek();
    if (p1) { console.log(p1); }
    console.log("assert:", 10);
    const v1: i32? = bh.pop();
    if (v1) { console.log(v1); }
    console.log("assert:", 10);
    const p2: i32? = bh.peek();
    if (p2) { console.log(p2); }
    console.log("assert:", 7);
    const v2: i32? = bh.pop();
    if (v2) { console.log(v2); }
    console.log("assert:", 7);
    const v3: i32? = bh.pop();
    if (v3) { console.log(v3); }
    console.log("assert:", 3);
    const v4: i32? = bh.pop();
    if (v4) { console.log(v4); }
    console.log("assert:", 1);

    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n\
         4\nassert: 4\nfalse\nassert: false\n\
         10\nassert: 10\n10\nassert: 10\n7\nassert: 7\n7\nassert: 7\n3\nassert: 3\n1\nassert: 1\n\
         true\nassert: true\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binary_heap_max_heap_order() {
    let project = AlgorithmProject::new(
        r#"import { BinaryHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinaryHeap<i32>();

    bh.push(1);
    bh.push(5);
    bh.push(3);
    bh.push(9);
    bh.push(7);

    const v1: i32? = bh.pop();
    if (v1) { console.log(v1); }
    console.log("assert:", 9);
    const v2: i32? = bh.pop();
    if (v2) { console.log(v2); }
    console.log("assert:", 7);
    const v3: i32? = bh.pop();
    if (v3) { console.log(v3); }
    console.log("assert:", 5);
    const v4: i32? = bh.pop();
    if (v4) { console.log(v4); }
    console.log("assert:", 3);
    const v5: i32? = bh.pop();
    if (v5) { console.log(v5); }
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "9\nassert: 9\n7\nassert: 7\n5\nassert: 5\n3\nassert: 3\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binary_heap_clear_and_reuse() {
    let project = AlgorithmProject::new(
        r#"import { BinaryHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinaryHeap<i32>();

    bh.push(10);
    bh.push(3);
    bh.push(7);

    console.log(bh.len());
    console.log("assert:", 3);

    bh.clear();

    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);

    bh.push(5);
    bh.push(1);

    console.log(bh.len());
    console.log("assert:", 2);
    const p: i32? = bh.peek();
    if (p) { console.log(p); }
    console.log("assert:", 5);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\ntrue\nassert: true\n0\nassert: 0\n2\nassert: 2\n5\nassert: 5\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binary_heap_pop_peek_empty_does_not_crash() {
    // Known bug: pop()/peek() on an empty heap read uninitialized memory because
    // codegen ignores the boolean return. This test verifies the program does not
    // crash or abort, and that len/isEmpty are correct around the operations.
    let project = AlgorithmProject::new(
        r#"import { BinaryHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinaryHeap<i32>();

    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);

    // pop and peek on empty heap, result is uninitialized but should not crash
    bh.pop();
    bh.peek();

    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);

    bh.push(42);

    console.log(bh.isEmpty());
    console.log("assert:", false);
    console.log(bh.len());
    console.log("assert:", 1);

    const p1: i32? = bh.peek();
    if (p1) { console.log(p1); }
    console.log("assert:", 42);
    const v1: i32? = bh.pop();
    if (v1) { console.log(v1); }
    console.log("assert:", 42);

    // Empty again after popping the only element
    bh.pop();
    bh.peek();

    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n\
         true\nassert: true\n0\nassert: 0\n\
         false\nassert: false\n1\nassert: 1\n\
         42\nassert: 42\n42\nassert: 42\n\
         true\nassert: true\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binary_heap_pop_returns_unknown_type() {
    // Known bug: BinaryHeap methods lack type-checker support. pop()/peek() return
    // `unknown` instead of `T?`, so ?? is rejected. This test documents the bug:
    // pop() is called inline in console.log, verifying the codegen path compiles
    // and runs. When the checker is fixed, add ?? and optional assertions.
    let project = AlgorithmProject::new(
        r#"import { BinaryHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinaryHeap<i32>();

    bh.push(10);
    bh.push(20);
    bh.push(30);

    const v1: i32? = bh.pop();
    if (v1) { console.log(v1); }
    console.log("assert:", 30);

    const v2: i32? = bh.pop();
    if (v2) { console.log(v2); }
    console.log("assert:", 20);

    const v3: i32? = bh.pop();
    if (v3) { console.log(v3); }
    console.log("assert:", 10);

    console.log(bh.isEmpty());
    console.log("assert:", true);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "30\nassert: 30\n20\nassert: 20\n10\nassert: 10\ntrue\nassert: true\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binary_heap_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { BinaryHeap } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const bh = BinaryHeap<i32>();
        bh.push(10);
        bh.push(3);
        bh.push(7);
        bh.pop();
        bh.pop();
        bh.peek();
        bh.len();
        bh.isEmpty();
        bh.clear();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── BinomialHeap ───

#[test]
fn binomial_heap_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { BinomialHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinomialHeap<i32>();

    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);

    bh.push(10);
    bh.push(3);
    bh.push(7);
    bh.push(1);

    console.log(bh.len());
    console.log("assert:", 4);
    console.log(bh.isEmpty());
    console.log("assert:", false);

    const p1: i32? = bh.peek();
    if (p1) { console.log(p1); }
    console.log("assert:", 10);
    const v1: i32? = bh.pop();
    if (v1) { console.log(v1); }
    console.log("assert:", 10);
    const p2: i32? = bh.peek();
    if (p2) { console.log(p2); }
    console.log("assert:", 7);
    const v2: i32? = bh.pop();
    if (v2) { console.log(v2); }
    console.log("assert:", 7);
    const v3: i32? = bh.pop();
    if (v3) { console.log(v3); }
    console.log("assert:", 3);
    const v4: i32? = bh.pop();
    if (v4) { console.log(v4); }
    console.log("assert:", 1);

    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n\
         4\nassert: 4\nfalse\nassert: false\n\
         10\nassert: 10\n10\nassert: 10\n7\nassert: 7\n7\nassert: 7\n3\nassert: 3\n1\nassert: 1\n\
         true\nassert: true\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binomial_heap_max_heap_order() {
    let project = AlgorithmProject::new(
        r#"import { BinomialHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinomialHeap<i32>();
    bh.push(1);
    bh.push(5);
    bh.push(3);
    bh.push(9);
    bh.push(7);

    console.log(bh.len());
    console.log("assert:", 5);

    const v1: i32? = bh.pop();
    if (v1) { console.log(v1); }
    console.log("assert:", 9);
    const v2: i32? = bh.pop();
    if (v2) { console.log(v2); }
    console.log("assert:", 7);
    const v3: i32? = bh.pop();
    if (v3) { console.log(v3); }
    console.log("assert:", 5);
    const v4: i32? = bh.pop();
    if (v4) { console.log(v4); }
    console.log("assert:", 3);
    const v5: i32? = bh.pop();
    if (v5) { console.log(v5); }
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "5\nassert: 5\n\
         9\nassert: 9\n7\nassert: 7\n5\nassert: 5\n3\nassert: 3\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binomial_heap_clear_and_reuse() {
    let project = AlgorithmProject::new(
        r#"import { BinomialHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinomialHeap<i64>();
    bh.push(30);
    bh.push(10);
    bh.push(20);
    console.log(bh.len());
    console.log("assert:", 3);

    bh.clear();
    console.log(bh.isEmpty());
    console.log("assert:", true);
    console.log(bh.len());
    console.log("assert:", 0);

    const v: i64? = bh.pop();
    if (v) { console.log(v); }
    console.log("assert:", "nope");

    bh.push(50);
    console.log(bh.len());
    console.log("assert:", 1);
    const top: i64? = bh.peek();
    if (top) { console.log(top); }
    console.log("assert:", 50);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\n\
         true\nassert: true\n0\nassert: 0\n\
         assert: nope\n\
         1\nassert: 1\n50\nassert: 50\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binomial_heap_pop_peek_empty_does_not_crash() {
    let project = AlgorithmProject::new(
        r#"import { BinomialHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinomialHeap<i32>();
    for (let i: i32 = 0; i < 8; i++) {
        bh.push(i);
    }
    for (let i: i32 = 0; i < 8; i++) {
        const v: i32? = bh.pop();
        if (v) { console.log(v); }
    }
    const empty: i32? = bh.pop();
    if (empty) { console.log(empty); }
    console.log("done");
    console.log("assert:", "done");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "7\n6\n5\n4\n3\n2\n1\n0\n\
         done\nassert: done\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binomial_heap_stress_meld_and_pop_order() {
    let project = AlgorithmProject::new(
        r#"import { BinomialHeap } from "tyrus:algorithm";

function runChecks(): void {
    const bh = BinomialHeap<i64>();
    for (let i: i64 = 0; i < 1000; i++) {
        bh.push(i);
    }
    console.log(bh.len());
    console.log("assert:", 1000);

    let sum: i64 = 0;
    let count: i64 = 0;
    for (let i: i64 = 0; i < 1000; i++) {
        const v: i64? = bh.pop();
        if (v) {
            sum += v;
            count += 1;
        }
    }
    console.log(count);
    console.log("assert:", 1000);
    console.log(sum);
    console.log("assert:", 499500);
    console.log(bh.isEmpty());
    console.log("assert:", true);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1000\nassert: 1000\n1000\nassert: 1000\n499500\nassert: 499500\ntrue\nassert: true\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn binomial_heap_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { BinomialHeap } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const bh = BinomialHeap<i32>();
        bh.push(10);
        bh.push(3);
        bh.push(7);
        bh.pop();
        bh.pop();
        bh.peek();
        bh.len();
        bh.isEmpty();
        bh.clear();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── Counter ───

#[test]
fn counter_most_common_uses_the_tuple_abi_and_stable_ties() {
    let project = AlgorithmProject::new(
        r#"import { Counter } from "tyrus:algorithm";

function runChecks(): void {
    const counter = Counter<i64>();
    counter.add(1000);
    counter.add(1000);
    counter.add(2000);
    counter.add(3000);
    const common = counter.mostCommon(3);
    console.log(common.length);
    console.log(common[0][0]);
    console.log(common[0][1]);
    console.log(common[1][0]);
    console.log(common[1][1]);
    console.log(common[2][0]);
    console.log(common[2][1]);
}

const main = (): void => {
    runChecks();
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\n1000\n2\n2000\n1\n3000\n1\n"
    );
}

#[test]
fn counter_rejects_non_positive_counts_without_a_rust_panic() {
    let project = AlgorithmProject::new(
        r#"import { Counter } from "tyrus:algorithm";

const main = (): void => {
    const counter = Counter<i32>();
    counter.add(1, 0);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Counter count must be greater than zero")
    );
}

#[test]
fn counter_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { Counter } from "tyrus:algorithm";

function runChecks(): void {
    const counter = Counter<i32>();

    console.log(counter.isEmpty());
    console.log("assert:", true);
    console.log(counter.unique());
    console.log("assert:", 0);
    console.log(counter.total());
    console.log("assert:", 0);

    counter.add(10);
    counter.add(20);
    counter.add(10);
    counter.add(30);
    counter.add(20);
    counter.add(10);

    console.log(counter.isEmpty());
    console.log("assert:", false);
    console.log(counter.unique());
    console.log("assert:", 3);
    console.log(counter.total());
    console.log("assert:", 6);

    console.log(counter.count(10));
    console.log("assert:", 3);
    console.log(counter.count(20));
    console.log("assert:", 2);
    console.log(counter.count(30));
    console.log("assert:", 1);
    console.log(counter.count(40));
    console.log("assert:", 0);

    counter.remove(10);
    console.log(counter.count(10));
    console.log("assert:", 2);
    counter.remove(10, 2);
    console.log(counter.count(10));
    console.log("assert:", 0);

    counter.clear();
    console.log(counter.isEmpty());
    console.log("assert:", true);
    console.log(counter.unique());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n\
         0\nassert: 0\n\
         0\nassert: 0\n\
         false\nassert: false\n\
         3\nassert: 3\n\
         6\nassert: 6\n\
         3\nassert: 3\n\
         2\nassert: 2\n\
         1\nassert: 1\n\
         0\nassert: 0\n\
         2\nassert: 2\n\
         0\nassert: 0\n\
         true\nassert: true\n\
         0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn counter_add_with_custom_count() {
    let project = AlgorithmProject::new(
        r#"import { Counter } from "tyrus:algorithm";

function runChecks(): void {
    const counter = Counter<i32>();

    counter.add(7, 5);
    console.log(counter.count(7));
    console.log("assert:", 5);

    counter.add(7, 3);
    console.log(counter.count(7));
    console.log("assert:", 8);

    counter.remove(7, 2);
    console.log(counter.count(7));
    console.log("assert:", 6);

    console.log(counter.total());
    console.log("assert:", 6);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "5\nassert: 5\n8\nassert: 8\n6\nassert: 6\n6\nassert: 6\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn counter_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { Counter } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const counter = Counter<i32>();
        counter.add(1);
        counter.add(2);
        counter.add(3);
        counter.count(1);
        counter.total();
        counter.unique();
        counter.clear();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── Deque ───

#[test]
fn deque_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { Deque } from "tyrus:algorithm";

function runChecks(): void {
    const dq = Deque<i32>();

    console.log(dq.isEmpty());
    console.log("assert:", true);
    console.log(dq.len());
    console.log("assert:", 0);

    dq.pushBack(10);
    dq.pushBack(20);
    dq.pushBack(30);

    console.log(dq.len());
    console.log("assert:", 3);
    console.log(dq.isEmpty());
    console.log("assert:", false);

    const pf1: i32? = dq.peekFront();
    if (pf1) { console.log(pf1); }
    console.log("assert:", 10);
    const pb1: i32? = dq.peekBack();
    if (pb1) { console.log(pb1); }
    console.log("assert:", 30);

    dq.pushFront(5);
    const pf2: i32? = dq.peekFront();
    if (pf2) { console.log(pf2); }
    console.log("assert:", 5);
    console.log(dq.len());
    console.log("assert:", 4);

    const v1: i32? = dq.popFront();
    if (v1) { console.log(v1); }
    console.log("assert:", 5);
    const v2: i32? = dq.popBack();
    if (v2) { console.log(v2); }
    console.log("assert:", 30);
    console.log(dq.len());
    console.log("assert:", 2);

    const v3: i32? = dq.popFront();
    if (v3) { console.log(v3); }
    console.log("assert:", 10);
    const v4: i32? = dq.popBack();
    if (v4) { console.log(v4); }
    console.log("assert:", 20);

    console.log(dq.isEmpty());
    console.log("assert:", true);
    console.log(dq.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n\
         3\nassert: 3\nfalse\nassert: false\n\
         10\nassert: 10\n30\nassert: 30\n\
         5\nassert: 5\n4\nassert: 4\n\
         5\nassert: 5\n30\nassert: 30\n2\nassert: 2\n\
         10\nassert: 10\n20\nassert: 20\n\
         true\nassert: true\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn deque_push_front_back_order() {
    let project = AlgorithmProject::new(
        r#"import { Deque } from "tyrus:algorithm";

function runChecks(): void {
    const dq = Deque<i32>();

    dq.pushBack(1);
    dq.pushBack(2);
    dq.pushBack(3);
    dq.pushFront(0);

    const v1: i32? = dq.popFront();
    if (v1) { console.log(v1); }
    console.log("assert:", 0);
    const v2: i32? = dq.popFront();
    if (v2) { console.log(v2); }
    console.log("assert:", 1);
    const v3: i32? = dq.popFront();
    if (v3) { console.log(v3); }
    console.log("assert:", 2);
    const v4: i32? = dq.popFront();
    if (v4) { console.log(v4); }
    console.log("assert:", 3);

    dq.pushFront(3);
    dq.pushFront(2);
    dq.pushFront(1);
    dq.pushBack(4);

    const v5: i32? = dq.popBack();
    if (v5) { console.log(v5); }
    console.log("assert:", 4);
    const v6: i32? = dq.popBack();
    if (v6) { console.log(v6); }
    console.log("assert:", 3);
    const v7: i32? = dq.popBack();
    if (v7) { console.log(v7); }
    console.log("assert:", 2);
    const v8: i32? = dq.popBack();
    if (v8) { console.log(v8); }
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0\nassert: 0\n1\nassert: 1\n2\nassert: 2\n3\nassert: 3\n\
         4\nassert: 4\n3\nassert: 3\n2\nassert: 2\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn deque_clear_and_reuse() {
    let project = AlgorithmProject::new(
        r#"import { Deque } from "tyrus:algorithm";

function runChecks(): void {
    const dq = Deque<i32>();

    dq.pushBack(10);
    dq.pushBack(20);
    dq.pushBack(30);

    console.log(dq.len());
    console.log("assert:", 3);

    dq.clear();

    console.log(dq.isEmpty());
    console.log("assert:", true);
    console.log(dq.len());
    console.log("assert:", 0);

    dq.pushFront(5);
    dq.pushBack(15);

    console.log(dq.len());
    console.log("assert:", 2);
    const pf: i32? = dq.peekFront();
    if (pf) { console.log(pf); }
    console.log("assert:", 5);
    const pb: i32? = dq.peekBack();
    if (pb) { console.log(pb); }
    console.log("assert:", 15);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\ntrue\nassert: true\n0\nassert: 0\n\
         2\nassert: 2\n5\nassert: 5\n15\nassert: 15\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn deque_pop_front_back_empty() {
    let project = AlgorithmProject::new(
        r#"import { Deque } from "tyrus:algorithm";

function runChecks(): void {
    const dq = Deque<i32>();

    // pop and peek on empty deque, result is uninitialized but should not crash
    dq.popFront();
    dq.popBack();
    dq.peekFront();
    dq.peekBack();

    console.log(dq.isEmpty());
    console.log("assert:", true);
    console.log(dq.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn deque_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { Deque } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const dq = Deque<i32>();
        dq.pushBack(10);
        dq.pushBack(20);
        dq.pushBack(30);
        dq.pushFront(5);
        dq.popFront();
        dq.popBack();
        dq.peekFront();
        dq.peekBack();
        dq.len();
        dq.isEmpty();
        dq.clear();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── BitSet ───

#[test]
fn bit_set_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { BitSet } from "tyrus:algorithm";

function runChecks(): void {
    const bs = BitSet(100);
    console.log(bs.isEmpty());
    console.log("assert:", true);
    console.log(bs.count());
    console.log("assert:", 0);

    bs.set(5);
    bs.set(10, true);
    bs.set(3, false);
    console.log(bs.get(5));
    console.log("assert:", true);
    console.log(bs.get(3));
    console.log("assert:", false);
    console.log(bs.count());
    console.log("assert:", 2);
    console.log(bs.isEmpty());
    console.log("assert:", false);

    bs.flip(3);
    console.log(bs.get(3));
    console.log("assert:", true);
    console.log(bs.count());
    console.log("assert:", 3);
    bs.flip(3);
    console.log(bs.get(3));
    console.log("assert:", false);

    bs.clear();
    console.log(bs.isEmpty());
    console.log("assert:", true);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n\
         true\nassert: true\nfalse\nassert: false\n2\nassert: 2\nfalse\nassert: false\n\
         true\nassert: true\n3\nassert: 3\nfalse\nassert: false\n\
         true\nassert: true\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn bit_set_set_operations() {
    let project = AlgorithmProject::new(
        r#"import { BitSet } from "tyrus:algorithm";

function runChecks(): void {
    const a = BitSet(100);
    a.set(1);
    a.set(3);
    a.set(5);
    const b = BitSet(100);
    b.set(3);
    b.set(5);
    b.set(7);

    console.log(a.isSubset(b));
    console.log("assert:", false);
    console.log(b.isSuperset(a));
    console.log("assert:", false);

    const sub = BitSet(100);
    sub.set(3);
    sub.set(5);
    console.log(sub.isSubset(b));
    console.log("assert:", true);
    console.log(b.isSuperset(sub));
    console.log("assert:", true);

    const u = a.union(b);
    console.log(u.count());
    console.log("assert:", 4);
    console.log(u.get(1));
    console.log("assert:", true);
    console.log(u.get(7));
    console.log("assert:", true);

    const i = a.intersection(b);
    console.log(i.count());
    console.log("assert:", 2);
    console.log(i.get(3));
    console.log("assert:", true);
    console.log(i.get(1));
    console.log("assert:", false);

    const d = a.difference(b);
    console.log(d.count());
    console.log("assert:", 1);
    console.log(d.get(1));
    console.log("assert:", true);
    console.log(d.get(3));
    console.log("assert:", false);

    const c = a.complement();
    console.log(c.get(1));
    console.log("assert:", false);
    console.log(c.get(0));
    console.log("assert:", true);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "false\nassert: false\nfalse\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\n\
         4\nassert: 4\ntrue\nassert: true\ntrue\nassert: true\n\
         2\nassert: 2\ntrue\nassert: true\nfalse\nassert: false\n\
         1\nassert: 1\ntrue\nassert: true\nfalse\nassert: false\n\
         false\nassert: false\ntrue\nassert: true\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn bit_set_from_array() {
    let project = AlgorithmProject::new(
        r#"import { BitSet } from "tyrus:algorithm";

function runChecks(): void {
    const indices: usize[] = [2, 4, 6];
    const bs = BitSet.fromArray(indices);
    console.log(bs.count());
    console.log("assert:", 3);
    console.log(bs.get(2));
    console.log("assert:", true);
    console.log(bs.get(4));
    console.log("assert:", true);
    console.log(bs.get(6));
    console.log("assert:", true);
    console.log(bs.get(5));
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\ntrue\nassert: true\ntrue\nassert: true\ntrue\nassert: true\nfalse\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn bit_set_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { BitSet } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const bs = BitSet(64);
        bs.set(3);
        bs.set(9);
        const other = BitSet(64);
        other.set(3);
        const u = bs.union(other);
        u.count();
        const i2 = bs.intersection(other);
        i2.count();
        const d = bs.difference(other);
        d.count();
        const c = bs.complement();
        c.count();
        bs.isSubset(other);
        other.isSuperset(bs);
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── LinkedList ───

#[test]
fn linked_list_operations() {
    let project = AlgorithmProject::new(
        r#"import { LinkedList } from "tyrus:algorithm";

function runChecks(): void {
    const ll = LinkedList<i64>();
    console.log(ll.isEmpty());
    console.log("assert:", true);
    console.log(ll.len());
    console.log("assert:", 0);

    ll.pushFront(10);
    ll.pushBack(20);
    ll.pushBack(30);
    console.log(ll.len());
    console.log("assert:", 3);
    console.log(ll.isEmpty());
    console.log("assert:", false);

    const f: i64? = ll.front();
    if (f) { console.log(f); }
    console.log("assert:", 10);
    const b: i64? = ll.back();
    if (b) { console.log(b); }
    console.log("assert:", 30);

    const p: i64? = ll.popFront();
    if (p) { console.log(p); }
    console.log("assert:", 10);
    console.log(ll.len());
    console.log("assert:", 2);

    const q: i64? = ll.popBack();
    if (q) { console.log(q); }
    console.log("assert:", 30);
    console.log(ll.len());
    console.log("assert:", 1);

    ll.clear();
    console.log(ll.isEmpty());
    console.log("assert:", true);
    console.log(ll.len());
    console.log("assert:", 0);

    const e: i64? = ll.front();
    console.log(e ?? -1);
    console.log("assert:", -1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n\
         3\nassert: 3\nfalse\nassert: false\n\
         10\nassert: 10\n30\nassert: 30\n\
         10\nassert: 10\n2\nassert: 2\n\
         30\nassert: 30\n1\nassert: 1\n\
         true\nassert: true\n0\nassert: 0\n\
         -1\nassert: -1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn linked_list_from_array() {
    let project = AlgorithmProject::new(
        r#"import { LinkedList } from "tyrus:algorithm";

function runChecks(): void {
    const vals: i32[] = [7, 8, 9];
    const ll = LinkedList.fromArray(vals);
    console.log(ll.len());
    console.log("assert:", 3);
    const f: i32? = ll.front();
    if (f) { console.log(f); }
    console.log("assert:", 7);
    const b: i32? = ll.back();
    if (b) { console.log(b); }
    console.log("assert:", 9);
    const p: i32? = ll.popFront();
    if (p) { console.log(p); }
    console.log("assert:", 7);
    console.log(ll.len());
    console.log("assert:", 2);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\n7\nassert: 7\n9\nassert: 9\n7\nassert: 7\n2\nassert: 2\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn linked_list_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { LinkedList } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const ll = LinkedList<i64>();
        ll.pushBack(10);
        ll.pushFront(20);
        ll.pushBack(30);
        ll.front();
        ll.back();
        ll.popFront();
        ll.popBack();
        ll.len();
        ll.isEmpty();
        ll.clear();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── DoubleLinkedList ───

#[test]
fn double_linked_list_operations() {
    let project = AlgorithmProject::new(
        r#"import { DoubleLinkedList } from "tyrus:algorithm";

function runChecks(): void {
    const dll = DoubleLinkedList<u8>();
    console.log(dll.isEmpty());
    console.log("assert:", true);
    console.log(dll.len());
    console.log("assert:", 0);

    dll.pushBack(1);
    dll.pushFront(0);
    dll.pushBack(2);
    console.log(dll.len());
    console.log("assert:", 3);

    const df: u8? = dll.front();
    if (df) { console.log(df); }
    console.log("assert:", 0);
    const db: u8? = dll.back();
    if (db) { console.log(db); }
    console.log("assert:", 2);

    const pf: u8? = dll.popFront();
    if (pf) { console.log(pf); }
    console.log("assert:", 0);
    console.log(dll.len());
    console.log("assert:", 2);

    const pb: u8? = dll.popBack();
    if (pb) { console.log(pb); }
    console.log("assert:", 2);
    console.log(dll.len());
    console.log("assert:", 1);

    dll.clear();
    console.log(dll.isEmpty());
    console.log("assert:", true);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n\
         3\nassert: 3\n\
         0\nassert: 0\n2\nassert: 2\n\
         0\nassert: 0\n2\nassert: 2\n\
         2\nassert: 2\n1\nassert: 1\n\
         true\nassert: true\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn double_linked_list_from_array() {
    let project = AlgorithmProject::new(
        r#"import { DoubleLinkedList } from "tyrus:algorithm";

function runChecks(): void {
    const floats: f64[] = [1.5, 2.5];
    const dll = DoubleLinkedList.fromArray(floats);
    console.log(dll.len());
    console.log("assert:", 2);
    const f: f64? = dll.front();
    if (f) { console.log(f); }
    console.log("assert:", 1.5);
    const b: f64? = dll.back();
    if (b) { console.log(b); }
    console.log("assert:", 2.5);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "2\nassert: 2\n1.5\nassert: 1.5\n2.5\nassert: 2.5\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn double_linked_list_no_leak_in_loop() {
    let project = AlgorithmProject::new(
        r#"import { DoubleLinkedList } from "tyrus:algorithm";

function runChecks(): void {
    for (let i: usize = 0; i < 1024; i++) {
        const dll = DoubleLinkedList<i32>();
        dll.pushBack(10);
        dll.pushFront(20);
        dll.pushBack(30);
        dll.front();
        dll.back();
        dll.popFront();
        dll.popBack();
        dll.len();
        dll.isEmpty();
        dll.clear();
    }
    console.log("no leak");
    console.log("assert:", "no leak");
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no leak\nassert: no leak\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn explicit_type_annotations_match_inferred_types() {
    let project = AlgorithmProject::new(
        r#"import {
    BinaryHeap,
    SortedSet,
    SortedMap,
    UnionFind,
    AvlTree,
    RedBlackTree,
    SplayTree,
    Treap,
    Counter,
    Deque,
    LinkedList,
    DoubleLinkedList,
    FenwickTree,
    SegmentTree,
    SparseTable,
    SegmentTreeLazy,
    BitSet,
} from "tyrus:algorithm";

function sizeOf(tree: AvlTree<i32>): usize {
    return tree.len();
}

function makeSet(): SortedSet<i32> {
    const s: SortedSet<i32> = SortedSet<i32>();
    s.insert(4);
    return s;
}

function runChecks(): void {
    const h: BinaryHeap<i32> = BinaryHeap<i32>();
    const s: SortedSet<i32> = makeSet();
    const m: SortedMap<string, i32> = SortedMap<string, i32>();
    const u: UnionFind = UnionFind(8);
    const um: UnionFind<u32> = UnionFind<u32>(8);
    const t: AvlTree<i32> = AvlTree<i32>();
    const r: RedBlackTree<i32> = RedBlackTree<i32>();
    const sp: SplayTree<i32> = SplayTree<i32>();
    const tr: Treap<i32> = Treap<i32>();
    const c: Counter<i32> = Counter<i32>();
    const d: Deque<i32> = Deque<i32>();
    const ll: LinkedList<i32> = LinkedList<i32>();
    const dll: DoubleLinkedList<i32> = DoubleLinkedList<i32>();
    const f: FenwickTree = FenwickTree(8);
    const seg: SegmentTree = SegmentTree([1, 2, 3]);
    const stab: SparseTable = SparseTable([1, 2, 3]);
    const segl: SegmentTreeLazy = SegmentTreeLazy([1, 2, 3]);
    const b: BitSet = BitSet(16);

    h.push(9);
    c.add(5);
    d.pushBack(7);
    ll.pushBack(3);
    dll.pushBack(2);
    f.add(0, 1);
    f.add(1, 2);
    u.union(1, 2);
    um.union(1, 2);
    t.insert(8);
    r.insert(8);
    sp.insert(8);
    tr.insert(8);
    m.set("a", 1);

    console.log(h.peek() ?? 0);
    console.log("assert:", 9);
    console.log(s.contains(4));
    console.log("assert:", true);
    console.log(m.get("a") ?? 0);
    console.log("assert:", 1);
    console.log(u.find(2));
    console.log("assert:", 1);
    console.log(um.find(2));
    console.log("assert:", 1);
    console.log(sizeOf(t));
    console.log("assert:", 1);
    console.log(r.contains(8));
    console.log("assert:", true);
    console.log(sp.contains(8));
    console.log("assert:", true);
    console.log(tr.contains(8));
    console.log("assert:", true);
    console.log(c.total());
    console.log("assert:", 1);
    console.log(d.peekBack() ?? 0);
    console.log("assert:", 7);
    console.log(ll.back() ?? 0);
    console.log("assert:", 3);
    console.log(dll.back() ?? 0);
    console.log("assert:", 2);
    console.log(f.sum(2));
    console.log("assert:", 3);
    console.log(seg.query(0, 2));
    console.log("assert:", 3);
    console.log(stab.query(0, 2));
    console.log("assert:", 1);
    console.log(segl.query(0, 2));
    console.log("assert:", 3);
    console.log(b.count());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "9\nassert: 9\ntrue\nassert: true\n1\nassert: 1\n1\nassert: 1\n1\nassert: 1\n\
         1\nassert: 1\ntrue\nassert: true\ntrue\nassert: true\ntrue\nassert: true\n\
         1\nassert: 1\n7\nassert: 7\n3\nassert: 3\n2\nassert: 2\n3\nassert: 3\n\
         3\nassert: 3\n1\nassert: 1\n3\nassert: 3\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── Trie ───

#[test]
fn trie_basic_operations() {
    let project = AlgorithmProject::new(
        r#"import { Trie } from "tyrus:algorithm";

function runChecks(): void {
    const trie = Trie();
    console.log(trie.isEmpty());
    console.log("assert:", true);
    console.log(trie.len());
    console.log("assert:", 0);

    trie.insert("apple");
    trie.insert("app");
    trie.insert("apply");
    trie.insert("banana");
    console.log(trie.len());
    console.log("assert:", 4);
    console.log(trie.isEmpty());
    console.log("assert:", false);

    console.log(trie.search("apple"));
    console.log("assert:", true);
    console.log(trie.search("app"));
    console.log("assert:", true);
    console.log(trie.search("ap"));
    console.log("assert:", false);
    console.log(trie.search("banana"));
    console.log("assert:", true);
    console.log(trie.search("grape"));
    console.log("assert:", false);

    console.log(trie.startsWith("app"));
    console.log("assert:", true);
    console.log(trie.startsWith("ban"));
    console.log("assert:", true);
    console.log(trie.startsWith("gra"));
    console.log("assert:", false);

    trie.clear();
    console.log(trie.len());
    console.log("assert:", 0);
    console.log(trie.isEmpty());
    console.log("assert:", true);
    console.log(trie.search("apple"));
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n0\nassert: 0\n\
         4\nassert: 4\nfalse\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\nfalse\nassert: false\n\
         true\nassert: true\nfalse\nassert: false\n\
         true\nassert: true\ntrue\nassert: true\nfalse\nassert: false\n\
         0\nassert: 0\ntrue\nassert: true\nfalse\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn trie_duplicate_insert_ignored() {
    let project = AlgorithmProject::new(
        r#"import { Trie } from "tyrus:algorithm";

function runChecks(): void {
    const trie = Trie();
    trie.insert("hello");
    trie.insert("hello");
    trie.insert("hello");
    console.log(trie.len());
    console.log("assert:", 1);
    console.log(trie.search("hello"));
    console.log("assert:", true);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1\nassert: 1\ntrue\nassert: true\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn trie_prefix_and_empty_word() {
    let project = AlgorithmProject::new(
        r#"import { Trie } from "tyrus:algorithm";

function runChecks(): void {
    const trie = Trie();
    trie.insert("");
    console.log(trie.len());
    console.log("assert:", 1);
    console.log(trie.search(""));
    console.log("assert:", true);
    console.log(trie.startsWith(""));
    console.log("assert:", true);

    trie.insert("x");
    console.log(trie.startsWith(""));
    console.log("assert:", true);
    console.log(trie.search("x"));
    console.log("assert:", true);
    console.log(trie.startsWith("xy"));
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1\nassert: 1\ntrue\nassert: true\ntrue\nassert: true\n\
         true\nassert: true\ntrue\nassert: true\nfalse\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── suffixArray ───

#[test]
fn suffix_array_sorts_suffixes() {
    let project = AlgorithmProject::new(
        r#"import { suffixArray } from "tyrus:algorithm";

function runChecks(): void {
    const indices: usize[] = suffixArray("banana");
    console.log(indices);
    console.log(indices.length);
    console.log("assert:", 6);
    console.log(indices[0]);
    console.log("assert:", 5);
    console.log(indices[1]);
    console.log("assert:", 3);
    console.log(indices[2]);
    console.log("assert:", 1);
    console.log(indices[3]);
    console.log("assert:", 0);
    console.log(indices[4]);
    console.log("assert:", 4);
    console.log(indices[5]);
    console.log("assert:", 2);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[5, 3, 1, 0, 4, 2]\n6\nassert: 6\n\
         5\nassert: 5\n3\nassert: 3\n1\nassert: 1\n\
         0\nassert: 0\n4\nassert: 4\n2\nassert: 2\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suffix_array_edge_cases() {
    let project = AlgorithmProject::new(
        r#"import { suffixArray } from "tyrus:algorithm";

function runChecks(): void {
    const empty: usize[] = suffixArray("");
    console.log(empty.length);
    console.log("assert:", 0);

    const single: usize[] = suffixArray("a");
    console.log(single);
    console.log(single[0]);
    console.log("assert:", 0);

    const repeated: usize[] = suffixArray("aaaa");
    console.log(repeated);
    console.log(repeated[0]);
    console.log("assert:", 3);
    console.log(repeated[3]);
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0\nassert: 0\n\
         [0]\n0\nassert: 0\n\
         [3, 2, 1, 0]\n3\nassert: 3\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suffix_array_rebind_and_namespace_import() {
    let project = AlgorithmProject::new(
        r#"import * as alg from "tyrus:algorithm";

function runChecks(): void {
    const first: usize[] = alg.suffixArray("mississippi");
    const rebind: usize[] = alg.suffixArray("cocoa");
    console.log(rebind);
    console.log(first[0]);
    console.log("assert:", 10);
    console.log(first[10]);
    console.log("assert:", 2);
    console.log(rebind[0]);
    console.log("assert:", 4);
    console.log(rebind[4]);
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[4, 2, 0, 3, 1]\n10\nassert: 10\n2\nassert: 2\n\
         4\nassert: 4\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── suffixTreeUkkonen ───

#[test]
fn suffix_tree_ukkonen_sorts_suffixes() {
    let project = AlgorithmProject::new(
        r#"import { suffixTreeUkkonen, SuffixTree } from "tyrus:algorithm";

function runChecks(): void {
    const tree: SuffixTree = suffixTreeUkkonen("banana");
    const indices: usize[] = tree.getSuffixArray();
    console.log(indices);
    console.log(indices.length);
    console.log("assert:", 6);
    console.log(indices[0]);
    console.log("assert:", 5);
    console.log(indices[1]);
    console.log("assert:", 3);
    console.log(indices[2]);
    console.log("assert:", 1);
    console.log(indices[3]);
    console.log("assert:", 0);
    console.log(indices[4]);
    console.log("assert:", 4);
    console.log(indices[5]);
    console.log("assert:", 2);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[5, 3, 1, 0, 4, 2]\n6\nassert: 6\n\
         5\nassert: 5\n3\nassert: 3\n1\nassert: 1\n\
         0\nassert: 0\n4\nassert: 4\n2\nassert: 2\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suffix_tree_edge_cases() {
    let project = AlgorithmProject::new(
        r#"import { suffixTreeUkkonen } from "tyrus:algorithm";

function runChecks(): void {
    const empty = suffixTreeUkkonen("");
    console.log(empty.getSuffixArray().length);
    console.log("assert:", 0);

    const single = suffixTreeUkkonen("a");
    console.log(single.getSuffixArray());
    console.log(single.getSuffixArray()[0]);
    console.log("assert:", 0);

    const repeated = suffixTreeUkkonen("aaaa");
    console.log(repeated.getSuffixArray());
    console.log(repeated.getSuffixArray()[0]);
    console.log("assert:", 3);
    console.log(repeated.getSuffixArray()[3]);
    console.log("assert:", 0);

    const mississippi = suffixTreeUkkonen("mississippi");
    console.log(mississippi.getSuffixArray());
    console.log(mississippi.getSuffixArray()[0]);
    console.log("assert:", 10);
    console.log(mississippi.getSuffixArray()[10]);
    console.log("assert:", 2);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0\nassert: 0\n\
         [0]\n0\nassert: 0\n\
         [3, 2, 1, 0]\n3\nassert: 3\n0\nassert: 0\n\
         [10, 7, 4, 1, 0, 9, 8, 6, 3, 5, 2]\n10\nassert: 10\n2\nassert: 2\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suffix_tree_rebind_and_namespace_import() {
    let project = AlgorithmProject::new(
        r#"import * as alg from "tyrus:algorithm";

function runChecks(): void {
    const first = alg.suffixTreeUkkonen("banana");
    const rebind = alg.suffixTreeUkkonen("cocoa");
    console.log(rebind.getSuffixArray());
    console.log(first.getSuffixArray()[0]);
    console.log("assert:", 5);
    console.log(first.getSuffixArray()[5]);
    console.log("assert:", 2);
    console.log(rebind.getSuffixArray()[0]);
    console.log("assert:", 4);
    console.log(rebind.getSuffixArray()[4]);
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[4, 2, 0, 3, 1]\n5\nassert: 5\n2\nassert: 2\n\
         4\nassert: 4\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn lca_binary_lifting_answers_queries() {
    let project = AlgorithmProject::new(
        r#"import { lcaBinaryLifting, Lca } from "tyrus:algorithm";

function runChecks(): void {
    const adj: usize[][] = [[1, 2], [0, 3, 4], [0, 5, 6], [1], [1], [2], [2]];
    const lca: Lca = lcaBinaryLifting(adj, 0);

    console.log(lca.query(3, 4));  // 1
    console.log("assert:", 1);
    console.log(lca.query(5, 6));  // 2
    console.log("assert:", 2);
    console.log(lca.query(3, 6));  // 0
    console.log("assert:", 0);
    console.log(lca.query(0, 2));  // 0
    console.log("assert:", 0);
    console.log(lca.query(4, 5));  // 0
    console.log("assert:", 0);
    console.log(lca.query(1, 1));  // 1
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1\nassert: 1\n2\nassert: 2\n0\nassert: 0\n\
         0\nassert: 0\n0\nassert: 0\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn lca_edge_cases_and_deep_paths() {
    let project = AlgorithmProject::new(
        r#"import { lcaBinaryLifting, Lca } from "tyrus:algorithm";

function runChecks(): void {
    const single: usize[][] = [[]];
    const l1: Lca = lcaBinaryLifting(single, 0);
    console.log(l1.query(0, 0));  // 0
    console.log("assert:", 0);

    const path: usize[][] = [[1], [0, 2], [1, 3], [2, 4], [3, 5], [4, 6], [5, 7], [6, 8], [7, 9], [8]];
    const lp: Lca = lcaBinaryLifting(path, 0);
    console.log(lp.query(9, 1));  // 1
    console.log("assert:", 1);
    console.log(lp.query(9, 8));  // 8
    console.log("assert:", 8);
    console.log(lp.query(5, 9));  // 5
    console.log("assert:", 5);
    console.log(lp.query(0, 9));  // 0
    console.log("assert:", 0);
    console.log(lp.query(9, 9));  // 9
    console.log("assert:", 9);

    const root: usize[][] = [[1, 2], [0], [0]];
    const lr: Lca = lcaBinaryLifting(root, 1);
    console.log(lr.query(2, 1));  // 1
    console.log("assert:", 1);
    console.log(lr.query(2, 0));  // 0
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0\nassert: 0\n\
         1\nassert: 1\n8\nassert: 8\n5\nassert: 5\n\
         0\nassert: 0\n9\nassert: 9\n\
         1\nassert: 1\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn lca_rebind_and_namespace_import() {
    let project = AlgorithmProject::new(
        r#"import * as alg from "tyrus:algorithm";

function runChecks(): void {
    const first = alg.lcaBinaryLifting([[1], [0, 2], [1]], 0);
    const rebind = alg.lcaBinaryLifting([[1, 2, 3], [0, 4], [0, 5], [0], [1], [2]], 0);
    console.log(rebind.query(4, 5));  // 0
    console.log("assert:", 0);
    console.log(first.query(1, 2));  // 1
    console.log("assert:", 1);
    console.log(rebind.query(4, 2));  // 0
    console.log("assert:", 0);
    console.log(first.query(0, 2));  // 0
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0\nassert: 0\n1\nassert: 1\n0\nassert: 0\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn heavy_light_decomposition_answers_path_queries() {
    let project = AlgorithmProject::new(
        r#"import { heavyLightDecomposition, Hld } from "tyrus:algorithm";

function runChecks(): void {
    const adj: usize[][] = [[1, 2], [0, 3, 4], [0, 5, 6], [1], [1], [2], [2]];
    const hld: Hld = heavyLightDecomposition(adj, 0);

    const r1 = hld.queryPath(3, 4);
    console.log(r1);  // path 3-1-4
    const r2 = hld.queryPath(3, 6);
    console.log(r2);  // path 3-1-0-2-6
    const r3 = hld.queryPath(5, 4);
    console.log(r3);  // path 5-2-0-1-4
    console.log(hld.position(3));
    console.log("assert:", 2);
    console.log(hld.position(2));
    console.log("assert:", 4);
    console.log(hld.head(3));
    console.log("assert:", 0);
    console.log(hld.head(5));
    console.log("assert:", 2);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[[3, 3], [1, 2]]\n\
         [[6, 6], [4, 4], [0, 2]]\n\
         [[4, 5], [3, 3], [0, 1]]\n\
         2\nassert: 2\n4\nassert: 4\n0\nassert: 0\n2\nassert: 2\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn heavy_light_decomposition_edge_cases_and_deep_paths() {
    let project = AlgorithmProject::new(
        r#"import { heavyLightDecomposition, Hld } from "tyrus:algorithm";

function runChecks(): void {
    const single: usize[][] = [[]];
    const h1: Hld = heavyLightDecomposition(single, 0);
    const r1: [usize, usize][] = h1.queryPath(0, 0);
    console.log(r1);  // [[0, 0]]
    const exp1: [usize, usize][] = [[0, 0]];
    console.log("assert:", exp1);
    console.log(h1.position(0));  // 0
    console.log("assert:", 0);
    console.log(h1.head(0));  // 0
    console.log("assert:", 0);

    const path: usize[][] = [[1], [0, 2], [1, 3], [2, 4], [3, 5], [4, 6], [5, 7], [6, 8], [7, 9], [8]];
    const hp: Hld = heavyLightDecomposition(path, 0);
    const rp1: [usize, usize][] = hp.queryPath(9, 1);
    console.log(rp1);  // single chain range
    const expp1: [usize, usize][] = [[1, 9]];
    console.log("assert:", expp1);
    const rp2: [usize, usize][] = hp.queryPath(0, 9);
    console.log(rp2);  // whole tree
    const expp2: [usize, usize][] = [[0, 9]];
    console.log("assert:", expp2);
    const rp3: [usize, usize][] = hp.queryPath(9, 9);
    console.log(rp3);  // single vertex
    const expp3: [usize, usize][] = [[9, 9]];
    console.log("assert:", expp3);

    const root: usize[][] = [[1, 2], [0], [0]];
    const hr: Hld = heavyLightDecomposition(root, 1);
    const rr1: [usize, usize][] = hr.queryPath(2, 1);
    console.log(rr1);  // root != 0
    const expr1: [usize, usize][] = [[0, 2]];
    console.log("assert:", expr1);
    const rr2: [usize, usize][] = hr.queryPath(2, 0);
    console.log(rr2);  // leaf to root
    const expr2: [usize, usize][] = [[1, 2]];
    console.log("assert:", expr2);
    console.log(hr.position(1));  // root vertex is position 0
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[[0, 0]]\nassert: [[0, 0]]\n0\nassert: 0\n0\nassert: 0\n\
         [[1, 9]]\nassert: [[1, 9]]\n\
         [[0, 9]]\nassert: [[0, 9]]\n\
         [[9, 9]]\nassert: [[9, 9]]\n\
         [[0, 2]]\nassert: [[0, 2]]\n\
         [[1, 2]]\nassert: [[1, 2]]\n\
         0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn heavy_light_decomposition_rebind_and_namespace_import() {
    let project = AlgorithmProject::new(
        r#"import * as alg from "tyrus:algorithm";

function runChecks(): void {
    const first = alg.heavyLightDecomposition([[1], [0, 2], [1]], 0);
    const rebind = alg.heavyLightDecomposition([[1, 2, 3], [0, 4], [0, 5], [0], [1], [2]], 0);
    console.log(rebind.queryPath(4, 5));  // path 4-1-0-2-5
    console.log(first.queryPath(1, 2));  // path 1-0-2
    console.log(rebind.queryPath(4, 2));  // path 4-1-0-2
    console.log(first.queryPath(0, 2));  // path 0-2
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[[4, 5], [0, 2]]\n\
         [[1, 2]]\n\
         [[4, 4], [0, 2]]\n\
         [[0, 2]]\n"
    );
    assert!(output.stderr.is_empty());
}

// ─── Centroid ───

#[test]
fn centroid_decomposition_parent_and_depth() {
    let project = AlgorithmProject::new(
        r#"import { centroidDecomposition, Centroid } from "tyrus:algorithm";

function runChecks(): void {
    const adj: usize[][] = [[1, 2], [0, 3, 4], [0, 5, 6], [1], [1], [2], [2]];
    const ct: Centroid = centroidDecomposition(adj);

    // The whole tree (7 vertices) is centered at vertex 0.
    console.log(ct.parentOf(0) ?? 99);  // global centroid has no parent
    console.log("assert:", 99);
    console.log(ct.depth(0));
    console.log("assert:", 0);

    // Subtree {1, 3, 4} is centered at vertex 1, leaves hang below it.
    console.log(ct.parentOf(3) ?? 0);
    console.log("assert:", 1);
    console.log(ct.depth(1));
    console.log("assert:", 1);
    console.log(ct.depth(3));
    console.log("assert:", 2);

    // Subtree {2, 5, 6} is centered at vertex 2.
    console.log(ct.parentOf(5) ?? 0);
    console.log("assert:", 2);
    console.log(ct.depth(5));
    console.log("assert:", 2);
    console.log(ct.depth(2));
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "99\nassert: 99\n0\nassert: 0\n1\nassert: 1\n1\nassert: 1\n2\nassert: 2\n2\nassert: 2\n2\nassert: 2\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn centroid_decomposition_edge_cases_and_deep_paths() {
    let project = AlgorithmProject::new(
        r#"import { centroidDecomposition, Centroid } from "tyrus:algorithm";

function runChecks(): void {
    const single: usize[][] = [[]];
    const cs: Centroid = centroidDecomposition(single);
    console.log(cs.parentOf(0) ?? 99);  // single vertex is its own centroid
    console.log("assert:", 99);
    console.log(cs.depth(0));
    console.log("assert:", 0);

    const star: usize[][] = [[1, 2, 3], [0], [0], [0]];
    const cst: Centroid = centroidDecomposition(star);
    console.log(cst.parentOf(0) ?? 99);  // center is the unique centroid
    console.log("assert:", 99);
    console.log(cst.parentOf(2) ?? 0);
    console.log("assert:", 0);
    console.log(cst.depth(2));
    console.log("assert:", 1);

    const path: usize[][] = [[1], [0, 2], [1, 3], [2, 4], [3, 5], [4]];
    const cp: Centroid = centroidDecomposition(path);
    console.log(cp.parentOf(2) ?? 99);  // path of 6 is centered at vertex 2
    console.log("assert:", 99);
    console.log(cp.parentOf(0) ?? 0);
    console.log("assert:", 1);
    console.log(cp.parentOf(4) ?? 0);
    console.log("assert:", 2);
    console.log(cp.depth(0));
    console.log("assert:", 2);
    console.log(cp.depth(1));
    console.log("assert:", 1);
    console.log(cp.depth(4));
    console.log("assert:", 1);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "99\nassert: 99\n0\nassert: 0\n\
         99\nassert: 99\n0\nassert: 0\n1\nassert: 1\n\
         99\nassert: 99\n1\nassert: 1\n2\nassert: 2\n2\nassert: 2\n1\nassert: 1\n1\nassert: 1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn centroid_decomposition_rebind_and_namespace_import() {
    let project = AlgorithmProject::new(
        r#"import * as alg from "tyrus:algorithm";

function runChecks(): void {
    const first = alg.centroidDecomposition([[1], [0, 2], [1]]);
    const rebind = alg.centroidDecomposition([[1, 2, 3], [0, 4], [0, 5], [0], [1], [2]]);
    console.log(rebind.parentOf(4) ?? 0);  // subtree {1, 4} centered at 1
    console.log(rebind.parentOf(1) ?? 0);  // root centroid 0
    console.log(first.parentOf(1) ?? 99);  // path of 3 centered at 1
    console.log(first.depth(0));
}
const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "1\n0\n99\n1\n");
    assert!(output.stderr.is_empty());
}

// ─── BloomFilter ───

#[test]
fn bloom_filter_insert_and_contains() {
    let project = AlgorithmProject::new(
        r#"import { BloomFilter } from "tyrus:algorithm";

function runChecks(): void {
    const bf: BloomFilter = BloomFilter(3, 256);

    bf.insert("apple");
    bf.insert("banana");
    bf.insert("cherry");
    bf.insert("date");

    // Inserted keys are always reported as members (no false negatives).
    console.log(bf.contains("apple"));
    console.log("assert:", true);
    console.log(bf.contains("banana"));
    console.log("assert:", true);
    console.log(bf.contains("cherry"));
    console.log("assert:", true);
    console.log(bf.contains("date"));
    console.log("assert:", true);

    // With k=3 and m=256 these absent keys hit at least one clear bit.
    console.log(bf.contains("egg"));
    console.log("assert:", false);
    console.log(bf.contains("fig"));
    console.log("assert:", false);
    console.log(bf.contains("grape"));
    console.log("assert:", false);
    console.log(bf.contains("honey"));
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\ntrue\nassert: true\ntrue\nassert: true\ntrue\nassert: true\n\
         false\nassert: false\nfalse\nassert: false\nfalse\nassert: false\nfalse\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn bloom_filter_separate_filters_rebind_and_namespace_import() {
    let project = AlgorithmProject::new(
        r#"import * as alg from "tyrus:algorithm";

function runChecks(): void {
    const first = alg.BloomFilter(4, 1024);
    first.insert("apple");
    console.log(first.contains("apple"));
    console.log("assert:", true);

    const empty = alg.BloomFilter(4, 1024);
    console.log(empty.contains("apple"));  // a fresh filter has every bit clear
    console.log("assert:", false);

    const rebind = alg.BloomFilter(4, 1024);
    rebind.insert("kiwi");
    console.log(rebind.contains("kiwi"));
    console.log("assert:", true);
    console.log(rebind.contains("apple"));  // separate filters stay independent
    console.log("assert:", false);
    console.log(first.contains("kiwi"));
    console.log("assert:", false);
}
const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\nfalse\nassert: false\ntrue\nassert: true\nfalse\nassert: false\nfalse\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn bloom_filter_zero_parameters_abort() {
    let project = AlgorithmProject::new(
        r#"import { BloomFilter } from "tyrus:algorithm";

const main = (): void => {
    BloomFilter(0, 128);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires k > 0 hash functions"));
}

#[test]
fn bloom_filter_zero_bits_abort() {
    let project = AlgorithmProject::new(
        r#"import { BloomFilter } from "tyrus:algorithm";

const main = (): void => {
    BloomFilter(3, 0);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires m > 0 bits"));
}

#[test]
fn bloom_filter_reports_arity_and_unknown_method_errors() {
    let project = AlgorithmProject::new(
        r#"import { BloomFilter } from "tyrus:algorithm";

const main = (): void => {
    const bf = BloomFilter(3, 256);
    const bad: BloomFilter = BloomFilter(3);
    const wrong: BloomFilter = BloomFilter("apple", 256);
    const badMethod: boolean = bf.insertX("apple");
    const tooFew: boolean = bf.contains();
    console.log(bf.contains("apple"));
};
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "build should fail: stdout:\n{}",
        String::from_utf8_lossy(&build.stdout)
    );
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("`BloomFilter` expects 2 arguments, but 1 were provided"),
        "missing constructor arity error:\n{stderr}"
    );
    assert!(
        stderr.contains("no method named `insertX` for `BloomFilter`; use `insert` or `contains`"),
        "missing unknown-method error:\n{stderr}"
    );
    assert!(
        stderr.contains("`contains` expects 1 argument(s), but 0 were provided"),
        "missing method arity error:\n{stderr}"
    );
}

#[test]
fn skip_list_insert_contains_remove_min_max() {
    let project = AlgorithmProject::new(
        r#"import { SkipList } from "tyrus:algorithm";

function runChecks(): void {
    const s = SkipList<i32>();
    console.log(s.isEmpty());
    console.log("assert:", true);

    console.log(s.insert(3));
    console.log("assert:", true);
    s.insert(1);
    s.insert(2);
    console.log(s.insert(2));
    console.log("assert:", false);
    console.log(s.len());
    console.log("assert:", 3);
    console.log(s.contains(2));
    console.log("assert:", true);
    console.log(s.contains(9));
    console.log("assert:", false);

    console.log(s.min() ?? 0);
    console.log("assert:", 1);
    console.log(s.max() ?? 0);
    console.log("assert:", 3);

    console.log(s.remove(2));
    console.log("assert:", true);
    console.log(s.remove(9));
    console.log("assert:", false);
    console.log(s.contains(2));
    console.log("assert:", false);
    console.log(s.len());
    console.log("assert:", 2);

    s.clear();
    console.log(s.isEmpty());
    console.log("assert:", true);
    console.log(s.min() ?? 0);
    console.log("assert:", 0);
    console.log(s.len());
    console.log("assert:", 0);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n\
         true\nassert: true\nfalse\nassert: false\n\
         3\nassert: 3\ntrue\nassert: true\nfalse\nassert: false\n\
         1\nassert: 1\n3\nassert: 3\n\
         true\nassert: true\nfalse\nassert: false\nfalse\nassert: false\n2\nassert: 2\n\
         true\nassert: true\n0\nassert: 0\n0\nassert: 0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn skip_list_string_elements_rebind_and_namespace_import() {
    let project = AlgorithmProject::new(
        r#"import * as alg from "tyrus:algorithm";

function runChecks(): void {
    const s = alg.SkipList<string>();
    s.insert("banana");
    s.insert("apple");
    s.insert("cherry");
    s.insert("apple");
    console.log(s.len());
    console.log("assert:", 3);
    console.log(s.contains("apple"));
    console.log("assert:", true);
    console.log(s.min() ?? "none");
    console.log("assert:", "apple");
    console.log(s.max() ?? "none");
    console.log("assert:", "cherry");

    s.remove("apple");
    console.log(s.contains("apple"));
    console.log("assert:", false);
    console.log(s.min() ?? "none");
    console.log("assert:", "banana");

    const empty = alg.SkipList<string>();
    console.log(empty.isEmpty());
    console.log("assert:", true);
    console.log(empty.min() ?? "none");
    console.log("assert:", "none");

    const rebind = alg.SkipList<i32>();
    rebind.insert(42);
    console.log(rebind.contains(42));
    console.log("assert:", true);
    console.log(rebind.contains(1));
    console.log("assert:", false);
    console.log(s.isEmpty());
    console.log("assert:", false);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\ntrue\nassert: true\n\
         apple\nassert: apple\ncherry\nassert: cherry\n\
         false\nassert: false\nbanana\nassert: banana\n\
         true\nassert: true\nnone\nassert: none\n\
         true\nassert: true\nfalse\nassert: false\nfalse\nassert: false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn skip_list_reports_type_arity_and_unknown_method_errors() {
    let project = AlgorithmProject::new(
        r#"import { SkipList } from "tyrus:algorithm";

const main = (): void => {
    const missingType = SkipList();
    const withArgs = SkipList<i32>(3);
    const badElement = SkipList<boolean[]>();
    const s = SkipList<i32>();
    const badMethod: boolean = s.insertX(1);
    const tooFew: boolean = s.contains();
    const wrongKind: boolean = s.insert("apple");
    console.log(s.contains(1));
};
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "build should fail: stdout:\n{}",
        String::from_utf8_lossy(&build.stdout)
    );
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("`SkipList` requires exactly one explicit element type"),
        "missing missing-type-argument error:\n{stderr}"
    );
    assert!(
        stderr.contains("`SkipList` expects 0 arguments, but 1 were provided"),
        "missing constructor arity error:\n{stderr}"
    );
    assert!(
        stderr.contains("`SkipList` element type must be an ordered type (numeric, boolean, or string), found `boolean[]`"),
        "missing unorderable element type error:\n{stderr}"
    );
    assert!(
        stderr.contains("no method named `insertX` for `SkipList<i32>`; use `insert`, `remove`, `contains`, `min`, `max`, `len`, `isEmpty`, or `clear`"),
        "missing unknown-method error:\n{stderr}"
    );
    assert!(
        stderr.contains("`contains` expects 1 argument(s), but 0 were provided"),
        "missing method arity error:\n{stderr}"
    );
    assert!(
        stderr.contains("type mismatch for `insert`: expected `i32`, found `string`"),
        "missing method argument type error:\n{stderr}"
    );
}

#[test]
fn graph_edges_map_edges_map_nodes_string_nodes() {
    let project = AlgorithmProject::new(
        r#"import { Graph } from "tyrus:algorithm";

function runChecks(): void {
    const g = Graph<string, i32>();
    g.addNode("a");
    g.addNode("b");
    g.addNode("c");
    g.addEdge("a", "b", 1);
    g.addEdge("b", "c", 2);
    g.addEdge("a", "c", 3);

    const edges = g.edges();
    console.log(edges.length);
    console.log("assert:", 3);
    console.log(edges[0][0], edges[0][1], edges[0][2]);
    console.log("assert:", "a b 1");
    console.log(edges[1][0], edges[1][1], edges[1][2]);
    console.log("assert:", "a c 3");
    console.log(edges[2][0], edges[2][1], edges[2][2]);
    console.log("assert:", "b c 2");
    console.log(g.edges().length);
    console.log("assert:", 3);

    const doubled = g.mapEdges((w: i32): i32 => w * 2);
    console.log(doubled.nodeCount(), doubled.edgeCount());
    console.log("assert:", 3, 3);
    console.log(doubled.edge("a", "b") ?? 0);
    console.log("assert:", 2);
    console.log(doubled.edge("b", "c") ?? 0);
    console.log("assert:", 4);
    console.log(doubled.edge("a", "c") ?? 0);
    console.log("assert:", 6);
    console.log(doubled.edge("x", "y") ?? 0);
    console.log("assert:", 0);

    const upper = g.mapNodes((n: string): string => n.toUpperCase());
    console.log(upper.nodeCount(), upper.edgeCount());
    console.log("assert:", 3, 3);
    console.log(upper.hasNode("A"), upper.hasNode("B"), upper.hasNode("C"));
    console.log("assert:", true, true, true);
    console.log(upper.edge("A", "B") ?? 0);
    console.log("assert:", 1);
    console.log(upper.edge("B", "C") ?? 0);
    console.log("assert:", 2);
    console.log(upper.edge("A", "C") ?? 0);
    console.log("assert:", 3);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\n\
         a b 1\nassert: a b 1\n\
         a c 3\nassert: a c 3\n\
         b c 2\nassert: b c 2\n\
         3\nassert: 3\n\
         3 3\nassert: 3 3\n\
         2\nassert: 2\n4\nassert: 4\n6\nassert: 6\n0\nassert: 0\n\
         3 3\nassert: 3 3\n\
         true true true\nassert: true true true\n\
         1\nassert: 1\n2\nassert: 2\n3\nassert: 3\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn graph_edges_map_edges_map_nodes_numeric_nodes() {
    let project = AlgorithmProject::new(
        r#"import { Graph } from "tyrus:algorithm";

function runChecks(): void {
    const g = Graph<i32, f64>();
    g.addEdge(1, 2, 1.5);
    g.addEdge(2, 3, 2.5);
    g.addEdge(1, 3, 3.5);

    const edges = g.edges();
    console.log(edges.length);
    console.log("assert:", 3);
    console.log(edges[0][0], edges[0][1], edges[0][2]);
    console.log("assert:", "1 2 1.5");
    console.log(edges[1][0], edges[1][1], edges[1][2]);
    console.log("assert:", "1 3 3.5");
    console.log(edges[2][0], edges[2][1], edges[2][2]);
    console.log("assert:", "2 3 2.5");

    const scaled = g.mapEdges((w: f64): f64 => w * 10.0);
    console.log(scaled.edge(1, 2) ?? 0.0);
    console.log("assert:", 15.0);
    console.log(scaled.edge(2, 3) ?? 0.0);
    console.log("assert:", 25.0);
    console.log(scaled.edge(1, 3) ?? 0.0);
    console.log("assert:", 35.0);

    const halved = g.mapNodes((n: i32): i32 => n / 2);
    console.log(halved.hasNode(0), halved.hasNode(1));
    console.log("assert:", true, true);
    console.log(halved.edge(0, 1) ?? 0.0);
    console.log("assert:", 3.5);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\nassert: 3\n\
         1 2 1.5\nassert: 1 2 1.5\n\
         1 3 3.5\nassert: 1 3 3.5\n\
         2 3 2.5\nassert: 2 3 2.5\n\
         15.0\nassert: 15.0\n25.0\nassert: 25.0\n35.0\nassert: 35.0\n\
         true true\nassert: true true\n\
         3.5\nassert: 3.5\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn graph_full_api_string_nodes() {
    let project = AlgorithmProject::new(
        r#"import { Graph } from "tyrus:algorithm";

function runChecks(): void {
    const g = Graph<string, i32>();
    g.addNode("a");
    g.addNode("b");
    g.addNode("c");
    g.addEdge("a", "b", 1);
    g.addEdge("b", "c", 2);
    g.addEdge("a", "c", 3);
    console.log(g.nodeCount(), g.edgeCount());
    console.log("assert:", 3, 3);
    console.log(g.isEmpty());
    console.log("assert:", false);
    console.log(g.hasNode("a"), g.hasNode("d"));
    console.log("assert:", true, false);
    console.log(g.hasEdge("a", "b"), g.hasEdge("b", "a"), g.hasEdge("a", "d"));
    console.log("assert:", true, false, false);
    console.log(g.edge("a", "b") ?? 0);
    console.log("assert:", 1);
    console.log(g.edge("b", "a") ?? 0);
    console.log("assert:", 0);
    console.log(g.degree("a"), g.degree("b"), g.degree("d"));
    console.log("assert:", 2, 1, 0);
    const nb = g.neighbors("a");
    console.log(nb.length, nb[0], nb[1]);
    console.log("assert:", 2, "b c");
    const none = g.neighbors("d");
    console.log(none.length);
    console.log("assert:", 0);
    const nodes = g.nodes();
    console.log(nodes.length, nodes[0], nodes[1], nodes[2]);
    console.log("assert:", 3, "a b c");
    console.log(g.removeEdge("a", "b"));
    console.log("assert:", true);
    console.log(g.removeEdge("a", "b"));
    console.log("assert:", false);
    console.log(g.hasEdge("a", "b"), g.edge("a", "b") ?? 0, g.edgeCount());
    console.log("assert:", false, 0, 2);
    console.log(g.removeNode("b"));
    console.log("assert:", true);
    console.log(g.removeNode("b"));
    console.log("assert:", false);
    console.log(g.nodeCount(), g.edgeCount(), g.hasNode("b"));
    console.log("assert:", 2, 1, false);
    const rev = g.reverse();
    console.log(rev.nodeCount(), rev.edgeCount());
    console.log("assert:", 2, 1);
    console.log(rev.hasEdge("c", "a"), rev.hasEdge("a", "c"));
    console.log("assert:", true, false);
    console.log(g.toString());
    console.log("assert:", 'Graph { "a" -> "c" [label="3"] }');
    console.log(g.toDot());
    console.log("assert:", 'digraph {\n    "a" -> "c" [label="3"];\n}');
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3 3\nassert: 3 3\n\
         false\nassert: false\n\
         true false\nassert: true false\n\
         true false false\nassert: true false false\n\
         1\nassert: 1\n0\nassert: 0\n\
         2 1 0\nassert: 2 1 0\n\
         2 b c\nassert: 2 b c\n\
         0\nassert: 0\n\
         3 a b c\nassert: 3 a b c\n\
         true\nassert: true\nfalse\nassert: false\n\
         false 0 2\nassert: false 0 2\n\
         true\nassert: true\nfalse\nassert: false\n\
         2 1 false\nassert: 2 1 false\n\
         2 1\nassert: 2 1\n\
         true false\nassert: true false\n\
         Graph { \"a\" -> \"c\" [label=\"3\"] }\nassert: Graph { \"a\" -> \"c\" [label=\"3\"] }\n\
         digraph {\n    \"a\" -> \"c\" [label=\"3\"];\n}\n\
         \nassert: digraph {\n    \"a\" -> \"c\" [label=\"3\"];\n}\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn a_collection_returned_from_a_function_is_released_by_its_caller() {
    // A `tyrus:algorithm` handle returned out of a function used to leak: the
    // constructor registered its result correctly, but nothing carried that
    // ownership across the call boundary, so the caller's binding claimed
    // nothing and no scope exit ever freed it
    // (`tests/leaks/clean/algorithm-collection-escaping-by-return.tyr`).
    //
    // The risk in fixing it is the opposite failure, which is worse than the
    // leak: a caller that claims a handle it does not own frees memory the
    // callee's caller still holds. `passthrough` is here for exactly that,
    // it returns its own parameter, so the handle it hands back is borrowed,
    // and a claim on it would be a double free of `made` at scope exit. The
    // program using both afterwards is what proves the two are told apart.
    let project = AlgorithmProject::new(
        r#"
import { SortedSet, Deque } from "tyrus:algorithm";

function makeSet(): SortedSet<i32> {
    const fresh: SortedSet<i32> = SortedSet<i32>();
    fresh.insert(4);
    return fresh;
}

function passthrough(borrowed: SortedSet<i32>): SortedSet<i32> {
    return borrowed;
}

function makeDeque(): Deque<i32> {
    const fresh: Deque<i32> = Deque<i32>();
    fresh.pushBack(7);
    return fresh;
}

function runChecks(): void {
    const made: SortedSet<i32> = makeSet();
    const same: SortedSet<i32> = passthrough(made);
    const queue: Deque<i32> = makeDeque();

    console.log(made.contains(4));
    console.log("assert:", true);
    console.log(same.contains(4));
    console.log("assert:", true);
    console.log(queue.peekBack() ?? 0);
    console.log("assert:", 7);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\n\
         true\nassert: true\n\
         7\nassert: 7\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn returning_out_of_a_for_of_over_a_native_iterable_keeps_the_element_alive() {
    // The array behind `g.nodes()` is released when the loop ends, and a
    // `return` inside the loop never reaches that block, it leaked
    // (`tests/leaks/clean/for-of-return-native-iterable.tyr`). Releasing it on
    // the return path instead raises the opposite question, which is what this
    // asserts: the value being returned lives *inside* the array about to be
    // freed, so the answer has to be a clone, not the original pointer. A
    // wrong answer here is a use-after-free, and the returned name comes back
    // empty or garbled rather than as the node it read.
    let project = AlgorithmProject::new(
        r#"
import { Graph } from "tyrus:algorithm";

function firstNode(g: Graph<string, i32>): string {
    for (const node of g.nodes()) {
        return node;
    }
    return "";
}

function countNodes(g: Graph<string, i32>): i32 {
    let seen: i32 = 0;
    for (const node of g.nodes()) {
        seen += 1;
        if (seen === 2) {
            return seen;
        }
    }
    return seen;
}

function runChecks(): void {
    const g: Graph<string, i32> = Graph<string, i32>();
    g.addNode("alpha");
    g.addNode("beta");
    g.addNode("gamma");

    console.log(firstNode(g));
    console.log("assert:", "alpha");
    console.log(countNodes(g));
    console.log("assert:", 2);
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "alpha\nassert: alpha\n2\nassert: 2\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn a_closure_inside_a_for_of_does_not_release_its_callers_iterable() {
    // Releasing the `for...of` iterable on the return path introduced a hazard
    // the other ownership stacks already guard against: a closure body is
    // emitted while the enclosing function's loop is still on the stack, so a
    // `return` inside the closure would emit that release into a *different*
    // LLVM function, a cross-function reference. Confirmed by building a
    // compiler without the swap: `invalid LLVM IR: "Instruction does not
    // dominate all uses"` on exactly this program.
    let project = AlgorithmProject::new(
        r#"
import { Graph } from "tyrus:algorithm";

const firstNode = (g: Graph<string, i32>): string => {
    for (const node of g.nodes()) {
        const pick = (): string => {
            return node;
        };
        return pick();
    }
    return "";
};

const main = (): void => {
    let total: i32 = 0;
    for (let i: i32 = 0; i < 3; i += 1) {
        const g = Graph<string, i32>();
        g.addNode("alpha");
        g.addNode("beta");
        total += firstNode(g).length as i32;
    }
    console.log(total);
    console.log("assert:", 15);
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "15\nassert: 15\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn a_throw_caught_in_the_same_function_leaves_outer_bindings_alone() {
    // `emit_throw` released every owned binding in the function, including
    // those declared outside the `try` that catches it, and the code after
    // the `catch` released them again. Two `free` calls on one buffer, with a
    // read of that buffer in between. The assertion that matters is the one
    // after the `catch`: reading `items.length` there was a use-after-free,
    // and it now reports the length the array actually has.
    let project = AlgorithmProject::new(
        r#"
function boom(): void throws string {
    let owned: i32[] = [];
    owned.push(1);
    throw "left";
}

function runChecks(): void {
    let items: i32[] = [];
    items.push(1);
    items.push(2);

    try {
        throw "caught";
    } catch (e: string) {
        console.log(e);
        console.log("assert:", "caught");
    }

    console.log(items.length);
    console.log("assert:", 2);

    try {
        boom();
    } catch (e: string) {
        console.log(e);
        console.log("assert:", "left");
    }
}

const main = (): void => {
    runChecks();
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "caught\nassert: caught\n2\nassert: 2\nleft\nassert: left\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn path_algorithms_read_a_graph_literal_written_in_the_call_with_the_declared_layout() {
    // The literal once took the type its own numbers suggested: an empty `[]`
    // left it without one and did not compile, and a negative weight made it
    // nested integer arrays that the runtime read as `[usize, i32]` pairs, out
    // of bounds.
    let project = AlgorithmProject::new(
        r#"import { AlgorithmError, bellmanFord, floydWarshall } from "tyrus:algorithm";

const main = (): void => {
    match (bellmanFord([[[1, 2]], [[2, -1]], []], 0)) {
        Ok(shortest) => { console.log(shortest[0]); }
        Err(error) => { console.log(error.code); }
    }
    match (bellmanFord([[[1, 1]], [[0, -3]]], 0)) {
        Ok(cycle) => { console.log(cycle[0]); }
        Err(error) => { console.log(error.code); }
    }
    const distances = floydWarshall([[0, 4], [1, 0]]);
    console.log(distances[0][1] ?? -1);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[0, 2, 1]\nERR_ALGORITHM_NEGATIVE_CYCLE\n4\n"
    );
}
