use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ssh_at_core::keys::scanner::scan_keys;
use std::fs;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn generate_test_keys(dir: &std::path::Path, count: usize) {
    // Create mock private key files for benchmarking
    for i in 0..count {
        let _key_type = match i % 4 {
            0 => "RSA",
            1 => "ED25519",
            2 => "ECDSA",
            _ => "DSA",
        };

        let content = format!(
            "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACBKjU0ZKqxCY7qxT9PiK4{}test_key_{}
-----END OPENSSH PRIVATE KEY-----",
            i, i
        );

        let key_path = dir.join(format!("test_key_{}", i));
        fs::write(&key_path, content).unwrap();
    }
}

fn benchmark_scan_keys(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("scan_10_keys", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let ssh_dir = temp_dir.path().join(".ssh");
            fs::create_dir(&ssh_dir).unwrap();
            generate_test_keys(&ssh_dir, 10);

            std::env::set_var("HOME", temp_dir.path());
            rt.block_on(async { black_box(scan_keys().await.unwrap()) })
        })
    });

    c.bench_function("scan_50_keys", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let ssh_dir = temp_dir.path().join(".ssh");
            fs::create_dir(&ssh_dir).unwrap();
            generate_test_keys(&ssh_dir, 50);

            std::env::set_var("HOME", temp_dir.path());
            rt.block_on(async { black_box(scan_keys().await.unwrap()) })
        })
    });

    c.bench_function("scan_100_keys", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let ssh_dir = temp_dir.path().join(".ssh");
            fs::create_dir(&ssh_dir).unwrap();
            generate_test_keys(&ssh_dir, 100);

            std::env::set_var("HOME", temp_dir.path());
            rt.block_on(async { black_box(scan_keys().await.unwrap()) })
        })
    });
}

criterion_group!(benches, benchmark_scan_keys);
criterion_main!(benches);
