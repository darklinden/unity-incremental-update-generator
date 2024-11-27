#[cfg(target_os = "windows")]
pub(crate) fn cyg_to_win(path: &str) -> String {
    path.replace("/cygdrive/c", "C:")
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn cyg_to_win(path: &str) -> String {
    path.to_string()
}

#[cfg(target_os = "windows")]
pub(crate) fn win_to_cyg(path: &str) -> String {
    path.replace("C:", "/cygdrive/c").replace("\\", "/")
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn win_to_cyg(path: &str) -> String {
    path.to_string()
}
