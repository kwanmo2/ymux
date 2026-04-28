//! IPC server — listens for connections from tool processes.
//!
//! On Unix: uses a Unix domain socket.
//! On Windows (or as fallback): uses TCP on localhost.

use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::protocol::IpcMessage;
use crate::IpcResult;

/// Callback invoked on the accept thread for each received message.
/// Receives the deserialized message and a sender that can write replies back
/// to the same client.
pub type MessageHandler = Box<dyn Fn(IpcMessage, &mut dyn Write) + Send + Sync>;

/// A blocking IPC server that accepts multiple clients.
pub struct IpcServer {
    /// The address string to give to clients (socket path or `tcp:host:port`).
    address: String,
    /// Signals the accept loop to stop.
    stop: Arc<AtomicBool>,
    /// Handle for the main accept thread.
    accept_thread: Option<thread::JoinHandle<()>>,
}

impl IpcServer {
    /// Start the IPC server. Returns immediately; the accept loop runs on a
    /// background thread. Each connected client is handled on its own thread.
    ///
    /// `handler` is called for every message received from any client.
    pub fn start(handler: MessageHandler) -> IpcResult<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(handler);

        #[cfg(unix)]
        let (address, accept_thread) = Self::start_unix(Arc::clone(&stop), handler)?;

        #[cfg(not(unix))]
        let (address, accept_thread) = Self::start_tcp(Arc::clone(&stop), handler)?;

