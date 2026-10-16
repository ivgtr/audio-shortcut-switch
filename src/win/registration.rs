use super::{ToolPaths, audio_command, bluetooth_command, bluetooth_connected, wide};
use crate::core::{self, Config};
use std::ffi::{OsStr, OsString, c_void};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};
use winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER;
use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, CoUninitialize};
use winapi::um::knownfolders::FOLDERID_Desktop;
use winapi::um::objidl::IPersistFile;
use winapi::um::shlobj::SHGetKnownFolderPath;
use winapi::um::shobjidl_core::{IShellLinkW, ShellLink};
use winapi::{Class, Interface};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_PRESENT, GUID_DEVCLASS_MEDIA, SP_DEVINFO_DATA, SPDRP_FRIENDLYNAME,
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
    SetupDiGetDeviceInstanceIdW, SetupDiGetDevicePropertyW, SetupDiGetDeviceRegistryPropertyW,
};
use windows_sys::Win32::Devices::Properties::DEVPKEY_Device_ContainerId;
use windows_sys::Win32::Devices::Properties::DEVPROP_TYPE_GUID;
use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Registry::REG_SZ;

const A2DP_PREFIX: &str = "BTHENUM\\{0000110B-0000-1000-8000-00805F9B34FB}";

fn device_guid(value: &windows_sys::core::GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        value.data1,
        value.data2,
        value.data3,
        value.data4[0],
        value.data4[1],
        value.data4[2],
        value.data4[3],
        value.data4[4],
        value.data4[5],
        value.data4[6],
        value.data4[7]
    )
}

struct DeviceInfoSet(isize);

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}

pub(super) fn stereo_driver_name(container_id: &str) -> Result<String, String> {
    let handle = unsafe {
        SetupDiGetClassDevsW(
            &GUID_DEVCLASS_MEDIA,
            ptr::null(),
            ptr::null_mut(),
            DIGCF_PRESENT,
        )
    };
    if handle == INVALID_HANDLE_VALUE as isize {
        return Err(format!(
            "音声ドライバー一覧を取得できません: {}",
            unsafe { GetLastError() }
        ));
    }
    let set = DeviceInfoSet(handle);
    let mut names = Vec::new();
    let mut index = 0;
    loop {
        let mut info = SP_DEVINFO_DATA::default();
        info.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;
        if unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut info) } == 0 {
            let error = unsafe { GetLastError() };
            if error == ERROR_NO_MORE_ITEMS {
                break;
            }
            return Err(format!("音声ドライバーの列挙に失敗しました: {error}"));
        }
        index += 1;
        let mut instance = [0u16; 4096];
        let mut needed = 0;
        if unsafe {
            SetupDiGetDeviceInstanceIdW(
                set.0,
                &info,
                instance.as_mut_ptr(),
                instance.len() as u32,
                &mut needed,
            )
        } == 0
        {
            return Err(format!("音声ドライバーIDを読めません: {}", unsafe {
                GetLastError()
            }));
        }
        let instance_id = String::from_utf16_lossy(&instance[..needed.saturating_sub(1) as usize]);
        if !instance_id.to_ascii_uppercase().starts_with(A2DP_PREFIX) {
            continue;
        }

        let mut guid = windows_sys::core::GUID::default();
        let mut kind = 0;
        let mut property_size = 0;
        if unsafe {
            SetupDiGetDevicePropertyW(
                set.0,
                &info,
                &DEVPKEY_Device_ContainerId,
                &mut kind,
                (&mut guid as *mut _) as *mut u8,
                size_of::<windows_sys::core::GUID>() as u32,
                &mut property_size,
                0,
            )
        } == 0
            || kind != DEVPROP_TYPE_GUID
            || property_size as usize != size_of::<windows_sys::core::GUID>()
        {
            return Err(format!(
                "A2DPドライバーのContainer IDを読めません: {}",
                unsafe { GetLastError() }
            ));
        }
        if !device_guid(&guid).eq_ignore_ascii_case(container_id) {
            continue;
        }

        let mut name = [0u16; 2048];
        let mut registry_type = 0;
        let mut name_size = 0;
        if unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                set.0,
                &info,
                SPDRP_FRIENDLYNAME,
                &mut registry_type,
                name.as_mut_ptr() as *mut u8,
                (name.len() * 2) as u32,
                &mut name_size,
            )
        } == 0
            || registry_type != REG_SZ
            || name_size < 2
            || name_size % 2 != 0
        {
            return Err(format!("A2DPドライバー名を読めません: {}", unsafe {
                GetLastError()
            }));
        }
        let text = String::from_utf16(&name[..name_size as usize / 2 - 1])
            .map_err(|_| "A2DPドライバー名が不正です。")?;
        if text.is_empty() {
            return Err("A2DPドライバー名が空です。".into());
        }
        names.push(text);
    }
    if names.len() != 1 {
        return Err("このBluetooth機器のA2DP Stereoドライバーを一意に確認できません。".into());
    }
    Ok(names.remove(0))
}

