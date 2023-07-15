use serde_json::Value;

const RESET_STYLES: &str = "\x1B[0m";
const BOLD: &str = "\x1B[1m";
const ITALIC: &str = "\x1B[3m";
const UNDERLINE: &str = "\x1B[4m";
const SLOW_BLINK: &str = "\x1B[5m";
const STRIKETHROUGH: &str = "\x1B[9m";

pub fn chat_to_str(text: &Value, apply_styles: bool) -> String {
    // Parse text as a JSON chat object and apply font styles
    parse_component(text, apply_styles)
}

#[derive(Copy, Clone, Debug)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

#[derive(Copy, Clone, Default)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    obfuscated: bool,
    color: Option<Color>,
}

fn parse_component(text: &Value, actually_apply_styles: bool) -> String {
    let mut str = String::new();

    // Parse all components recursively and implement style inheritance for the current system (doesn't apply for the old system)
    let mut components = vec![(text, Style::default())];
    while let Some((comp, style)) = components.pop() {
        match comp {
            Value::Null => {} // Null is ignored
            Value::String(t) => apply_styles(t, &mut str, style, actually_apply_styles),
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
                    apply_styles(s, &mut str, style, actually_apply_styles);
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
            t => apply_styles(&t.to_string(), &mut str, style, actually_apply_styles), // Convert booleans and numbers into a string
        }
    }
    str
}

fn apply_styles(str: &str, out: &mut String, style: Style, actually_apply_styles: bool) {
    // Apply formatting using the current style inheritance system. Override styles from the parent style if needed.
    let mut str_iter = str.chars();
    let string_to_style: String = str_iter.by_ref().take_while(|c| *c != '§').collect();

    if actually_apply_styles {
        if let Some(color) = style.color {
            let red = color.red.to_string();
            let green = color.green.to_string();
            let blue = color.blue.to_string();
            push_ansi_color_sequence(out, &red, &green, &blue);
        }

        if style.bold {
            out.push_str(BOLD);
        }

        if style.italic {
            out.push_str(ITALIC);
        }

        if style.underline {
            out.push_str(UNDERLINE);
        }

        if style.strikethrough {
            out.push_str(STRIKETHROUGH);
        }

        if style.obfuscated {
            // ANSI colors doesn't support showing random text, so we blink it instead. Better than nothing, I guess...
            out.push_str(SLOW_BLINK);
        }
    }

    out.push_str(&string_to_style);
    if actually_apply_styles {
        out.push_str(RESET_STYLES);
    }

    // Apply formatting using the old system. This system takes precedence over the current system and doesn't participate
    // in the style inheritance system, so any styles applied here don't propagate to child components.
    // The way this old system work is very similar to ANSI colors in terminals. It will apply a style based on a control
    // sequence until it finds a reset sequence. It is possible to apply multiple styles at once.
    while let Some(control_sequence) = str_iter.next() {
        let string_to_style: String = str_iter.by_ref().take_while(|c| *c != '§').collect();
        if actually_apply_styles {
            match control_sequence {
                // Colors
                '0' => push_ansi_color_sequence(out, "0", "0", "0"),
                '1' => push_ansi_color_sequence(out, "0", "0", "170"),
                '2' => push_ansi_color_sequence(out, "0", "170", "0"),
                '3' => push_ansi_color_sequence(out, "0", "170", "170"),
                '4' => push_ansi_color_sequence(out, "170", "0", "0"),
                '5' => push_ansi_color_sequence(out, "170", "0", "170"),
                '6' => push_ansi_color_sequence(out, "255", "170", "0"),
                '7' => push_ansi_color_sequence(out, "170", "170", "170"),
                '8' => push_ansi_color_sequence(out, "85", "85", "85"),
                '9' => push_ansi_color_sequence(out, "85", "85", "255"),
                'a' => push_ansi_color_sequence(out, "85", "255", "85"),
                'b' => push_ansi_color_sequence(out, "85", "255", "255"),
                'c' => push_ansi_color_sequence(out, "255", "85", "85"),
                'd' => push_ansi_color_sequence(out, "255", "85", "255"),
                'e' => push_ansi_color_sequence(out, "255", "255", "85"),
                'f' => push_ansi_color_sequence(out, "255", "255", "255"),

                // Styles
                'k' => out.push_str(SLOW_BLINK), // Obfuscated
                'l' => out.push_str(BOLD),
                'm' => out.push_str(STRIKETHROUGH),
                'n' => out.push_str(UNDERLINE),
                'o' => out.push_str(ITALIC),
                'r' => out.push_str(RESET_STYLES),

                _ => {}
            };
        }

        out.push_str(string_to_style.as_ref());
        // NOTE: We should only reset styles only if we encounter the 'r' character or we stop using the old style system
    }

    if actually_apply_styles {
        out.push_str(RESET_STYLES);
    }
}

fn push_ansi_color_sequence(out: &mut String, red: &str, green: &str, blue: &str) {
    // Using 24-bit colors in the format of "38;2;R;G;B", where R, G and B are decimal values in the range of [0-255]
    out.push_str("\x1B[38;2;");

    out.push_str(red);
    out.push(';');

    out.push_str(green);
    out.push(';');

    out.push_str(blue);
    out.push('m');
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

    const APPLY_FONT_STYLES: bool = false;

    #[test]
    fn test_parse_null() {
        let text = json!(null);
        let expected = "";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_boolean() {
        let text = json!(true);
        let expected = "true";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_number() {
        let text = json!(23.4);
        let expected = "23.4";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_string() {
        let text = json!("THIS IS TEXT");
        let expected = "THIS IS TEXT";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_empty_object_component() {
        let text = json!({});
        let expected = "";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
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
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
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
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
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
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
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
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
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
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
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
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_empty_array() {
        let text = json!([]);
        let expected = "";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_array_of_primitive_types() {
        let text = json!([true, false, 45.6]);
        let expected = "truefalse45.6";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_array_of_strings() {
        let text = json!(["Hello, ", "world!"]);
        let expected = "Hello, world!";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_nested_arrays_of_strings() {
        let text = json!([[["Hello, ", "world!"]]]);
        let expected = "Hello, world!";
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
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
        let result = chat_to_str(&text, APPLY_FONT_STYLES);
        assert_eq!(expected, result);
    }
}
