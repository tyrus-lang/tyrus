use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::ptr;
use tyrus_runtime::{
    TyrusArray, tyrus_array_unique_values, tyrus_map_free, tyrus_map_group_push,
    tyrus_map_new_grouped,
};

fn unique_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_utils");
    for size in [32_usize, 1024, 16_384] {
        let values = (0..size)
            .map(|value| (value % 257) as i32)
            .collect::<Vec<_>>();
        let baseline_values = (0..size as i32).collect::<Vec<_>>();
        let input = TyrusArray {
            data: values.as_ptr() as *mut u8,
            length: values.len(),
            capacity: values.len(),
        };
        group.bench_with_input(
            BenchmarkId::new("unique_values", size),
            &input,
            |bench, input| {
                bench.iter(|| unsafe {
                    let mut output = TyrusArray {
                        data: ptr::null_mut(),
                        length: 0,
                        capacity: 0,
                    };
                    let status = tyrus_array_unique_values(
                        black_box(input),
                        &mut output,
                        std::mem::size_of::<i32>() as u64,
                        0,
                    );
                    assert_eq!(status, 0);
                    libc::free(output.data.cast());
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("unique_quadratic_baseline", size),
            &baseline_values,
            |bench, values| {
                bench.iter(|| {
                    let mut distinct = Vec::new();
                    for value in values.iter().copied() {
                        if !distinct.contains(&value) {
                            distinct.push(value);
                        }
                    }
                    black_box(distinct);
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("group_by_copy", size),
            &values,
            |bench, values| {
                bench.iter(|| unsafe {
                    let map = tyrus_map_new_grouped(
                        std::mem::size_of::<i32>(),
                        0,
                        std::mem::size_of::<i32>(),
                    );
                    for value in values {
                        let key = *value % 2;
                        assert_eq!(
                            tyrus_map_group_push(
                                map,
                                (&key as *const i32).cast(),
                                (value as *const i32).cast(),
                            ),
                            0
                        );
                    }
                    tyrus_map_free(map);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, unique_benchmark);
criterion_main!(benches);
