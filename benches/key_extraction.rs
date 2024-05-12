use std::{
    fs,
    path::{self, Path, PathBuf},
};

use codspeed_criterion_compat::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion,
};
use miette::{Context as _, IntoDiagnostic};
use oxc::allocator::Allocator;

use keyhunter::ApiKeyExtractor;

#[cfg(not(tarpaulin_include))]
fn fixtures() -> PathBuf {
    let filepath = path::PathBuf::from(file!()).canonicalize().unwrap();

    let fixtures = filepath.parent().unwrap().join("fixtures");
    assert!(fixtures.is_absolute());
    assert!(fixtures.is_dir());

    fixtures
}

#[cfg(not(tarpaulin_include))]
fn read_js_fixtures<P: AsRef<Path>>(fixtures_dir: P) -> Vec<(PathBuf, String)> {
    let fixtures = fs::read_dir(fixtures_dir.as_ref())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|ent| ent.file_type().is_ok_and(|ft| ft.is_file()))
        .map(|ent| ent.path())
        .map(|p| {
            let src = fs::read_to_string(&p)
                .into_diagnostic()
                .with_context(|| format!("Could not open {}", p.display()))
                .unwrap();
            (p, src)
        })
        .collect::<Vec<_>>();
    assert!(!fixtures.is_empty());

    fixtures
}

// fn benchmark_monaco(c: &mut Criterion) {
//     const URL: &str =
//         "https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.47.0/min/vs/editor/editor.main.js";
//     let source_text = ureq::get(URL).call().unwrap().into_string().unwrap();
//     let collector = ApiKeyExtractor::default();
//     // let alloc = Allocator::default();
//     c.bench_function("ApiKeyExtractor::extract_api_keys", |b| {
//         b.iter_with_large_drop(|| {
//             let alloc = Allocator::default();
//             let keys = collector.extract_api_keys(&alloc, black_box(&source_text));
//             drop(keys);
//             alloc
//         })
//     });
// }

#[cfg(not(tarpaulin_include))]
fn benchmark_microbenches(c: &mut Criterion) {
    let fixture_path = fixtures().join("microbenches");
    let microbenches = read_js_fixtures(fixture_path);

    let extractor = ApiKeyExtractor::default();
    let mut group = c.benchmark_group("microbenches");

    for (page_path, source_text) in microbenches {
        let filename = page_path.file_name().unwrap();
        group.sample_size(200).bench_with_input(
            BenchmarkId::new("extract_api_keys ", filename.to_str().unwrap()),
            &source_text,
            |b, source_text| {
                b.iter_with_large_drop(|| {
                    let alloc = Allocator::default();
                    let keys = extractor.extract_api_keys(&alloc, black_box(source_text));
                    drop(keys);
                    alloc
                });
            },
        );
    }
    group.finish()
}

#[cfg(not(tarpaulin_include))]
fn benchmark_page_samples(c: &mut Criterion) {
    let fixture_path = fixtures().join("website-chunks");
    let website_chunks = read_js_fixtures(fixture_path);

    let extractor = ApiKeyExtractor::default();
    let mut group = c.benchmark_group("Page samples from the wild");

    for (page_path, source_text) in website_chunks {
        let filename = page_path.file_name().unwrap();
        group.bench_with_input(
            BenchmarkId::new("extract_api_keys ", filename.to_str().unwrap()),
            &source_text,
            |b, source_text| {
                b.iter_with_large_drop(|| {
                    let alloc = Allocator::default();
                    let keys = extractor.extract_api_keys(&alloc, black_box(source_text));
                    drop(keys);
                    alloc
                });
            },
        );
    }
    group.finish()
}

#[cfg(not(tarpaulin_include))]
fn benchmark_js_libs(c: &mut Criterion) {
    let libs = ["https://unpkg.com/browse/three@0.77.0/three.js"]
        .iter()
        .map(|url| {
            let source_text = ureq::get(url).call().unwrap().into_string().unwrap();
            (*url, source_text)
        })
        .collect::<Vec<_>>();

    let mut group = c.benchmark_group("js_libraries");
    let extractor = ApiKeyExtractor::default();

    for (lib_url, source_text) in libs {
        let filename = lib_url.split('/').last().unwrap();
        group.bench_function(filename, |b| {
            b.iter_with_large_drop(|| {
                let alloc = Allocator::default();
                let keys = extractor.extract_api_keys(&alloc, black_box(&source_text));
                drop(keys);
                alloc
            });
        });
    }

    group.finish()
}

criterion_group!(
    name = key_collection;
    config = Criterion::default();
    targets = benchmark_microbenches, benchmark_js_libs, benchmark_page_samples
    // targets = benchmark_monaco, benchmark_page_samples
);
#[cfg(not(tarpaulin_include))]
criterion_main!(key_collection);
