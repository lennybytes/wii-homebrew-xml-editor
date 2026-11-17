use std::path::{Path, PathBuf};

use eframe::egui;
use egui::{Align, Color32, FontFamily, FontId, Layout, RichText, Stroke, TextStyle};

use crate::meta::{self, Level, Meta};

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

const ACCENT: Color32 = Color32::from_rgb(122, 162, 255);
const TEXT: Color32 = Color32::from_rgb(228, 232, 240);
const MUTED: Color32 = Color32::from_rgb(148, 156, 170);
const BORDER: Color32 = Color32::from_rgb(46, 51, 63);
const BG: Color32 = Color32::from_rgb(17, 19, 25);
const TOOLBAR_BG: Color32 = Color32::from_rgb(22, 25, 32);
const STATUS_BG: Color32 = Color32::from_rgb(14, 16, 21);
const OK: Color32 = Color32::from_rgb(120, 220, 150);
const WARN: Color32 = Color32::from_rgb(255, 184, 92);
const ERROR: Color32 = Color32::from_rgb(255, 110, 118);

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct MetaEditorApp {
    meta: Meta,
    saved: Meta,
    path: Option<PathBuf>,
    status: Option<Status>,
    confirm_new: bool,
    last_title: String,
}

impl MetaEditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        setup_style(&cc.egui_ctx);

        let meta = Meta::new();
        let mut app = Self {
            saved: meta.clone(),
            meta,
            path: None,
            status: Some(Status {
                kind: StatusKind::Info,
                text: "New document. Fill in the fields, then save as meta.xml.".to_string(),
            }),
            confirm_new: false,
            last_title: String::new(),
        };

        if let Some(path) = initial_path {
            app.open_path(path);
        }

        app
    }

    fn is_dirty(&self) -> bool {
        self.meta != self.saved
    }

    fn set_status(&mut self, kind: StatusKind, text: impl Into<String>) {
        self.status = Some(Status {
            kind,
            text: text.into(),
        });
    }

    // -- Actions ------------------------------------------------------------

    fn request_new(&mut self) {
        if self.is_dirty() {
            self.confirm_new = true;
        } else {
            self.new_document();
        }
    }

    fn new_document(&mut self) {
        self.meta = Meta::new();
        self.saved = self.meta.clone();
        self.path = None;
        self.confirm_new = false;
        self.set_status(StatusKind::Info, "New document created.");
    }

    fn open_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Open meta.xml")
            .add_filter("Wii meta.xml", &["xml"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.open_path(path);
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(text) => match Meta::parse(&text) {
                Ok(meta) => {
                    self.meta = meta;
                    self.saved = self.meta.clone();
                    self.confirm_new = false;
                    let name = file_label(&path);
                    self.path = Some(path);
                    self.set_status(StatusKind::Success, format!("Opened {name}."));
                }
                Err(err) => self.set_status(
                    StatusKind::Error,
                    format!("Could not parse {}: {err}", file_label(&path)),
                ),
            },
            Err(err) => self.set_status(
                StatusKind::Error,
                format!("Could not open {}: {err}", file_label(&path)),
            ),
        }
    }

    fn save(&mut self) {
        match self.path.clone() {
            Some(path) => self.write_to(&path),
            None => self.save_as_dialog(),
        }
    }

    fn save_as_dialog(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save meta.xml")
            .add_filter("Wii meta.xml", &["xml"])
            .set_file_name("meta.xml");

        if let Some(dir) = self.path.as_ref().and_then(|path| path.parent()) {
            dialog = dialog.set_directory(dir);
        }

        if let Some(mut path) = dialog.save_file() {
            if path.extension().is_none() {
                path.set_extension("xml");
            }
            self.write_to(&path);
        }
    }

    fn write_to(&mut self, path: &Path) {
        let xml = self.meta.to_xml();
        match std::fs::write(path, xml) {
            Ok(()) => {
                let issues = self.meta.validate();
                let errors = issues.iter().filter(|i| i.level == Level::Error).count();
                let warnings = issues.len() - errors;
                let name = file_label(path);

                self.saved = self.meta.clone();
                self.path = Some(path.to_path_buf());

                if errors > 0 {
                    self.set_status(
                        StatusKind::Warning,
                        format!("Saved {name} with {errors} error(s) and {warnings} warning(s)."),
                    );
                } else if warnings > 0 {
                    self.set_status(
                        StatusKind::Warning,
                        format!("Saved {name} with {warnings} warning(s)."),
                    );
                } else {
                    self.set_status(StatusKind::Success, format!("Saved {name}."));
                }
            }
            Err(err) => self.set_status(StatusKind::Error, format!("Could not save: {err}")),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};

        let save_as = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
        let save = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
        let open = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
        let new = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);

        if ctx.input_mut(|input| input.consume_shortcut(&save_as)) {
            self.save_as_dialog();
        } else if ctx.input_mut(|input| input.consume_shortcut(&save)) {
            self.save();
        } else if ctx.input_mut(|input| input.consume_shortcut(&open)) {
            self.open_dialog();
        } else if ctx.input_mut(|input| input.consume_shortcut(&new)) {
            self.request_new();
        }
    }

    fn handle_dropped_file(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .first()
                .map(|file| file.path().to_path_buf())
        });

        if let Some(path) = dropped {
            self.open_path(path);
        }
    }

    // -- UI -----------------------------------------------------------------

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Wii Meta XML Editor")
                        .size(19.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("meta.xml builder for the Homebrew Channel")
                        .size(11.0)
                        .color(MUTED),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button("Save As…")
                    .on_hover_text("Save to a new file (Ctrl+Shift+S)")
                    .clicked()
                {
                    self.save_as_dialog();
                }
                let save_button =
                    egui::Button::new(RichText::new("Save").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(48, 84, 150));
                if ui
                    .add(save_button)
                    .on_hover_text("Save the current file (Ctrl+S)")
                    .clicked()
                {
                    self.save();
                }
                if ui
                    .button("Open…")
                    .on_hover_text("Open an existing meta.xml (Ctrl+O)")
                    .clicked()
                {
                    self.open_dialog();
                }
                if ui
                    .button("New")
                    .on_hover_text("Create a new meta.xml (Ctrl+N)")
                    .clicked()
                {
                    self.request_new();
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let path_text = self
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No file yet".to_string());

            ui.label(
                RichText::new(path_text)
                    .font(FontId::new(11.0, FontFamily::Monospace))
                    .color(MUTED),
            );

            if self.is_dirty() {
                ui.label(RichText::new("● unsaved changes").size(11.0).color(WARN));
            }

            if let Some(status) = &self.status {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = egui::Label::new(
                        RichText::new(&status.text)
                            .size(11.0)
                            .color(status.kind.color()),
                    )
                    .truncate();
                    ui.add(label).on_hover_text(&status.text);
                });
            }
        });
    }

    fn columns(&mut self, ui: &mut egui::Ui) {
        let gap = 18.0;
        let total_width = ui.available_width();
        let form_width = ((total_width - gap) * 0.54).clamp(300.0, 660.0);
        let preview_width = (total_width - form_width - gap).max(180.0);
        let height = ui.available_height();

        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(form_width, height),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("form_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.form_ui(ui));
                },
            );

            ui.add_space(gap);

            ui.allocate_ui_with_layout(
                egui::vec2(preview_width, height),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("preview_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.preview_ui(ui));
                },
            );
        });
    }

    fn form_ui(&mut self, ui: &mut egui::Ui) {
        section_heading(ui, "APPLICATION");

        text_field(ui, "Name", "My Homebrew App", &mut self.meta.name);

        ui.columns(2, |columns| {
            text_field(
                &mut columns[0],
                "Coder",
                "Your name or team",
                &mut self.meta.coder,
            );
            text_field(&mut columns[1], "Version", "1.0", &mut self.meta.version);
        });

        ui.horizontal(|ui| {
            ui.label(field_label("Release date"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .small_button("Set to now")
                    .on_hover_text("Insert the current UTC time")
                    .clicked()
                {
                    self.meta.release_date = meta::now_release_date();
                }
            });
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.meta.release_date)
                .hint_text("YYYYMMDDHHMMSS")
                .font(TextStyle::Monospace)
                .desired_width(f32::INFINITY),
        );
        ui.label(
            RichText::new("Format: year, month, day, hour, minute, second (UTC)")
                .size(10.5)
                .color(MUTED),
        );
        ui.add_space(10.0);

        section_heading(ui, "DESCRIPTIONS");

        text_field(
            ui,
            "Short description",
            "One-line summary shown in the Homebrew Channel",
            &mut self.meta.short_description,
        );

        multiline_field(
            ui,
            "Long description",
            "Full description shown on the app page",
            &mut self.meta.long_description,
            8,
            false,
        );

        section_heading(ui, "ARGUMENTS");

        multiline_field(
            ui,
            "Arguments",
            "One argument per line, e.g. --fullscreen",
            &mut self.meta.arguments,
            3,
            true,
        );
        ui.label(
            RichText::new("An empty list is written as a single <arg/> element.")
                .size(10.5)
                .color(MUTED),
        );
        ui.add_space(12.0);

        egui::CollapsingHeader::new(RichText::new("Advanced").size(12.0).color(MUTED)).show(
            ui,
            |ui| {
                text_field(
                    ui,
                    "App version (XML attribute)",
                    "1",
                    &mut self.meta.app_version,
                );
                ui.label(
                    RichText::new("This is the version=\"…\" value of <app>. Usually 1.")
                        .size(10.5)
                        .color(MUTED),
                );
            },
        );

        ui.add_space(12.0);
    }

    fn preview_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("XML PREVIEW")
                    .size(12.0)
                    .strong()
                    .color(ACCENT),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button("Copy")
                    .on_hover_text("Copy the generated XML to the clipboard")
                    .clicked()
                {
                    ui.ctx().copy_text(self.meta.to_xml());
                    self.set_status(StatusKind::Success, "Copied XML to the clipboard.");
                }
            });
        });

        ui.add_space(6.0);
        validation_ui(ui, &self.meta);
        ui.add_space(8.0);

        let xml = self.meta.to_xml();
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8)
            .inner_margin(10)
            .show(ui, |ui| {
                let mut text: &str = &xml;
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .code_editor()
                        .frame(egui::Frame::NONE)
                        .desired_rows(22)
                        .desired_width(f32::INFINITY)
                        .text_color(TEXT),
                );
            });

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "The preview updates live. Save writes this XML to the selected meta.xml file.",
            )
            .size(10.5)
            .color(MUTED),
        );
    }

    fn confirm_new_window(&mut self, ctx: &egui::Context) {
        let mut close = false;

        egui::Window::new("Unsaved changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("The current document has unsaved changes.");
                ui.label("Create a new meta.xml anyway?");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button("Discard and create new").clicked() {
                        self.new_document();
                        close = true;
                    }
                });
            });

        if close {
            self.confirm_new = false;
        }
    }

    fn update_window_title(&mut self, ctx: &egui::Context) {
        let name = self
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".to_string());
        let dirty = if self.is_dirty() { " ●" } else { "" };
        let title = format!("Wii Meta XML Editor — {name}{dirty}");

        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }
}

