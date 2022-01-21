#![deny(missing_docs)]

//!
//! ElectrumD
//!
//! Utility to run an headless Electrum wallet process, useful in integration testing environment
//!
//! ```no_run
//! use jsonrpc::serde_json;
//! let electrumd = electrumd::ElectrumD::new("/usr/local/bin/electrum.AppImage").unwrap();
//! println!("{}", electrumd.call("version", &serde_json::json!([])).unwrap().as_str().unwrap());
//! ```

mod versions;

use jsonrpc::serde_json::{self, json, value::to_raw_value, Value};
use jsonrpc::{arg, Client};
use log::debug;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;
use std::{env, fmt, thread};
use std::{ffi::OsStr, fs};
use tempfile::TempDir;

pub extern crate jsonrpc;
pub extern crate tempfile;

/// Struct representing the electrum process with related information
pub struct ElectrumD {
    /// Process child handle, used to terminate the process when this struct is dropped
    process: Child,
    /// Rpc client linked to this electrum process
    pub client: Client,
    /// Work directory, where the node store blocks and other stuff. It is kept in the struct so that
    /// directory is deleted only when this struct is dropped
    _work_dir: TempDir,

    /// Contains information to connect to this node
    pub params: ConnectParams,
}

#[derive(Debug, Clone)]
/// Contains all the information to connect to this node
pub struct ConnectParams {
    /// Path to the node datadir
    pub datadir: PathBuf,
    /// Url of the rpc of the wallet rpc
    pub rpc_socket: SocketAddrV4,
}

/// All the possible error in this crate
pub enum Error {
    /// Wrapper of io Error
    Io(std::io::Error),
    /// Wrapper of jsonrpc Error
    Rpc(jsonrpc::Error),
    /// Wrapper for jsonrpc simple_http errors
    RpcSimpleHttp(jsonrpc::simple_http::Error),
    /// Wrapper for serde json errors
    Json(serde_json::Error),
    /// Returned when calling methods requiring a feature to be activated, but it's not
    NoFeature,
    /// Returned when calling methods requiring a env var to exist, but it's not
    NoEnvVar,
    /// Returned when calling methods requiring either a feature or env var, but both are absent
    NeitherFeatureNorEnvVar,
    /// Returned when calling methods requiring either a feature or anv var, but both are present
    BothFeatureAndEnvVar,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{:?}", e),
            Error::Rpc(e) => write!(f, "{:?}", e),
            Error::RpcSimpleHttp(e) => write!(f, "{:?}", e),
            Error::Json(e) => write!(f, "{:?}", e),
            Error::NoFeature => write!(f, "Called a method requiring a feature to be set, but it's not"),
            Error::NoEnvVar => write!(f, "Called a method requiring env var `ELECTRUMD_EXE` to be set, but it's not"),
            Error::NeitherFeatureNorEnvVar =>  write!(f, "Called a method requiring env var `ELECTRUMD_EXE` or a feature to be set, but neither are set"),
            Error::BothFeatureAndEnvVar => write!(f, "Called a method requiring env var `ELECTRUMD_EXE` or a feature to be set, but both are set"),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

const LOCAL_IP: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

/// The node configuration parameters, implements a convenient [Default] for most common use.
///
/// `#[non_exhaustive]` allows adding new parameters without breaking downstream users.
/// Users cannot instantiate the struct directly, they need to create it via the `default()` method
/// and mutate fields according to their preference.
///
/// Default values:
/// ```
/// let mut conf = electrumd::Conf::default();
/// conf.view_stdout = false;
/// conf.network = "regtest";
/// conf.tmpdir = None;
/// assert_eq!(conf, electrumd::Conf::default());
/// ```
///
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub struct Conf<'a> {
    /// Electrum command line arguments containing no spaces like `vec!["--oneserver"]`
    pub args: Vec<&'a str>,

    /// if `true` electrum log output will not be suppressed
    pub view_stdout: bool,

    /// Must match what specified in args without dashes, needed to locate the cookie file
    /// directory with different/esoteric networks
    pub network: &'a str,

    /// Optionally specify the root of where the temporary directories will be created.
    /// If none and the env var `TEMPDIR_ROOT` is set, the env var is used.
    /// If none and the env var `TEMPDIR_ROOT` is not set, the default temp dir of the OS is used.
    /// It may be useful for example to set to a ramdisk so that electrum wallets spawn very fast
    /// because their datadirs are in RAM
    pub tmpdir: Option<PathBuf>,
}

impl Default for Conf<'_> {
    fn default() -> Self {
        Conf {
            args: vec![],
            view_stdout: false,
            network: "regtest",
            tmpdir: None,
        }
    }
}

impl ElectrumD {
    /// Launch the electrum process from the given `exe` executable with default args.
    ///
    /// Waits for the node to be ready to accept connections before returning
    pub fn new<S: AsRef<OsStr>>(exe: S) -> Result<ElectrumD, Error> {
        ElectrumD::with_conf(exe, &Conf::default())
    }

