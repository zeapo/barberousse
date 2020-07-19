use anyhow::*;
use bat::PrettyPrinter;
use clap::Clap;
use serde::{Deserialize, Serialize};

#[derive(Clap, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ContentFormat {
    JSON,
    YAML,
    TEXT,
}

/// Pretty print the content using a given format
pub fn pretty_print(content: String, plain_print: bool, print_format: ContentFormat) -> Result<()> {
    // if the user requested plain print, or we're the stdin for another program (pipe)
    // then just print the content without bat
    if plain_print || atty::isnt(atty::Stream::Stdout) {
        println!("{}", content);
        Ok(())
    } else {
        let mut printer = PrettyPrinter::new();
        let printer = match print_format {
            ContentFormat::JSON => printer.language("json"),
            ContentFormat::YAML => printer.language("yaml"),
            _ => &mut printer,
        };

        let res = printer
            .grid(true)
            .line_numbers(true)
            .paging_mode(bat::PagingMode::QuitIfOneScreen)
            .pager("less")
            .theme("OneHalfDark")
            .input_from_bytes(content.as_bytes())
            .print();

        // avoid having to deal with missing Sync in std::error::Error
        if let Err(_) = res {
            Err(anyhow!(format!(
                "Unable to pretty print the secret's content"
            )))
        } else {
            Ok(())
        }
    }
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
                .map_err(|e| Error::new(e).context("Unable to parse JSON".to_string()))?;

            match destination_format {
                ContentFormat::JSON => serde_json::to_string_pretty(&json)?,
                ContentFormat::YAML => serde_yaml::to_string(&json)?,
                ContentFormat::TEXT => String::from(content),
            }
        }
        ContentFormat::YAML => {
            let yaml: serde_yaml::Value = serde_yaml::from_str(content)
                .map_err(|e| Error::new(e).context("Unable to parse YAML".to_string()))?;

            match destination_format {
                ContentFormat::JSON => serde_json::to_string_pretty(&yaml)?,
                ContentFormat::YAML => serde_yaml::to_string(&yaml)?,
                ContentFormat::TEXT => String::from(content),
            }
        }
        ContentFormat::TEXT => String::from(content),
    })
}
