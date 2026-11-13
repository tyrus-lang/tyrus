use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tyrus_runtime::{TyrusDecimal, tyrus_decimal_add, tyrus_decimal_div, tyrus_decimal_mul};

fn decimal_arithmetic(c: &mut Criterion) {
    let left = TyrusDecimal {
        coefficient: 1,
        exponent: -1,
    };
    let right = TyrusDecimal {
        coefficient: 2,
        exponent: -1,
    };
    let mut group = c.benchmark_group("decimal_arithmetic");
    group.bench_function("decimal_add", |bench| {
        bench.iter(|| unsafe {
            let mut out = TyrusDecimal {
                coefficient: 0,
                exponent: 0,
            };
            tyrus_decimal_add(&mut out, black_box(&left), black_box(&right));
            black_box(out);
        });
    });
    group.bench_function("f64_add", |bench| {
        bench.iter(|| black_box(black_box(0.1_f64) + black_box(0.2_f64)));
    });
    group.bench_function("decimal_mul", |bench| {
        bench.iter(|| unsafe {
            let mut out = TyrusDecimal {
                coefficient: 0,
                exponent: 0,
            };
            tyrus_decimal_mul(&mut out, black_box(&left), black_box(&right));
            black_box(out);
        });
    });
    group.bench_function("decimal_div", |bench| {
        bench.iter(|| unsafe {
            let mut out = TyrusDecimal {
                coefficient: 0,
                exponent: 0,
            };
            tyrus_decimal_div(&mut out, black_box(&left), black_box(&right));
            black_box(out);
        });
    });
    group.finish();
}

criterion_group!(benches, decimal_arithmetic);
criterion_main!(benches);
