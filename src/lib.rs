use std::env;
use std::io::prelude::*;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::str;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::{Duration, Instant};

use rand_core::SeedableRng;

use log;
use pretty_env_logger;
use tungstenite::protocol::Message;
use tungstenite::server::accept;

use askama::Template;

mod thread_pool;
pub mod world;

const TICK_RATE_MS: u64 = 100;

pub struct Config {
    pub host_address: String,
}

impl Config {
    pub fn new() -> Config {
        // This could be a value passed to the compiler
        let host_address = env::var("HOST_ADDRESS").unwrap_or_else(|_| String::from("localhost"));
        Config { host_address }
    }
}

pub struct ConfiguredWorld {
    world: world::World,
    tick_rate: u64,
    randomizer: rand_pcg::Pcg32,
}

pub fn run(config: Config) {
    pretty_env_logger::init();

    let world = world::World::default();
    let configured_world = ConfiguredWorld {
        world: world,
        tick_rate: TICK_RATE_MS,
        randomizer: rand_pcg::Pcg32::from_seed(*b"somebody once to"),
    };
    let world_ref_counter = Arc::new(RwLock::new(configured_world));
    let primary_world_instance = Arc::clone(&world_ref_counter);
    thread::spawn(move || {
        let mut randomizer = rand_pcg::Pcg32::from_seed(*b"somebody once to");
        let mut start;
        let mut frame_time;
        let mut lock_time;
        loop {
            start = Instant::now();

            // A possible optimization: Have world calculate its value without
            // getting a lock. Only lock when updating the value. Would need to
            // test how often the lock is preventing reads.

            // This scope is created to ensure the lock is dropped ASAP
            {
                let mut w = primary_world_instance.write().unwrap();
                lock_time = start.elapsed().as_millis();
                w.world.update_if_active(&mut randomizer);
            }
            frame_time = start.elapsed().as_millis() as u64;

            log::info!(
                "Frame processing took: {} (time to acquire lock: {})",
                frame_time,
                lock_time
            );
            {
                let w = primary_world_instance.read().unwrap();
                if frame_time > w.tick_rate {
                    log::warn!(
                        "WARNING: Frame processing ({}) took longer than tick rate ({})",
                        frame_time,
                        w.tick_rate
                    );
                    frame_time = w.tick_rate; // Prevent subtraction below from going negative
                }
                thread::sleep(Duration::from_millis(w.tick_rate - frame_time));
            }
        }
    });

    start_tcp_server(&world_ref_counter, config);
}

pub fn start_tcp_server(world_ref_counter: &Arc<RwLock<ConfiguredWorld>>, config: Config) {
    log::info!("Server started");
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();
    let pool = thread_pool::ThreadPool::new(4);

    let host_address = Arc::new(config.host_address);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        let mut buffer = [0; 512]; // Dynamically size; will overflow as world size grows
        stream.peek(&mut buffer).unwrap();

        let world_ref = Arc::clone(&world_ref_counter);
        let address_ref = Arc::clone(&host_address);

        pool.execute(move || {
            let index = b"GET / HTTP/1.1\r\n";
            let debug_index = b"GET /?debug=9933212 HTTP/1.1\r\n";
            let world_status = b"GET /world_status HTTP/1.1\r\n";
            let websocket = b"GET /websocket";

            if buffer.starts_with(index) {
                handle_index(&stream, &address_ref, &world_ref)
            } else if buffer.starts_with(debug_index) {
                handle_debug_index(&stream, &address_ref, &world_ref)
            } else if buffer.starts_with(world_status) {
                handle_world_status(&stream, &world_ref)
            } else if buffer.starts_with(websocket) {
                handle_websocket(&stream, &world_ref)
            } else {
                handle_404(&stream)
            };
        });
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    host_address: &'a str,
    width: i32,
    height: i32,
    debug: bool,
}

const HTTP_OK: &str = "HTTP/1.1 200 OK\r\n\r\n";
const HTTP_SERVER_ERROR: &str = "HTTP/1.1 200 OK\r\n\r\n";

