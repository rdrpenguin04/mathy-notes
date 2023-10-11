#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![windows_subsystem = "windows"]

use eframe::egui::{self, Modifiers, Ui};
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
    notes_text: String,
}

impl NotesApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.storage.map_or_else(Self::default, |storage| Self {
            notes_text: storage.get_string("notes_text").unwrap_or_default(),
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_sized(ui.available_size(), move |ui: &mut Ui| {
                    let text_edit = egui::TextEdit::multiline(&mut self.notes_text);
                    let mut output = text_edit.show(ui);
                    if eval {
                        if let Some(cursor) = output.cursor_range {
                            let p_idx = cursor.primary.ccursor.index;
                            let s_idx = cursor.secondary.ccursor.index;
                            let start = if p_idx == s_idx {
                                self.notes_text[..p_idx]
                                    .rfind(|x| matches!(x, ':' | '=' | '\n'))
                                    .map_or(0, |x| x + 1)
                            } else {
                                p_idx.min(s_idx)
                            };
                            let end = p_idx.max(s_idx);
                            let text = &self.notes_text[start..end];
                            let result = evaluate(text);
                            let insertion = format!(
                                " = {}",
                                match result {
                                    Ok(x) => x.to_string(),
                                    Err(x) => x.to_string(),
                                }
                            );
                            output.state.set_ccursor_range(Some(
                                egui::widgets::text_edit::CCursorRange {
                                    primary: egui::text::CCursor {
                                        index: end + insertion.len(),
                                        prefer_next_row: true,
                                    },
                                    secondary: egui::text::CCursor {
                                        index: end + insertion.len(),
                                        prefer_next_row: true,
                                    },
                                },
                            ));
                            output.state.store(ctx, output.response.id);
                            self.notes_text.insert_str(end, &insertion);
                        }
                    }
                    output.response
                })
            });
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("notes_text", self.notes_text.clone());
        storage.flush();
    }
}