    /// Launch the electrum process from the given `exe` executable with given [Conf] param
    pub fn with_conf<S: AsRef<OsStr>>(exe: S, conf: &Conf) -> Result<ElectrumD, Error> {
        let work_dir = match &conf.tmpdir {
            Some(path) => TempDir::new_in(path),
            None => match env::var("TEMPDIR_ROOT") {
                Ok(env_path) => TempDir::new_in(env_path),
                Err(_) => TempDir::new(),
            },
        }?;
        debug!("work_dir: {:?}", work_dir);

        let rpc_port = get_available_port()?;
        let rpc_pass = rand_string();

        let datadir = work_dir.path().to_path_buf();
        let network_subdir = datadir.join(conf.network);
        let wallet_path = network_subdir
            .clone()
            .join("wallets")
            .join("default_wallet");
        let config_path = network_subdir.clone().join("config");

        fs::create_dir_all(&network_subdir)?;
        fs::create_dir_all(wallet_path.parent().unwrap())?;
        fs::write(
            config_path,
            json!({
                "rpcport": rpc_port,
                "rpcuser": "electrumd",
                "rpcpassword": rpc_pass,
                "log_to_file": true,
            })
            .to_string(),
        )?;

        let stdout = if conf.view_stdout {
            Stdio::inherit()
        } else {
            Stdio::null()
        };

        debug!("launching {:?} in {:?}", exe.as_ref(), datadir);
        let process = Command::new(exe)
            .args(&["daemon", "--dir", datadir.to_str().unwrap()])
            .args(&[format!("--{}", conf.network)])
            .args(&conf.args)
            .stdout(stdout)
            .spawn()?;

        debug!("launched process");

        // Init client
        let rpc_url = format!("http://{}:{}/", LOCAL_IP, rpc_port);
        let client = Client::simple_http(&rpc_url, Some("electrumd".into()), Some(rpc_pass))?;
        let noargs = jsonrpc::empty_args();

        // Wait for the RPC server to respond
        while client.call::<Value>("version", &noargs).is_err() {
            thread::sleep(Duration::from_millis(250));
            assert_eq!(process.stderr, None);
        }

        // Create and load the default wallet
        let _wallet: Value = client.call("create", &noargs)?;
        let _loaded: Value =
            client.call("load_wallet", &arg(&json!({ "wallet_path": wallet_path })))?;

        Ok(ElectrumD {
            process,
            client,
            _work_dir: work_dir,
            params: ConnectParams {
                datadir,
                rpc_socket: SocketAddrV4::new(LOCAL_IP, rpc_port),
            },
        })
    }

    /// Call the RPC method with the given args
    pub fn call(&self, method: &str, args: &Value) -> Result<Value, Error> {
        let args = to_raw_value(args)?;
        Ok(self.client.call(method, &args)?)
    }

    /// Returns the rpc URL including the schema eg. http://127.0.0.1:44842
    pub fn rpc_url(&self) -> String {
        format!("http://{}", self.params.rpc_socket)
    }

    /// Stop the process, waiting for its termination
    pub fn stop(&mut self) -> Result<ExitStatus, Error> {
        self.call("stop", &json!([]))?;
        Ok(self.process.wait()?)
    }
}

impl Drop for ElectrumD {
    // Kill the process immediately (SIGKILL like)
    fn drop(&mut self) {
        let _ = self.call("stop", &json!([]));
        let _ = self.process.kill();
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<jsonrpc::Error> for Error {
    fn from(e: jsonrpc::Error) -> Self {
        Error::Rpc(e)
    }
}

impl From<jsonrpc::simple_http::Error> for Error {
    fn from(e: jsonrpc::simple_http::Error) -> Self {
        Error::RpcSimpleHttp(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

/// Returns a non-used local port if available.
///
/// Note there is a race condition during the time the method check availability and the caller
pub fn get_available_port() -> Result<u16, Error> {
    // using 0 as port let the system assign a port available
    let t = TcpListener::bind(("127.0.0.1", 0))?; // 0 means the OS choose a free port
    Ok(t.local_addr().map(|s| s.port())?)
}

fn rand_string() -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect()
}

/// Provide the electrum executable path if a version feature has been specified
pub fn downloaded_exe_path() -> Result<String, Error> {
    if versions::HAS_FEATURE {
        Ok(format!(
            "{}/electrum/electrum-{}/electrum.AppImage",
            env!("OUT_DIR"),
            versions::VERSION
        ))
    } else {
        Err(Error::NoFeature)
    }
}

/// Returns the daemon executable path if it's provided as a feature or as `ELECTRUMD_EXE` env var.
/// Returns error if none or both are set
pub fn exe_path() -> Result<String, Error> {
    match (downloaded_exe_path(), std::env::var("ELECTRUMD_EXE")) {
        (Ok(_), Ok(_)) => Err(Error::BothFeatureAndEnvVar),
        (Ok(path), Err(_)) => Ok(path),
        (Err(_), Ok(path)) => Ok(path),
        (Err(_), Err(_)) => Err(Error::NeitherFeatureNorEnvVar),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_electrumd() {
        let exe = init();
        println!("{}", exe);

        let electrumd = ElectrumD::new(exe).unwrap();
        let version = electrumd.call("version", &serde_json::json!([])).unwrap();
        assert_eq!(version.as_str(), Some(versions::VERSION));
    }

    fn init() -> String {
        let _ = env_logger::try_init();
        exe_path().unwrap()
    }
}
