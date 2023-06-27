use colored::Colorize;
use serde_json::Value;

pub fn chat_to_str(text: &Value) -> String {
    // Parse text as a JSON chat object and apply font styles
    parse_component(text)
}

#[derive(Copy, Clone, Debug)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl From<Color> for colored::Color {
    fn from(value: Color) -> Self {
        colored::Color::TrueColor {
            r: value.red,
            g: value.green,
            b: value.blue,
        }
    }
}

#[derive(Copy, Clone)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    obfuscated: bool,
    color: Option<Color>,
}

impl Style {
    fn default() -> Self {
        Style {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            obfuscated: false,
            color: None,
        }
    }
}

fn parse_component(text: &Value) -> String {
    let mut str = String::new();

    // Parse all components recursively and implement style inheritance for the current system (doesn't apply for the old system)
    let mut components = vec![(text, Style::default())];
    while let Some((comp, style)) = components.pop() {
        match comp {
            Value::Null => {} // Null is ignored
            Value::String(t) => apply_styles(t, &mut str, style),
            Value::Object(chat_object) => {
                // Set styles for this component
                let mut style = style;
                if let Some(Value::Bool(bold)) = chat_object.get("bold") {
                    style.bold = *bold;
                }

                if let Some(Value::Bool(italic)) = chat_object.get("italic") {
                    style.italic = *italic;
                }

                if let Some(Value::Bool(underline)) = chat_object.get("underlined") {
                    style.underline = *underline;
                }

                if let Some(Value::Bool(strikethrough)) = chat_object.get("strikethrough") {
                    style.strikethrough = *strikethrough;
                }

                if let Some(Value::Bool(obfuscated)) = chat_object.get("obfuscated") {
                    style.obfuscated = *obfuscated;
                }

                if let Some(Value::String(color)) = chat_object.get("color") {
                    style.color = parse_color(color);
                }

                // Parse string
                if let Some(Value::String(s)) = &chat_object.get("text") {
                    apply_styles(s, &mut str, style);
                }

                // Parse sibling components. If the "extra" property is not an array we ignore it.
                if let Some(value) = &chat_object.get("extra") {
                    if value.is_array() {
                        components.push((*value, style));
                    }
                }
            }
            Value::Array(siblings) => {
                for sibling in siblings.iter().rev() {
                    components.push((sibling, style));
                }
            }
            t => apply_styles(&t.to_string(), &mut str, style), // Convert booleans and numbers into a string
        }
    }
    str
}

