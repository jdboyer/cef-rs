#[cfg(target_os = "windows")]
include!("src/shared/resources.rs");

#[cfg(target_os = "windows")]
fn main() {
    winres::WindowsResource::new()
        .set_icon_with_id("resources/win/message_router.ico", &IDI_MESSAGE_ROUTER.to_string())
        .set_icon_with_id("resources/win/small.ico", &IDI_SMALL.to_string())
        .compile()
        .unwrap();
}

#[cfg(target_os = "linux")]
fn main() {
    use std::{env, fs, path::Path};

    let out_dir = env::var("OUT_DIR").unwrap();
    // OUT_DIR is something like target/debug/build/<pkg>/out
    // We need target/debug/
    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .expect("could not find target dir");

    // Copy HTML resources to target/<profile>/files/
    let files_dir = target_dir.join("files");
    fs::create_dir_all(&files_dir).unwrap();

    let resources_dir = Path::new("resources/linux");
    if resources_dir.exists() {
        for entry in fs::read_dir(resources_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), files_dir.join(entry.file_name())).unwrap();
            }
        }
    }

    println!("cargo::rerun-if-changed=resources/linux");
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn main() {}
