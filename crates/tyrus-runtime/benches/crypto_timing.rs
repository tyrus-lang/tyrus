use std::hint::black_box;
use std::time::Instant;
use tyrus_runtime::crypto_bench;

const INPUT_BYTES: usize = 1024;
const SAMPLES_PER_CLASS: usize = 10_000;
const COMPARISONS_PER_SAMPLE: usize = 64;
const T_THRESHOLD: f64 = 10.0;

#[derive(Default)]
struct Moments {
    count: u64,
    mean: f64,
    squared_deviation: f64,
}

impl Moments {
    fn observe(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        self.squared_deviation += delta * (value - self.mean);
    }

    fn variance(&self) -> f64 {
        self.squared_deviation / (self.count - 1) as f64
    }
}

fn measure_batch(left: &[u8], right: &[u8]) -> f64 {
    let started = Instant::now();
    for _ in 0..COMPARISONS_PER_SAMPLE {
        black_box(crypto_bench::timing_safe_equal(
            black_box(left),
            black_box(right),
        ));
    }
    started.elapsed().as_nanos() as f64
}

fn welch_t(first: &Moments, second: &Moments) -> f64 {
    let standard_error =
        (first.variance() / first.count as f64 + second.variance() / second.count as f64).sqrt();
    if standard_error == 0.0 {
        0.0
    } else {
        (first.mean - second.mean) / standard_error
    }
}

fn main() {
    let baseline = [0x5au8; INPUT_BYTES];
    let mut early_mismatch = baseline;
    let mut late_mismatch = baseline;
    early_mismatch[0] ^= 1;
    late_mismatch[INPUT_BYTES - 1] ^= 1;

    for _ in 0..10_000 {
        black_box(crypto_bench::timing_safe_equal(
            black_box(&baseline),
            black_box(&early_mismatch),
        ));
    }

    let mut early = Moments::default();
    let mut late = Moments::default();
    let mut random_state = 0x4d595df4d0f33173u64;
    while early.count < SAMPLES_PER_CLASS as u64 || late.count < SAMPLES_PER_CLASS as u64 {
        random_state ^= random_state << 13;
        random_state ^= random_state >> 7;
        random_state ^= random_state << 17;
        let select_early = random_state & 1 == 0;
        if select_early && early.count < SAMPLES_PER_CLASS as u64 {
            early.observe(measure_batch(&baseline, &early_mismatch));
        } else if late.count < SAMPLES_PER_CLASS as u64 {
            late.observe(measure_batch(&baseline, &late_mismatch));
        }
    }

    let t = welch_t(&early, &late);
    println!(
        "timingSafeEqual early mismatch mean: {:.2} ns/batch",
        early.mean
    );
    println!(
        "timingSafeEqual late mismatch mean: {:.2} ns/batch",
        late.mean
    );
    println!("Welch t statistic: {t:.3}");
    println!("Evidence threshold: |t| <= {T_THRESHOLD:.1}");

    if !t.is_finite() || t.abs() > T_THRESHOLD {
        eprintln!(
            "timingSafeEqual produced timing evidence above the configured threshold; repeat on an isolated, frequency-pinned CPU before interpreting the result"
        );
        std::process::exit(1);
    }
}
