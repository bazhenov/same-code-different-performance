#![feature(fn_align)]

use paste::paste;
use same_code_different_performance::make_asm_nops;
use std::{hint::black_box, io::Write, time::Instant};

// Creates __asm_nops() functions with sequence of NOP instructions. The number of instructions
// is given in NOP_COUNT env variable at compile time
make_asm_nops!();

/// This factorial function must always be inlined to produce different aligned version of the same function
#[inline(always)]
fn factorial<const N: u64>(mut n: u64) -> u64 {
    // The linker is smart enough to collapse identical functions into a single one.
    // This is dummy code needed to prevent the linker from doing that.
    unsafe { std::ptr::read_volatile(&N) };

    let mut m = 1u64;
    while n > 1 {
        m = m.saturating_mul(n);
        n -= 1;
        unsafe {
            // Those nops are dummy payload to produce the loop of a specific length
            // The number of nops is the same for all the versions of factorial functions.
            // But because different functions have different alignment in memory the loops are
            // also aligned differently. This has significant impact on the performance.
            __asm_nops();
        }
    }
    m
}

macro_rules! factorial {
    ($n:expr, $ctx:ident) => {
        paste! {
            #[inline(never)]
            #[cfg_attr(feature = "align", repr(align(32)))]
            fn [<factorial_ $n>](n: u64) -> u64 {
                factorial::<$n>(n)
            }
        }
    };
}

// Helper macro to produce the same code multiple times with different values
macro_rules! define_multiple {
    ($macro:ident, $ctx:ident, $n:expr) => {
        $macro!($n, $ctx);
    };
    ($macro:ident, $ctx:ident, $n:expr, $($rest:expr),*) => {
        $macro!($n, $ctx);
        define_multiple!($macro, $ctx, $($rest),*);
    };
}

// Defining multiple identical factorial functions with different names
define_multiple!(factorial, skip, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);

#[cfg(feature = "criterion")]
mod criterion_support {
    use super::*;
    use criterion::{black_box, Criterion};
    use std::time::Duration;

    macro_rules! factorial_benchmark {
        ($n:expr, $ctx:ident) => {
            paste! {
                $ctx.bench_function(concat!("factorial_", $n), |b| b.iter(|| [<factorial_ $n>](black_box(100))));
            }
        };
    }

    pub fn bench(c: &mut Criterion) {
        let mut g = c.benchmark_group("factorials");

        define_multiple!(factorial_benchmark, g, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
    }
}

#[cfg(feature = "criterion")]
criterion::criterion_group!(benches, criterion_support::bench);

#[cfg(feature = "criterion")]
criterion::criterion_main!(benches);

#[cfg(not(feature = "criterion"))]
fn main() {
    use rand::{seq::SliceRandom, thread_rng};
    use same_code_different_performance::nop_count;
    use std::io::stderr;

    let mut rnd = thread_rng();

    let mut min = u64::max_value();
    let mut max = u64::min_value();

    let mut functions: [(usize, fn(u64) -> u64); 10] = [
        (1, factorial_1),
        (2, factorial_2),
        (3, factorial_3),
        (4, factorial_4),
        (5, factorial_5),
        (6, factorial_6),
        (7, factorial_7),
        (8, factorial_8),
        (9, factorial_9),
        (10, factorial_10),
    ];

    // randomizing function run order to get rid of the "first function is the slowest" effect
    functions.shuffle(&mut rnd);

    for (i, f) in functions.into_iter() {
        let value = measure(f);
        writeln!(stderr(), "factorial_{} = {}", i, value).unwrap();
        min = min.min(value);
        max = max.max(value);
    }

    println!(
        "NOP_COUNT={} max-min difference = {}",
        nop_count!(),
        max - min
    )
}

#[cfg(not(feature = "criterion"))]
#[inline(never)]
fn measure(f: fn(u64) -> u64) -> u64 {
    const SAMPLES: usize = 10000;
    const SAMPLE_SIZE: usize = 100;
    let mut min = u64::max_value();

    // Warm up iterations to familiarize CPU with the code
    for _ in 0..(SAMPLES / 10) {
        black_box(f(black_box(100)));
    }

    for _ in 0..SAMPLES {
        let time = Instant::now();
        for _ in 0..SAMPLE_SIZE {
            black_box(f(black_box(100)));
        }
        let time = time.elapsed().as_nanos() as u64 / SAMPLE_SIZE as u64;

        // Measuring minimum execution time as a measure of the performance.
        // For more information about why and when it is appropriate see:
        //  https://betterprogramming.pub/the-mean-misleads-why-the-minimum-is-the-true-measure-of-a-functions-run-time-47fa079075b0
        min = min.min(time);
    }

    min
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_equavalent() {
        // Sanechecking that all the factorial functions are producing the same results
        for i in 1..10 {
            assert_eq!(factorial_1(i), factorial_10(i));
        }
    }
}