impl eframe::App for MetaEditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.handle_shortcuts(&ctx);
        self.handle_dropped_file(&ctx);

        egui::Panel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(TOOLBAR_BG)
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show_separator_line(false)
            .show(ui, |ui| self.toolbar(ui));

        egui::Panel::bottom("statusbar")
            .frame(
                egui::Frame::new()
                    .fill(STATUS_BG)
                    .inner_margin(egui::Margin::symmetric(16, 6))
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show_separator_line(false)
            .show(ui, |ui| self.status_bar(ui));

        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| self.columns(ui));

        if self.confirm_new {
            self.confirm_new_window(&ctx);
        }

        self.update_window_title(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        BG.to_normalized_gamma_f32()
    }
}

// ---------------------------------------------------------------------------
// Small UI helpers
// ---------------------------------------------------------------------------

fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.all_styles_mut(|style| {
        style.visuals = dark_visuals();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(11.0, 6.0);
        style.spacing.interact_size.y = 24.0;
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(21.0)),
            (TextStyle::Body, FontId::proportional(13.5)),
            (TextStyle::Button, FontId::proportional(13.5)),
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Monospace, FontId::monospace(12.0)),
        ]
        .into();
    });
}

fn dark_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = BG;
    visuals.window_fill = Color32::from_rgb(26, 29, 37);
    visuals.extreme_bg_color = Color32::from_rgb(11, 13, 17);
    visuals.faint_bg_color = Color32::from_rgb(25, 28, 36);
    visuals.code_bg_color = Color32::from_rgb(13, 15, 20);
    visuals.weak_text_color = Some(MUTED);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = Color32::from_rgb(44, 78, 140);
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = egui::CornerRadius::same(10);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    visuals.text_cursor.stroke = Stroke::new(2.0, ACCENT);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 27, 34);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(24, 27, 34);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(31, 35, 44);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(31, 35, 44);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(211, 216, 226));
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 47, 58);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(42, 47, 58);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.active.bg_fill = Color32::from_rgb(53, 60, 74);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(53, 60, 74);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(5);

    visuals
}

