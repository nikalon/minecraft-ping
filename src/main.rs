mod data_types;
mod chat;

use core::panic;
use std::{net::{TcpStream, ToSocketAddrs}, io::{Write, Read, BufReader, BufWriter, stderr, stdout}, env::args, time::Instant};
use base64::{Engine as _, engine::general_purpose};
use data_types::*;

const MINECRAFT_PROTOCOL_VERSION: i32 = 758;
const SUPER_DUPER_MAGIC_VALUE: i64 = 873646492;

struct Arguments {
    get_favicon: bool,
    raw_response: bool,
    verbose: bool,
    host: String,
    port: u16
}

impl Arguments {
    fn parse<T: Iterator<Item = String>>(args: &mut T) -> Self {
        let mut arguments = Arguments{
            get_favicon: false,
            raw_response: false,
            verbose: false,
            host: "".to_owned(),
            port: 25565
        };
        let args = args.skip(1).collect::<Vec<String>>();

        // Parse optional flags
        let mut positional_i = 0;
        for (i, arg) in args.iter().enumerate() {
            match arg.as_ref() {
                "-v" | "--v" => { arguments.verbose = true; }
                "-f" | "--favicon" => { arguments.get_favicon = true; }
                "-r" | "--raw-response" => { arguments.raw_response = true; }
                _ => {
                    positional_i = i;
                    break;
                }
            }
        }

        // Required positional argument: hostname
        arguments.host = args.get(positional_i)
                             .expect("No address provided")
                             .to_string();

        // Optional positional argument: port
        if let Some(port) = args.get(positional_i + 1) {
            arguments.port = port.parse().expect("Invalid port");
        }

        arguments
    }
}

fn main() {
    let arguments = Arguments::parse(&mut args());
    let address = (arguments.host.as_ref(), arguments.port)
                    .to_socket_addrs()
                    .expect("Invalid host address")
                    .next()
                    .expect("Invalid host address");
    let tcp_connection = TcpStream::connect(address).expect("Could not connect to server");
    let mut buf_reader = BufReader::new(&tcp_connection);
    let mut buf_writer = BufWriter::new(&tcp_connection);
    print_line_verbose(format!("Connection established to {}", &arguments.host).as_ref(), &arguments);

    // We need to ensure that we send the hostname (if provided) instead of the IP address because otherwise some servers
    // may not respond at all
    send_handshake(&mut buf_writer, &arguments.host, arguments.port);
    print_line_verbose("Handshake request sent!", &arguments);

    send_status_request(&mut buf_writer);
    print_line_verbose("Status request sent!", &arguments);

    let status_response_json = read_status_response(&mut buf_reader);
    print_line_verbose("Received status response!", &arguments);
    let server_response: Response = serde_json::from_str(&status_response_json).unwrap();

    // Calculate server response time
    let start_time = send_ping_request(&mut buf_writer);
    print_line_verbose("Sent ping request!", &arguments);

    read_ping_request(&mut buf_reader);
    let response_elapsed_time = start_time.elapsed();
    print_line_verbose("Received pong response!", &arguments);
    print_line_verbose(format!("Delay: {} ms", response_elapsed_time.as_millis()).as_ref(), &arguments);

    print_line_verbose("Disconnected", &arguments);

    if arguments.get_favicon {
        // Print decoded favicon to stdout
        if let Some(favicon) = server_response.favicon {
            const FORMAT: &str = "data:image/png;base64,";
            if favicon.starts_with(FORMAT) {
                let mut buf = Vec::with_capacity(favicon.len());
                // Delete prefix and decode the image as Base64
                let favicon = favicon.strip_prefix(FORMAT).unwrap().as_bytes();
                general_purpose::STANDARD.decode_vec(favicon, &mut buf).unwrap();
                let _ = stdout().write_all(&buf);
            } else {
                eprintln!("WARNING: Could not decode favicon because it has an unknown format. Printing it as raw data...");
                let _ = stdout().write_all(favicon.as_bytes());
            }
        } else {
            eprintln!("WARNING: This server doesn't have a favicon.");
        }
    } else if arguments.raw_response {
        // Print raw response data
        println!("{status_response_json}");
    } else {
        // Parse status response JSON and print data
        let server_description = chat::chat_to_str(&server_response.description);
        println!("{}", server_description);
        println!("{:>24}: {}", "Server version", server_response.version.name);
        println!("{:>24}: {}", "Protocol",server_response.version.protocol);
        println!("{:>24}: {current}/{max}", "Players", current=server_response.players.online, max=server_response.players.max);

        let favicon = if let Some(f) = server_response.favicon {
            if f.is_empty() {
                "(No data available)"
            } else {
                "(Base64 data)"
            }
        } else {
            "(No data available)"
        };
        println!("{:>24}: {favicon}", "Favicon");

        let enforces_secure_chat = if server_response.enforces_secure_chat.unwrap_or(false) {
            "Yes"
        } else {
            "No"
        };
        println!("{:>24}: {enforces_secure_chat}", "Enforces secure chat");

        let previews_chat = if server_response.previews_chat.unwrap_or(false) {
            "Yes"
        } else {
            "No"
        };
        println!("{:>24}: {previews_chat}", "Previews chat");

        println!("{:>24}: {} ms", "Server latency", response_elapsed_time.as_millis());
    }
}