fn active_endpoint(tools: &ToolPaths, stereo_name: &str) -> Result<Option<String>, String> {
    let path = tools
        .base
        .join("logs")
        .join(format!("audio-registration-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    let result = (|| {
        audio_command(
            tools,
            &[
                OsString::from("/SaveFileEncoding"),
                OsString::from("3"),
                OsString::from("/sjson"),
                path.as_os_str().to_owned(),
            ],
        )?;
        let bytes = fs::read(&path).map_err(|e| format!("音声機器一覧を読めません: {e}"))?;
        core::find_active_endpoint(&bytes, stereo_name)
    })();
    let _ = fs::remove_file(&path);
    result
}

fn desktop_path() -> Result<PathBuf, String> {
    let mut raw = ptr::null_mut();
    let result = unsafe { SHGetKnownFolderPath(&FOLDERID_Desktop, 0, ptr::null_mut(), &mut raw) };
    if result < 0 {
        return Err(format!("デスクトップの場所を取得できません: {result:#x}"));
    }
    if raw.is_null() {
        return Err("デスクトップの場所が空です。".into());
    }
    let length = (0..32768).find(|&i| unsafe { *raw.add(i) } == 0);
    let path = length
        .map(|length| OsString::from_wide(unsafe { std::slice::from_raw_parts(raw, length) }));
    unsafe { windows_sys::Win32::System::Com::CoTaskMemFree(raw as *const c_void) };
    path.map(PathBuf::from)
        .ok_or("デスクトップのパスが長すぎます。".into())
}

fn check_hresult(result: i32, operation: &str) -> Result<(), String> {
    if result < 0 {
        Err(format!(
            "ショートカットの{operation}に失敗しました: {result:#x}"
        ))
    } else {
        Ok(())
    }
}

fn create_shortcut(
    link_path: &Path,
    exe_path: &Path,
    config_path: &Path,
    base: &Path,
) -> Result<(), String> {
    let init = unsafe {
        CoInitializeEx(
            ptr::null_mut(),
            winapi::um::objbase::COINIT_APARTMENTTHREADED,
        )
    };
    check_hresult(init, "初期化")?;
    let result = (|| {
        let mut link: *mut IShellLinkW = ptr::null_mut();
        let created = unsafe {
            CoCreateInstance(
                &ShellLink::uuidof(),
                ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &IShellLinkW::uuidof(),
                (&mut link as *mut *mut IShellLinkW).cast(),
            )
        };
        check_hresult(created, "作成")?;
        let result = (|| {
            let arguments = format!("--connect \"{}\"", config_path.display());
            unsafe {
                check_hresult(
                    (*link).SetPath(wide(exe_path.as_os_str()).as_ptr()),
                    "実行ファイルの設定",
                )?;
                check_hresult(
                    (*link).SetArguments(wide(OsStr::new(&arguments)).as_ptr()),
                    "引数の設定",
                )?;
                check_hresult(
                    (*link).SetWorkingDirectory(wide(base.as_os_str()).as_ptr()),
                    "作業フォルダーの設定",
                )?;
                check_hresult(
                    (*link).SetDescription(
                        wide(OsStr::new(
                            "登録済みBluetooth機器へ接続し、Stereo再生出力へ切り替える",
                        ))
                        .as_ptr(),
                    ),
                    "説明の設定",
                )?;
            }
            let mut persist: *mut IPersistFile = ptr::null_mut();
            let queried = unsafe {
                (*link).QueryInterface(
                    &IPersistFile::uuidof(),
                    (&mut persist as *mut *mut IPersistFile).cast(),
                )
            };
            check_hresult(queried, "保存準備")?;
            let saved = unsafe { (*persist).Save(wide(link_path.as_os_str()).as_ptr(), 1) };
            unsafe { (*persist).Release() };
            check_hresult(saved, "保存")
        })();
        unsafe { (*link).Release() };
        result
    })();
    unsafe { CoUninitialize() };
    result
}

