use sqlx::sqlite::{SqlitePoolOptions, SqlitePool};
use std::path::Path;

pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    Ok(pool)
}

/// Runs all migration SQL files found in the migrations directory, in alphabetical order.
/// Each file is executed independently; failures in one file don't block the others,
/// which is convenient for additive ALTER TABLE statements that may already be applied.
pub async fn run_migrations(pool: &SqlitePool, migrations_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(migrations_dir);
    if !dir.exists() || !dir.is_dir() {
        tracing::warn!("Directorio de migraciones no encontrado: {}", migrations_dir);
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("sql"))
        .collect();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let sql = std::fs::read_to_string(&path)?;
        match sqlx::raw_sql(&sql).execute(pool).await {
            Ok(_) => tracing::info!("Migración aplicada: {}", path.display()),
            Err(e) => tracing::warn!("Migración omitida/fallida ({}): {}", path.display(), e),
        }
    }

    Ok(())
}
