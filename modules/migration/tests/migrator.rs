#[cfg(feature = "stable-test")]
#[cfg(test)]
mod tests {
    use omnius_core_migration::Migrator;

    #[tokio::test]
    async fn simple_create_table_test() {
        let docker = testcontainers::clients::Cli::default();
        let container = PostgresContainer::new(&docker);

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

    #[tokio::test]
    async fn create_table_syntax_error_test() {
        let docker = testcontainers::clients::Cli::default();
        let container = PostgresContainer::new(&docker);

        let migrator = Migrator::new(
            &container.connection_string,
            "./tests/cases/create_table_syntax_error",
            "test01",
            "test01_description",
        )
        .await
        .expect("Migrator new error");

        migrator
            .migrate()
            .await
            .expect_err("Error expected but successful.");
    }

    #[tokio::test]
    async fn migrate_twice_test() {
        let docker = testcontainers::clients::Cli::default();
        let container = PostgresContainer::new(&docker);

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

    use testcontainers::{clients::Cli, core::WaitFor, images::generic::GenericImage, Container};

    struct PostgresContainer<'a> {
        #[allow(unused)]
        pub container: Container<'a, GenericImage>,
        pub connection_string: String,
    }

    impl<'a> PostgresContainer<'a> {
        fn new(docker: &'a Cli) -> Self {
            let db = "postgres-db-test";
            let user = "postgres-user-test";
            let password = "postgres-password-test";

            let generic_postgres = GenericImage::new("postgres", "15.1")
                .with_wait_for(WaitFor::message_on_stderr(
                    "database system is ready to accept connections",
                ))
                .with_env_var("POSTGRES_DB", db)
                .with_env_var("POSTGRES_USER", user)
                .with_env_var("POSTGRES_PASSWORD", password);

            let container: Container<'a, GenericImage> = docker.run(generic_postgres);

            let connection_string = format!(
                "postgres://{}:{}@127.0.0.1:{}/{}",
                user,
                password,
                container.get_host_port_ipv4(5432),
                db
            );

            Self {
                container,
                connection_string,
            }
        }
    }
}