pub(super) fn register_device(id: &str, shortcut_name: &str) -> Result<(), String> {
    let tools = ToolPaths::locate()?;
    tools.require_tools()?;
    let devices = super::list_bluetooth()?;
    core::verify_registered_device(&devices, id)?;
    let device_name = devices
        .into_iter()
        .find(|device| device.container_id.eq_ignore_ascii_case(id))
        .unwrap()
        .name;
    if shortcut_name.is_empty()
        || shortcut_name.chars().count() > 100
        || shortcut_name.ends_with([' ', '.'])
        || shortcut_name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
    {
        return Err("ショートカット名に使えない文字が含まれています。".into());
    }
    let desktop = desktop_path()?;
    let link_path = desktop.join(format!("{shortcut_name}.lnk"));
    if link_path.exists() {
        return Err("同じ名前のショートカットが既にあります。".into());
    }
    let configs = tools.base.join("configs");
    fs::create_dir_all(&configs).map_err(|e| format!("設定フォルダーを作れません: {e}"))?;
    let config_path = configs.join(format!(
        "{}.json",
        id.trim_matches(['{', '}']).to_ascii_lowercase()
    ));
    let mut created = false;
    if config_path.exists() {
        let config = Config::read(&config_path)?;
        if !config.bluetooth_container_id.eq_ignore_ascii_case(id)
            || config.display_name != device_name
        {
            return Err("既存設定とBluetooth機器が一致しません。".into());
        }
    } else {
        let stereo_name = stereo_driver_name(id)?;
        if !bluetooth_connected(&tools, id)? {
            bluetooth_command(
                &tools,
                &["connect-by-container-id", id],
                Duration::from_secs(15),
            )?;
        }
        let deadline = Instant::now() + Duration::from_secs(35);
        let endpoint_id = loop {
            if bluetooth_connected(&tools, id)? {
                if let Some(id) = active_endpoint(&tools, &stereo_name)? {
                    break id;
                }
            }
            if Instant::now() >= deadline {
                return Err("35秒以内にStereo出力を確認できませんでした。".into());
            }
            thread::sleep(Duration::from_millis(700));
        };
        let config = Config {
            schema_version: 1,
            display_name: device_name,
            bluetooth_container_id: id.to_owned(),
            render_endpoint_id: endpoint_id,
            stereo_device_name: stereo_name,
        };
        config.validate()?;
        let json =
            serde_json::to_vec_pretty(&config).map_err(|e| format!("設定を作れません: {e}"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&config_path)
            .map_err(|e| format!("設定ファイルを作れません: {e}"))?;
        if let Err(error) = file.write_all(&json) {
            drop(file);
            let _ = fs::remove_file(&config_path);
            return Err(format!("設定ファイルを書けません: {error}"));
        }
        created = true;
    }
    let exe_path = tools.base.join("EarbudsPC.exe");
    let result = create_shortcut(&link_path, &exe_path, &config_path, &tools.base);
    if result.is_err() && created {
        let _ = fs::remove_file(&config_path);
    }
    result
}
