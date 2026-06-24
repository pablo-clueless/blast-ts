use fake::{Fake, faker};
use serde_json::Value;
use std::collections::HashMap;
// use rand::Rng;
use uuid;

pub type Context = HashMap<String, String>;

pub fn resolve_map(map: &HashMap<String, String>, ctx: &Context) -> HashMap<String, String> {
    map.iter()
        .map(|(k, v)| (k.clone(), resolve_str(v, ctx)))
        .collect()
}

pub fn resolve(value: &Value, ctx: &Context) -> Value {
    match value {
        Value::String(s) => Value::String(resolve_str(s, ctx)),
        Value::Object(map) => {
            let resolve = map
                .iter()
                .map(|(k, v)| (k.clone(), resolve(v, ctx)))
                .collect();

            Value::Object(resolve)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(|v| resolve(v, ctx)).collect()),
        other => other.clone(),
    }
}

fn resolve_str(s: &str, ctx: &Context) -> String {
    if s.starts_with("{{") && s.ends_with("}}") {
        let key = s[2..s.len() - 2].trim();
        resolve_key(key, ctx)
    } else if s.contains("{{") {
        resolved_mixed(s, ctx)
    } else {
        s.to_string()
    }
}

fn resolved_mixed(s: &str, ctx: &Context) -> String {
    let mut result = String::new();
    let mut rest = s;

    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);

        match rest[start..].find("}}") {
            Some(end) => {
                let key = &rest[start + 2..start + end].trim();
                result.push_str(&resolve_key(key, ctx));
                rest = &rest[start + end + 2..];
            }
            None => {
                result.push_str(&rest[start..]);
                return result;
            }
        }
    }

    result.push_str(rest);
    result
}

fn resolve_key(key: &str, ctx: &Context) -> String {
    if let Some(val) = ctx.get(key) {
        return val.clone();
    }
    match key {
        // internet
        "fake.email" => faker::internet::en::FreeEmail().fake(),
        "fake.username" => faker::internet::en::Username().fake(),
        "fake.password" => faker::internet::en::Password(8..16).fake(),
        "fake.url" => faker::internet::en::DomainSuffix().fake::<String>(),

        // names
        "fake.name" => faker::name::en::Name().fake(),
        "fake.firstname" => faker::name::en::FirstName().fake(),
        "fake.lastname" => faker::name::en::LastName().fake(),

        // lorem
        "fake.word" => faker::lorem::en::Word().fake(),
        "fake.sentence" => faker::lorem::en::Sentence(3..8).fake(),
        "fake.paragraph" => faker::lorem::en::Paragraph(1..3).fake(),

        // company
        "fake.company" => faker::company::en::CompanyName().fake(),

        // address
        "fake.city" => faker::address::en::CityName().fake(),
        "fake.country" => faker::address::en::CountryName().fake(),

        // ids
        "fake.uuid" => uuid::Uuid::new_v4().to_string(),
        // "fake.number"    => rand::thread_rng().gen_range(1..1000_u32).to_string(),

        // unknown placeholder — warn and return the original so the user notices
        _ => {
            eprintln!("warning: unknown placeholder {{{{{key}}}}} — left unchanged");
            format!("{{{{{key}}}}}")
        }
    }
}
