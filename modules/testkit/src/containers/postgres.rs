use testcontainers::{ContainerAsync, GenericImage, ImageExt as _, core::WaitFor, runners::AsyncRunner};

use crate::Result;

pub struct PostgresContainer {
    #[allow(unused)]
    pub container: ContainerAsync<GenericImage>,
    pub connection_string: String,
}

impl PostgresContainer {
    #[allow(unused)]
    pub async fn new(tag: &str) -> Result<Self> {
        let db = "postgres-db-test";
        let user = "postgres-user-test";
        let password = "postgres-password-test";

        let container = GenericImage::new("postgres", tag)
            .with_wait_for(WaitFor::message_on_stderr("database system is ready to accept connections"))
            .with_env_var("POSTGRES_DB", db)
            .with_env_var("POSTGRES_USER", user)
            .with_env_var("POSTGRES_PASSWORD", password)
            .start()
            .await?;

        let connection_string = format!(
            "postgres://{}:{}@127.0.0.1:{}/{}",
            user,
            password,
            container.get_host_port_ipv4(5432).await?,
            db
        );

        Ok(Self {
            container,
            connection_string,
        })
    }
}
