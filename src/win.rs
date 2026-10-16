use crate::core::{self, AudioState, BluetoothDevice, Config};
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

mod registration;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone)]
struct ToolPaths {
    base: PathBuf,
    bluetooth: PathBuf,
    audio: PathBuf,
}

impl ToolPaths {
    fn locate() -> Result<Self, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("実行ファイルの場所を取得できません: {e}"))?;
        let base = exe
            .parent()
            .ok_or("実行ファイルのフォルダーを取得できません。")?
            .to_path_buf();
        Ok(Self {
            bluetooth: base.join("tools/ToothTrayCLI.exe"),
            audio: base.join("tools/SoundVolumeView/SoundVolumeView.exe"),
            base,
        })
    }

    fn require_tools(&self) -> Result<(), String> {
        if !self.bluetooth.is_file() || !self.audio.is_file() {
            return Err(
                "補助ツールが見つかりません。配布物をインストールし直してください。".into(),
            );
        }
        Ok(())
    }

    fn log_path(&self) -> PathBuf {
        self.base.join("logs/switch.log")
    }
}

struct Logger {
    path: PathBuf,
}

impl Logger {
    fn new(path: PathBuf) -> Result<Self, String> {
        let parent = path.parent().ok_or("ログの保存先が不正です。")?;
        fs::create_dir_all(parent).map_err(|e| format!("ログのフォルダーを作れません: {e}"))?;
        Ok(Self { path })
    }

    fn write(&self, message: &str) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |time| time.as_secs());
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "{stamp} {message}");
        }
    }
}

struct SwitchMutex(HANDLE);

impl SwitchMutex {
    fn acquire() -> Result<Option<Self>, String> {
        let name = wide(OsStr::new("Local\\EarbudsPC.Switch"));
        // SAFETY: The pointer is a valid zero-terminated UTF-16 string for this call.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "多重起動を防ぐMutexを作れません: {}",
                unsafe { GetLastError() }
            ));
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe { CloseHandle(handle) };
            return Ok(None);
        }
        Ok(Some(Self(handle)))
    }
}

impl Drop for SwitchMutex {
    fn drop(&mut self) {
        // SAFETY: self.0 is a live handle returned by CreateMutexW.
        unsafe { CloseHandle(self.0) };
    }
}

struct Snapshot(PathBuf);

impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn error_dialog(message: &str) {
    let title = wide(OsStr::new("イヤホンをPCで使う"));
    let text = wide(OsStr::new(message));
    // SAFETY: The two strings are zero-terminated and valid for this synchronous call.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn run_command(mut command: Command, timeout: Duration) -> Result<String, String> {
    command
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("補助処理を起動できません: {e}"))?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("補助処理が制限時間内に終了しませんでした。".into());
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("補助処理の状態を確認できません: {e}"));
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("補助処理の結果を取得できません: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "補助処理が失敗しました ({:?}): {} {}",
            output.status.code(),
            stderr.trim(),
            stdout
        ));
    }
    Ok(stdout)
}

fn bluetooth_command(
    tools: &ToolPaths,
    arguments: &[&str],
    timeout: Duration,
) -> Result<String, String> {
    let mut command = Command::new(&tools.bluetooth);
    command.args(arguments);
    run_command(command, timeout)
}

fn bluetooth_connected(tools: &ToolPaths, id: &str) -> Result<bool, String> {
    match bluetooth_command(
        tools,
        &["is-connected-by-container-id", id],
        Duration::from_secs(8),
    )?
    .as_str()
    {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err("Bluetooth接続状態の応答が不正です。".into()),
    }
}

fn audio_command(tools: &ToolPaths, arguments: &[OsString]) -> Result<(), String> {
    let mut command = Command::new(&tools.audio);
    command.args(arguments);
    run_command(command, Duration::from_secs(8)).map(|_| ())
}

fn audio_state(tools: &ToolPaths, config: &Config) -> Result<AudioState, String> {
    let path = tools
        .base
        .join("logs")
        .join(format!("audio-{}.json", std::process::id()));
    let snapshot = Snapshot(path);
    let _ = fs::remove_file(&snapshot.0);
    audio_command(
        tools,
        &[
            OsString::from("/SaveFileEncoding"),
            OsString::from("3"),
            OsString::from("/sjson"),
            snapshot.0.as_os_str().to_owned(),
        ],
    )?;
    let bytes =
        fs::read(&snapshot.0).map_err(|e| format!("音声機器の状態ファイルを読めません: {e}"))?;
    core::parse_audio_state(
        &bytes,
        &config.render_endpoint_id,
        &config.stereo_device_name,
    )
}

fn set_default(tools: &ToolPaths, id: &str, role: &str) -> Result<(), String> {
    audio_command(
        tools,
        &[
            OsString::from("/SetDefault"),
            OsString::from(id),
            OsString::from(role),
        ],
    )
}

pub(crate) fn list_bluetooth() -> Result<Vec<BluetoothDevice>, String> {
    let tools = ToolPaths::locate()?;
    tools.require_tools()?;
    let output = bluetooth_command(&tools, &["list"], Duration::from_secs(8))?;
    core::parse_bluetooth_list(&output)
}

