#[cfg(all(test, feature = "stable-test", feature = "postgres"))]
mod tests {
    use omnius_core_migration::postgres::PostgresMigrator;
    use omnius_core_testkit::containers::postgres::PostgresContainer;
    use serial_test::serial;
    use testresult::TestResult;

    const POSTGRES_VERSION: &str = "15.1";

    #[serial(migrate)]
    #[tokio::test]
    async fn simple_create_table_test() -> TestResult {
        let container = PostgresContainer::new(POSTGRES_VERSION).await?;

        let migrator = PostgresMigrator::new(
            &container.connection_string,
            "./tests/cases/simple_create_table",
            "test01",
            "test01_description",
        )
        .await?;

        migrator.migrate().await?;

        Ok(())
    }

    #[serial(migrate)]
    #[tokio::test]
    async fn create_table_syntax_error_test() -> TestResult {
        let container = PostgresContainer::new(POSTGRES_VERSION).await?;

        let migrator = PostgresMigrator::new(
            &container.connection_string,
            "./tests/cases/create_table_syntax_error",
            "test01",
            "test01_description",
        )
        .await?;

        assert!(migrator.migrate().await.is_err());

        Ok(())
    }

    #[serial(migrate)]
    #[tokio::test]
    async fn migrate_twice_test() -> TestResult {
        let container = PostgresContainer::new(POSTGRES_VERSION).await?;

        let migrator1 = PostgresMigrator::new(
            &container.connection_string,
            "./tests/cases/simple_create_table",
            "test01",
            "test01_description",
        )
        .await?;

        migrator1.migrate().await?;

        let migrator2 = PostgresMigrator::new(
            &container.connection_string,
            "./tests/cases/simple_create_table",
            "test01",
            "test01_description",
        )
        .await?;

        migrator2.migrate().await?;

        Ok(())
    }
}
