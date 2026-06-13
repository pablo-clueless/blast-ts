use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone )]
pub struct BlastConfig {
    pub base_url:String,
    pub headers: Option<HashMap<String, String>>,
    pub endpoints: Vec<Endpoint>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Endpoint {
    pub name:          String,
    pub method:        String,
    pub path:          String,
    pub headers:       Option<HashMap<String, String>>,
    pub body:          Option<serde_json::Value>,
    pub expect_status: Option<u16>,
    pub extract:       Option<HashMap<String, String>>,
    pub tags:          Option<Vec<String>>
}

pub const CONFIG_FILENAME:&str = "blast.config.json";

impl BlastConfig {

    pub fn validate_path(path:&Path) -> Result<PathBuf>{
        let absolute = fs::canonicalize(path)
            .with_context(||format!(
                "directory doesn't exist {}",
                path.display()
        ))?;
        
        if !absolute.is_dir() {
            anyhow::bail!(
                "{} is not a directory",
                path.display()
            )
        }
        Ok(absolute.join(CONFIG_FILENAME))
    }

    pub fn create(path:&Path) -> Result<PathBuf> {
        let config_path = Self::validate_path(path)?;

        if config_path.exists() {
            anyhow::bail!(
                "{} already exists — delete it first to reinitialise",
                config_path.display()
            )
        }

        let contents = serde_json::to_string_pretty(&Self::template()).with_context(
            ||format!(
                "failed to serialized default config"
            )
        )?;
        
        fs::write(&config_path, contents).with_context(
            ||format!(
                "failed to write {}",
                config_path.display()
            )
        )?;

        Ok(config_path)
    }
    
    pub fn load(path: &Path) -> Result<Self>{
        let config_path = if path.is_dir() {
            path.join(CONFIG_FILENAME)
        } else {
            path.to_path_buf()
        };

        let file_content = fs::read_to_string(&config_path).with_context(
            ||format!(
                "failed to read file from {}",
                path.display(),
            )
        )?;

        let config:Self = serde_json::from_str(&file_content).with_context(
            ||format!(
                "failed to deserialized the config file"
            )
        )?;

        //checking is the value in the config are valid
        config.validate()?;
        
        Ok(config)
    }

    // pub fn load_from_cwd() -> Result<Self> {
    //     let cwd = std::env::current_dir()
    //         .context("could not determine current directory")?;
    //     Self::load(&cwd)
    // }

    fn template() -> Self {
        Self { 
            base_url: String::from("http://localhost:3000/") , 
            headers: Some(HashMap::from([(
                String::from("Content-Type"),
                String::from("application/json")
            )])), 
            endpoints: vec![
                Endpoint {
                    name:          "health check".to_string(),
                    method:        "GET".to_string(),
                    path:          "/health".to_string(),
                    headers:       None,
                    body:          None,
                    expect_status: Some(200),
                    extract:       None,
                    tags:          Some(vec!["check".to_string(), "seed".to_string() ])
                },
                Endpoint {
                    name:   "register user".to_string(),
                    method: "POST".to_string(),
                    path:   "/api/v1/auth/register".to_string(),
                    headers: None,
                    body:   Some(serde_json::json!({
                        "email":    "{{fake.email}}",
                        "password": "{{fake.password}}"
                    })),
                    expect_status: Some(201),
                    extract:       None,
                    tags:          None,
                },
                Endpoint {
                    name:   "login".to_string(),
                    method: "POST".to_string(),
                    path:   "/api/v1/auth/login".to_string(),
                    headers: None,
                    body:   Some(serde_json::json!({
                        "email":    "test@example.com",
                        "password": "Seed1234!"
                    })),
                    expect_status: Some(200),
                    extract: Some(HashMap::from([
                        ("access_token".to_string(), "data.access_token".to_string()),
                    ])),
                    tags:   Some(vec![String::from("check"), String::from("seed")])
                }
            ]
        }
    }

    fn validate(&self) -> Result<()>{
        if self.base_url.is_empty() {
            anyhow::bail!(
                "base_url cannot be empty"
            )
        };

        if self.endpoints.is_empty(){
            anyhow::bail!(
                "endpoints cannot be empty"
            )
        };

        for (i, ep) in self.endpoints.iter().enumerate(){
            if ep.name.is_empty() {
                anyhow::bail!("endpoint {} is missing a name", i);
            }
            if ep.path.is_empty() {
                anyhow::bail!("endpoint \"{}\" is missing a path", ep.name);
            }

            let valid = ["POST", "GET", "PATCH", "DELETE", "PUT"];
            let method_upper = ep.method.to_uppercase();

            if !valid.contains(&method_upper.as_str()){
                anyhow::bail!(
                    "endpoint \"{}\" has invalid method \"{}\"\nvalid: {}",
                    ep.name, ep.method, valid.join(", ")
                )
            }
        }
        
        Ok(())
    }


    pub fn endpoint_for(&self, tag:&str) -> Vec<&Endpoint>{
        let have_any_tags = self.endpoints
            .iter()
            .any(|e| e.tags.is_some());

        if !have_any_tags {
            return self.endpoints.iter().collect()
        }

        self.endpoints.iter()
            .filter(
                |e| {
                    e.tags.as_ref().map(
                        |tags| tags.iter().any(|t| t == tag)
                    ).unwrap_or(false)
                }
            ).collect()
    }
}