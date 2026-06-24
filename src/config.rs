use crate::error::BlastError;
use crate::{extractor, runner};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlastConfig {
    pub base_url: String,
    pub headers: Option<HashMap<String, String>>,
    pub endpoints: Vec<Endpoint>,
    pub setup: Option<Vec<Endpoint>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Endpoint {
    pub name: String,
    pub method: String,
    pub path: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
    pub expect_status: Option<u16>,
    pub extract: Option<HashMap<String, String>>,
    pub tags: Option<Vec<String>>,
}

pub const CONFIG_FILENAME: &str = "blast.config.json";

impl BlastConfig {
    pub fn validate_path(path: &Path) -> Result<PathBuf, BlastError> {
        let absolute = fs::canonicalize(path).map_err(|e| {
            BlastError::Config(format!("directory doesn't exist {}: {e}", path.display()))
        })?;

        if !absolute.is_dir() {
            return Err(BlastError::Config(format!(
                "{} is not a directory",
                path.display()
            )));
        }
        Ok(absolute.join(CONFIG_FILENAME))
    }

    pub fn create(path: &Path) -> Result<PathBuf, BlastError> {
        let config_path = Self::validate_path(path)?;

        if config_path.exists() {
            return Err(BlastError::Config(format!(
                "{} already exists — delete it first to reinitialise",
                config_path.display()
            )));
        }

        let contents = serde_json::to_string_pretty(&Self::template())?;

        fs::write(&config_path, contents).map_err(|e| {
            BlastError::Config(format!("failed to write {}: {e}", config_path.display()))
        })?;

        Ok(config_path)
    }

    pub fn load(path: &Path) -> Result<Self, BlastError> {
        let config_path = if path.is_dir() {
            path.join(CONFIG_FILENAME)
        } else {
            path.to_path_buf()
        };

        let file_content = fs::read_to_string(&config_path).map_err(|e| {
            BlastError::Config(format!(
                "failed to read file from {}: {e}",
                config_path.display()
            ))
        })?;

        let config: Self = serde_json::from_str(&file_content)
            .map_err(|e| BlastError::Config(format!("failed to parse the config file: {e}")))?;

        //checking is the value in the config are valid
        config.validate()?;

        Ok(config)
    }

    fn template() -> Self {
        Self {
            base_url: String::from("http://localhost:3000/"),
            headers: Some(HashMap::from([(
                String::from("Content-Type"),
                String::from("application/json"),
            )])),
            setup: Some(vec![Endpoint {
                name: "health check".to_string(),
                method: "GET".to_string(),
                path: "/health".to_string(),
                headers: None,
                body: None,
                expect_status: Some(200),
                extract: None,
                tags: Some(vec!["check".to_string(), "seed".to_string()]),
            }]),
            endpoints: vec![
                Endpoint {
                    name: "health check".to_string(),
                    method: "GET".to_string(),
                    path: "/health".to_string(),
                    headers: None,
                    body: None,
                    expect_status: Some(200),
                    extract: None,
                    tags: Some(vec!["check".to_string(), "seed".to_string()]),
                },
                Endpoint {
                    name: "register user".to_string(),
                    method: "POST".to_string(),
                    path: "/api/v1/auth/register".to_string(),
                    headers: None,
                    body: Some(serde_json::json!({
                        "email":    "{{fake.email}}",
                        "password": "{{fake.password}}"
                    })),
                    expect_status: Some(201),
                    extract: None,
                    tags: None,
                },
                Endpoint {
                    name: "login".to_string(),
                    method: "POST".to_string(),
                    path: "/api/v1/auth/login".to_string(),
                    headers: None,
                    body: Some(serde_json::json!({
                        "email":    "test@example.com",
                        "password": "Seed1234!"
                    })),
                    expect_status: Some(200),
                    extract: Some(HashMap::from([(
                        "access_token".to_string(),
                        "data.access_token".to_string(),
                    )])),
                    tags: Some(vec![String::from("check"), String::from("seed")]),
                },
            ],
        }
    }

    fn validate(&self) -> Result<(), BlastError> {
        if self.base_url.is_empty() {
            return Err(BlastError::Config("base_url cannot be empty".to_string()));
        };

        if self.endpoints.is_empty() {
            return Err(BlastError::Config("endpoints cannot be empty".to_string()));
        };

        for (i, ep) in self.endpoints.iter().enumerate() {
            if ep.name.is_empty() {
                return Err(BlastError::Config(format!(
                    "endpoint {i} is missing a name"
                )));
            }
            if ep.path.is_empty() {
                return Err(BlastError::Config(format!(
                    "endpoint \"{}\" is missing a path",
                    ep.name
                )));
            }

            let valid = ["POST", "GET", "PATCH", "DELETE", "PUT"];
            let method_upper = ep.method.to_uppercase();

            if !valid.contains(&method_upper.as_str()) {
                return Err(BlastError::Config(format!(
                    "endpoint \"{}\" has invalid method \"{}\"\nvalid: {}",
                    ep.name,
                    ep.method,
                    valid.join(", ")
                )));
            }
        }

        Ok(())
    }

    pub async fn load_setup(
        &self,
        client: &reqwest::Client,
    ) -> Result<HashMap<String, String>, BlastError> {
        let mut ctx = HashMap::new();

        let setup_endpoint = match &self.setup {
            Some(s) => s,
            None => return Ok(ctx),
        };

        for endpoint in setup_endpoint {
            let result = runner::execute(client, endpoint, &self.base_url, &ctx).await;

            if !result.passed {
                return Err(BlastError::Setup(format!(
                    "setup endpoint \"{}\" failed with status {} — cannot continue\nresponse: {}",
                    endpoint.name,
                    result.actual_status,
                    result.error.unwrap_or_default()
                )));
            };

            if let (Some(rules), Some(body)) = (&endpoint.extract, &result.body) {
                extractor::extract(body, rules, &mut ctx);
            }
        }

        Ok(ctx)
    }

    pub fn endpoint_for(&self, tag: &str) -> Vec<&Endpoint> {
        let have_any_tags = self.endpoints.iter().any(|e| e.tags.is_some());

        if !have_any_tags {
            return self.endpoints.iter().collect();
        }

        self.endpoints
            .iter()
            .filter(|e| {
                e.tags
                    .as_ref()
                    .map(|tags| tags.iter().any(|t| t == tag))
                    .unwrap_or(false)
            })
            .collect()
    }
}
