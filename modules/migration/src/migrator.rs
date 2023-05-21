use std::collections::HashSet;

use chrono::NaiveDateTime;

pub struct Migrator {
    client: tokio_postgres::Client,
    path: String,
    username: String,
    description: String,
}

impl Migrator {
    pub async fn new(
        url: &str,
        path: &str,
        username: &str,
        description: &str,
    ) -> anyhow::Result<Migrator> {
        // Get DB client and connection
        let (client, connection) = tokio_postgres::connect(url, tokio_postgres::NoTls).await?;

        // Spawn connection
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("Connection error: {}", error);
            }
        });

        Ok(Migrator {
            client,
            path: path.to_string(),
            username: username.to_string(),
            description: description.to_string(),
        })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        self.init().await?;

        let histories: Vec<MigrationHistory> = self.fetch_migration_histories().await?;
        let ignore_set: HashSet<String> = histories.iter().map(|n| n.filename.clone()).collect();

        let files: Vec<MigrationFile> = self.load_migration_files().await?;
        let files: Vec<MigrationFile> = files
            .into_iter()
            .filter(|x| !ignore_set.contains(x.filename.as_str()))
            .collect();

        if files.is_empty() {
            return Ok(());
        }

        self.semaphore_lock().await?;
        let result = self.execute_migration_queries(files).await;
        self.semaphore_unlock().await?;

        result
    }

    async fn init(&self) -> anyhow::Result<()> {
        let queries = "\
create table if not exists _migrations (
    filename varchar(255) NOT NULL,
    queries text NOT NULL,
    executed_at timestamp without time zone default CURRENT_TIMESTAMP,
    primary key (filename)
);
create table if not exists _semaphores (
    username varchar(255) NOT NULL,
    description text NOT NULL,
    executed_at timestamp without time zone default CURRENT_TIMESTAMP,
    primary key (username)
);
";
        self.client.batch_execute(queries).await?;

        Ok(())
    }

    async fn load_migration_files(&self) -> anyhow::Result<Vec<MigrationFile>> {
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
            let result = MigrationFile {
                filename: name,
                queries,
            };

            results.push(result);
        }

        Ok(results)
    }

    async fn fetch_migration_histories(&self) -> anyhow::Result<Vec<MigrationHistory>> {
        let mut results: Vec<MigrationHistory> = Vec::new();

        let query = "\
select filename, executed_at from _migrations
";
        let rows = self.client.query(query, &[]).await?;

        for row in rows {
            let result = MigrationHistory {
                filename: row.get("filename"),
                executed_at: row.get("executed_at"),
            };
            results.push(result);
        }

        Ok(results)
    }

    async fn execute_migration_queries(&self, files: Vec<MigrationFile>) -> anyhow::Result<()> {
        for f in files {
            self.client.batch_execute(&f.queries).await?;
            self.insert_migration_history(&f.filename, &f.queries)
                .await?;
        }

        Ok(())
    }

    async fn insert_migration_history(&self, filename: &str, queries: &str) -> anyhow::Result<()> {
        let statement = "\
insert into _migrations (filename, queries) values ($1, $2)
";
        self.client
            .execute(statement, &[&filename, &queries])
            .await?;

        Ok(())
    }

    async fn semaphore_lock(&self) -> anyhow::Result<()> {
        let query = "\
insert into _semaphores (username, description) values ($1, $2)
";
        self.client
            .execute(query, &[&self.username, &self.description])
            .await?;

        Ok(())
    }

    async fn semaphore_unlock(&self) -> anyhow::Result<()> {
        let query = "\
delete from _semaphores where username = $1
";
        self.client.execute(query, &[&self.username]).await?;

        Ok(())
    }
}

struct MigrationFile {
    pub filename: String,
    pub queries: String,
}

struct MigrationHistory {
    pub filename: String,
    #[allow(unused)]
    pub executed_at: NaiveDateTime,
}
