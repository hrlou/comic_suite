use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
fn generate_inno_installer() {
    let version = env!("CARGO_PKG_VERSION");
let iss_content = format!(
    r#"
[Setup]
AppName=Comic Suite
AppVersion={}
DefaultDirName={{pf}}\Comic Suite
DefaultGroupName=Comic Suite
OutputDir=dist
OutputBaseFilename=Comic Suite Installer
Compression=lzma
SolidCompression=yes

[Files]
Source: "release\comic_reader.exe"; DestDir: "{{app}}"; Flags: ignoreversion
Source: "release\comic_thumbgen.exe"; DestDir: "{{app}}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{{app}}"; Flags: ignoreversion
Source: "..\LICENSE.md"; DestDir: "{{app}}"; Flags: ignoreversion
Source: "..\comic_reader\assets\*"; DestDir: "{{app}}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{{group}}\Comic Suite"; Filename: "{{app}}\comic_reader.exe"
Name: "{{group}}\Uninstall Comic Suite"; Filename: "{{uninstallexe}}"

[Registry]
// Associate .cbz files
Root: HKCR; Subkey: ".cbz"; ValueType: string; ValueName: ""; ValueData: "ComicSuite.cbz"; Flags: uninsdeletevalue
Root: HKCR; Subkey: "ComicSuite.cbz"; ValueType: string; ValueName: ""; ValueData: "Comic Book Zip"; Flags: uninsdeletekey
Root: HKCR; Subkey: "ComicSuite.cbz\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{{app}}\comic_reader.exe,0"
Root: HKCR; Subkey: "ComicSuite.cbz\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{{app}}\comic_reader.exe"" ""%1""" 
"#,

    version
);

    let iss_path = Path::new("../target/installer.iss");
    fs::write(&iss_path, iss_content).expect("Failed to write installer.iss");

    let iscc = which::which("ISCC.exe").or_else(|_| which::which("iscc.exe"));
    match iscc {
        Ok(iscc_path) => {
            let status = Command::new(iscc_path)
                .arg(&iss_path)
                .status()
                .expect("Failed to run ISCC.exe");
            if !status.success() {
                eprintln!("ISCC.exe failed to build the installer.");
            }
        }
        Err(_) => {
            eprintln!(
                "Inno Setup (ISCC.exe) not found in PATH. Please install Inno Setup to build the installer."
            );
        }
    }
}

fn main() {
    embed_resource::compile("app.rc", std::iter::empty::<&str>());

    #[cfg(windows)]
    {
        if env::var("PROFILE").unwrap_or_default() == "release" {
            println!("Generating installer.iss...");
            generate_inno_installer();
        }
    }
}
