use super::window_manager::{CSQLWindow, ExitStatus};
use crate::backend::backend_manager::Communication;
use crate::backend::csv_handler::ImportedData;
use crate::backend::database_handler::{TableField, Tables};
use egui::{ComboBox, Context, Ui};
use egui_extras::{Column, TableBuilder};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, MutexGuard};
pub struct SpreadSheetWindow {
    sender: Sender<Communication>,
    csv_data_handle: Arc<Mutex<ImportedData>>,
    db_table_data_handle: Arc<Mutex<Tables>>,
    current_table: Option<usize>,
    return_status: Option<ExitStatus>,
    debug_autoload_file_sent: bool,
    is_table_description_request_sent: bool,
}
impl SpreadSheetWindow {
    pub fn default(
        sender: Sender<Communication>,
        commit_to_db_sender: Sender<usize>,
        csv_data_handle: Arc<Mutex<ImportedData>>,
        db_table_data_handle: Arc<Mutex<Tables>>,
    ) -> SpreadSheetWindow {
        SpreadSheetWindow {
            sender,
            csv_data_handle,
            db_table_data_handle,
            current_table: None,
            debug_autoload_file_sent: false,
            is_table_description_request_sent: false,
            return_status: None,
        }
    }
}
impl CSQLWindow for SpreadSheetWindow {
    fn refresh(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        frame: &mut eframe::Frame,
    ) -> Option<Result<ExitStatus, Box<dyn std::error::Error>>> {
        self.ui(ui, ctx);
        if let Some(return_status) = self.return_status {
            self.return_status = None;
            return Some(Ok(return_status));
        }
        None
    }
}
impl SpreadSheetWindow {
    fn ui(&mut self, ui: &mut Ui, ctx: &Context) {
        /* if csv was auto mapped and parsed */
        let is_csv_parsed: bool;
        if let Ok(csv_data) = self.csv_data_handle.try_lock() {
            is_csv_parsed = csv_data.is_parsed.clone();
        } else {
            is_csv_parsed = false;
        }
        /* Program Menu */
        ui.horizontal(|ui| {
            if ui.button("Log into DB").clicked() {
                self.return_status = Some(ExitStatus::StartLoginWindow);
            }
            ui.add_space(ui.available_width());
        });
        ui.separator();
        /* CSV & DB menu */
        ui.horizontal(|ui| {
            ui.columns(2, |uis| {
                uis[0].horizontal(|ui| {
                    if ui.button("Import file").clicked() {
                        self.open_file();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                    }
                    if ui.button("Save as...").clicked() {
                        self.save_file();
                    }

                    ui.add_space(ui.available_width());
                    ui.separator();
                });
                uis[1].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(is_csv_parsed, |ui| {
                        if ui.button("Append to DB").clicked() {
                            todo!();
                        }
                        if ui.button("Save to DB").clicked() {
                            self.return_status = Some(ExitStatus::StartTransactionWindow);
                        }
                    });
                    if ui.button("Import DB").clicked() {
                        todo!();
                    }
                    self.table_options(ui);
                    ui.add_space(ui.available_width());
                });
            })
        });

        /* Handle file drops */
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.open_dropped_file(i);
            }
        });
        SpreadSheetWindow::preview_files_being_dropped(ctx);

        if self.current_table.is_some() {
            ui.group(|ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    self.table_builder(ui)
                        .unwrap_or_else(|e| println!("Table un-intd, {:?}", e));
                });
            });
        }
    }
    fn table_options(&mut self, ui: &mut Ui) {
        /* Create table select option, only enable if the tables are discovered yet*/
        if let Ok(db_table_data) = &mut self.db_table_data_handle.try_lock() {
            ui.add_enabled_ui(db_table_data.tables.get(0).is_some(), |ui| {
                let mut select_table = ComboBox::from_label("Select Table").width(100.0);
                if let Some(table_index) = self.current_table {
                    if let Some(current_table) = &db_table_data.tables.get(table_index) {
                        select_table = select_table.selected_text(current_table.name.clone());
                    }
                }
                /* autoselect oc_product in debug mode */
                if cfg!(debug_assertions) && self.current_table.is_none() {
                    self.current_table = Some(88);
                    if let Some(table_88) = &db_table_data.tables.get(88) {
                        select_table = select_table.selected_text(table_88.name.clone());
                    }
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
            });
        }
    }
    fn table_builder(&mut self, ui: &mut Ui) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(csv_data) = &mut self.csv_data_handle.try_lock() {
            if let Ok(db_table_data) = &mut self.db_table_data_handle.try_lock() {
                //ref to all fields in curr db table
                let mut curr_db_table_fields: &mut Vec<TableField> = match db_table_data
                    .tables
                    .get_mut(self.current_table.unwrap())
                    .ok_or("no current table")?
                    .fields
                    .as_mut()
                    .ok_or("no fields yet")
                {
                    Ok(val) => val,
                    Err(e) => {
                        if !self.is_table_description_request_sent {
                            self.sender.try_send(Communication::GetTableDescription(
                                self.current_table.unwrap(),
                            ))?;
                            self.is_table_description_request_sent = true;
                        }
                        return Err(Box::from(
                            <String as Into<Box<dyn std::error::Error>>>::into(e.to_string()),
                        ));
                    }
                };
                if csv_data.data.is_empty() {
                    return Err(<String as Into<Box<dyn std::error::Error>>>::into(
                        "no rows yet".to_string(),
                    ));
                }
                let table = TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    /*First collumn for row actions */
                    .column(Column::auto().at_most(30.0).resizable(false).clip(false));

                table
                .columns(
                        Column::auto().resizable(true).clip(false),
                        csv_data.data.cols(),
                    )
                    .min_scrolled_height(0.0)
                    .header(20., |mut header| {
                        /* First Col for row mutators */
                        header.col(|ui|{
                        });
                        SpreadSheetWindow::add_header_cols(&mut header, csv_data,&mut curr_db_table_fields, &mut self.sender);
                    })
                    .body(|body| {
                        body.rows(15., csv_data.data.rows() - csv_data.are_headers as usize, |row_index, mut row| {
                            let headers_i_offset = csv_data.are_headers as usize;
                            row.col(|ui|{
                                if ui.button("x").clicked(){}
                            });
                            for curr_cell in
                                csv_data.data.iter_row_mut(row_index + headers_i_offset)
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
        } else {
            ui.centered_and_justified(|ui| ui.heading("Drag and drop or Open a file..."));
        }
        Ok(())
    }
}

impl SpreadSheetWindow {
    pub fn add_header_cols(
        header: &mut egui_extras::TableRow,
        csv_data: &mut MutexGuard<ImportedData>,
        curr_db_table_fields: &mut Vec<TableField>,
        sender: &mut Sender<Communication>,
    ) {
        for i in 0..csv_data.data.cols() {
            header.col(|ui| {
                ui.vertical_centered_justified(|ui| {
                    if ui.button("x").clicked() {}
                    if csv_data.parsed_cols.contains(&i) {
                        /* If the whole col is parsed, change header bg to green, else normal */
                        ui.style_mut().visuals.override_text_color = Some(egui::Color32::GREEN);
                    } else {
                        ui.style_mut().visuals.override_text_color = Some(egui::Color32::RED);
                    }
                    let mut combo_box: ComboBox;
                    if csv_data.are_headers {
                        combo_box = ComboBox::new(i, csv_data.data.get(0, i).unwrap().data.clone());
                    } else {
                        combo_box = ComboBox::new(i, "");
                    }

                    //if any field is assinged to this combobox, show it's text, else "----"
                    if let Some(selected_field) = curr_db_table_fields
                        .iter()
                        .find(|field| field.mapped_to_col == Some(i))
                    {
                        combo_box =
                            combo_box.selected_text(selected_field.description.field.clone());
                    } else {
                        combo_box = combo_box.selected_text("-----");
                    }

                    /* When a Field gets attached to Col,  */
                    combo_box.show_ui(ui, |ui| {
                        for field in curr_db_table_fields.iter_mut() {
                            if ui
                                .selectable_value(
                                    &mut field.mapped_to_col,
                                    Some(i),
                                    field.description.field.clone(),
                                )
                                .clicked()
                            {
                                match sender.try_send(Communication::TryParseCol(i)) {
                                    Ok(_) => {
                                        for cel in csv_data.data.iter_col_mut(i) {
                                            cel.curr_field_description =
                                                Some(field.description.clone());
                                        }
                                    }
                                    Err(e) => println!("failed sending parsecol request, {}", e),
                                }
                            }
                        }
                    });
                    ui.reset_style();
                });
            });
        }
    }

    pub fn save_file(&mut self) {
        let mut save_name = "to-csv".to_owned();
        if let Some(table_i) = self.current_table {
            if let Ok(db_table_data) = self.db_table_data_handle.try_lock() {
                save_name = db_table_data.tables.get(table_i).unwrap().name.clone();
            }
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("db-{}-converted.csv", save_name).as_str())
            .save_file()
        {
            self.sender
                .try_send(Communication::SaveCSV(path.display().to_string()))
                .unwrap_or_else(|err| println!("failed to send loadimportpath, {}", err));
        };
    }

    pub fn open_file(&mut self) {
        let final_path;
        /*preloads file in debug */
        if cfg!(debug_assertions) && !self.debug_autoload_file_sent {
            final_path = std::path::PathBuf::from("/home/djkato/Dokumenty/csql/oc_product.csv");
            println!(" * Preloading \"{:?}\"...", final_path);
            self.debug_autoload_file_sent = true;
        } else {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Spreadsheets", &["csv"])
                .pick_file()
            {
                final_path = path;
            } else {
                return;
            }
            self.sender
                .try_send(Communication::LoadImportFilePath(
                    final_path.display().to_string(),
                ))
                .unwrap_or_else(|err| println!("failed to send loadimportpath, {}", err));
        }
    }

    pub fn open_dropped_file(&mut self, path: &egui::InputState) {
        let path = path
            .raw
            .dropped_files
            .clone()
            .get(0)
            .unwrap()
            .path
            .as_ref()
            .unwrap()
            .display()
            .to_string();

        self.sender
            .try_send(Communication::LoadImportFilePath(path))
            .unwrap_or_else(|err| println!("failed to send loadimportpathdropped, {}", err));
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
