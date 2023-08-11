use testcontainers::{clients::Cli, core::WaitFor, images::generic::GenericImage, Container};

pub struct PostgresContainer<'a> {
    #[allow(unused)]
    pub container: Container<'a, GenericImage>,
    pub connection_string: String,
}

impl<'a> PostgresContainer<'a> {
    #[allow(unused)]
    pub fn new(docker: &'a Cli, tag: &str) -> Self {
        let db = "postgres-db-test";
        let user = "postgres-user-test";
        let password = "postgres-password-test";

        let generic_postgres = GenericImage::new("postgres", tag)
            .with_wait_for(WaitFor::message_on_stderr("database system is ready to accept connections"))
            .with_env_var("POSTGRES_DB", db)
            .with_env_var("POSTGRES_USER", user)
            .with_env_var("POSTGRES_PASSWORD", password);

        let container: Container<'a, GenericImage> = docker.run(generic_postgres);

        let connection_string = format!("postgres://{}:{}@127.0.0.1:{}/{}", user, password, container.get_host_port_ipv4(5432), db);

        Self {
            container,
            connection_string,
        }
    }
}
