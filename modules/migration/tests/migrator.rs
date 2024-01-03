#[cfg(feature = "stable-test")]
#[cfg(test)]
mod tests {
    use core_migration::postgres::Migrator;
    use core_testkit::containers::postgres::PostgresContainer;
    use serial_test::serial;

    const POSTGRES_VERSION: &str = "15.1";

    #[serial(migrate)]
    #[tokio::test]
    async fn simple_create_table_test() {
        let docker = testcontainers::clients::Cli::default();
        let container = PostgresContainer::new(&docker, POSTGRES_VERSION);

        let migrator = Migrator::new(
            &container.connection_string,
            "./tests/cases/simple_create_table",
            "test01",
            "test01_description",
        )
        .await
        .expect("Migrator new error");

        migrator.migrate().await.expect("Migrator migrate error");
    }

    #[serial(migrate)]
    #[tokio::test]
    async fn create_table_syntax_error_test() {
        let docker = testcontainers::clients::Cli::default();
        let container = PostgresContainer::new(&docker, POSTGRES_VERSION);

        let migrator = Migrator::new(
            &container.connection_string,
            "./tests/cases/create_table_syntax_error",
            "test01",
            "test01_description",
        )
        .await
        .expect("Migrator new error");

        migrator.migrate().await.expect_err("Error expected but successful.");
    }

    #[serial(migrate)]
    #[tokio::test]
    async fn migrate_twice_test() {
        let docker = testcontainers::clients::Cli::default();
        let container = PostgresContainer::new(&docker, POSTGRES_VERSION);

        let migrator1 = Migrator::new(
            &container.connection_string,
            "./tests/cases/simple_create_table",
            "test01",
            "test01_description",
        )
        .await
        .expect("Migrator new error");

        migrator1.migrate().await.expect("Migrator migrate error");

        let migrator2 = Migrator::new(
            &container.connection_string,
            "./tests/cases/simple_create_table",
            "test01",
            "test01_description",
        )
        .await
        .expect("Migrator new error");

        migrator2.migrate().await.expect("Migrator migrate error");
    }
}
