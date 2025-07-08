use huml_rs::serde::from_str;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    app_name: String,
    port: u16,
    debug: bool,
    features: Vec<String>,
    database: Database,
}

#[derive(Debug, Deserialize)]
struct Database {
    host: String,
    port: u16,
    name: String,
    ssl: bool,
}

fn main() {
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

    match from_str::<Config>(huml) {
        Ok(config) => {
            println!("Successfully parsed HUML config:");
            println!("{:#?}", config);

            println!("\nAccessing fields:");
            println!("App: {} running on port {}", config.app_name, config.port);
            println!("Debug mode: {}", config.debug);
            println!("Features: {:?}", config.features);
            println!(
                "Database: {}:{}/{}?ssl={}",
                config.database.host,
                config.database.port,
                config.database.name,
                config.database.ssl
            );
        }
        Err(e) => {
            eprintln!("Failed to parse HUML: {}", e);
        }
    }
}
