use std::collections::HashSet;

use chrono::NaiveDateTime;
use tracing::info;

use crate::prelude::*;

pub struct PostgresMigrator {
    client: tokio_postgres::Client,
    path: String,
    username: String,
    description: String,
}

impl PostgresMigrator {
    pub async fn new(url: &str, path: &str, username: &str, description: &str) -> Result<PostgresMigrator> {
        // Get DB client and connection
        let (client, connection) = tokio_postgres::connect(url, tokio_postgres::NoTls).await?;

        // Spawn connection
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("Connection error: {}", error);
            }
        });

        Ok(PostgresMigrator {
            client,
            path: path.to_string(),
            username: username.to_string(),
            description: description.to_string(),
        })
    }

    pub async fn migrate(&self) -> Result<()> {
        self.init().await?;

        let histories: Vec<MigrationHistory> = self.fetch_migration_histories().await?;
        let ignore_set: HashSet<String> = histories.iter().map(|n| n.file_name.clone()).collect();

        let files: Vec<MigrationFile> = self.load_migration_files().await?;
        let files: Vec<MigrationFile> = files.into_iter().filter(|x| !ignore_set.contains(x.file_name.as_str())).collect();

        if files.is_empty() {
            return Ok(());
        }

        self.semaphore_lock().await?;
        let res = self.execute_migration_queries(files).await;
        self.semaphore_unlock().await?;

        res
    }

    async fn init(&self) -> Result<()> {
        let queries = "\
CREATE TABLE IF NOT EXISTS _migrations (
    file_name VARCHAR(255) NOT NULL,
    queries TEXT NOT NULL,
    executed_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (file_name)
);
CREATE TABLE IF NOT EXISTS _semaphores (
    username varchar(255) NOT NULL,
    description text NOT NULL,
    executed_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (username)
);
";
        self.client.batch_execute(queries).await?;

        Ok(())
    }

    async fn load_migration_files(&self) -> Result<Vec<MigrationFile>> {
        let mut results: Vec<MigrationFile> = Vec::new();

        for entry in std::fs::read_dir(std::path::Path::new(&self.path))? {
            let entry = entry?;
            let path = entry.path();
            let metadata = std::fs::metadata(&path)?;

            if !metadata.is_file() {
                continue;
            }

            let name: String = path.file_name().unwrap().to_str().unwrap().to_string();
            let queries: String = std::fs::read_to_string(path)?;
            let result = MigrationFile { file_name: name, queries };

            results.push(result);
        }

        results.sort_by(|x, y| x.file_name.cmp(&y.file_name));

        Ok(results)
    }

    async fn fetch_migration_histories(&self) -> Result<Vec<MigrationHistory>> {
        let mut results: Vec<MigrationHistory> = Vec::new();

        let query = "\
SELECT file_name, executed_at FROM _migrations
";
        let rows = self.client.query(query, &[]).await?;

        for row in rows {
            let result = MigrationHistory {
                file_name: row.get("file_name"),
                executed_at: row.get("executed_at"),
            };
            results.push(result);
        }

        Ok(results)
    }

    async fn execute_migration_queries(&self, files: Vec<MigrationFile>) -> Result<()> {
        for f in files {
            self.client.batch_execute(&f.queries).await?;
            self.insert_migration_history(&f.file_name, &f.queries).await?;
            info!(file_name = f.file_name, "processed migration file")
        }

        Ok(())
    }

    async fn insert_migration_history(&self, file_name: &str, queries: &str) -> Result<()> {
        let statement = "\
INSERT INTO _migrations (file_name, queries) VALUES ($1, $2)
";
        self.client.execute(statement, &[&file_name, &queries]).await?;

        Ok(())
    }

    async fn semaphore_lock(&self) -> Result<()> {
        let query = "\
INSERT INTO _semaphores (username, description) VALUES ($1, $2)
";
        self.client.execute(query, &[&self.username, &self.description]).await?;

        Ok(())
    }

    async fn semaphore_unlock(&self) -> Result<()> {
        let query = "\
DELETE FROM _semaphores WHERE username = $1
";
        self.client.execute(query, &[&self.username]).await?;

        Ok(())
    }
}

struct MigrationFile {
    pub file_name: String,
    pub queries: String,
}

struct MigrationHistory {
    pub file_name: String,
    #[allow(unused)]
    pub executed_at: NaiveDateTime,
}
