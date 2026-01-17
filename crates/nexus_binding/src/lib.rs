use napi_derive::napi;
use std::sync::Once;

static INIT: Once = Once::new();

#[napi]
pub fn start_server(root: String, port: u16) -> napi::Result<()> {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    });

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to build tokio runtime: {}", e);
                    return;
                }
            };
            
        if let Err(e) = rt.block_on(nexus_core::start_server(root, port)) {
            tracing::error!("Server caught error: {}", e);
        }
    });
    
    Ok(())
}