fn apply_styles(str: &str, out: &mut String, style: Style) {
    // TODO: We are allocating a lot of memory in this function unnecessarily because I haven't found a way to use the API
    // efficiently. Maybe I should drop the 'colored' crate and implement ANSI colors myself? Could be easier, idk.

    let mut str_iter = str.chars();

    // Apply formatting using the current style inheritance system. Override styles from the parent style if needed.
    let mut styled_string = str_iter
        .by_ref()
        .take_while(|c| *c != '§')
        .collect::<String>()
        .normal()
        .clear();

    if style.bold {
        styled_string = styled_string.bold();
    }

    if style.italic {
        styled_string = styled_string.italic();
    }

    if style.underline {
        styled_string = styled_string.underline();
    }

    if style.strikethrough {
        styled_string = styled_string.strikethrough();
    }

    if style.obfuscated {
        styled_string = styled_string.blink();
    }

    if let Some(color) = style.color {
        styled_string = styled_string.color(color);
    }

    out.push_str(styled_string.to_string().as_ref());

    // Apply formatting using the old system. This system takes precedence over the current system and doesn't participate
    // in the style inheritance system, so any styles applied here don't propagate to child components.
    // The way this old system work is very similar to ANSI colors in terminals. It will apply a style based on a control
    // sequence until it finds a reset sequence. It is possible to apply multiple styles at once.
    let mut style = Style::default();
    while let Some(control_sequence) = str_iter.next() {
        let mut styled_string = str_iter
            .by_ref()
            .take_while(|c| *c != '§')
            .collect::<String>()
            .normal()
            .clear();
        match control_sequence {
            // Colors
            '0' => {
                style.color = Some(Color {
                    red: 0x00,
                    green: 0x00,
                    blue: 0x00,
                })
            }
            '1' => {
                style.color = Some(Color {
                    red: 0x00,
                    green: 0x00,
                    blue: 0xaa,
                })
            }
            '2' => {
                style.color = Some(Color {
                    red: 0x00,
                    green: 0xaa,
                    blue: 0x00,
                })
            }
            '3' => {
                style.color = Some(Color {
                    red: 0x00,
                    green: 0xaa,
                    blue: 0xaa,
                })
            }
            '4' => {
                style.color = Some(Color {
                    red: 0xaa,
                    green: 0x00,
                    blue: 0x00,
                })
            }
            '5' => {
                style.color = Some(Color {
                    red: 0xaa,
                    green: 0x00,
                    blue: 0xaa,
                })
            }
            '6' => {
                style.color = Some(Color {
                    red: 0xff,
                    green: 0xaa,
                    blue: 0x00,
                })
            }
            '7' => {
                style.color = Some(Color {
                    red: 0xaa,
                    green: 0xaa,
                    blue: 0xaa,
                })
            }
            '8' => {
                style.color = Some(Color {
                    red: 0x55,
                    green: 0x55,
                    blue: 0x55,
                })
            }
            '9' => {
                style.color = Some(Color {
                    red: 0x55,
                    green: 0x55,
                    blue: 0xff,
                })
            }
            'a' => {
                style.color = Some(Color {
                    red: 0x55,
                    green: 0xff,
                    blue: 0x55,
                })
            }
            'b' => {
                style.color = Some(Color {
                    red: 0x55,
                    green: 0xff,
                    blue: 0xff,
                })
            }
            'c' => {
                style.color = Some(Color {
                    red: 0xff,
                    green: 0x55,
                    blue: 0x55,
                })
            }
            'd' => {
                style.color = Some(Color {
                    red: 0xff,
                    green: 0x55,
                    blue: 0xff,
                })
            }
            'e' => {
                style.color = Some(Color {
                    red: 0xff,
                    green: 0xff,
                    blue: 0x55,
                })
            }
            'f' => {
                style.color = Some(Color {
                    red: 0xff,
                    green: 0xff,
                    blue: 0xff,
                })
            }

            // Styles
            'k' => style.obfuscated = true, // Obfuscated / Random
            'l' => style.bold = true,
            'm' => style.strikethrough = true,
            'n' => style.underline = true,
            'o' => style.italic = true,
            'r' => style = Style::default(), // Reset

            _ => {}
        };

        if style.bold {
            styled_string = styled_string.bold();
        }

        if style.italic {
            styled_string = styled_string.italic();
        }

        if style.underline {
            styled_string = styled_string.underline();
        }

        if style.strikethrough {
            styled_string = styled_string.strikethrough();
        }

        if style.obfuscated {
            styled_string = styled_string.blink();
        }

        if let Some(color) = style.color {
            styled_string = styled_string.color(color);
        }

        out.push_str(styled_string.to_string().as_ref());
    }
}

fn parse_color(color: &str) -> Option<Color> {
    match color {
        "black" => Some(Color {
            red: 0x00,
            green: 0x00,
            blue: 0x00,
        }),
        "dark_blue" => Some(Color {
            red: 0x00,
            green: 0x00,
            blue: 0xaa,
        }),
        "dark_green" => Some(Color {
            red: 0x00,
            green: 0xaa,
            blue: 0x00,
        }),
        "dark_aqua" => Some(Color {
            red: 0x00,
            green: 0xaa,
            blue: 0xaa,
        }),
        "dark_red" => Some(Color {
            red: 0xaa,
            green: 0x00,
            blue: 0x00,
        }),
        "dark_purple" => Some(Color {
            red: 0xaa,
            green: 0x00,
            blue: 0xaa,
        }),
        "gold" => Some(Color {
            red: 0xff,
            green: 0xaa,
            blue: 0x00,
        }),
        "gray" => Some(Color {
            red: 0xaa,
            green: 0xaa,
            blue: 0xaa,
        }),
        "dark_gray" => Some(Color {
            red: 0x55,
            green: 0x55,
            blue: 0x55,
        }),
        "blue" => Some(Color {
            red: 0x55,
            green: 0x55,
            blue: 0xff,
        }),
        "green" => Some(Color {
            red: 0x55,
            green: 0xff,
            blue: 0x55,
        }),
        "aqua" => Some(Color {
            red: 0x55,
            green: 0xff,
            blue: 0xff,
        }),
        "red" => Some(Color {
            red: 0xff,
            green: 0x55,
            blue: 0x55,
        }),
        "light_purple" => Some(Color {
            red: 0xff,
            green: 0x55,
            blue: 0xff,
        }),
        "yellow" => Some(Color {
            red: 0xff,
            green: 0xff,
            blue: 0x55,
        }),
        "white" => Some(Color {
            red: 0xff,
            green: 0xff,
            blue: 0xff,
        }),
        _ => parse_web_color(color),
    }
}

