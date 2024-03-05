#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn main(
    #[shuttle_secrets::Secrets] secret_store: shuttle_secrets::SecretStore,
) -> Result<impl shuttle_runtime::Service, shuttle_runtime::Error> {
    Ok(NoopService)
}

struct NoopService;

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for NoopService {
    async fn bind(self, _addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Ok(())
    }
}
