use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub display_name: String,
    pub bluetooth_container_id: String,
    pub render_endpoint_id: String,
    pub stereo_device_name: String,
}

impl Config {
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|e| format!("設定ファイルを読めません: {e}"))?;
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        let config: Self = serde_json::from_slice(bytes)
            .map_err(|e| format!("設定ファイルの形式が正しくありません: {e}"))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("未対応の設定ファイル版です。".into());
        }
        if !valid_guid(&self.bluetooth_container_id) {
            return Err("Bluetooth Container IDが不正です。".into());
        }
        if !valid_endpoint_id(&self.render_endpoint_id) {
            return Err("Stereo再生エンドポイントIDが不正です。".into());
        }
        if self.display_name.trim().is_empty() || self.display_name.contains(['\r', '\n']) {
            return Err("機器名が不正です。".into());
        }
        if self.stereo_device_name.trim().is_empty()
            || self.stereo_device_name.contains(['\r', '\n'])
        {
            return Err("Stereoデバイス名が不正です。".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub connected: bool,
    pub container_id: String,
    pub name: String,
}

pub fn parse_bluetooth_list(output: &str) -> Result<Vec<BluetoothDevice>, String> {
    let mut devices = Vec::new();
    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let mut parts = line.splitn(3, ' ');
        let status = parts.next().unwrap_or_default();
        let id = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        if !matches!(status, "0" | "1") || !valid_guid(id) || name.is_empty() {
            return Err(format!("Bluetoothツールの機器一覧形式が不正です: {line}"));
        }
        devices.push(BluetoothDevice {
            connected: status == "1",
            container_id: id.to_owned(),
            name: name.to_owned(),
        });
    }
    Ok(devices)
}

pub fn verify_registered_device(devices: &[BluetoothDevice], id: &str) -> Result<(), String> {
    let count = devices
        .iter()
        .filter(|device| device.container_id.eq_ignore_ascii_case(id))
        .count();
    if count != 1 {
        return Err("設定されたBluetooth機器を一意に確認できません。".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioState {
    pub available: bool,
    pub console_default: bool,
    pub multimedia_default: bool,
}

pub fn parse_audio_state(
    json: &[u8],
    endpoint_id: &str,
    stereo_name: &str,
) -> Result<AudioState, String> {
    let bytes = json.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(json);
    let items: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("音声機器一覧のJSONが不正です: {e}"))?;
    let items = items
        .as_array()
        .ok_or("音声機器一覧が配列ではありません。")?;
    let mut match_item = None;
    for item in items {
        let get = |key| item.get(key).and_then(serde_json::Value::as_str);
        if get("Type") == Some("Device")
            && get("Direction") == Some("Render")
            && get("Item ID").is_some_and(|id| id.eq_ignore_ascii_case(endpoint_id))
        {
            if match_item.replace(item).is_some() {
                return Err("同一の再生エンドポイントIDが複数あります。".into());
            }
        }
    }
    let Some(item) = match_item else {
        return Ok(AudioState::default());
    };
    let required = |key| {
        item.get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("音声機器一覧に{key}がありません。"))
    };
    if required("Device Name")? != stereo_name {
        return Err("Stereoデバイス名が設定と異なります。".into());
    }
    Ok(AudioState {
        available: required("Device State")? == "Active",
        console_default: required("Default")? == "Render",
        multimedia_default: required("Default Multimedia")? == "Render",
    })
}

fn find_stereo_endpoint(
    json: &[u8],
    stereo_name: &str,
    active_only: bool,
) -> Result<Option<String>, String> {
    let bytes = json.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(json);
    let items: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("音声機器一覧のJSONが不正です: {e}"))?;
    let items = items
        .as_array()
        .ok_or("音声機器一覧が配列ではありません。")?;
    let mut found = None;
    for item in items {
        let field = |name| item.get(name).and_then(serde_json::Value::as_str);
        if field("Type") == Some("Device")
            && field("Direction") == Some("Render")
            && field("Device Name") == Some(stereo_name)
            && (!active_only || field("Device State") == Some("Active"))
        {
            let id = field("Item ID").ok_or("Stereo出力のIDがありません。")?;
            if !valid_endpoint_id(id) {
                return Err("Stereo出力のIDが不正です。".into());
            }
            if found.replace(id.to_owned()).is_some() {
                return Err("Stereo出力を一意に確認できません。".into());
            }
        }
    }
    Ok(found)
}

