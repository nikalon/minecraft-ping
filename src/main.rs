mod arguments;
mod chat;
mod data_types;

use arguments::CommandLineArguments;
use base64::{engine::general_purpose, Engine as _};
use data_types::*;
use std::process::{ExitCode, Termination};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    env::args,
    io::{stderr, stdout, BufReader, BufWriter, IsTerminal, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket},
    time::Instant,
};

const MIN_MINECRAFT_PROTOCOL_VERSION: i32 = 0;
const RESET_COLORS: &str = "\x1B[0m";
const FG_YELLOW: &str = "\x1B[93m";

// Error codes based on BSD sysexits (https://man.freebsd.org/cgi/man.cgi?query=sysexits&apropos=0&sektion=0&manpath=FreeBSD+11.2-stable&arch=default&format=html)
enum ErrorCode {
    Ok = 0,
    IncorrectParameters = 65,
    HostDoesNotExist = 68,
    Protocol = 76,
}

impl Termination for ErrorCode {
    fn report(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

fn main() -> ErrorCode {
    let arguments = match CommandLineArguments::parse(&mut args()) {
        Ok(args) => args,
        Err(e) => {
            // TODO: Print usage or implement -h flag
            eprintln!("{e}");
            return ErrorCode::IncorrectParameters;
        }
    };
    if arguments.open_to_lan {
        do_open_to_lan(&arguments)
    } else {
        do_server_list_ping(&arguments)
    }
}

fn do_server_list_ping(arguments: &CommandLineArguments) -> ErrorCode {
    let address = (arguments.host.as_ref(), arguments.port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut addr| addr.next());
    let address = match address {
        Some(addr) => addr,
        None => {
            eprintln!("Invalid address \'{}\'", arguments.host);
            return ErrorCode::IncorrectParameters;
        }
    };

    print_line_verbose("Attempting to connect...", arguments);
    let tcp_connection = match TcpStream::connect(address) {
        Ok(connection) => connection,
        Err(_) => {
            eprintln!("Could not connect to server");
            return ErrorCode::HostDoesNotExist;
        }
    };
    let mut buf_reader = BufReader::new(&tcp_connection);
    let mut buf_writer = BufWriter::new(&tcp_connection);
    print_line_verbose(
        format!("Connection established to {}", &arguments.host).as_ref(),
        arguments,
    );

    // We need to ensure that we send the hostname (if provided) instead of the IP address because otherwise some servers
    // may not respond at all
    match send_handshake(&mut buf_writer, &arguments.host, arguments.port) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Error: Could not send handshake");
            eprintln!("More details: {e}");
            return ErrorCode::Protocol;
        }
    };
    print_line_verbose("Handshake request sent!", arguments);

