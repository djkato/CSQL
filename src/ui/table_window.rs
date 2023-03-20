use crate::backend::backend_manager::Communication;
use crate::backend::csv_handler::ImportedData;
use crate::backend::database_handler::Tables;
use eframe::glow::CONSTANT_COLOR;
use egui::{ComboBox, Context, Ui};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::{mpsc::Sender, oneshot};
pub struct SpreadSheetWindow {
    sender: Sender<Communication>,
    csv_data_handle: Arc<Mutex<ImportedData>>,
    db_table_data_handle: Arc<Mutex<Tables>>,
    is_file_loaded_receiver: Option<oneshot::Receiver<bool>>,
    is_file_loaded: bool,
    current_table: Option<usize>,
    is_current_table_described: bool,
}
impl SpreadSheetWindow {
    pub fn default(
        sender: Sender<Communication>,
        csv_data_handle: Arc<Mutex<ImportedData>>,
        db_table_data_handle: Arc<Mutex<Tables>>,
    ) -> SpreadSheetWindow {
        SpreadSheetWindow {
            sender,
            is_file_loaded_receiver: None,
            is_file_loaded: false,
            csv_data_handle,
            db_table_data_handle,
            current_table: None,
            is_current_table_described: false,
        }
    }
}
impl SpreadSheetWindow {
    pub fn show(&mut self, ctx: &Context, ui: &mut Ui, is_db_connection_verified: bool) {
        self.ui(ui, ctx, is_db_connection_verified);
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &Context, is_db_connection_verified: bool) {
        /* Program Menu */

        ui.horizontal_top(|ui| {
            if ui.button("Open File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Spreadsheets", &["csv"])
                    .pick_file()
                {
                    self.open_file(path.display().to_string());
                };
            }
            if ui.button("Save File").clicked() {
                println!("Saving file lol");
            }
            //if ui.button("Current Working Table: ")
        });

        /* if db isn't connected, don't allow imports */

        if !is_db_connection_verified {
            return;
        };

