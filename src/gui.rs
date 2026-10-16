use crate::win::{self, ChoiceList, DeviceChoice};
use native_windows_gui as nwg;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

enum WorkerMessage {
    Devices(Result<ChoiceList, String>),
    Registered(Result<(), String>),
    Checked(Result<(), String>),
}

struct Ui {
    window: nwg::Window,
    device_label: nwg::Label,
    devices_box: nwg::ComboBox<String>,
    refresh_button: nwg::Button,
    shortcut_label: nwg::Label,
    shortcut_input: nwg::TextInput,
    create_button: nwg::Button,
    check_button: nwg::Button,
    status_label: nwg::Label,
    notice: nwg::Notice,
    devices: RefCell<Vec<DeviceChoice>>,
    busy: Cell<bool>,
    sender: Sender<WorkerMessage>,
    receiver: Receiver<WorkerMessage>,
}

impl Ui {
    fn build() -> Result<Rc<Self>, String> {
        nwg::init().map_err(|e| format!("設定画面を初期化できません: {e}"))?;
        nwg::Font::set_global_family("Segoe UI")
            .map_err(|e| format!("設定画面のフォントを設定できません: {e}"))?;
        let (sender, receiver) = mpsc::channel();
        let mut ui = Self {
            window: Default::default(),
            device_label: Default::default(),
            devices_box: Default::default(),
            refresh_button: Default::default(),
            shortcut_label: Default::default(),
            shortcut_input: Default::default(),
            create_button: Default::default(),
            check_button: Default::default(),
            status_label: Default::default(),
            notice: Default::default(),
            devices: RefCell::new(Vec::new()),
            busy: Cell::new(false),
            sender,
            receiver,
        };
        nwg::Window::builder()
            .title("Audio Shortcut Switch - 設定")
            .size((620, 290))
            .position((360, 240))
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
            .build(&mut ui.window)
            .map_err(|e| e.to_string())?;
        nwg::Label::builder()
            .text("切り替える再生出力")
            .position((20, 18))
            .size((570, 24))
            .parent(&ui.window)
            .build(&mut ui.device_label)
            .map_err(|e| e.to_string())?;
        nwg::ComboBox::builder()
            .collection(Vec::<String>::new())
            .position((20, 46))
            .size((475, 180))
            .parent(&ui.window)
            .build(&mut ui.devices_box)
            .map_err(|e| e.to_string())?;
        nwg::Button::builder()
            .text("再検出")
            .position((505, 46))
            .size((90, 28))
            .parent(&ui.window)
            .build(&mut ui.refresh_button)
            .map_err(|e| e.to_string())?;
        nwg::Label::builder()
            .text("デスクトップのショートカット名")
            .position((20, 92))
            .size((570, 24))
            .parent(&ui.window)
            .build(&mut ui.shortcut_label)
            .map_err(|e| e.to_string())?;
        nwg::TextInput::builder()
            .position((20, 120))
            .size((575, 28))
            .parent(&ui.window)
            .build(&mut ui.shortcut_input)
            .map_err(|e| e.to_string())?;
        nwg::Button::builder()
            .text("利用可能か確認")
            .position((20, 162))
            .size((130, 28))
            .parent(&ui.window)
            .build(&mut ui.check_button)
            .map_err(|e| e.to_string())?;
        nwg::Button::builder()
            .text("作成")
            .position((505, 162))
            .size((90, 28))
            .parent(&ui.window)
            .build(&mut ui.create_button)
            .map_err(|e| e.to_string())?;
        nwg::Label::builder()
            .text("機器を検出しています。")
            .position((20, 205))
            .size((575, 56))
            .parent(&ui.window)
            .build(&mut ui.status_label)
            .map_err(|e| e.to_string())?;
        nwg::Notice::builder()
            .parent(&ui.window)
            .build(&mut ui.notice)
            .map_err(|e| e.to_string())?;
        Ok(Rc::new(ui))
    }

    fn set_busy(&self, busy: bool, status: &str) {
        self.busy.set(busy);
        self.refresh_button.set_enabled(!busy);
        self.create_button
            .set_enabled(!busy && !self.devices.borrow().is_empty());
        self.check_button
            .set_enabled(!busy && !self.devices.borrow().is_empty());
        self.devices_box.set_enabled(!busy);
        self.shortcut_input.set_enabled(!busy);
        self.status_label.set_text(status);
    }

    fn refresh(&self) {
        if self.busy.get() {
            return;
        }
        self.set_busy(true, "再生出力を調べています。");
        let sender = self.sender.clone();
        let notice = self.notice.sender();
        thread::spawn(move || {
            let result = win::list_choices();
            let _ = sender.send(WorkerMessage::Devices(result));
            notice.notice();
        });
    }

    fn selected(&self) -> Option<DeviceChoice> {
        let index = self.devices_box.selection()?;
        self.devices.borrow().get(index).cloned()
    }

    fn update_shortcut_name(&self) {
        if let Some(choice) = self.selected() {
            match choice {
                DeviceChoice::Bluetooth(device) => {
                    self.shortcut_input
                        .set_text(&format!("{} をPCで使う", device.name));
                    self.check_button.set_text("接続確認");
                }
                DeviceChoice::Audio(endpoint) => {
                    self.shortcut_input
                        .set_text(&format!("{} に切り替え", endpoint.name));
                    self.check_button.set_text("利用可能か確認");
                }
            }
        }
    }

