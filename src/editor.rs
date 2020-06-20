use std::{
    env::var,
    io::{Read, Seek, SeekFrom, Write},
    process::{Command, ExitStatus},
};

use anyhow::*;

use crate::utils::{format_convert, ContentFormat};

/// Opens an editor to modify the `content`, and returns the
/// edited content
pub fn edit_content(
    editor: Option<String>,
    remote_content: &String,
    secret_format: ContentFormat,
    edit_format: ContentFormat,
) -> Result<String> {
    // FIXME this is not secure as the tempfile will be visible in /tmp
    //  would be better idea to create a folder in in /dev/shm with a file mode 600
    //  so that only the user can edit/see it, then put the file in it
    let mut tf = tempfile::NamedTempFile::new()?;

    let formatted_content: String = format_convert(&remote_content, &secret_format, &edit_format)?;

    // write the yaml to content
    write!(tf, "{}", formatted_content)?;

    // try to edit this secret, until we succeed , or that the
    let edited_content: String = loop {
        // Open the editor \o/
        open_editor(
            editor.clone(),
            tf.as_ref()
                .to_owned()
                .to_str()
                .expect("Unable to handle temp file... this should not happen"),
        )?;

        // read the file back
        tf.seek(SeekFrom::Start(0))?;
        let mut saved_content = String::new();
        tf.read_to_string(&mut saved_content)?;

        // convert the content back to its original format
        let edited = format_convert(&saved_content, &edit_format, &secret_format);

        match edited {
            Ok(content) => {
                break content;
            }
            Err(e) => {
                // TODO add a yes/no/ignore question to see if we continue, discard, ignore and save as text
                eprintln!("{:?}", e);
                let decision = promptly::prompt_default("Do you want to edit again?", true)?;
                if !decision {
                    break remote_content.clone();
                }
            }
        };
    };

    // this should not be required as per documentation
    tf.close()?;
    Ok(edited_content)
}

/// Opens the editor to edit a specific file
fn open_editor(editor: Option<String>, path: &str) -> Result<ExitStatus> {
    // Open the editor \o/
    let editor = editor.unwrap_or_else(|| {
        // yeah, default to nano if nothing is available
        var("EDITOR").unwrap_or("nano".to_string())
    });

    let exit = Command::new(editor)
        .arg(path)
        .spawn()
        .map_err(|e| Error::new(e).context("Unable to launch editor".to_string()))?
        .wait()?;

    Ok(exit)
}