        Ok(Self {
            address,
            stop,
            accept_thread: Some(accept_thread),
        })
    }

    /// The address string clients should use to connect. This is the value to
    /// put in the `YMUX_IPC` environment variable.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Signal the server to stop accepting new connections. Existing client
    /// handler threads will finish naturally when their client disconnects.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        // Connect to ourselves to unblock the accept() call so it can check
        // the stop flag and exit.
        #[cfg(unix)]
        {
            let _ = std::os::unix::net::UnixStream::connect(&self.address);
        }
        #[cfg(not(unix))]
        {
            if let Some(addr) = self.address.strip_prefix("tcp:") {
                let _ = std::net::TcpStream::connect(addr);
            }
        }
    }

    /// Stop the server and wait for the accept thread to finish.
    pub fn shutdown(mut self) {
        self.stop();
        if let Some(t) = self.accept_thread.take() {
            let _ = t.join();
        }
    }

    // ─── Unix implementation ─────────────────────────────────────────────

    #[cfg(unix)]
    fn start_unix(
        stop: Arc<AtomicBool>,
        handler: Arc<MessageHandler>,
    ) -> IpcResult<(String, thread::JoinHandle<()>)> {
        use std::os::unix::net::UnixListener;

        let session_id = uuid::Uuid::new_v4();
        let path = format!("/tmp/ymux-{session_id}.sock");

        // Remove stale socket if it exists.
        let _ = std::fs::remove_file(&path);

        let listener = UnixListener::bind(&path)?;
        // Set a timeout on accept so we can periodically check the stop flag.
        listener.set_nonblocking(false)?;

        let address = path.clone();
        let stop2 = Arc::clone(&stop);

        let handle = thread::Builder::new()
            .name("ymux-ipc-accept".into())
            .spawn(move || {
                // Set a short timeout so we wake up to check the stop flag.
                let _ = listener.set_nonblocking(false);

                for stream in listener.incoming() {
                    if stop2.load(Ordering::SeqCst) {
                        break;
                    }
                    match stream {
                        Ok(stream) => {
                            let handler = Arc::clone(&handler);
                            let stop3 = Arc::clone(&stop2);
                            thread::Builder::new()
                                .name("ymux-ipc-client".into())
                                .spawn(move || {
                                    Self::handle_client_unix(stream, &handler, &stop3);
                                })
                                .ok();
                        }
                        Err(e) => {
                            if stop2.load(Ordering::SeqCst) {
                                break;
                            }
                            eprintln!("ipc accept error: {e}");
                        }
                    }
                }
                // Clean up socket file.
                let _ = std::fs::remove_file(&path);
            })?;

        Ok((address, handle))
    }

    #[cfg(unix)]
    fn handle_client_unix(
        stream: std::os::unix::net::UnixStream,
        handler: &MessageHandler,
        stop: &AtomicBool,
    ) {
        let writer = match stream.try_clone() {
            Ok(w) => w,
            Err(_) => return,
        };
        let writer = Arc::new(Mutex::new(writer));
        let reader = BufReader::new(stream);

        for line in reader.lines() {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            match line {
                Ok(l) if l.is_empty() => continue,
                Ok(l) => match IpcMessage::from_line(&l) {
                    Ok(msg) => {
                        let mut w = writer.lock().unwrap();
                        handler(msg, &mut *w);
                    }
                    Err(e) => {
                        eprintln!("ipc parse error: {e}");
                    }
                },
                Err(_) => break, // Connection closed or error
            }
        }
    }

    // ─── TCP fallback (Windows / non-unix) ───────────────────────────────

    #[cfg(not(unix))]
    fn start_tcp(
        stop: Arc<AtomicBool>,
        handler: Arc<MessageHandler>,
    ) -> IpcResult<(String, thread::JoinHandle<()>)> {
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0")?;
        let local_addr = listener.local_addr()?;
        let address = format!("tcp:{}", local_addr);

        let stop2 = Arc::clone(&stop);

        let handle = thread::Builder::new()
            .name("ymux-ipc-accept".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stop2.load(Ordering::SeqCst) {
                        break;
                    }
                    match stream {
                        Ok(stream) => {
                            let handler = Arc::clone(&handler);
                            let stop3 = Arc::clone(&stop2);
                            thread::Builder::new()
                                .name("ymux-ipc-client".into())
                                .spawn(move || {
                                    Self::handle_client_tcp(stream, &handler, &stop3);
                                })
                                .ok();
                        }
                        Err(e) => {
                            if stop2.load(Ordering::SeqCst) {
                                break;
                            }
                            eprintln!("ipc accept error: {e}");
                        }
                    }
                }
            })?;

        Ok((address, handle))
    }

    #[cfg(not(unix))]
    fn handle_client_tcp(stream: std::net::TcpStream, handler: &MessageHandler, stop: &AtomicBool) {
        let writer = match stream.try_clone() {
            Ok(w) => w,
            Err(_) => return,
        };
        let writer = Arc::new(Mutex::new(writer));
        let reader = BufReader::new(stream);

        for line in reader.lines() {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            match line {
                Ok(l) if l.is_empty() => continue,
                Ok(l) => match IpcMessage::from_line(&l) {
                    Ok(msg) => {
                        let mut w = writer.lock().unwrap();
                        handler(msg, &mut *w);
                    }
                    Err(e) => {
                        eprintln!("ipc parse error: {e}");
                    }
                },
                Err(_) => break,
            }
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Unblock the accept loop.
        #[cfg(unix)]
        {
            let _ = std::os::unix::net::UnixStream::connect(&self.address);
        }
        #[cfg(not(unix))]
        {
            if let Some(addr) = self.address.strip_prefix("tcp:") {
                let _ = std::net::TcpStream::connect(addr);
            }
        }
        if let Some(t) = self.accept_thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[cfg(unix)]
    #[test]
    fn server_starts_and_stops() {
        let (tx, rx) = mpsc::channel();
        let handler: MessageHandler = Box::new(move |msg, _writer| {
            tx.send(msg).ok();
        });
        let server = IpcServer::start(handler).unwrap();
        let addr = server.address().to_string();
        assert!(addr.starts_with("/tmp/ymux-"));
        assert!(addr.ends_with(".sock"));

        // Ensure the socket file exists.
        assert!(std::path::Path::new(&addr).exists());

        server.shutdown();

        // Socket file should be cleaned up.
        assert!(!std::path::Path::new(&addr).exists());

        // No messages should have been received.
        assert!(rx.try_recv().is_err());
    }
}
