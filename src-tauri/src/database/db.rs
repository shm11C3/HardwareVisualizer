use crate::utils;
use sqlx::sqlite::SqlitePool;

pub async fn get_pool() -> Result<SqlitePool, sqlx::Error> {
  let dir_path = utils::file::get_app_data_dir("hv-database.db");
  let database_url = format!("sqlite:{dir_path}", dir_path = dir_path.to_str().unwrap());

  let pool = SqlitePool::connect(&database_url).await?;

  Ok(pool)
}
