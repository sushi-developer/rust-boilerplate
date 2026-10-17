use sqlx::PgPool;

use crate::errors::app_error::AppError;

/// One `<NNN>_<table>.sql` per table.
const SCHEMA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/dao/schemas");

/// Local only: called by `utils::mocked_db_connection::get_mocked_infra` on the empty local
/// container. Staging and production schemas are managed outside the app.
pub async fn ensure_schema(db: &PgPool) -> Result<(), AppError> {
    let mut files: Vec<_> = std::fs::read_dir(SCHEMA_DIR)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();

    files.sort();
    for file in files {
        sqlx::raw_sql(&std::fs::read_to_string(&file)?)
            .execute(db)
            .await?;
    }
    Ok(())
}