pub fn find_endpoint(json: &[u8], stereo_name: &str) -> Result<Option<String>, String> {
    find_stereo_endpoint(json, stereo_name, false)
}

pub fn find_active_endpoint(json: &[u8], stereo_name: &str) -> Result<Option<String>, String> {
    find_stereo_endpoint(json, stereo_name, true)
}

fn valid_guid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 38 || bytes[0] != b'{' || bytes[37] != b'}' {
        return false;
    }
    bytes[1..37].iter().enumerate().all(|(i, byte)| {
        if matches!(i, 8 | 13 | 18 | 23) {
            *byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    })
}

fn valid_endpoint_id(value: &str) -> bool {
    const PREFIX: &str = "{0.0.0.00000000}.";
    value
        .get(..PREFIX.len())
        .is_some_and(|part| part.eq_ignore_ascii_case(PREFIX))
        && value.get(PREFIX.len()..).is_some_and(valid_guid)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUID: &str = "{12345678-1234-4567-89AB-0123456789AB}";
    const ENDPOINT: &str = "{0.0.0.00000000}.{A77A09B2-1EC6-49C3-860A-68945904A2F1}";

    #[test]
    fn strict_config_rejects_unknown_fields_and_invalid_ids() {
        let data = format!(
            r#"{{"schemaVersion":1,"displayName":"AirPods Pro","bluetoothContainerId":"{GUID}","renderEndpointId":"{ENDPOINT}","stereoDeviceName":"Headphones","extra":true}}"#
        );
        assert!(serde_json::from_str::<Config>(&data).is_err());
        assert!(!valid_guid("{not-a-guid}"));
        assert!(!valid_endpoint_id(GUID));
    }

    #[test]
    fn audio_state_requires_exact_endpoint_and_name() {
        let data = format!(
            r#"[{{"Type":"Device","Direction":"Render","Item ID":"{ENDPOINT}","Device Name":"Headphones","Device State":"Active","Default":"Render","Default Multimedia":"Render"}}]"#
        );
        let state = parse_audio_state(data.as_bytes(), ENDPOINT, "Headphones").unwrap();
        assert_eq!(
            state,
            AudioState {
                available: true,
                console_default: true,
                multimedia_default: true
            }
        );
        assert!(parse_audio_state(data.as_bytes(), ENDPOINT, "Hands-Free").is_err());
        assert_eq!(
            parse_audio_state(data.as_bytes(), GUID, "Headphones").unwrap(),
            AudioState::default()
        );
    }

    #[test]
    fn bluetooth_registration_uses_container_id() {
        let list = parse_bluetooth_list(&format!("0 {GUID} AirPods Pro\r\n")).unwrap();
        verify_registered_device(&list, GUID).unwrap();
        assert!(verify_registered_device(&[list[0].clone(), list[0].clone()], GUID).is_err());
    }

    #[test]
    fn registration_accepts_an_inactive_but_unique_stereo_endpoint() {
        let item = format!(
            r#"{{"Type":"Device","Direction":"Render","Device Name":"AirPods Stereo","Device State":"Not Present","Item ID":"{ENDPOINT}"}}"#
        );
        assert_eq!(
            find_endpoint(format!("[{item}]").as_bytes(), "AirPods Stereo").unwrap(),
            Some(ENDPOINT.to_owned())
        );
        assert_eq!(
            find_active_endpoint(format!("[{item}]").as_bytes(), "AirPods Stereo").unwrap(),
            None
        );
        assert!(find_endpoint(format!("[{item},{item}]").as_bytes(), "AirPods Stereo").is_err());
        assert_eq!(
            find_endpoint(format!("[{item}]").as_bytes(), "Another Device").unwrap(),
            None
        );
    }
}
