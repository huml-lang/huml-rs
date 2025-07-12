use criterion::{Criterion, black_box, criterion_group, criterion_main};
use huml_rs::serde::Result;

fn benchmark_serde_parse(c: &mut Criterion) {
    #[derive(Debug, serde::Deserialize)]
    struct Config {
        app_name: String,
        port: u16,
        debug: bool,
        features: Vec<String>,
        database: Database,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Database {
        host: String,
        port: u16,
        name: String,
        ssl: bool,
    }

    let huml = r#"
app_name: "My Awesome App"
port: 8080
debug: true
features:: "auth", "logging", "metrics", "caching"
database::
  host: "localhost"
  port: 5432
  name: "myapp_db"
  ssl: true
"#;

    c.bench_function("parse_serde_struct", |b| {
        b.iter(|| {
            let result: Result<Config> = huml_rs::serde::from_str(huml);
            black_box(result)
        });
    });
}

criterion_group!(benches, benchmark_serde_parse);

criterion_main!(benches);