    match send_status_request(&mut buf_writer) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Error: Could not send status request");
            eprintln!("More details: {e}");
            return ErrorCode::Protocol;
        }
    };
    print_line_verbose("Status request sent!", arguments);

    let status_response_json = match read_status_response(&mut buf_reader) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Error: Could not read status response");
            eprintln!("More details: {e}");
            return ErrorCode::Protocol;
        }
    };
    print_line_verbose("Received status response!", arguments);
    let server_response: Response = match serde_json::from_str(&status_response_json) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Error: Could not decode response because it has malformed JSON data");
            eprintln!("More details: {e}");
            return ErrorCode::Protocol;
        }
    };

    // Calculate server response time
    let system_time_sec = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(t) => t.as_secs() as i64,
        Err(_) => 0,
    };
    let start_time = match send_ping_request(&mut buf_writer, system_time_sec) {
        Ok(time) => time,
        Err(e) => {
            eprintln!("Error: Could not send ping request");
            eprintln!("More details: {e}");
            return ErrorCode::Protocol;
        }
    };
    print_line_verbose("Sent ping request!", arguments);

    let payload = match read_pong_response(&mut buf_reader) {
        Ok(payload) => payload,
        Err(e) => {
            eprintln!("Error: Could not read pong response");
            eprintln!("More details: {e}");
            return ErrorCode::Protocol;
        }
    };
    if payload != system_time_sec {
        eprintln!("Error: the server's pong response is an invalid value: 0x{payload:x}. Sent: 0x{system_time_sec:x}");
        return ErrorCode::Protocol;
    }

    let response_elapsed_time = start_time.elapsed();
    print_line_verbose("Received pong response!", arguments);
    print_line_verbose(
        format!("Delay: {} ms", response_elapsed_time.as_millis()).as_ref(),
        arguments,
    );
    print_line_verbose("Disconnected", arguments);

    if arguments.get_favicon {
        // Print decoded favicon to stdout
        if let Some(favicon) = server_response.favicon {
            const FORMAT: &str = "data:image/png;base64,";
            if favicon.is_empty() {
                print_warning("This server doesn't have a favicon.");
            } else if favicon.starts_with(FORMAT) {
                if arguments.raw_response {
                    let _ = stdout().write_all(favicon.as_bytes());
                } else {
                    let mut buf = Vec::with_capacity(favicon.len());
                    // Delete prefix and decode the image as Base64
                    let result = favicon
                        .strip_prefix(FORMAT)
                        .map(|favicon| favicon.as_bytes())
                        .map(|favicon| general_purpose::STANDARD.decode_vec(favicon, &mut buf))
                        .map(|_| stdout().write_all(&buf));
                    if result.is_none() {
                        eprintln!("Error: Could not decode favicon")
                    }
                }
            } else {
                print_warning("Could not decode favicon because it has an unknown format. Printing it as raw data...");
                let _ = stdout().write_all(favicon.as_bytes());
            }
        } else {
            print_warning("This server doesn't have a favicon.");
        }
    } else if arguments.raw_response {
        // Print raw response data
        println!("{status_response_json}");
    } else {
        // Parse status response JSON and print data
        let apply_font_styles = can_print_colors(&std::io::stdout());
        let server_description = chat::chat_to_str(&server_response.description, apply_font_styles);
        println!("{server_description}");
        println!("{:<24} {}", "Server version", server_response.version.name);
        println!("{:<24} {}", "Protocol", server_response.version.protocol);
        println!(
            "{:<24} {current}/{max}",
            "Players",
            current = server_response.players.online,
            max = server_response.players.max
        );

        let favicon = if let Some(f) = server_response.favicon {
            if f.is_empty() {
                "(No data available)"
            } else {
                "(Base64 data)"
            }
        } else {
            "(No data available)"
        };
        println!("{:<24} {favicon}", "Favicon");

        let enforces_secure_chat = if server_response.enforces_secure_chat.unwrap_or(false) {
            "Yes"
        } else {
            "No"
        };
        println!("{:<24} {enforces_secure_chat}", "Enforces secure chat");

        let previews_chat = if server_response.previews_chat.unwrap_or(false) {
            "Yes"
        } else {
            "No"
        };
        println!("{:<24} {previews_chat}", "Previews chat");

        println!(
            "{:<24} {} ms",
            "Server latency",
            response_elapsed_time.as_millis()
        );
    }

    ErrorCode::Ok
}

