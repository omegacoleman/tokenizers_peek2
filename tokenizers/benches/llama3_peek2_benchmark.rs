#[macro_use]
extern crate criterion;

mod common;

use common::{iter_bench_encode_batch, iter_bench_train};
use criterion::{Criterion, Throughput, SamplingMode};
use std::{hint::black_box, time::Duration};
use tokenizers::{
    models::{bpe::BpeTrainerBuilder, TrainerWrapper},
    pre_tokenizers::peektwo::PeekTwo,
    pre_tokenizers::split::Split,
    EncodeInput, Tokenizer, PreTokenizer, PreTokenizedString
};

static BATCH_SIZE: usize = 1_000;

fn run_pretokenizer_benches(c: &mut Criterion, prefix: &str, dataset: &str, pretok: &(impl PreTokenizer + std::marker::Sync)) {
    let data = std::fs::read_to_string(format!("peek2data/{}_big.txt", dataset)).unwrap();
    let mut group = c.benchmark_group(format!("pretok-{}-{}", prefix, dataset));
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("pretok", |b| {
        let mut pretokenized = PreTokenizedString::from(black_box(data.clone()));
        b.iter(|| {
            pretok.pre_tokenize(&mut pretokenized).unwrap()
        })
    });
}

pub fn pretok_original(c: &mut Criterion) {
    let split_s = r#"
    {
        "type": "Split",
        "pattern": {
          "Regex": "(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?\\p{L}+|\\p{N}{1,3}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+"
        },
        "behavior": "Isolated",
        "invert": false
    }"#;
    let pretok = serde_json::from_str::<Split>(split_s).unwrap();
    run_pretokenizer_benches(c, "original", "en", &pretok);
    run_pretokenizer_benches(c, "original", "cn", &pretok);
    run_pretokenizer_benches(c, "original", "code", &pretok);
    run_pretokenizer_benches(c, "original", "math", &pretok);
}

pub fn pretok_peek2(c: &mut Criterion) {
    let pretok = PeekTwo::new().unwrap();
    run_pretokenizer_benches(c, "peek2", "en", &pretok);
    run_pretokenizer_benches(c, "peek2", "cn", &pretok);
    run_pretokenizer_benches(c, "peek2", "code", &pretok);
    run_pretokenizer_benches(c, "peek2", "math", &pretok);
}

fn run_llama3_benches(c: &mut Criterion, prefix: &str, dataset: &str, tokenizer_path: &str) {
    let data = std::fs::read_to_string(format!("peek2data/{}_big.txt", dataset)).unwrap();
    let mut group = c.benchmark_group(format!("llama3-{}-{}", prefix, dataset));
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Bytes(data.len() as u64));
    let mut lines: Vec<EncodeInput> = vec![];
    let mut batches: Vec<Vec<EncodeInput>> = vec![vec![]];
    for line in data.lines() {
        let line: EncodeInput = line.into();
        lines.push(line.clone());
        if batches.last().unwrap().len() >= BATCH_SIZE {
            batches.push(vec![]);
        }
        batches.last_mut().unwrap().push(line);
    }
    let tokenizer = Tokenizer::from_file(tokenizer_path).unwrap();
    group.bench_function("encode", |b| {
        b.iter_custom(|iters| iter_bench_encode_batch(iters, &tokenizer, &batches))
    });
    let mut trainer: TrainerWrapper = BpeTrainerBuilder::default()
        .show_progress(false)
        .build()
        .into();
    let mut tokenizer = Tokenizer::from_file(tokenizer_path).unwrap();
    group.bench_function("bpe-training", |b| {
        b.iter_custom(|iters| {
            iter_bench_train(
                iters,
                &mut tokenizer,
                &mut trainer,
                vec![format!("peek2data/{}_big.txt", dataset).to_string()],
            )
        })
    });
    group.finish();
} 

pub fn llama3_original(c: &mut Criterion) {
    run_llama3_benches(c, "original", "en", "peek2data/llama-3-tokenizer-original.json");
    run_llama3_benches(c, "original", "cn", "peek2data/llama-3-tokenizer-original.json");
    run_llama3_benches(c, "original", "code", "peek2data/llama-3-tokenizer-original.json");
    run_llama3_benches(c, "original", "math", "peek2data/llama-3-tokenizer-original.json");
}

pub fn llama3_peek2(c: &mut Criterion) {
    run_llama3_benches(c, "peek2", "en", "peek2data/llama-3-tokenizer-peek2.json");
    run_llama3_benches(c, "peek2", "cn", "peek2data/llama-3-tokenizer-peek2.json");
    run_llama3_benches(c, "peek2", "code", "peek2data/llama-3-tokenizer-peek2.json");
    run_llama3_benches(c, "peek2", "math", "peek2data/llama-3-tokenizer-peek2.json");
}

criterion_group! {
    name = llama_3;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10));
    targets = llama3_original, llama3_peek2, pretok_original, pretok_peek2
}

criterion_main!(llama_3);
