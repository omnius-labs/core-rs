use std::collections::HashSet;

use chrono::NaiveDateTime;
use sqlx::SqlitePool;

use crate::prelude::*;

pub struct SqliteMigrator;

impl SqliteMigrator {
    pub async fn migrate(db: &SqlitePool, requests: Vec<MigrationRequest>) -> Result<()> {
        Self::init(db).await?;

        let histories = Self::fetch_migration_histories(db).await?;
        let ignore_set: HashSet<String> = histories.iter().map(|n| n.name.clone()).collect();

        let requests: Vec<MigrationRequest> = requests.into_iter().filter(|x| !ignore_set.contains(x.name.as_str())).collect();

        if requests.is_empty() {
            return Ok(());
        }

        Self::execute_migration_queries(db, requests).await?;

        Ok(())
    }

    async fn init(db: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS _migrations (
    name TEXT NOT NULL,
    queries TEXT NOT NULL,
    executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (name)
);
"#,
        )
        .execute(db)
        .await?;

        Ok(())
    }

    async fn fetch_migration_histories(db: &SqlitePool) -> Result<Vec<MigrationHistory>> {
        let res: Vec<MigrationHistory> = sqlx::query_as(
            r#"
SELECT name, executed_at FROM _migrations
"#,
        )
        .fetch_all(db)
        .await?;

        Ok(res)
    }

    async fn execute_migration_queries(db: &SqlitePool, requests: Vec<MigrationRequest>) -> Result<()> {
        for r in requests {
            for query in r.queries.split(';') {
                if query.trim().is_empty() {
                    continue;
                }
                sqlx::query(query).execute(db).await?;
            }

            Self::insert_migration_history(db, r.name.as_str(), r.queries.as_str()).await?;
        }

        Ok(())
    }

    async fn insert_migration_history(db: &SqlitePool, name: &str, queries: &str) -> Result<()> {
        sqlx::query(
            r#"
INSERT INTO _migrations (name, queries) VALUES ($1, $2)
"#,
        )
        .bind(name)
        .bind(queries)
        .execute(db)
        .await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct MigrationRequest {
    pub name: String,
    pub queries: String,
}

#[derive(sqlx::FromRow)]
struct MigrationHistory {
    pub name: String,
    #[allow(unused)]
    pub executed_at: NaiveDateTime,
}

#[cfg(all(test, feature = "stable-test", feature = "sqlite"))]
mod tests {
    use std::{path::Path, sync::Arc};

    use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};
    use testresult::TestResult;

    use super::SqliteMigrator;

    #[tokio::test]
    pub async fn success_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let dir_path = dir.path().as_os_str().to_str().unwrap();

        let path = Path::new(dir_path).join("sqlite.db");
        let path = path.to_str().unwrap();

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let db = Arc::new(SqlitePool::connect_with(options).await?);

        let requests = vec![super::MigrationRequest {
            name: "test".to_string(),
            queries: r#"
CREATE TABLE test (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);
"#
            .to_string(),
        }];

        // Migrate
        SqliteMigrator::migrate(&db, requests.clone()).await.unwrap();

        // Migrate again
        SqliteMigrator::migrate(&db, requests).await.unwrap();

        Ok(())
    }

    #[tokio::test]
    pub async fn error_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let dir_path = dir.path().as_os_str().to_str().unwrap();

        let path = Path::new(dir_path).join("sqlite.db");
        let path = path.to_str().unwrap();

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let db = Arc::new(SqlitePool::connect_with(options).await?);

        let requests = vec![super::MigrationRequest {
            name: "test".to_string(),
            queries: r#"
CREATE TABLE test (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,,,,
);
"#
            .to_string(),
        }];

        assert!(SqliteMigrator::migrate(&db, requests).await.is_err());

        Ok(())
    }
}