fn send_handshake<T: Write>(output: &mut T, server_address: &str, port: u16) -> Result<(), String> {
    let mut buffer: Vec<u8> = Vec::with_capacity(4096);

    // Packet ID
    write_var_int(&mut buffer, 0)?;

    // Protocol version
    write_var_int(&mut buffer, MIN_MINECRAFT_PROTOCOL_VERSION)?;

    // Server address
    write_string(&mut buffer, server_address)?;

    // Server port
    write_unsigned_short(&mut buffer, port)?;

    // Next state
    write_var_int(&mut buffer, 1)?;

    // Packet length
    let packet_size = buffer.len();
    write_var_int(output, packet_size as i32)?;

    output.write_all(&buffer).map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn send_status_request<T: Write>(output: &mut T) -> Result<(), String> {
    // Packet length
    write_var_int(output, 1)?; // Packet size should be one byte...

    // Packet ID
    write_var_int(output, 0)?; // ...because zero is represented as one byte for a VarInt
    output.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn send_ping_request<T: Write>(output: &mut T, payload: i64) -> Result<Instant, String> {
    // Packet length
    write_var_int(output, 9)?; // 1 + 8 bytes

    // Packet ID
    write_var_int(output, 1)?; // Should be one byte

    // Payload
    write_long(output, payload)?; // Should be 8 bytes
    output.flush().map_err(|e| e.to_string())?;

    Ok(Instant::now())
}

fn read_status_response<T: Read>(input: &mut T) -> Result<String, String> {
    // Packet length
    let packet_length = read_var_int(input)?;
    if packet_length < 0 {
        return Err(format!("Invalid packet length: {packet_length}"));
    }

    // Here we will ensure that we don't read more than **packet_length** bytes for this packet
    let mut input = input.take(packet_length as u64);

    // Packet ID
    let packet_id = read_var_int(&mut input)?;
    if packet_id != 0 {
        return Err(format!(
            "Error: The server responded with an unknown packet ID: 0x{packet_id:x}"
        ));
    }

    // JSON response
    let server_info = read_string(&mut input);

    // Check if all bytes were read successfully
    let bytes_left = input.bytes().count();
    if bytes_left != 0 {
        return Err(format!("ERROR: could not deserialize packet. Packet length is {packet_length}, but we only processed {} bytes.", packet_length - bytes_left as i32));
    }

    server_info
}

fn read_pong_response<T: Read>(input: &mut T) -> Result<i64, String> {
    // Packet length
    let packet_length = read_var_int(input)?;
    if packet_length < 0 {
        return Err(format!("Invalid packet length: {}", packet_length));
    }

    // Here we will ensure that we don't read more than **packet_length** bytes for this packet
    let mut input = input.take(packet_length as u64);

    // Packet ID
    let packet_id = read_var_int(&mut input)?;
    if packet_id != 1 {
        return Err(format!(
            "Error: The server responded with an unknown packet ID: 0x{packet_id:x}"
        ));
    }

    // Payload
    let payload = read_long(&mut input)?;

    // Check if all bytes were read successfully
    let bytes_left = input.bytes().count();
    if bytes_left != 0 {
        return Err(format!("ERROR: could not deserialize packet. Packet length is {packet_length}, but we only processed {} bytes.", packet_length - bytes_left as i32));
    }

    Ok(payload)
}

fn do_open_to_lan(arguments: &CommandLineArguments) -> ErrorCode {
    // Will listen for Open to LAN games in the local network. The game only supports Ipv4 sockets.
    let bind_address = SocketAddr::from(([0, 0, 0, 0], 4445));
    let ip = bind_address.ip().to_string();
    let port = bind_address.port().to_string();
    print_line_verbose(
        format!("Attempting to bind to {ip}:{port}").as_ref(),
        arguments,
    );
    let socket = match UdpSocket::bind(bind_address) {
        Ok(socket) => socket,
        Err(_) => {
            eprintln!("Error: Could not bind socket to {ip}:{port}");
            return ErrorCode::Protocol;
        }
    };
    print_line_verbose("Socket bind successful", arguments);

    let multicast_group = Ipv4Addr::from([224, 0, 2, 60]);
    let any_interface = Ipv4Addr::from([0, 0, 0, 0]);
    print_line_verbose(
        format!("Attempting to join multicast {multicast_group}").as_ref(),
        arguments,
    );
    if socket
        .join_multicast_v4(&multicast_group, &any_interface)
        .is_err()
    {
        let multicast_group_ip = multicast_group.to_string();
        eprintln!("Error: Could not join multicast {multicast_group_ip}");
        return ErrorCode::Protocol;
    }
    print_line_verbose("Joined multicast grop successfully", arguments);

    print_line_verbose("Listening for incoming packets...", arguments);
    let mut buffer = [0; 2048];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((packet_length, origin_socket)) => {
                let origin_socket_ip = origin_socket.ip().to_string();
                let origin_socket_port = origin_socket.port().to_string();

                // Parse received data. I refuse to use regular expressions because the format of the message is too simple
                // to bother adding another dependency.
                let buffer_portion: Vec<u8> = buffer.iter().cloned().take(packet_length).collect();
                let message = match String::from_utf8(buffer_portion) {
                    Ok(s) => s,
                    Err(_) => {
                        // Invalid format. Skip this packet.
                        continue;
                    }
                };
                let message = message.trim();
                if message.starts_with("[MOTD]") && message.ends_with("[/AD]") {
                    // Remove [MOTD] and [/AD] from the message
                    let i_start = "[MOTD]".len();
                    let i_end = message.len() - "[/AD]".len();
                    let trimmed_message = &message[i_start..i_end];

                    let mut split = trimmed_message.split("[/MOTD][AD]");
                    let motd = match split.next() {
                        Some(motd) => motd,
                        None => {
                            // Invalid format. Skip this packet.
                            continue;
                        }
                    };
                    let port = match split.next() {
                        Some(port) => port,
                        None => {
                            // Invalid format. Skip this packet.
                            continue;
                        }
                    };
                    if split.count() != 0 {
                        // We should've read everything in the packet already. If that's not the case this is considered an
                        // invalid format. Skip it.
                        continue;
                    }

                    print_line_verbose(format!("Received a packet of {packet_length} bytes from {origin_socket_ip}:{origin_socket_port}").as_ref(), arguments);
                    if arguments.raw_response {
                        println!("{message}");
                    } else {
                        let with_styles = can_print_colors(&std::io::stdout());
                        let styled_motd = chat::parse_string(motd, with_styles);
                        println!("{styled_motd}");
                        println!("Available at {origin_socket_ip}:{port}");
                        println!();
                    }

                    if let Err(e) = socket.leave_multicast_v4(&multicast_group, &any_interface) {
                        print_warning(format!("There was an error when attempting to leave multicast group {multicast_group}. More details below:").as_ref());
                        eprintln!("{e}");
                        return ErrorCode::Protocol;
                    } else {
                        print_line_verbose(
                            format!("Left multicast group {multicast_group}").as_ref(),
                            arguments,
                        );
                    }

                    break;
                } else {
                    print_line_verbose(format!("Ignored packet from {origin_socket_ip}:{origin_socket_port} because the format is not valid").as_ref(), arguments);
                }
            }
            Err(e) => {
                eprintln!("Error: I/O error when reading incoming data from a multicast socket");
                eprintln!("{e}");
                return ErrorCode::Protocol;
            }
        }
    }
    ErrorCode::Ok
}

fn print_line_verbose(msg: &str, arguments: &CommandLineArguments) {
    if arguments.verbose {
        let _ = stderr().write_all(msg.as_bytes());
        let _ = stderr().write_all("\n".as_bytes());
    }
}

fn print_warning(msg: &str) {
    let stderr = std::io::stderr().lock();
    let print_colors = can_print_colors(&stderr);
    if print_colors {
        eprint!("{FG_YELLOW}");
    }
    eprint!("WARNING: {msg}");
    if print_colors {
        eprint!("{RESET_COLORS}");
    }
    eprintln!();
}

fn can_print_colors<T: IsTerminal>(stream_handle: &T) -> bool {
    // Determines whether we should show ANSI colors and other font styles or not. Based on http://bixense.com/clicolors/
    let no_color_set = std::env::var("NO_COLOR").map_or(false, |v| v == "1");
    if no_color_set {
        return false;
    }

    let clicolor_force_set = std::env::var("CLICOLOR_FORCE").map_or(false, |v| v == "1");
    if clicolor_force_set {
        return true;
    }

    stream_handle.is_terminal()
}
