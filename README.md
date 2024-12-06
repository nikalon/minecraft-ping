# minecraft-ping
A small CLI tool that queries information from Minecraft Java Edition servers.

It supports colored output, icon downloading, IPv4, IPv6 and ping via Open To LAN for singleplayer maps. Example:

![Screenshot of an example of this program](images\example.png)

## Building
Install a [Rust compiler](https://www.rust-lang.org/tools/install) and execute the following command:
```bash
cargo build
```
The generated executable is `mping` and will be located at `target/debug/` directory.

## Usage
Provide an IP address or a domain name as the first argument. Optionally, you can set the port as the second argument. If no port is provided it will default to 25565.
```bash
mping [vfr] ADDRESS [PORT]
```
Examples:
```bash
$ mping 127.0.0.1
$ mping -v 127.0.0.1 8123
$ mping -f ::1
$ mping superduperserver.net 1234
```

When you use `-l` or `--lan` flag you don't have to provide any more arguments. Example:
```bash
$ mping -l
```

The following flags are supported:
- `-v`, `--verbose`: prints debugging information when connecting to the remote server.
- `-f`, `--favicon`: downloads the server icon into a png file.
- `-r`, `--raw-response`: prints the raw response from the server directly.
- `-l`, `--lan`: keep listening for singleplayer maps in the local network. When a local game is available it prints the IP and port.
