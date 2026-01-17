use napi_derive::napi;

#[napi]
pub fn start_server(root: String, port: u16) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(nexus_core::start_server(root, port));
    });
}
