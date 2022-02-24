pub(crate) mod spectate {
    tonic::include_proto!("spectate_proto");
}
pub use env_logger::Target;
//use env_logger::Target;
pub use log;
use log::trace;
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
    fn new() -> Self {
        let (sender, receiver) = channel();
        let _ = env_logger::try_init();

        //wrap the receiver, runtime and client so we can move it across threads
        let receiver = Arc::new(Mutex::new(receiver));
        //we set the thread priority of the internal tokio runtime lower so as not the interfere
        //with the host application's main thread
        let runtime = Arc::new(Mutex::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("spectate_runtime_thread")
                .on_thread_start(move || set_current_thread_priority(WORKER_PRIORITY))
                .build()
                .expect("Creating spectate_runtime_thread"),
        ));

        //initialize the client connection and wrap it for cross thread calls
        let client = Arc::new(Mutex::new(
            Spectate::client_init(&runtime).expect("Couldn't initialize connection"),
        ));

        Self {
            sender,
            receiver,
            runtime,
            client,
        }
    }

    //spawns a thread to receive data from the transport channel and then stream to the existing
    //grpc client connection
    fn flush(&self) {
        //clone to pass into new thread
        let receiver = self.receiver.clone();
        let client = self.client.clone();
        let runtime = self.runtime.clone();

        //spawn a thread to stream in the background
        let send_thread = thread::spawn(move || {
            //lock the thread safe objects so we can work with them in this thread
            let receiver = receiver.lock().expect("");
            let runtime = runtime.lock().expect("");
            let mut client = client.lock().expect("");

            //create log data for transport based on the potential contents of the channel
            let entries = vec![LogEntry {
                data: receiver.try_iter().collect(),
            }];
            //only make a send request if there is something to send
            if !entries.is_empty() {
                //construct the future
                let send_future = client.send_records(futures_util::stream::iter(entries));
                //send it
                runtime.block_on(async move {
                    send_future.await.expect("Couldn't send records");
                });
            }
        });
        //join the thread so we can continue
        send_thread.join().expect("join send_thread");
    }

    pub fn target() -> Target {
        //Prepare a Target to provide to env_logger
        Target::Pipe(Box::new(LogTarget(Spectate::new())))
    }

    //Spawns a background thread to create a grpc client, then connects to the grpc server and
    //returns the client when the thread is joined
    fn client_init(
        runtime: &Arc<Mutex<Runtime>>,
    ) -> Result<SpectateClient<Channel>, Box<dyn std::error::Error>> {
        //clone our runtime to move across threads
        let runtime = runtime.clone();
        let runtime_thread = thread::spawn(move || {
            //get access to our mutex
            let runtime = runtime.lock().expect("lock runtime");
            trace!("Connecting to grpc server");
            runtime.block_on(async {
                //connect to the grpc server
                SpectateClient::connect("http://[::1]:50051")
                    .await
                    .expect("couldn't connect")
            })
        });

        //join the background thread so we can return the client
        let client = runtime_thread.join().expect("Couldn't join runtime_thread");
        Ok(client)
    }
}

#[derive(Debug, Clone)]
struct LogTarget(Spectate);

impl LogTarget {}

impl io::Write for LogTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        //read all chars
        for char in buf {
            //send through channel
            self.0.sender.send(*char).ok();
        }
        //flush the channel after each buffer send
        self.flush()?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        //flush
        self.0.flush();
        Ok(())
    }
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
