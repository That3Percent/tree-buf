use criterion::{black_box, criterion_group, criterion_main, Criterion};


/*

fn increasing_floats(c: &mut Criterion) {
    let mut data = Vec::new();
    let mut f = 0.0;
    for i in 0..10000 {
        f += i as f64 * 0.001;
        data.push(f);
    }

    c.bench_function("encode", |b| b.iter_with_large_drop(|| black_box(encode(&data))));

    let data_ser = write(&data);
    c.bench_function("decode", |b| b.iter_with_large_drop(|| black_box(decode::<Vec<f64>>(&data_ser))));
}
criterion_group!(benches, increasing_floats);
*/
use tree_buf::internal::varint::size_for_varint;

fn varints(c: &mut Criterion) {
    let mut data = Vec::new();
    for i in 0..100 {
        data.push(i * i);
    }

    c.bench_function("varint", |b| b.iter(|| black_box(data.iter().cloned().map(size_for_varint).sum::<usize>())));
}

criterion_group!(benches, varints);
criterion_main!(benches);
