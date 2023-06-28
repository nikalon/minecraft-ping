#[derive(Clone, PartialEq, Debug)]
pub struct CommandLineArguments {
    pub get_favicon: bool,
    pub raw_response: bool,
    pub verbose: bool,
    pub host: String,
    pub port: u16,
}

impl CommandLineArguments {
    pub fn parse<T: Iterator<Item = String>>(args: &mut T) -> Result<Self, String> {
        let mut arguments = CommandLineArguments {
            get_favicon: false,
            raw_response: false,
            verbose: false,
            host: "".to_owned(),
            port: 25565,
        };

        // Skip executable name
        let mut args = args.skip(1).peekable();

        // Parse flags
        let flags_iter = args.by_ref();
        while let Some(flag) = flags_iter.peek() {
            if flag.starts_with('-') {
                // Consume the next item to advance the iterator
                let flag = flags_iter.next().unwrap(); // TODO: Remove unwrap()
                match flag.as_ref() {
                    "-v" | "--verbose" => arguments.verbose = true,
                    "-f" | "--favicon" => arguments.get_favicon = true,
                    "-r" | "--raw-response" => arguments.raw_response = true,
                    _ => return Err(format!("Unrecognized flag: {flag}")),
                }
            } else {
                // No more flags left to parse
                break;
            }
        }

        // Parse address as a required argument
        match args.next() {
            Some(host) => arguments.host = host,
            None => return Err("No address provided".to_owned()),
        }

        // Parse port as an optional argument
        if let Some(port) = args.next() {
            arguments.port = port
                .parse()
                .map_err(|_| format!("Invalid port \"{port}\""))?;
        }

        // There should be no more arguments to parse
        if args.count() != 0 {
            return Err("Invalid arguments".to_owned());
        }

        Ok(arguments)
    }
}

#[cfg(test)]
mod cli_arguments_tests {
    use super::*;

    #[test]
    fn test_parse_when_no_arguments_given() {
        let cli_args = [String::from("./command")];
        let args = CommandLineArguments::parse(&mut cli_args.into_iter());

        assert!(args.is_err());
    }

    #[test]
    fn test_parse_address() {
        let cli_args = [String::from("./command"), String::from("127.0.0.1")];
        let args = CommandLineArguments::parse(&mut cli_args.into_iter());
        let expected = Ok(CommandLineArguments {
            get_favicon: false,
            raw_response: false,
            verbose: false,
            host: "127.0.0.1".to_owned(),
            port: 25565,
        });
        assert_eq!(expected, args);
    }

    #[test]
    fn test_parse_address_and_port() {
        let cli_args = [
            String::from("./command"),
            String::from("127.0.0.1"),
            String::from("25560"),
        ];
        let args = CommandLineArguments::parse(&mut cli_args.into_iter());
        let expected = Ok(CommandLineArguments {
            get_favicon: false,
            raw_response: false,
            verbose: false,
            host: "127.0.0.1".to_owned(),
            port: 25560,
        });
        assert_eq!(expected, args);
    }

    #[test]
    fn test_parse_unrecognized_flag() {
        let cli_args = [
            String::from("./command"),
            String::from("--unrecognized-flag"),
            String::from("localhost"),
        ];
        let args = CommandLineArguments::parse(&mut cli_args.into_iter());
        assert!(args.is_err());
    }

    #[test]
    fn test_parse_verbose_flag() {
        let cli_args = [
            String::from("./command"),
            String::from("-v"),
            String::from("localhost"),
        ];
        let args = CommandLineArguments::parse(&mut cli_args.into_iter());
        let expected = Ok(CommandLineArguments {
            get_favicon: false,
            raw_response: false,
            verbose: true,
            host: "localhost".to_owned(),
            port: 25565,
        });
        assert_eq!(expected, args);
    }

    #[test]
    fn test_parse_verbose_flag_and_address_and_port() {
        let cli_args = [
            String::from("./command"),
            String::from("-v"),
            String::from("localhost"),
            String::from("1000"),
        ];
        let args = CommandLineArguments::parse(&mut cli_args.into_iter());
        let expected = Ok(CommandLineArguments {
            get_favicon: false,
            raw_response: false,
            verbose: true,
            host: "localhost".to_owned(),
            port: 1000,
        });
        assert_eq!(expected, args);
    }

    #[test]
    fn test_parse_disordered_flags() {
        let cli_args = [
            String::from("./command"),
            String::from("localhost"),
            String::from("-v"),
            String::from("1000"),
        ];
        let args = CommandLineArguments::parse(&mut cli_args.into_iter());
        assert!(args.is_err());
    }

    #[test]
    fn test_parse_flags_at_end() {
        let cli_args = [
            String::from("./command"),
            String::from("localhost"),
            String::from("1000"),
            String::from("-v"),
        ];
        let args = CommandLineArguments::parse(&mut cli_args.into_iter());
        assert!(args.is_err());
    }
}