fn handle_index(
    mut stream: &TcpStream,
    address_ref: &str,
    world_ref: &Arc<RwLock<ConfiguredWorld>>,
) {
    // SECURITY: Even with debug = false, the ws could send arbitrary data
    // This is decidedly unsecure but better than nothing
    let w = &world_ref.read().unwrap();
    let content = IndexTemplate {
        host_address: &address_ref,
        height: w.world.height,
        width: w.world.width,
        debug: false,
    };
    let response = format!("{}{}", HTTP_OK, content);

    stream.read(&mut [0; 512]).unwrap(); // Ensure stream is empty before writing
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn handle_debug_index(
    mut stream: &TcpStream,
    address_ref: &str,
    world_ref: &Arc<RwLock<ConfiguredWorld>>,
) {
    // SECURITY: Even with debug = false, the ws could send arbitrary data
    // This is decidedly unsecure but better than nothing
    let w = &world_ref.read().unwrap();
    let content = IndexTemplate {
        host_address: &address_ref,
        height: w.world.height,
        width: w.world.width,
        debug: true,
    };
    let response = format!("{}{}", HTTP_OK, content);

    stream.read(&mut [0; 512]).unwrap(); // Ensure stream is empty before writing
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn handle_world_status(mut stream: &TcpStream, world_ref: &Arc<RwLock<ConfiguredWorld>>) {
    let w = &world_ref.read().unwrap();
    let rendered_entities = w.world.render();
    let response;
    match serde_json::to_string(&rendered_entities) {
        Ok(serialized_player) => response = format!("{}{}", HTTP_OK, serialized_player),
        Err(e) => {
            log::error!("Unable to serialize player: {}", e);
            response = String::from(HTTP_SERVER_ERROR);
        }
    };

    // ensure stream is empty before writing
    stream.read(&mut [0; 512]).unwrap();
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate {}

fn handle_404(mut stream: &TcpStream) {
    let not_found = NotFoundTemplate {};
    let contents = not_found.render().unwrap();
    let status_line = "HTTP/1.1 200 OK\r\n\r\n";
    let response = format!("{}{}", status_line, contents);
    // ensure stream is empty before writing
    let mut buffer = [0; 512]; // Dynamically size; will overflow as world size grows
    stream.read(&mut buffer).unwrap();
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn handle_websocket(stream: &TcpStream, world_ref: &Arc<RwLock<ConfiguredWorld>>) {
    let mut websocket = accept(stream).unwrap();
    websocket.get_mut().set_nodelay(true).unwrap(); // Disables Nagle's Algorithm, reduces stream delays
    websocket.get_mut().set_nonblocking(true).unwrap();
    let mut tick_rate;
    loop {
        match websocket.read_message() {
            Ok(msg) => match msg {
                Message::Close(_) => {
                    if let Err(e) = websocket.close(None) {
                        if let tungstenite::Error::ConnectionClosed = e {
                            log::warn!("Attempted to close websocket but it was already closed");
                            return;
                        } else if let tungstenite::Error::Io(_) = e {
                            log::warn!("Attempted to close websocket but got unknown error: {}", e);
                            return;
                        } else {
                            log::error!("Unexpected error while closing websocket: {}", e);
                        };
                    }
                    return;
                }
                Message::Text(msg_string) => {
                    handle_ws_text_msg(&msg_string[..], world_ref);
                }
                _ => log::error!("Unexpected type of websocket message: {}", msg),
            },
            Err(e) => {
                match e {
                    tungstenite::Error::ConnectionClosed => return,
                    tungstenite::Error::AlreadyClosed => return,
                    // IO errors such as WouldBlock can be ignored as we're not blocking
                    tungstenite::Error::Io(_) => (),
                    _ => {
                        log::error!("Unexpected websocker error: {}", e);
                        return;
                    }
                }
            }
        };
        let result;
        let rendered_entities;
        // Scope reduces time the world lock is held
        {
            let w = world_ref.read().unwrap();
            rendered_entities = w.world.render();
            tick_rate = w.tick_rate;
        }
        // TODO: Re-rendering the entites for every open websocket is unecessary
        match serde_json::to_string(&rendered_entities) {
            Ok(serialized_player) => result = format!("{}", serialized_player),
            Err(e) => {
                log::error!("Unable to serialize player: {}", e);
                return;
            }
        };
        let response = Message::text(result);
        websocket.write_message(response).unwrap();

        thread::sleep(Duration::from_millis(tick_rate));
    }
}

fn handle_ws_text_msg(msg_string: &str, world_ref: &Arc<RwLock<ConfiguredWorld>>) {
    match msg_string {
        "pause" => {
            let mut w = world_ref.write().unwrap();
            w.world.pause();
        }
        "unpause" => {
            let mut w = world_ref.write().unwrap();
            w.world.unpause();
        }
        "update" => {
            let w = &mut *world_ref.write().unwrap();
            w.world.update(&mut w.randomizer);
        }
        // For now, assume anything with a number is a tick rate change
        tick_rate if tick_rate.chars().any(|c| c.is_numeric()) => {
            let w = &mut *world_ref.write().unwrap();
            let tick_rate_vector: Vec<u32> =
                tick_rate.chars().filter_map(|c| c.to_digit(10)).collect();
            let new_tick_rate = tick_rate_vector.iter().fold(0, |acc, elem| acc * 10 + elem);
            w.tick_rate = new_tick_rate as u64;
        }
        _ => log::warn!("Unknown websocket text message: {}", msg_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use native_tls::TlsStream;
    use std::thread::{sleep, spawn};
    use tungstenite::{connect, stream::Stream, WebSocket};

    fn get_mock_config() -> Config {
        Config {
            host_address: String::from("localhost"),
        }
    }

    #[test]
    fn test_handle_index() {
        let _ = spawn(move || {
            let server =
                TcpListener::bind("localhost:7880").expect("Can't listen, is port already used?");
            let world_ref_counter = Arc::new(RwLock::new(world::World::default()));
            let stream = server.incoming().next().unwrap().unwrap();
            let mock_config = get_mock_config();
            handle_index(&stream, &mock_config.host_address[..], &world_ref_counter);
        });

        let mut client = TcpStream::connect("localhost:7880").expect("Can't connect to port");
        client.write(b"/index").unwrap(); // Unblocks ".next()" in server. Ideally we could get a stream without this

        let mut buffer = [0; 2048];
        client.read(&mut buffer).unwrap();
        let response = String::from_utf8_lossy(&buffer);

        let expected_response = "<canvas id=\"game-canvas\"></canvas>";
        assert!(response.contains(expected_response));
    }

    // Websocket testing fn borrowed from:
    // https://github.com/snapview/tungstenite-rs/blob/master/tests/connection_reset.rs
    type Sock = WebSocket<Stream<TcpStream, TlsStream<TcpStream>>>;

    fn do_test<CT>(client_task: CT)
    where
        CT: FnOnce(Sock) + Send + 'static,
    {
        spawn(|| {
            sleep(Duration::from_secs(5));
            println!("Unit test executed too long, perhaps stuck on WOULDBLOCK...");
        });

        let server =
            TcpListener::bind("localhost:7881").expect("Can't listen, is port already used?");
        let client_thread = spawn(move || {
            let (client, _) = connect("ws://localhost:7881/socket").expect("Can't connect to port");
            client_task(client);
        });

        let stream = server.incoming().next().unwrap().unwrap();

        // Setup world instance
        // ==============================
        // Warning: As world creation expands this will need to be mocked
        let world_ref_counter = Arc::new(RwLock::new(world::World::default()));
        let primary_world_instance = Arc::clone(&world_ref_counter);
        thread::spawn(move || {
            let mut randomizer = rand_pcg::Pcg32::from_seed(*b"somebody once to");
            loop {
                thread::sleep(Duration::from_millis(TICK_RATE_MS));
                let mut w = primary_world_instance.write().unwrap();
                w.update(&mut randomizer);
            }
        });
        let world_ref = Arc::clone(&world_ref_counter);
        // ===============================

        // Begin websocket handler
        handle_websocket(&stream, &world_ref);

        client_thread.join().unwrap();
        println!("Done");
    }

    #[test]
    fn test_handle_websocket() {
        do_test(|mut cli_sock| {
            sleep(Duration::from_secs(1));
            println!("Starting ws client...");

            let first_message = cli_sock.read_message().unwrap();
            assert!(first_message.is_text());
            println!("  First message!");

            thread::sleep(Duration::from_millis(TICK_RATE_MS));

            let second_message = cli_sock.read_message().unwrap();
            assert!(second_message.is_text());
            println!("  Second message!");

            // Check that a different world state was returned

            println!("...closing ws client.");
            cli_sock.close(None).unwrap();
        });
    }
}
