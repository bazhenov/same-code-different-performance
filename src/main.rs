#![feature(fn_align)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use paste::paste;
use std::time::Duration;

/// This factorial function must always be inlined to produce different aligned version of the same function
#[inline(always)]
fn factorial<const N: u64>(mut n: u64) -> u64 {
    // This is a dummy code needed to prevent from collapsing all the factorial functions into one by linker
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
            #[rustfmt::skip]
            std::arch::asm!{
                "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",

                // This block is using 5 byte long nop instructions which are much
                // more efficient in the terms of uops caching and does not produce such
                // a big performance difference between different versions of the same function.
                // "nop qword ptr [rax + rax]", "nop qword ptr [rax + rax]",
                // "nop qword ptr [rax + rax]", "nop qword ptr [rax + rax]",
                // "nop qword ptr [rax + rax]", "nop qword ptr [rax + rax]",
                // "nop qword ptr [rax + rax]", "nop",

            }
        }
    }
    m
}

macro_rules! factorial_benchmark {
    ($n:expr, $ctx:ident) => {
        paste! {
            $ctx.bench_function(concat!("factorial_", $n), |b| b.iter(|| [<factorial_ $n>](black_box(100))));
        }
    };
}

macro_rules! factorial {
    ($n:expr, $ctx:ident) => {
        paste! {
            #[inline(never)]
            #[repr(align(16))]
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

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("factorials");
    g.measurement_time(Duration::from_secs(1));
    g.warm_up_time(Duration::from_millis(100));

    // Sanechecking that all the factorial functions are producing the same results
    assert_eq!(factorial_1(10), factorial_10(10));

    define_multiple!(factorial_benchmark, g, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
