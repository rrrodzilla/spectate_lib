pub use env_logger;
use env_logger::Target;
pub use log;
use spectate::spectate_client::SpectateClient;
use spectate::LogEntry;
use std::thread;
use std::{
    io,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};
use tokio::runtime::Runtime;
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub struct Spectate {
    sender: Sender<u8>,
    receiver: Arc<Mutex<Receiver<u8>>>,
    runtime: Arc<Mutex<Runtime>>,
    client: Arc<Mutex<SpectateClient<Channel>>>,
}

impl Default for Spectate {
    fn default() -> Self {
        Self::new()
    }
}

impl Spectate {
    pub fn new() -> Self {
        let (sender, receiver) = channel();

        //wrap the receiver and runtime so we can move it into a new thread
        let receiver = Arc::new(Mutex::new(receiver));
        let runtime = Arc::new(Mutex::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("spectate_runtime_thread")
                .on_thread_start(move || set_current_thread_priority(WORKER_PRIORITY))
                .build()
                .expect("Creating spectate_runtime_thread"),
        ));

        let client = Arc::new(Mutex::new(Spectate::init(&runtime).expect("Couldn't init")));

        Self {
            sender,
            receiver,
            runtime,
            client,
        }
    }

    //unforunately for now we have to flush at the end of the main application
    pub fn flush(&self) {
        let receiver = self.receiver.clone();
        let client = self.client.clone();
        let runtime = self.runtime.clone();
        let hot_loop = thread::spawn(move || {
            let receiver = receiver.lock().expect("");
            let runtime = runtime.lock().expect("");

            //loop {
            let mut client = client.lock().expect("");
            let data: Vec<u8> = receiver.try_iter().collect();
            let entries = vec![LogEntry { log: data }];
            if entries.len() > 0 {
                let send_future = client.send_records(futures_util::stream::iter(entries));
                runtime.block_on(async move {
                    send_future.await.expect("Couldn't send records");
                });
            }
            //}
        });
        hot_loop.join().expect("join hot loop");
    }
    pub fn target() -> Target {
        let spectate = Spectate::new();
        //before we send back the Target
        Target::Pipe(Box::new(LogTarget(spectate)))
    }

    fn init(
        runtime: &Arc<Mutex<Runtime>>,
    ) -> Result<SpectateClient<Channel>, Box<dyn std::error::Error>> {
        //clone our reciever to move across threads
        let runtime = runtime.clone();
        let runtime_thread = thread::spawn(move || {
            //get access to our mutex
            let runtime = runtime.lock().expect("lock runtime");
            runtime.block_on(async {
                //println!("CONNECTED");

                SpectateClient::connect("http://[::1]:50051")
                    .await
                    .expect("couldn't connect")
            })
        });

        let client = runtime_thread.join().expect("Couldn't join runtime_thread");
        Ok(client)
    }
}

#[derive(Debug, Clone)]
struct LogTarget(Spectate);

impl LogTarget {}

impl io::Write for LogTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        //do we need to send to a channel here or can we simply block on a call?
        for char in buf {
            self.0.sender.send(*char).ok();
        }
        self.0.flush();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) mod spectate {
    tonic::include_proto!("spectate_proto");
}

/// The default worker priority (value passed to `libc::setpriority`);
const WORKER_PRIORITY: i32 = 10;
#[cfg(unix)]
fn set_current_thread_priority(priority: i32) {
    // on linux setpriority sets the current thread's priority
    // (as opposed to the current process).
    unsafe { libc::setpriority(0, 0, priority) };
}

#[allow(dead_code)]
#[cfg(not(unix))]
fn set_current_thread_priority(priority: i32) {
    warn!("Setting worker thread priority not supported on this platform");
}
