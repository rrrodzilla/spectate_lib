use env_logger;
use log::{debug, info, trace};
use spectate_lib::Spectate;

#[test]
fn test_client() {
    std::env::set_var("RUST_LOG", "TRACE");
    //initialize a logger
    let spectate = Spectate::default();
    let _ = env_logger::builder()
        .is_test(true)
        // here we add the custom logging target to env_logger
        .target(spectate.target())
        .try_init();
    info!("Calling info from test_client");
    debug!("Calling debug from test_client");
    trace!("Calling trace from test_client");

    spectate.flush();
    //at this point we should block and wait for the logger to finish sending
}
