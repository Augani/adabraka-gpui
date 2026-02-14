#![allow(dead_code)]

use crate::platform::TrayMenuItem;

struct GpuiTray {
    icon_data: Vec<u8>,
    tooltip: String,
    menu_items: Vec<TrayMenuItem>,
}

impl ksni::Tray for GpuiTray {
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        if self.icon_data.is_empty() {
            return vec![];
        }
        if let Ok(img) = image::load_from_memory(&self.icon_data) {
            let rgba = img.to_rgba8();
            let width = rgba.width() as i32;
            let height = rgba.height() as i32;
            let raw = rgba.as_raw();
            let mut argb_data = Vec::with_capacity(raw.len());
            for pixel in raw.chunks_exact(4) {
                argb_data.push(pixel[3]);
                argb_data.push(pixel[0]);
                argb_data.push(pixel[1]);
                argb_data.push(pixel[2]);
            }
            vec![ksni::Icon {
                width,
                height,
                data: argb_data,
            }]
        } else {
            vec![]
        }
    }

    fn title(&self) -> String {
        self.tooltip.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.tooltip.clone(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        self.menu_items
            .iter()
            .map(|item| convert_menu_item(item))
            .collect()
    }
}

fn convert_menu_item(item: &TrayMenuItem) -> ksni::MenuItem<GpuiTray> {
    match item {
        TrayMenuItem::Action { label, .. } => ksni::MenuItem::Standard(ksni::menu::StandardItem {
            label: label.to_string(),
            ..Default::default()
        }),
        TrayMenuItem::Separator => ksni::MenuItem::Separator,
        TrayMenuItem::Submenu { label, items } => {
            ksni::MenuItem::Standard(ksni::menu::StandardItem {
                label: label.to_string(),
                submenu: items.iter().map(|i| convert_menu_item(i)).collect(),
                ..Default::default()
            })
        }
        TrayMenuItem::Toggle { label, checked, .. } => {
            ksni::MenuItem::Standard(ksni::menu::StandardItem {
                label: label.to_string(),
                icon_name: if *checked {
                    "checkbox-checked-symbolic".to_string()
                } else {
                    String::new()
                },
                ..Default::default()
            })
        }
    }
}

pub struct LinuxTray {
    handle: Option<ksni::Handle<GpuiTray>>,
}

impl LinuxTray {
    pub fn new() -> Self {
        Self { handle: None }
    }

    fn ensure_started(&mut self) {
        if self.handle.is_some() {
            return;
        }
        let tray = GpuiTray {
            icon_data: Vec::new(),
            tooltip: String::new(),
            menu_items: Vec::new(),
        };
        let service = ksni::TrayService::new(tray);
        self.handle = Some(service.handle());
        service.spawn();
    }

    pub fn set_icon(&mut self, icon_data: Option<&[u8]>) {
        self.ensure_started();
        if let Some(handle) = &self.handle {
            let data = icon_data.unwrap_or(&[]).to_vec();
            handle.update(move |tray: &mut GpuiTray| {
                tray.icon_data = data.clone();
            });
        }
    }

    pub fn set_tooltip(&mut self, tooltip: &str) {
        self.ensure_started();
        if let Some(handle) = &self.handle {
            let tooltip = tooltip.to_string();
            handle.update(move |tray: &mut GpuiTray| {
                tray.tooltip = tooltip.clone();
            });
        }
    }

    pub fn set_menu(&mut self, items: Vec<TrayMenuItem>) {
        self.ensure_started();
        if let Some(handle) = &self.handle {
            handle.update(move |tray: &mut GpuiTray| {
                tray.menu_items = items.clone();
            });
        }
    }
}
