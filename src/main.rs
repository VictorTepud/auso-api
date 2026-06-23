mod config;
mod db;
mod errors;
mod models;
mod handlers;
mod middleware;
mod routes;
mod services;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Cargar variables de entorno desde el .env del proyecto (forzar override)
    let exe_dir = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let env_path = exe_dir.join(".env");
    if env_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&env_path) {
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, val)) = line.split_once('=') {
                    let key = key.trim();
                    let val = val.trim();
                    if !key.is_empty() {
                        std::env::set_var(key, val);
                    }
                }
            }
        }
    }

    // Inicializar logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "auso_api=debug,actix_web=info".into()),
        )
        .init();

    // Cargar configuración
    let config = config::Config::from_env();
    let server_host = config.server_host.clone();
    let server_port = config.server_port;
    let database_url = config.database_url.clone();

    tracing::info!("🚀 Iniciando AUSO (AURA SOCIAL) API");
    tracing::info!("📦 Base de datos: {}", database_url);
    tracing::info!("🌐 Servidor: {}:{}", server_host, server_port);

    // Inicializar pool de base de datos
    let pool = db::init_pool(&database_url)
        .await
        .expect("Error inicializando base de datos");

    // Ejecutar migraciones (todos los archivos .sql de la carpeta migrations/, en orden)
    let migrations_path = std::env::current_dir()
        .map(|p| p.join("migrations").to_string_lossy().to_string())
        .unwrap_or_else(|_| "./migrations".to_string());

    db::run_migrations(&pool, &migrations_path)
        .await
        .expect("Error ejecutando migraciones");

    // Crear directorios de uploads
    let upload_dir = &config.upload_dir;
    let dirs = [
        format!("{}/images", upload_dir),
        format!("{}/videos", upload_dir),
        format!("{}/videos/temp", upload_dir),
        format!("{}/profiles/profile", upload_dir),
        format!("{}/profiles/cover", upload_dir),
        format!("{}/communities", upload_dir),
        format!("{}/groups", upload_dir),
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir).ok();
    }

    // Compartir estado
    let config_data = web::Data::new(config);
    let pool_data = web::Data::new(pool);

    // Iniciar servidor
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(config_data.clone())
            .app_data(pool_data.clone())
            .configure(routes::configure_routes)
    })
    .bind(format!("{}:{}", server_host, server_port))?
    .run()
    .await
}
