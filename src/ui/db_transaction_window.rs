use egui::{Context, Ui};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;

use crate::backend::backend_manager::Communication;
use crate::backend::database_handler::QueryResult;

use super::window_manager::{CSQLWindow, ExitStatus};

pub struct DBTransactionWindow {
    sender: Sender<Communication>,
    log_history: Vec<QueryResult>,
    logs_receiver: Option<Receiver<QueryResult>>,
    is_log_finished_receiver: Option<oneshot::Receiver<bool>>,
    is_log_finished: bool,
    is_finished: bool,
    final_result_receiver: Option<oneshot::Receiver<QueryResult>>,
    working_table_index: usize,
}
impl DBTransactionWindow {
    pub fn default(
        sender: Sender<Communication>,
        working_table_index: usize,
    ) -> DBTransactionWindow {
        DBTransactionWindow {
            sender,
            log_history: vec![],
            is_finished: false,
            logs_receiver: None,
            final_result_receiver: None,
            is_log_finished_receiver: None,
            is_log_finished: false,
            working_table_index,
        }
    }
}
impl CSQLWindow for DBTransactionWindow {
    fn refresh(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        frame: &mut eframe::Frame,
    ) -> Option<Result<ExitStatus, Box<dyn std::error::Error>>> {
        println!("Opening Transaction window...");
        egui::Window::new("Database Transactions")
            .id(egui::Id::new("Database Transactions"))
            .resizable(false)
            .collapsible(true)
            .title_bar(true)
            .movable(false)
            .scroll2([false, true])
            .enabled(true)
            .fixed_size(egui::Vec2::new(
                frame.info().window_info.size.x / 1.3,
                frame.info().window_info.size.y / 1.5,
            ))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                self.log();
                self.ui(ctx, ui, frame);
            });

        if self.is_finished {
            return Some(Ok(ExitStatus::Ok));
        }
        None
    }
}
impl DBTransactionWindow {
    pub fn log(&mut self) {
        if let Some(finish_receiver) = self.final_result_receiver.as_mut() {
            if let Ok(result) = finish_receiver.try_recv() {
                if result.result.is_ok() {
                    self.is_finished = true;
                }
            }
        }
        if self.logs_receiver.is_some() {
            if let Ok(log) = self.logs_receiver.as_mut().unwrap().try_recv() {
                self.log_history.push(log);
            }
        }
        if self.is_log_finished_receiver.is_some() {
            if let Ok(finished) = self.is_log_finished_receiver.as_mut().unwrap().try_recv() {
                self.is_log_finished = finished;
                println!("FINISHED QUERYING!!!");
            }
        }
        if self.is_log_finished_receiver.is_none() && self.logs_receiver.is_none() {
            let (log_sender, log_receiver) = channel(2);
            let (finished_sender, finished_receiver) = oneshot::channel();
            self.is_log_finished_receiver = Some(finished_receiver);
            self.logs_receiver = Some(log_receiver);
            self.sender
                .try_send(Communication::StartInserting(
                    self.working_table_index,
                    log_sender,
                    finished_sender,
                ))
                .unwrap_or_else(|_| println!("Failed to send startInserting"));
        }
    }
    pub fn ui(&mut self, ctx: &Context, ui: &mut Ui, frame: &mut eframe::Frame) {
        ui.add_enabled_ui(self.is_log_finished, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Roll back").clicked() {
                    if self.final_result_receiver.is_none() {
                        let (sender, receiver) = oneshot::channel();
                        self.final_result_receiver = Some(receiver);
                        self.sender
                            .try_send(Communication::TryRollBack(sender))
                            .unwrap_or_else(|_| println!("failed sending TryCommit receiver"));
                    }
                }
                if ui.button("Commit").clicked() {
                    if self.final_result_receiver.is_none() {
                        let (sender, receiver) = oneshot::channel();
                        self.final_result_receiver = Some(receiver);
                        self.sender
                            .try_send(Communication::TryCommit(sender))
                            .unwrap_or_else(|_| println!("failed sending TryCommit receiver"));
                    }
                }
            });
        });
        /* DB Output Stuff */
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let table = egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
                    .column(
                        egui_extras::Column::remainder()
                            .clip(true)
                            .at_most(frame.info().window_info.size.x / 1.3 / 2.0),
                    )
                    .stick_to_bottom(true)
                    .column(
                        egui_extras::Column::remainder()
                            .clip(true)
                            .at_most(frame.info().window_info.size.x / 1.3 / 2.0),
                    )
                    .stick_to_bottom(true);

                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Query");
                        });
                        header.col(|ui| {
                            ui.strong("Result");
                        });
                    })
                    .body(|body| {
                        body.rows(15.0, self.log_history.len(), |row_index, mut row| {
                            let log = self.log_history.get(row_index).unwrap();
                            row.col(|ui| {
                                ui.label(&log.query);
                            });
                            row.col(|ui| {
                                match &log.result {
                                    Ok(rows) => {
                                        ui.style_mut().visuals.override_text_color =
                                            Some(egui::Color32::LIGHT_GREEN);
                                        ui.label(format!(
                                            "Success! Rows affected: {}",
                                            rows.rows_affected()
                                        ));
                                    }
                                    Err(e) => {
                                        ui.style_mut().visuals.override_text_color =
                                            Some(egui::Color32::LIGHT_RED);
                                        ui.label(format!("Error! {}", e));
                                    }
                                }
                                ui.reset_style();
                            });
                        });
                    });
                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
            });
    }
}

pub struct OptionsDBTransactionWindow {
    substitute_zero_dates_for_null: bool,
    remove_id_field_from_insert: bool,
}
