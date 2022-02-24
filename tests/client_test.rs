use env_logger::Env;
use log::{debug, info, trace};
use spectate_lib::Spectate;

#[test]
fn test_client() {
    //initialize a logger
    //let spectate = Spectate::new();
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("client_test=trace"))
        .is_test(true)
        // here we add the custom logging target to env_logger
        .target(Spectate::target())
        .try_init();
    for _ in 1..10 {
        //std::thread::sleep(Duration::from_secs(1));
        info!("Calling info from test_client");
        debug!("Calling debug from test_client");
        trace!("Calling trace from test_client");
        //   spectate.flush();
    }

    //at this point we should block and wait for the logger to finish sending
}
