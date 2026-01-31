use crate::models::AppInfo;
use std::path::PathBuf;
use std::fs;

pub struct AppScanner;

impl AppScanner {
    pub async fn scan_installed_apps() -> Vec<AppInfo> {
        let mut apps = Vec::new();

        #[cfg(target_os = "windows")]
        {
            let start_menu = PathBuf::from("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs");
            Self::scan_dir_for_lnk(&start_menu, &mut apps);

            if let Ok(user_profile) = std::env::var("USERPROFILE") {
                let user_start = PathBuf::from(user_profile).join("AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs");
                Self::scan_dir_for_lnk(&user_start, &mut apps);
            }
        }

        #[cfg(target_os = "linux")]
        {
            let paths = vec![
                PathBuf::from("/usr/share/applications"),
                PathBuf::from("/usr/local/share/applications"),
                dirs::home_dir().map(|h| h.join(".local/share/applications")).unwrap_or_default(),
            ];

            for path in paths {
                Self::scan_linux_desktop_entries(&path, &mut apps);
            }
        }

        apps.sort_by(|a, b| a.name.cmp(&b.name));
        apps.dedup_by(|a, b| a.path == b.path);
        
        apps
    }

    #[cfg(target_os = "windows")]
    fn scan_dir_for_lnk(dir: &std::path::Path, apps: &mut Vec<AppInfo>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::scan_dir_for_lnk(&path, apps);
                } else if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
                    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string();
                    apps.push(AppInfo {
                        name,
                        path: path.to_string_lossy().to_string(),
                        icon: None,
                    });
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn scan_linux_desktop_entries(dir: &std::path::Path, apps: &mut Vec<AppInfo>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut name = String::new();
                        let mut exec = String::new();
                        for line in content.lines() {
                            if line.starts_with("Name=") && name.is_empty() {
                                name = line[5..].to_string();
                            } else if line.starts_with("Exec=") && exec.is_empty() {
                                exec = line[5..].to_string();
                            }
                        }
                        if !name.is_empty() && !exec.is_empty() {
                            apps.push(AppInfo { name, path: exec, icon: None });
                        }
                    }
                }
            }
        }
    }
}
