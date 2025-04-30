use std::sync::Once;
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
use tracing_subscriber::{fmt, EnvFilter};

static INIT: Once = Once::new();

pub fn init_tracing() {
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        fmt().with_env_filter(filter).with_test_writer().init();
    });
}

pub struct TestRedis {
    container: Option<ContainerAsync<GenericImage>>,
    host: String,
    port: u16,
}

impl TestRedis {
    pub async fn start() -> Self {
        // Start a Redis container using testcontainers async API
        let container = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(6379.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .with_network("bridge")
            .with_env_var("DEBUG", "1")
            .start()
            .await
            .expect("Failed to start Redis");
        let host_port = container.get_host_port_ipv4(6379).await.unwrap();
        let host = "127.0.0.1".to_string();
        let instance = Self {
            container: Some(container),
            host,
            port: host_port,
        };
        tracing::info!("Started Redis testcontainer at {}", instance.redis_url());
        instance
    }

    pub fn redis_url(&self) -> String {
        format!("redis://{}:{}/", self.host, self.port)
    }

    pub async fn cleanup(&mut self) {
        if let Some(container) = self.container.take() {
            container
                .rm()
                .await
                .expect("Failed to remove Redis container");
        }
    }
}