fn parse_web_color(color: &str) -> Option<Color> {
    // TODO: Support more formats
    if color.starts_with('#') && color.len() == 7 {
        // Color in the format of "#RRGGBB", where RR, GG, BB are hexadecimal numbers
        let hexnum = u32::from_str_radix(&color[1..], 16);
        if let Ok(hexnum) = hexnum {
            return Some(Color {
                red: (hexnum >> 16) as u8,
                green: (hexnum >> 8) as u8,
                blue: hexnum as u8,
            });
        }
    }

    None
}

#[cfg(test)]
mod chat_component_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_null() {
        let text = json!(null);
        let expected = "";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_boolean() {
        let text = json!(true);
        let expected = "true";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_number() {
        let text = json!(23.4);
        let expected = "23.4";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_string() {
        let text = json!("THIS IS TEXT");
        let expected = "THIS IS TEXT";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_empty_object_component() {
        let text = json!({});
        let expected = "";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_simple_object_component() {
        let text = json!(
            {
                "text": "THIS IS TEXT"
            }
        );
        let expected = "THIS IS TEXT";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_object_component_with_siblings() {
        let text = json!(
            {
                "text": "THIS",
                "extra": [
                    {
                        "text": " IS"
                    },
                    {
                        "text": " TEXT"
                    }
                ]
            }
        );
        let expected = "THIS IS TEXT";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_object_component_with_nested_siblings() {
        let text = json!(
            {
                "text": "THIS",
                "extra": [
                    {
                        "text": " IS",
                        "extra": [
                            {
                                "text": " TEXT"
                            }
                        ]
                    }
                ]
            }
        );
        let expected = "THIS IS TEXT";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_object_component_with_even_more_nested_siblings() {
        let text = json!(
            {
                "text": "THIS",
                "extra": [
                    {
                        "text": " IS",
                        "extra": [
                            {
                                "text": " SOME"
                            }
                        ]
                    },
                    {
                        "text": " TEXT"
                    }
                ]
            }
        );
        let expected = "THIS IS SOME TEXT";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_object_component_with_invalid_text() {
        // The vanilla Minecraft client may fail to parse the following chat object, but we ignore any invalid properties
        // instead
        let text = json!(
            {
                "text": true
            }
        );
        let expected = "";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_object_component_with_invalid_extra_field() {
        // The vanilla Minecraft client may fail to parse the following chat object, but we ignore any invalid properties
        // instead
        let text = json!(
            {
                "text": "THIS IS A",
                "extra": " TEST"
            }
        );
        let expected = "THIS IS A";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_empty_array() {
        let text = json!([]);
        let expected = "";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_array_of_primitive_types() {
        let text = json!([true, false, 45.6]);
        let expected = "truefalse45.6";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_array_of_strings() {
        let text = json!(["Hello, ", "world!"]);
        let expected = "Hello, world!";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_nested_arrays_of_strings() {
        let text = json!([[["Hello, ", "world!"]]]);
        let expected = "Hello, world!";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_array_of_objects() {
        let text = json!(
            [
                {
                    "text": "Hello, world!"
                }
            ]
        );
        let expected = "Hello, world!";
        let result = chat_to_str(&text);
        assert_eq!(expected, result);
    }
}
