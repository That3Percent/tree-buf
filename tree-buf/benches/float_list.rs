use tree_buf::prelude::*;


use criterion::{black_box, criterion_group, criterion_main, Criterion};



fn increasing_floats(c: &mut Criterion) {
    let mut data = Vec::new();
    let mut f = 0.0;
    for i in 0..10000 {
        f += i as f64 * 0.001;
        data.push(f);
    }

    

    c.bench_function("write", |b| b.iter_with_large_drop(|| black_box(write(&data))));

    let data_ser = write(&data);
    c.bench_function("read", |b| b.iter_with_large_drop(|| black_box(read::<Vec<f64>>(&data_ser))));
}

criterion_group!(benches, increasing_floats);
criterion_main!(benches);