fn section_heading(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(11.0).strong().color(ACCENT));
    ui.add_space(3.0);
}

fn field_label(label: &str) -> RichText {
    RichText::new(label).size(12.0).color(TEXT)
}

fn text_field(ui: &mut egui::Ui, label: &str, hint: &str, value: &mut String) {
    ui.label(field_label(label));
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(f32::INFINITY),
    );
    ui.add_space(8.0);
}

fn multiline_field(
    ui: &mut egui::Ui,
    label: &str,
    hint: &str,
    value: &mut String,
    rows: usize,
    monospace: bool,
) {
    ui.label(field_label(label));
    let mut edit = egui::TextEdit::multiline(value)
        .hint_text(hint)
        .desired_rows(rows)
        .desired_width(f32::INFINITY);
    if monospace {
        edit = edit.font(TextStyle::Monospace);
    }
    ui.add(edit);
    ui.add_space(8.0);
}

fn validation_ui(ui: &mut egui::Ui, meta: &Meta) {
    let mut issues = meta.validate();
    issues.sort_by_key(|issue| issue.level == Level::Warning);

    if issues.is_empty() {
        ui.label(
            RichText::new("✓ Looks good — ready to save")
                .size(12.0)
                .color(OK),
        );
        return;
    }

    let errors = issues.iter().filter(|i| i.level == Level::Error).count();
    let warnings = issues.len() - errors;
    let summary = match (errors, warnings) {
        (0, w) => format!("Valid, but {w} warning(s)"),
        (e, 0) => format!("{e} error(s)"),
        (e, w) => format!("{e} error(s), {w} warning(s)"),
    };
    let color = if errors > 0 { ERROR } else { WARN };

    ui.label(RichText::new(summary).size(12.0).strong().color(color));
    for issue in &issues {
        let (icon, color) = match issue.level {
            Level::Error => ("✕", ERROR),
            Level::Warning => ("!", WARN),
        };
        ui.label(
            RichText::new(format!("{icon} {}", issue.message))
                .size(11.5)
                .color(color),
        );
    }
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum StatusKind {
    Info,
    Success,
    Warning,
    Error,
}

impl StatusKind {
    fn color(self) -> Color32 {
        match self {
            Self::Info => MUTED,
            Self::Success => OK,
            Self::Warning => WARN,
            Self::Error => ERROR,
        }
    }
}

struct Status {
    kind: StatusKind,
    text: String,
}
