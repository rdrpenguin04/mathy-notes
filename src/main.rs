#![deny(clippy::all)] 
#![warn(clippy::pedantic, clippy::nursery)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![windows_subsystem = "windows"]

use eframe::egui::{self, Modifiers, TextBuffer, TextStyle, Ui};
use expr::evaluate;

pub mod expr;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Notes",
        native_options,
        Box::new(|cc| Box::new(NotesApp::new(cc))),
    )
    .unwrap();
}

#[derive(Default)]
struct NotesApp {
    notes_list: Vec<String>,
    settings_open: bool,
    fixed_width: bool,
}

impl NotesApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.storage.map_or_else(Self::default, |storage| Self {
            notes_list: storage
                .get_string("notes_list")
                .map(|x| x.split("\x02").map(str::to_owned).collect())
                .unwrap_or_else(|| vec![storage.get_string("notes_text").unwrap_or_default()]),
            settings_open: false,
            fixed_width: matches!(storage.get_string("fixed_width").as_deref(), Some("true")),
        })
    }
}

impl eframe::App for NotesApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let eval = ui.input_mut(|x| {
                x.consume_key(Modifiers::CTRL, egui::Key::Enter)
                    || x.consume_key(Modifiers::SHIFT, egui::Key::Enter)
            });
            self.settings_open ^= ui.button("Settings").clicked();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_sized(ui.available_size(), |ui: &mut Ui| {
                    let text_edit = egui::TextEdit::multiline(&mut self.notes_list[0]).font(
                        if self.fixed_width {
                            TextStyle::Monospace
                        } else {
                            TextStyle::Body
                        },
                    );
                    let mut output = text_edit.show(ui);
                    if eval {
                        if let Some(cursor) = output.cursor_range {
                            let p_idx = cursor.primary.ccursor.index;
                            let s_idx = cursor.secondary.ccursor.index;
                            let start = if p_idx == s_idx {
                                self.notes_list[0].char_range(0..p_idx)
                                    .rfind(|x| matches!(x, ':' | '=' | '\n'))
                                    .map_or(0, |x| x + 1)
                            } else {
                                p_idx.min(s_idx)
                            };
                            let end_ch = p_idx.max(s_idx);
                            let end_byte = self.notes_list[0].byte_index_from_char_index(end_ch);
                            let text = &self.notes_list[0][start..end_byte];
                            let result = evaluate(text);
                            let insertion = format!(
                                " = {}",
                                match result {
                                    Ok(x) => x.to_string(),
                                    Err(x) => x.to_string(),
                                }
                            );
                            output.state.cursor.set_char_range(Some(
                                egui::text::CCursorRange {
                                    primary: egui::text::CCursor {
                                        index: end_ch + insertion.len(),
                                        prefer_next_row: true,
                                    },
                                    secondary: egui::text::CCursor {
                                        index: end_ch + insertion.len(),
                                        prefer_next_row: true,
                                    },
                                },
                            ));
                            output.state.store(ctx, output.response.id);
                            self.notes_list[0].insert_str(end_byte, &insertion);
                        }
                    }
                    output.response
                })
            });
        });
        egui::Window::new("Settings")
            .open(&mut self.settings_open)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.fixed_width, "Enable monospace / fixed-width font");
            });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("notes_list", self.notes_list.clone().join("\x02")); // non-printable separator
        storage.set_string("fixed_width", self.fixed_width.to_string());
        storage.flush();
    }
}
