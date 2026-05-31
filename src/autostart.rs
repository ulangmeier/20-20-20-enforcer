use std::env;

pub fn install() {
    #[cfg(target_os = "windows")]
    install_windows();

    #[cfg(target_os = "linux")]
    install_linux();

    #[cfg(target_os = "macos")]
    install_macos();
}

#[cfg(target_os = "windows")]
fn install_windows() {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = hkcu.create_subkey(path).unwrap();

    let exe_path = env::current_exe().unwrap();
    let exe_str = exe_path.to_str().unwrap();

    if let Ok(existing) = key.get_value::<String, _>("TwentyTwentyTwenty") {
        if existing == exe_str {
            return;
        }
    }

    key.set_value("TwentyTwentyTwenty", &exe_str).unwrap();
}

#[cfg(target_os = "linux")]
fn install_linux() {
    use std::fs;

    let home = env::var("HOME").expect("HOME not set");
    let autostart_dir = PathBuf::from(home).join(".config/autostart");
    fs::create_dir_all(&autostart_dir).unwrap();

    let exe = env::current_exe().unwrap();
    let desktop = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=TwentyTwentyTwenty\n\
         Exec={}\n\
         Hidden=false\n\
         NoDisplay=false\n\
         X-GNOME-Autostart-enabled=true\n",
        exe.display()
    );

    fs::write(
        autostart_dir.join("twenty-twenty-twenty.desktop"),
        desktop,
    )
    .unwrap();
}

#[cfg(target_os = "macos")]
fn install_macos() {
    use std::fs;

    let home = env::var("HOME").expect("HOME not set");
    let launch_agents = PathBuf::from(home).join("Library/LaunchAgents");
    fs::create_dir_all(&launch_agents).unwrap();

    let exe = env::current_exe().unwrap();
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.twentytwentytwenty.app</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
        exe.display()
    );

    fs::write(
        launch_agents.join("com.twentytwentytwenty.app.plist"),
        plist,
    )
    .unwrap();
}
