use bitcoin_hashes::{sha256, Hash};
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::str::FromStr;

include!("src/versions.rs");

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn download_filename() -> String {
    format!("electrum-{}-x86_64.AppImage", &VERSION)
}
// other platforms are currently unsupported

fn get_expected_sha256() -> Result<sha256::Hash, ()> {
    let sha256sum_filename = format!("sha256/electrum-{}-SHA256SUM", &VERSION);
    let contents = fs::read_to_string(sha256sum_filename).expect("SHA256SUM file to exists");
    let hash = sha256::Hash::from_str(&contents).expect("SHA256SUM file to be valid");
    Ok(hash)
}

fn main() {
    if !HAS_FEATURE || std::env::var_os("ELECTRUMD_SKIP_DOWNLOAD").is_some() {
        return;
    }
    let download_filename = download_filename();
    let expected_hash = get_expected_sha256().unwrap();
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let download_dir = Path::new(&out_dir)
        .join("electrum")
        .join(format!("electrum-{}", VERSION));
    if !download_dir.exists() {
        fs::create_dir_all(&download_dir).unwrap();
    }
    let filepath = download_dir.join("electrum.AppImage");

    if !filepath.exists() {
        println!(
            "filename:{} version:{} hash:{}",
            download_filename, VERSION, expected_hash
        );

        let url = format!(
            "https://download.electrum.org/{}/{}",
            VERSION, download_filename
        );
        let mut downloaded_bytes = Vec::new();

        let _size = ureq::get(&url)
            .call()
            .into_reader()
            .read_to_end(&mut downloaded_bytes)
            .unwrap();

        let downloaded_hash = sha256::Hash::hash(&downloaded_bytes);
        assert_eq!(expected_hash, downloaded_hash);
        fs::write(&filepath, downloaded_bytes).unwrap();

        // chmod +x
        let mut perms = fs::metadata(&filepath).unwrap().permissions();
        perms.set_mode(0o744);
        fs::set_permissions(&filepath, perms).unwrap();
    }
}
