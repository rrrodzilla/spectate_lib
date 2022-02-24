use env_logger::Env;
use log::{debug, info, trace};
use spectate_lib::Spectate;

#[test]
fn test_client() {
    //initialize a logger
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("client_test=trace"))
        .is_test(true)
        // here we add spectate as a custom target
        .target(Spectate::target())
        .try_init();

    //generate a bunch of log messages
    for _ in 1..10 {
        info!("Calling info from test_client");
        debug!("Calling debug from test_client");
        trace!("Calling trace from test_client");
    }
}
