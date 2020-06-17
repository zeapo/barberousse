use anyhow::Result;
use clap::Clap;
use serde::{Deserialize, Serialize};

#[derive(Clap, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ContentFormat {
    JSON,
    YAML,
    TEXT,
}

/// Takes a string in [source_format] and outputs a string in [destination_format]
pub fn format_convert(
    content: &String,
    source_format: &ContentFormat,
    destination_format: &ContentFormat,
) -> Result<String> {
    Ok(match source_format {
        ContentFormat::JSON => {
            let json: serde_json::Value = serde_json::from_str(content)
                .map_err(|e| anyhow::Error::new(e).context("Unable to parse JSON".to_string()))?;

            match destination_format {
                ContentFormat::JSON => serde_json::to_string_pretty(&json)?,
                ContentFormat::YAML => serde_yaml::to_string(&json)?,
                ContentFormat::TEXT => String::from(content),
            }
        }
        ContentFormat::YAML => {
            let yaml: serde_yaml::Value = serde_yaml::from_str(content)
                .map_err(|e| anyhow::Error::new(e).context("Unable to parse YAML".to_string()))?;

            match destination_format {
                ContentFormat::JSON => serde_json::to_string_pretty(&yaml)?,
                ContentFormat::YAML => serde_yaml::to_string(&yaml)?,
                ContentFormat::TEXT => String::from(content),
            }
        }
        ContentFormat::TEXT => String::from(content),
    })
}
