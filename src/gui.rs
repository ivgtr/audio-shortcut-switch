use crate::win::{self, ChoiceList, DeviceChoice};
use slint::{
    CloseRequestResponse, ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

slint::include_modules!();

enum WorkerMessage {
    Devices(Result<ChoiceList, String>),
    Registered(Result<(), String>),
    Checked(Result<(), String>),
}

struct UiState {
    window: slint::Weak<AppWindow>,
    devices: RefCell<Vec<DeviceChoice>>,
    busy: Cell<bool>,
    timer: Timer,
    sender: Sender<WorkerMessage>,
    receiver: Receiver<WorkerMessage>,
}

impl UiState {
    fn new(window: slint::Weak<AppWindow>) -> Rc<Self> {
        let (sender, receiver) = mpsc::channel();
        Rc::new(Self {
            window,
            devices: RefCell::new(Vec::new()),
            busy: Cell::new(false),
            timer: Timer::default(),
            sender,
            receiver,
        })
    }

    fn set_busy(&self, busy: bool, status: &str) {
        self.busy.set(busy);
        if let Some(ui) = self.window.upgrade() {
            ui.set_busy(busy);
            ui.set_status(status.into());
        }
    }

    fn start_work(
        self: &Rc<Self>,
        status: &str,
        task: impl FnOnce() -> WorkerMessage + Send + 'static,
    ) {
        if self.busy.get() {
            return;
        }
        self.set_busy(true, status);
        let weak = Rc::downgrade(self);
        self.timer
            .start(TimerMode::Repeated, Duration::from_millis(50), move || {
                if let Some(state) = weak.upgrade() {
                    state.receive();
                }
            });
        let sender = self.sender.clone();
        thread::spawn(move || {
            let _ = sender.send(task());
        });
    }

    fn refresh(self: &Rc<Self>) {
        self.start_work("再生出力を調べています。", || {
            WorkerMessage::Devices(win::list_choices())
        });
    }

    fn selected(&self) -> Option<DeviceChoice> {
        let ui = self.window.upgrade()?;
        let index = usize::try_from(ui.get_selected_index()).ok()?;
        self.devices.borrow().get(index).cloned()
    }

    fn update_shortcut_name(&self, index: i32) {
        let Some(ui) = self.window.upgrade() else {
            return;
        };
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let Some(choice) = self.devices.borrow().get(index).cloned() else {
            return;
        };
        match choice {
            DeviceChoice::Bluetooth(device) => {
                ui.set_shortcut_name(format!("{} をPCで使う", device.name).into());
                ui.set_check_label("接続確認".into());
            }
            DeviceChoice::Audio(endpoint) => {
                ui.set_shortcut_name(format!("{} に切り替え", endpoint.name).into());
                ui.set_check_label("利用可能か確認".into());
            }
        }
    }

    fn register(self: &Rc<Self>) {
        if self.busy.get() {
            return;
        }
        let Some(choice) = self.selected() else {
            win::error_dialog("再生出力を選択してください。");
            return;
        };
        let Some(ui) = self.window.upgrade() else {
            return;
        };
        let name = ui.get_shortcut_name().to_string();
        if name.trim().is_empty() {
            win::error_dialog("ショートカット名を入力してください。");
            return;
        }
        self.start_work(
            "設定を保存し、ショートカットを作成しています。",
            move || WorkerMessage::Registered(win::register_choice(&choice, &name)),
        );
    }

    fn check(self: &Rc<Self>) {
        if self.busy.get() {
            return;
        }
        let Some(choice) = self.selected() else {
            win::error_dialog("再生出力を選択してください。");
            return;
        };
        self.start_work(
            "選択した再生出力を確認しています。",
            move || WorkerMessage::Checked(win::check_choice(&choice)),
        );
    }

    fn receive(&self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                WorkerMessage::Devices(Ok(list)) => {
                    let devices = list.choices;
                    let labels: Vec<SharedString> = devices
                        .iter()
                        .map(|choice| {
                            match choice {
                                DeviceChoice::Bluetooth(device) if device.connected => {
                                    format!("[Bluetooth・接続中] {}", device.name)
                                }
                                DeviceChoice::Bluetooth(device) => {
                                    format!("[Bluetooth] {}", device.name)
                                }
                                DeviceChoice::Audio(endpoint) => {
                                    format!(
                                        "[出力のみ] {} ({})",
                                        endpoint.name, endpoint.device_name
                                    )
                                }
                            }
                            .into()
                        })
                        .collect();
                    let count = devices.len();
                    *self.devices.borrow_mut() = devices;
                    if let Some(ui) = self.window.upgrade() {
                        ui.set_selected_index(-1);
                        ui.set_device_labels(ModelRc::new(VecModel::from(labels)));
                        if count > 0 {
                            ui.set_selected_index(0);
                            self.update_shortcut_name(0);
                        } else {
                            ui.set_shortcut_name("".into());
                        }
                    }
                    if count == 0 {
                        self.set_busy(
                            false,
                            "出力先が見つかりません。接続を確認して再検出してください。",
                        );
                    } else {
                        self.set_busy(false, &format!("{count}件の出力先を検出しました。"));
                    }
                    if let Some(error) = list.bluetooth_error {
                        win::error_dialog(&format!(
                            "Bluetooth機器を取得できません。通常の再生出力は利用できます。\n{error}"
                        ));
                    }
                }
                WorkerMessage::Devices(Err(error)) => {
                    self.devices.borrow_mut().clear();
                    if let Some(ui) = self.window.upgrade() {
                        ui.set_selected_index(-1);
                        ui.set_device_labels(ModelRc::default());
                        ui.set_shortcut_name("".into());
                    }
                    self.set_busy(false, "機器の検出に失敗しました。再検出してください。");
                    win::error_dialog(&error);
                }
                WorkerMessage::Registered(Ok(())) => {
                    self.set_busy(false, "デスクトップにショートカットを作成しました。");
                }
                WorkerMessage::Registered(Err(error)) => {
                    self.set_busy(false, "作成に失敗しました。設定を確認してください。");
                    win::error_dialog(&error);
                }
                WorkerMessage::Checked(Ok(())) => {
                    self.set_busy(
                        false,
                        "再生出力を確認しました。既定出力は変更していません。",
                    );
                }
                WorkerMessage::Checked(Err(error)) => {
                    self.set_busy(false, "再生出力を確認できませんでした。");
                    win::error_dialog(&error);
                }
            }
        }
        if !self.busy.get() {
            self.timer.stop();
        }
    }
}

pub fn run() -> Result<(), String> {
    let ui = AppWindow::new().map_err(|e| format!("設定画面を初期化できません: {e}"))?;
    let state = UiState::new(ui.as_weak());

    let refresh_state = Rc::clone(&state);
    ui.on_refresh(move || refresh_state.refresh());
    let selection_state = Rc::clone(&state);
    ui.on_device_selected(move |index| selection_state.update_shortcut_name(index));
    let check_state = Rc::clone(&state);
    ui.on_check(move || check_state.check());
    let create_state = Rc::clone(&state);
    ui.on_create(move || create_state.register());
    let close_state = Rc::clone(&state);
    ui.window().on_close_requested(move || {
        if close_state.busy.get() {
            close_state.set_busy(true, "処理が終わるまでお待ちください。");
            CloseRequestResponse::KeepWindowShown
        } else {
            CloseRequestResponse::HideWindow
        }
    });

    state.refresh();
    ui.run()
        .map_err(|e| format!("設定画面を表示できません: {e}"))
}
