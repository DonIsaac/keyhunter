use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oxc::allocator::Allocator;
use ureq;

use keyhunter::{ApiKeyCollector, ApiKeyExtractor};

fn benchmark_monaco(c: &mut Criterion) {
    const URL: &'static str =
        "https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.47.0/min/vs/editor/editor.main.js";
    let source_text = ureq::get(URL).call().unwrap().into_string().unwrap();
    let collector = ApiKeyExtractor::default();
    // let alloc = Allocator::default();
    c.bench_function("monaco", |b| {
        b.iter_with_large_drop(||  {
            let alloc = Allocator::default();
            let keys = collector.extract_api_keys(&alloc, black_box(&source_text));
            drop(keys);
            alloc
        })
    });
}
criterion_group!(benches, benchmark_monaco);
criterion_main!(benches);