        /* Handle file drops */

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.open_file(
                    i.raw
                        .dropped_files
                        .clone()
                        .get(0)
                        .unwrap()
                        .path
                        .as_ref()
                        .unwrap()
                        .display()
                        .to_string(),
                );
            }
        });
        self.check_file();
        SpreadSheetWindow::preview_files_being_dropped(ctx);

        if !self.is_file_loaded {
            ui.centered_and_justified(|ui| ui.heading("Drag and drop or Open a file..."));
        }

        if self.is_file_loaded {
            ui.group(|ui| {
                StripBuilder::new(ui) //So the window doesn't grow with the innerts
                    .size(Size::remainder().at_least(100.0)) // for the table
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            egui::ScrollArea::horizontal().show(ui, |ui| self.table_options(ui));
                        });
                    });
            });
        };
    }
    fn table_options(&mut self, ui: &mut Ui) {
        /* Create table select option */

        if let Ok(db_table_data) = &mut self.db_table_data_handle.try_lock() {
            let mut select_table = ComboBox::from_label("Select Table");
            if let Some(table_index) = self.current_table {
                select_table = select_table
                    .selected_text(&db_table_data.tables.get(table_index).unwrap().name);
            }
            ui.vertical(|ui| {
                select_table.show_ui(ui, |ui| {
                    for (table_i, table) in db_table_data.tables.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.current_table,
                            Some(table_i.clone()),
                            &table.name,
                        );
                    }
                });
            });

            /* If a table is selected, try if it's fields are discovered */
            if let Some(table_i) = self.current_table {
                if db_table_data.tables.get(table_i).unwrap().fields.is_some() {
                    self.is_current_table_described = true;
                } else {
                    self.sender
                        .try_send(Communication::GetTableDescription(table_i))
                        .unwrap_or_else(|e| println!("Failed asking to describe table, {}", e));
                }
            }
        }
        if self.is_current_table_described {
            self.table_builder(ui);
        }
    }
    fn table_builder(&mut self, ui: &mut Ui) {
        if let Ok(csv_data) = &mut self.csv_data_handle.try_lock() {
            if let Ok(db_table_data) = &mut self.db_table_data_handle.try_lock() {
                let mut table = TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

                for _i in 0..csv_data.data.cols() {
                    table = table.column(Column::auto().resizable(true).clip(false));
                }

                table
                    .column(Column::remainder())
                    .min_scrolled_height(0.0)
                    .header(20., |mut header| {
                        for i in 0..csv_data.data.cols() {
                            header.col(|ui| {
                                let mut combo_box: ComboBox;
                                if csv_data.are_headers {
                                    combo_box = ComboBox::new(
                                        i,
                                        csv_data.data.get(0, i).unwrap().data.clone(),
                                    );
                                } else {
                                    combo_box = ComboBox::new(i, "");
                                }
                                //if any field is assinged to this combobox, show it's text, else "----"
                                if let Some(selected_field) = db_table_data
                                    .tables
                                    .get(self.current_table.unwrap())
                                    .unwrap()
                                    .fields
                                    .as_ref()
                                    .unwrap()
                                    .iter()
                                    .find(|field| field.mapped_to_col == Some(i))
                                {
                                    combo_box = combo_box
                                        .selected_text(selected_field.description.field.clone());
                                } else {
                                    combo_box = combo_box.selected_text("-----");
                                }

                                /* When a Field gets attached to Col,  */
                                combo_box.show_ui(ui, |ui| {
                                    for field in db_table_data
                                        .tables
                                        .get_mut(self.current_table.unwrap())
                                        .unwrap()
                                        .fields
                                        .as_mut()
                                        .unwrap()
                                        .iter_mut()
                                    {
                                        if ui
                                            .selectable_value(
                                                &mut field.mapped_to_col,
                                                Some(i),
                                                field.description.field.clone(),
                                            )
                                            .clicked()
                                        {
                                            self.is_current_table_described = false;
                                            match self.sender.try_send(Communication::TryParseCol(i)){
                                                Ok(_) => {
                                                    for cel in csv_data.data.iter_col_mut(i) {
                                                        cel.curr_field_description =
                                                            Some(field.description.clone());
                                                    }
                                                }
                                                Err(e) => println!("failed sending parsecol request, {}",e)
                                            }
                                        }   
                                    }
                                });
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(15., csv_data.data.rows(), |row_index, mut row| {
                            for curr_cell in
                                csv_data.data.iter_row_mut(row_index)
                            {
                                /* If cell is bound to a field, color it's bg according to is_parsed */
                                row.col(|ui| {
                                    let mut err: Option<Arc<dyn Error>> = None;
                                    if curr_cell.curr_field_description.is_some() {
                                        match &curr_cell.is_parsed{
                                            Some(parse) => match parse{
                                                Ok(_) => ui.style_mut().visuals.extreme_bg_color = egui::Color32::DARK_GREEN,
                                                Err(arc) => {
                                                    err = Some(arc.clone());
                                                    ui.style_mut().visuals.extreme_bg_color = egui::Color32::DARK_RED;
                                                }
                                            }
                                            None => ui.style_mut().visuals.extreme_bg_color = egui::Color32::DARK_GRAY,
                                        }
                                    }
                                    if err.is_some(){
                                        ui.text_edit_singleline(&mut curr_cell.data).on_hover_text(format!("{}", &err.unwrap()));
                                    } else{
                                        ui.text_edit_singleline(&mut curr_cell.data);
                                    }

                                    if curr_cell.curr_field_description.is_some() {
                                        ui.reset_style();
                                    }
                                });
                            }
                        });
                    });
            }
        }
    }
}

impl SpreadSheetWindow {
    pub fn check_file(&mut self) {
        if let Some(receiver) = &mut self.is_file_loaded_receiver {
            match receiver.try_recv() {
                Ok(boole) => {
                    if boole {
                        self.is_file_loaded_receiver = None;
                    }
                    self.is_file_loaded = boole;
                }
                Err(e) => println!("Failed receiving load file callback, {}", e),
            }
        }
    }

    pub fn open_file(&mut self, path: String) {
        if !self.is_file_loaded_receiver.is_some() {
            let (sed, rec) = oneshot::channel();
            self.sender
                .try_send(Communication::LoadImportFilePath(path, sed))
                .unwrap_or_else(|err| println!("failed to send loadimportpath, {}", err));
            self.is_file_loaded_receiver = Some(rec);
        }
    }

    pub fn preview_files_being_dropped(ctx: &egui::Context) {
        use egui::*;

        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let text = "Dropping files...".to_owned();
                text
            });

            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }
    }
}
