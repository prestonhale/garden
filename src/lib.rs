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

use tungstenite::protocol::Message;
use tungstenite::server::accept;

// Console renderer
use askama::Template;
use console::{Term, Style, Color};

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

pub fn run(config: Config) {
    let world_ref_counter = Arc::new(RwLock::new(world::World::default()));
    let primary_world_instance = Arc::clone(&world_ref_counter);
    thread::spawn(move || {
        let mut randomizer = rand_pcg::Pcg32::from_seed(*b"somebody once to");
        let mut start;
        let mut frame_time;
        // let mut lock_time;
        loop {
            start = Instant::now();

            // A possible optimization: Have world calculate its value without
            // getting a lock. Only lock when updating the value. Would need to
            // test how often the lock is preventing reads.

            // This scope is created to ensure the lock is dropped ASAP
            {
                let mut w = primary_world_instance.write().unwrap();
                // lock_time = start.elapsed().as_millis();
                w.update_if_active(&mut randomizer);
            }
            frame_time = start.elapsed().as_millis() as u64;

            // println!("Frame processing took: {}; lock time {}", frame_time, lock_time);
            if frame_time > TICK_RATE_MS {
                println!("WARNING: Frame processing ({}) took longer than tick rate ({})", frame_time, TICK_RATE_MS);
                frame_time = TICK_RATE_MS; // Prevent subtraction below from going negative
            }

            thread::sleep(Duration::from_millis(TICK_RATE_MS - frame_time));
        }
    });

    if cfg!(feature = "console-renderer") {
        // Console Renderer
        let world_instance = Arc::clone(&world_ref_counter);
        loop {
            let term = Term::stdout();
            // Scope allows us to rapidly release world lock
            let world_height;
            let world_width;
            let rendered_entities;
            {
                let world = world_instance.read().unwrap();
                rendered_entities = world.render();
                world_height = *world.get_height(); // could be cached
                world_width = *world.get_height(); // could be cached
            }
            
            match term.clear_screen() {
                Ok(_) => (),
                _ => panic!("Failed to clear screen"),
            };
            for y in 0..world_height {
                for x in 0..world_width{
                    let mut style = Style::new().bg(Color::Green);  // default background color
                    for entity in rendered_entities.iter() {
                        if entity.position.x == x && entity.position.y == y {
                            style = match &entity.color[..] {
                                world::RED => style.bg(Color::Red),
                                world::BROWN => style.bg(Color::Yellow),
                                world::BLACK => style.bg(Color::Black),
                                _ => style.bg(Color::Magenta) // Magenta, the color of "oh no"
                            };
                        }
                    }
                    print!("{}", style.apply_to("  "))
                }
                println!("")
            }
            thread::sleep(Duration::from_millis(TICK_RATE_MS));
        }
    } else {
        start_tcp_server(&world_ref_counter, config);
    }
}

pub fn start_tcp_server(world_ref_counter: &Arc<RwLock<world::World>>, config: Config) {
    println!("Server started");
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
            let world_status = b"GET /world_status HTTP/1.1\r\n";
            let websocket = b"GET /websocket";

            if buffer.starts_with(index) {
                handle_index(&stream, &address_ref, &world_ref)
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
}

fn handle_index(mut stream: &TcpStream, address_ref: &str, world_ref: &Arc<RwLock<world::World>>) {
    let w = &world_ref.read().unwrap();
    let hello = IndexTemplate {
        host_address: &address_ref,
        height: w.height,
        width: w.width,
    };
    let contents = hello.render().unwrap();
    let status_line = "HTTP/1.1 200 OK\r\n\r\n";
    let response = format!("{}{}", status_line, contents);
    
    stream.read(&mut [0;512]).unwrap();
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn handle_world_status(mut stream: &TcpStream, world_ref: &Arc<RwLock<world::World>>) {
    let status_line = "HTTP/1.1 200 OK\r\n\r\n";
    let w = &world_ref.read().unwrap();
    let rendered_entities = w.render();
    let response;
    match serde_json::to_string(&rendered_entities) {
        Ok(serialized_player) => response = format!("{}{}", status_line, serialized_player),
        _ => panic!("Unable to serialize player"),
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

fn handle_websocket(stream: &TcpStream, world_ref: &Arc<RwLock<world::World>>) {
    let mut websocket = accept(stream).unwrap();
    websocket.get_mut().set_nodelay(true).unwrap(); // Disables Nagle's Algorithm, reduces stream delays
    websocket.get_mut().set_nonblocking(true).unwrap();
    
    // let mut start_time;
    // let mut socket_loop_time;
    loop {
        // start_time = Instant::now();
        match websocket.read_message() {
            Ok(msg) => {
                match msg {
                    Message::Close(_) => {
                        if let Err(e) = websocket.close(None) {
                            if let tungstenite::Error::ConnectionClosed = e {
                                return
                            } else if let tungstenite::Error::Io(_) = e {
                                return
                            } else {
                                panic!("Unexpected error closing websocket: {:?}", e)
                            };
                        }
                        return;
                    },
                    Message::Text(msg_string) => {
                        handle_ws_text_msg(&msg_string[..], world_ref);
                    },
                    _ => panic!("received unrecognized type of websocket message")
                }
            },
            Err(e) => {
                match e {
                    tungstenite::Error::ConnectionClosed => return,
                    tungstenite::Error::AlreadyClosed => return,
                    // IO errors such as WouldBlock can be ignored as we're not blocking
                    tungstenite::Error::Io(_) => (),
                    _ => panic!("Got unexpected websocket error: {:?}", e),
                }
            }
        };
        let result;
        let rendered_entities;
        // Scope reduces time the world lock is held
        {
            let w = world_ref.read().unwrap();
            rendered_entities = w.render();
        }
        // TODO: Re-rendering the entites for every open websocket is unecessary
        match serde_json::to_string(&rendered_entities) {
            Ok(serialized_player) => result = format!("{}", serialized_player),
            _ => panic!("Unable to serialize player"),
        };
        let response = Message::text(result);
        websocket.write_message(response).unwrap();

        // socket_loop_time = start_time.elapsed().as_millis();
        // println!("Socket loop took: {}", socket_loop_time);

        thread::sleep(Duration::from_millis(TICK_RATE_MS));
    }
}

fn handle_ws_text_msg(msg_string: &str, world_ref: &Arc<RwLock<world::World>>) {
    match msg_string{
        "pause" => {
            let mut w = world_ref.write().unwrap();
            w.pause();
        },
        "unpause" => {
            let mut w = world_ref.write().unwrap();
            w.unpause();
        },
        _ => panic!("unknown websocket message received")
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
            let server = TcpListener::bind("localhost:7880").expect("Can't listen, is port already used?");
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
        CT: FnOnce(Sock) + Send + 'static
    {
        spawn(|| {
            sleep(Duration::from_secs(5));
            println!("Unit test executed too long, perhaps stuck on WOULDBLOCK...");
        });
        
        let server = TcpListener::bind("localhost:7881").expect("Can't listen, is port already used?");

        let client_thread = spawn(move || {
            let (client, _) = connect("ws://localhost:7881/socket")
                .expect("Can't connect to port");
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
        do_test(
            |mut cli_sock| {
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
            },
        );
    }
}
