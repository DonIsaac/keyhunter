use std::time::Duration;

use codspeed_criterion_compat::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion,
};
use miette::{Context as _, IntoDiagnostic};
use oxc::allocator::Allocator;

use keyhunter::ApiKeyExtractor;

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

fn benchmark_page_samples(c: &mut Criterion) {
    use std::{fs, path};

    let fixure_path = path::PathBuf::from(file!())
        .canonicalize()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures");

    // println!("{}", fixure_path.display());
    let page_samples = fs::read_dir(fixure_path)
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
    assert!(!page_samples.is_empty());

    let extractor = ApiKeyExtractor::default();
    let mut group = c.benchmark_group("Page samples from the wild");

    for (page_path, source_text) in page_samples {
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

// fn benchmark_vercel(c: &mut Criterion) {
//     use rayon::prelude::*;
//     use std::thread;

//     const URL: &str = "https://vercel.com/";

//     let (walker, receiver) = WebsiteWalkBuilder::default()
//         .with_max_walks(1)
//         .build_with_channel();

//     let script_handle: thread::JoinHandle<Vec<(Url, String)>> = thread::spawn(move || {
//         receiver
//             .into_iter()
//             .flatten()
//             .fold(vec![], |mut acc, scripts| {
//                 acc.extend(scripts);
//                 acc
//             })
//             .into_par_iter()
//             .take(5)
//             .map(|script_url| {
//                 let script = ureq::get(script_url.as_str())
//                     .call()
//                     .into_diagnostic()
//                     .unwrap()
//                     .into_string()
//                     .into_diagnostic()
//                     .unwrap();
//                 (script_url, script)
//             })
//             .collect()
//     });
//     walker.walk(URL).unwrap();
//     let scripts = script_handle.join().unwrap();

//     let group = c
//         .benchmark_group("Vercel")
//         .measurement_time(Duration::from_secs(120));
//     let collector = ApiKeyExtractor::default();
//     for (url, script) in scripts {
//         c.measurement_time(Duration::from_secs(120))
//             .bench_with_input(
//                 BenchmarkId::new("extract_api_keys", url),
//                 &script,
//                 |b, source_text| {
//                     b.iter_with_large_drop(|| {
//                         let alloc = Allocator::default();
//                         let keys = collector.extract_api_keys(&alloc, black_box(source_text));
//                         drop(keys);
//                         alloc
//                     })
//                 },
//             );
//     }
// }
criterion_group!(
    name = key_collection;
    config = Criterion::default().measurement_time(Duration::from_secs(20));
    targets = benchmark_page_samples
    // targets = benchmark_monaco, benchmark_page_samples
);
criterion_main!(key_collection);