    fn register(&self) {
        if self.busy.get() {
            return;
        }
        let Some(choice) = self.selected() else {
            nwg::modal_error_message(&self.window, "機器を選択", "再生出力を選択してください。");
            return;
        };
        let name = self.shortcut_input.text();
        if name.trim().is_empty() {
            nwg::modal_error_message(
                &self.window,
                "名前を入力",
                "ショートカット名を入力してください。",
            );
            return;
        }
        self.set_busy(true, "設定を保存し、ショートカットを作成しています。");
        let sender = self.sender.clone();
        let notice = self.notice.sender();
        thread::spawn(move || {
            let result = win::register_choice(&choice, &name);
            let _ = sender.send(WorkerMessage::Registered(result));
            notice.notice();
        });
    }

    fn check(&self) {
        if self.busy.get() {
            return;
        }
        let Some(choice) = self.selected() else {
            nwg::modal_error_message(&self.window, "機器を選択", "再生出力を選択してください。");
            return;
        };
        self.set_busy(true, "選択した再生出力を確認しています。");
        let sender = self.sender.clone();
        let notice = self.notice.sender();
        thread::spawn(move || {
            let result = win::check_choice(&choice);
            let _ = sender.send(WorkerMessage::Checked(result));
            notice.notice();
        });
    }

    fn receive(&self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                WorkerMessage::Devices(Ok(list)) => {
                    let devices = list.choices;
                    let labels = devices
                        .iter()
                        .map(|choice| match choice {
                            DeviceChoice::Bluetooth(device) if device.connected => {
                                format!("[Bluetooth・接続中] {}", device.name)
                            }
                            DeviceChoice::Bluetooth(device) => {
                                format!("[Bluetooth] {}", device.name)
                            }
                            DeviceChoice::Audio(endpoint) if endpoint.active => {
                                format!("[出力のみ] {} ({})", endpoint.name, endpoint.device_name)
                            }
                            DeviceChoice::Audio(endpoint) => {
                                format!(
                                    "[出力のみ・現在利用不可] {} ({})",
                                    endpoint.name, endpoint.device_name
                                )
                            }
                        })
                        .collect();
                    self.devices_box.set_collection(labels);
                    *self.devices.borrow_mut() = devices;
                    if !self.devices.borrow().is_empty() {
                        self.devices_box.set_selection(Some(0));
                        self.update_shortcut_name();
                    }
                    let count = self.devices.borrow().len();
                    self.set_busy(
                        false,
                        &format!("{count}件の出力先を検出しました。作成時は切り替えません。"),
                    );
                    if let Some(error) = list.bluetooth_error {
                        nwg::modal_error_message(
                            &self.window,
                            "Bluetooth機器を取得できません",
                            &format!("通常の再生出力は利用できます。\n{error}"),
                        );
                    }
                }
                WorkerMessage::Devices(Err(error)) => {
                    self.devices_box.set_collection(Vec::new());
                    self.devices.borrow_mut().clear();
                    self.set_busy(false, "機器の検出に失敗しました。再検出してください。");
                    nwg::modal_error_message(&self.window, "検出失敗", &error);
                }
                WorkerMessage::Registered(Ok(())) => {
                    self.set_busy(false, "ショートカットを作成しました。");
                    nwg::modal_info_message(
                        &self.window,
                        "作成完了",
                        "デスクトップにショートカットを作成しました。必要なら確認ボタンで出力先を確認してください。",
                    );
                }
                WorkerMessage::Registered(Err(error)) => {
                    self.set_busy(false, "作成に失敗しました。設定を確認してください。");
                    nwg::modal_error_message(&self.window, "作成失敗", &error);
                }
                WorkerMessage::Checked(Ok(())) => {
                    self.set_busy(false, "再生出力を確認しました。");
                    nwg::modal_info_message(
                        &self.window,
                        "確認完了",
                        "再生出力を確認しました。既定出力は変更していません。",
                    );
                }
                WorkerMessage::Checked(Err(error)) => {
                    self.set_busy(false, "再生出力を確認できませんでした。");
                    nwg::modal_error_message(&self.window, "確認失敗", &error);
                }
            }
        }
    }
}

pub fn run() -> Result<(), String> {
    let ui = Ui::build()?;
    let handler_ui = Rc::clone(&ui);
    let handler =
        nwg::full_bind_event_handler(&ui.window.handle, move |event, _data, handle| match event {
            nwg::Event::OnWindowClose if handle == handler_ui.window.handle => {
                if handler_ui.busy.get() {
                    nwg::modal_info_message(
                        &handler_ui.window,
                        "処理中",
                        "処理が終わるまでお待ちください。",
                    );
                } else {
                    nwg::stop_thread_dispatch();
                }
            }
            nwg::Event::OnButtonClick if handle == handler_ui.refresh_button.handle => {
                handler_ui.refresh();
            }
            nwg::Event::OnButtonClick if handle == handler_ui.create_button.handle => {
                handler_ui.register();
            }
            nwg::Event::OnButtonClick if handle == handler_ui.check_button.handle => {
                handler_ui.check();
            }
            nwg::Event::OnComboxBoxSelection if handle == handler_ui.devices_box.handle => {
                handler_ui.update_shortcut_name();
            }
            nwg::Event::OnNotice if handle == handler_ui.notice.handle => {
                handler_ui.receive();
            }
            _ => {}
        });
    ui.refresh();
    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
    Ok(())
}
