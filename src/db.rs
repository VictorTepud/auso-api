use sqlx::sqlite::{SqlitePoolOptions, SqlitePool};
use std::path::Path;

pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool, migrations_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let migration_file = Path::new(migrations_path);
    if migration_file.exists() {
        let sql = std::fs::read_to_string(migration_file)?;
        sqlx::raw_sql(&sql).execute(pool).await?;
        tracing::info!("Migraciones ejecutadas correctamente");
    } else {
        tracing::warn!("Archivo de migración no encontrado: {}", migrations_path);
    }
    Ok(())
}
