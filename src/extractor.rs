use serde_json::Value;
use std::collections::HashMap;

pub type Context = HashMap<String, String>;

pub fn extract(body: &Value, extract_rule: &HashMap<String, String>, ctx: &mut Context) {
    for (var_name, path) in extract_rule {
        match get_path(body, path) {
            Some(value) => {
                let extracted = match value {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    Value::Bool(b) => Some(b.to_string()),
                    _ => {
                        eprintln!(
                            "warning: extract path \"{path}\" resolved to an object or array — skipped"
                        );
                        None
                    }
                };

                if let Some(val) = extracted {
                    ctx.insert(var_name.clone(), val);
                }
            }

            None => {
                eprintln!(
                    "warning: extract path \"{path}\" not found in response — \"{var_name}\" not set in context"
                );
            }
        }
    }
}

fn get_path<'a>(value: &'a Value, path: &'a str) -> Option<&'a Value> {
    let mut current = value;

    for segment in path.split('.') {
        current = match current {
            Value::Object(map) => map.get(segment)?,
            Value::Array(arr) => {
                let index = segment.parse::<usize>().ok()?;
                arr.get(index)?
            }
            _ => return None,
        }
    }

    Some(current)
}
