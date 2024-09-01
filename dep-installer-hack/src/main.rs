#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn main(
    #[shuttle_secrets::Secrets] secret_store: shuttle_secrets::SecretStore,
) -> Result<impl shuttle_runtime::Service, shuttle_runtime::Error> {
    let pkgs =
        "libfontconfig1-dev libgraphite2-dev libharfbuzz-dev libicu-dev libssl-dev zlib1g-dev poppler-utils";

    if std::env::var("HOSTNAME")
        .unwrap_or_default()
        .contains("shuttle")
    {
        if !std::process::Command::new("apt")
            .arg("install")
            .arg("-y")
            .args(pkgs.split(' '))
            .status()
            .expect("failed to run apt")
            .success()
        {
            panic!("failed to install dependencies");
        }

        std::process::Command::new("pdftocairo")
            .arg("-h")
            .status()
            .expect("pdftocairo to be runnable");
    } else {
        panic!("Run on shuttle")
    }

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
