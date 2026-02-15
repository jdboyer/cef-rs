const BUNDLE_NAME: &str = "message_router";
const IDENTIFIER: &str = "com.example.message_router";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const EXECUTABLE_NAME: &str = "message_router";

#[cfg(target_os = "macos")]
mod macos {
    use cef::build_util::mac::*;
    use clap::Parser;
    use semver::Version;
    use std::{env, path::PathBuf};

    #[derive(Parser, Debug)]
    #[command(about, long_about = None)]
    struct Args {
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long, default_value = "English")]
        region: String,
    }

    pub fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let args = Args::parse();
        let output = match args.output {
            Some(output) => PathBuf::from(output),
            None => {
                let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("target")
                    .join("bundle");

                if !output_dir.exists() {
                    std::fs::create_dir_all(&output_dir)?;
                }

                output_dir
            }
        };

        let bundle_info = BundleInfo {
            name: crate::BUNDLE_NAME.to_owned(),
            identifier: crate::IDENTIFIER.to_owned(),
            display_name: crate::BUNDLE_NAME.to_owned(),
            development_region: args.region,
            version: Version::parse(crate::VERSION)?,
        };

        let bundle_path = output.join(format!("{}.app", crate::BUNDLE_NAME));
        if bundle_path.exists() {
            std::fs::remove_dir_all(&bundle_path)?;
        }

        let original_bundle_path =
            build_bundle(output.as_path(), crate::EXECUTABLE_NAME, bundle_info)?;
        std::fs::rename(&original_bundle_path, bundle_path)?;

        Ok(())
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        macos::main()
    }
    #[cfg(not(target_os = "macos"))]
    unimplemented!("Bundle creation is only implemented for macOS.");
}
