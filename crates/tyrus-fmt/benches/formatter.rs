use std::hint::black_box;
use std::time::{Duration, Instant};

use tyrus_fmt::{
    benchmark::{
        anchor_comments, build_line_map, parse_program, print_program, reinsert_comments,
        remove_unused_imports, sort_imports,
    },
    comments::extract_comments,
    config::Config,
    format_source,
};

fn option_value(name: &str, default: usize) -> usize {
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == name {
            return args
                .next()
                .unwrap_or_else(|| panic!("missing value for {name}"))
                .parse()
                .unwrap_or_else(|_| panic!("invalid value for {name}"));
        }
    }
    default
}

fn report(name: &str, iterations: usize, mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    let percentile = |fraction: f64| {
        let index = ((fraction * samples.len() as f64).ceil() as usize)
            .saturating_sub(1)
            .min(samples.len() - 1);
        samples[index]
    };
    let p50 = percentile(0.50);
    let p95 = percentile(0.95);
    println!(
        "{name}: p50={:.3}ms p95={:.3}ms n={iterations}",
        p50.as_secs_f64() * 1000.0,
        p95.as_secs_f64() * 1000.0
    );
    p95
}

fn measure(name: &str, iterations: usize, mut operation: impl FnMut()) -> Duration {
    for _ in 0..3 {
        operation();
    }
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        operation();
        samples.push(started.elapsed());
    }
    report(name, iterations, samples)
}

fn measure_batched<T>(
    name: &str,
    iterations: usize,
    mut setup: impl FnMut() -> T,
    mut operation: impl FnMut(T),
) -> Duration {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let input = setup();
        let started = Instant::now();
        operation(input);
        samples.push(started.elapsed());
    }
    report(name, iterations, samples)
}

fn main() {
    let lines = option_value("--lines", 1000).max(4);
    let comment_every = option_value("--comment-every", 2);
    let pipe_depth = option_value("--pipe-depth", 0);
    let iterations = option_value("--iterations", 20).max(1);
    let enforce_budget = std::env::args().any(|argument| argument == "--enforce-budget");
    let mut source = String::from(
        "import { unused } from \"./unused\";\nconst double = (value: i32): i32 => value * 2;\nconst main = (): void => {\n",
    );
    for index in 0..lines {
        if comment_every > 0 && index % comment_every == 0 {
            source.push_str(&format!("    // Formatter benchmark comment {index}\n"));
        }
        if pipe_depth == 0 {
            source.push_str(&format!("    print(double({index}));\n"));
        } else {
            source.push_str(&format!("    print({index}"));
            for _ in 0..pipe_depth {
                source.push_str(" |> double");
            }
            source.push_str(");\n");
        }
    }
    source.push_str("};\n");
    let comment_count = source
        .lines()
        .filter(|line| line.trim_start().starts_with("//"))
        .count();
    println!(
        "fixture: sourceLines={} comments={comment_count} pipeDepth={pipe_depth} iterations={iterations}",
        source.lines().count()
    );
    let config = Config::default();
    let comments = extract_comments(&source);
    let anchored = anchor_comments(&source, &comments);
    let program = parse_program(&source).expect("benchmark fixture parses");
    let formatted = print_program(&source, &program, &config);
    let line_map = build_line_map(&source, &formatted);

    measure("comment extraction", iterations, || {
        black_box(extract_comments(black_box(&source)));
    });
    measure("comment anchoring", iterations, || {
        black_box(anchor_comments(black_box(&source), black_box(&comments)));
    });
    measure("parsing", iterations, || {
        black_box(parse_program(black_box(&source)));
    });
    measure_batched(
        "unused import removal",
        iterations,
        || program.clone(),
        |mut program| remove_unused_imports(&mut program),
    );
    measure_batched(
        "import sorting",
        iterations,
        || program.clone(),
        |mut program| sort_imports(&mut program),
    );
    measure("printing", iterations, || {
        black_box(print_program(&source, &program, &config));
    });
    measure("line map", iterations, || {
        black_box(build_line_map(black_box(&source), black_box(&formatted)));
    });
    measure("comment reinsertion", iterations, || {
        black_box(reinsert_comments(
            &source, &formatted, &comments, &anchored, &line_map,
        ));
    });
    let formatter_p95 = measure("complete formatter", iterations, || {
        black_box(format_source(black_box(&source), &config));
    });
    if enforce_budget {
        let source_lines = source.lines().count();
        let budget = if pipe_depth >= 8 && source_lines > 2000 {
            Duration::from_millis(75)
        } else if source_lines <= 200 {
            Duration::from_millis(2)
        } else if source_lines <= 2000 {
            Duration::from_millis(9)
        } else {
            Duration::from_millis(42)
        };
        assert!(
            formatter_p95 <= budget,
            "complete formatter p95 {:.3}ms exceeded the {:.3}ms budget for {source_lines} lines",
            formatter_p95.as_secs_f64() * 1000.0,
            budget.as_secs_f64() * 1000.0
        );
        println!(
            "formatter p95 budget: PASS ({:.3}ms)",
            budget.as_secs_f64() * 1000.0
        );
    }
}