pub(crate) fn register_device(id: &str, shortcut_name: &str) -> Result<(), String> {
    let result = registration::register_device(id, shortcut_name);
    if let Ok(tools) = ToolPaths::locate() {
        if let Ok(logger) = Logger::new(tools.log_path()) {
            match &result {
                Ok(()) => logger.write(&format!("登録成功: {id} {shortcut_name}")),
                Err(error) => logger.write(&format!("登録失敗: {id} {shortcut_name}: {error}")),
            }
        }
    }
    result
}

pub(crate) fn check_device(id: &str) -> Result<(), String> {
    let result = registration::check_device(id);
    if let Ok(tools) = ToolPaths::locate() {
        if let Ok(logger) = Logger::new(tools.log_path()) {
            match &result {
                Ok(()) => logger.write(&format!("接続確認成功: {id}")),
                Err(error) => logger.write(&format!("接続確認失敗: {id}: {error}")),
            }
        }
    }
    result
}

fn switch(config_path: &Path, tools: &ToolPaths, log: &Logger) -> Result<(), String> {
    let Some(_lock) = SwitchMutex::acquire()? else {
        log.write("重複起動を無視しました。");
        return Ok(());
    };
    tools.require_tools()?;
    let config = Config::read(config_path)?;
    log.write(&format!(
        "開始: {} ({})",
        config.display_name, config.bluetooth_container_id
    ));
    let devices = list_bluetooth()?;
    core::verify_registered_device(&devices, &config.bluetooth_container_id)?;

    let mut connected = bluetooth_connected(tools, &config.bluetooth_container_id)?;
    let mut audio = audio_state(tools, &config)?;
    log.write(&format!(
        "初期状態: connected={} available={} console={} multimedia={}",
        connected, audio.available, audio.console_default, audio.multimedia_default
    ));
    if !connected {
        log.write("Bluetooth接続を要求します。");
        bluetooth_command(
            tools,
            &["connect-by-container-id", &config.bluetooth_container_id],
            Duration::from_secs(15),
        )?;
    }

    let deadline = Instant::now() + Duration::from_secs(35);
    while !(connected && audio.available) {
        if Instant::now() >= deadline {
            return Err("35秒以内にイヤホンのStereo出力を確認できませんでした。".into());
        }
        thread::sleep(Duration::from_millis(700));
        connected = bluetooth_connected(tools, &config.bluetooth_container_id)?;
        audio = audio_state(tools, &config)?;
    }
    if registration::stereo_driver_name(&config.bluetooth_container_id)?
        != config.stereo_device_name
    {
        return Err("Bluetooth機器とStereo出力の対応が設定と異なります。".into());
    }
    if !audio.console_default {
        log.write("Consoleの既定出力を設定します。");
        set_default(tools, &config.render_endpoint_id, "0")?;
    }
    if !audio.multimedia_default {
        log.write("Multimediaの既定出力を設定します。");
        set_default(tools, &config.render_endpoint_id, "1")?;
    }

    let verify_deadline = Instant::now() + Duration::from_secs(8);
    loop {
        connected = bluetooth_connected(tools, &config.bluetooth_container_id)?;
        audio = audio_state(tools, &config)?;
        if connected && audio.available && audio.console_default && audio.multimedia_default {
            log.write("成功: Bluetooth接続、Stereo出力、Console/Multimedia既定を確認しました。");
            return Ok(());
        }
        if Instant::now() >= verify_deadline {
            return Err("最終状態で接続と既定出力を確認できませんでした。".into());
        }
        thread::sleep(Duration::from_millis(350));
    }
}

pub fn entry() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || (args.len() == 1 && args[0] == "--setup") {
        if let Err(error) = crate::gui::run() {
            error_dialog(&error);
            std::process::exit(1);
        }
        return;
    }
    if args.len() == 4 && args[0] == "--register" && args[3] == "--no-dialog" {
        let id = args[1].to_string_lossy();
        let name = args[2].to_string_lossy();
        let result = register_device(&id, &name);
        if result.is_err() {
            std::process::exit(1);
        }
        return;
    }
    let no_dialog = args.len() == 3 && args[2] == "--no-dialog";
    if (args.len() == 2 || no_dialog) && args[0] == "--connect" {
        let config_path = PathBuf::from(&args[1]);
        let tools = match ToolPaths::locate() {
            Ok(tools) => tools,
            Err(error) => {
                if !no_dialog {
                    error_dialog(&error);
                }
                std::process::exit(1);
            }
        };
        let log_path = tools.log_path();
        let logger = match Logger::new(log_path.clone()) {
            Ok(logger) => logger,
            Err(error) => {
                if !no_dialog {
                    error_dialog(&error);
                }
                std::process::exit(1);
            }
        };
        if let Err(error) = switch(&config_path, &tools, &logger) {
            logger.write(&format!("ERROR: {error}"));
            if !no_dialog {
                error_dialog(&format!(
                    "イヤホンへの切り替えに失敗しました。\n{error}\nログ: {}",
                    log_path.display()
                ));
            }
            std::process::exit(1);
        }
        return;
    }
    error_dialog("起動引数が不正です。設定画面を開くには引数なしで実行してください。");
    std::process::exit(2);
}