fn send_handshake<T: Write>(output: &mut T, server_address: &str, port: u16) {
    // TODO: Write to output only once for performance reasons
    let mut buffer: Vec<u8> = Vec::with_capacity(4096);

    // Packet ID
    write_var_int(&mut buffer, 0);

    // Protocol version
    write_var_int(&mut buffer, MINECRAFT_PROTOCOL_VERSION);

    // Server address
    write_string(&mut buffer, server_address);

    // Server port
    write_unsigned_short(&mut buffer, port);

    // Next state
    write_var_int(&mut buffer, 1);

    // Packet length
    let packet_size = buffer.len();
    write_var_int(output, packet_size as i32);

    output.write_all(&buffer).unwrap();
    output.flush().unwrap();
}

fn send_status_request<T: Write>(output: &mut T) {
    // Packet length
    write_var_int(output, 1); // Packet size should be one byte...

    // Packet ID
    write_var_int(output, 0); // ...because zero is represented as one byte for a VarInt
    output.flush().unwrap();
}

fn send_ping_request<T: Write>(output: &mut T) -> Instant {
    // Packet length
    write_var_int(output, 9); // 1 + 8 bytes

    // Packet ID
    write_var_int(output, 1); // Should be one byte

    // Payload
    write_long(output, SUPER_DUPER_MAGIC_VALUE); // Should be 8 bytes
    output.flush().unwrap();

    Instant::now()
}

fn read_status_response<T: Read>(input: &mut T) -> String {
    // Packet length
    let packet_length = read_var_int(input);
    if packet_length < 0 {
        panic!("Invalid packet length: {}", packet_length);
    }

    // Here we will ensure that we don't read more than **packet_length** bytes for this packet
    let mut input = input.take(packet_length as u64);

    // Packet ID
    let packet_id = read_var_int(&mut input);
    if packet_id != 0 {
        panic!("Error: The server responded with an unknown packet ID: 0x{packet_id:x}");
    }

    // JSON response
    let server_info = read_string(&mut input);

    // Checks if all bytes were read. If it didn't we probably screwed up somewhere.
    if input.bytes().count() != 0 {
        panic!("ERROR: There are still some bytes to read in the current packet");
    }

    server_info
}

fn read_ping_request<T: Read>(input: &mut T) {
    // Packet length
    let packet_length = read_var_int(input);
    if packet_length < 0 {
        panic!("Invalid packet length: {}", packet_length);
    }

    // Here we will ensure that we don't read more than **packet_length** bytes for this packet
    let mut input = input.take(packet_length as u64);

    // Packet ID
    let packet_id = read_var_int(&mut input);
    if packet_id != 1 {
        panic!("Error: The server responded with an unknown packet ID: 0x{packet_id:x}");
    }

    // Payload
    let payload = read_long(&mut input);
    if payload != SUPER_DUPER_MAGIC_VALUE {
        panic!("Error: the server's pong response was an invalid value: 0x{payload:x}");
    }

    // Checks if all bytes were read. If it didn't we probably screwed up somewhere.
    if input.bytes().count() != 0 {
        panic!("ERROR: There are still some bytes to read in the current packet");
    }
}

fn print_line_verbose(msg: &str, arguments: &Arguments) {
    if arguments.verbose {
        let _ = stderr().write_all(msg.as_bytes());
        let _ = stderr().write_all("\n".as_bytes());
    }
}