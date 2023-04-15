use crate::backend::backend_manager::Communication;
use crate::backend::csv_handler::ImportedData;
use crate::backend::database_handler::Tables;
use crate::ui::db_login_window::DBLoginWindow;
use crate::ui::table_window::SpreadSheetWindow;
use eframe::{run_native, App, NativeOptions};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use super::db_transaction_window::DBTransactionWindow;
use super::{db_login_window, db_transaction_window};

pub fn create_ui(
    sender: Sender<Communication>,
    csv_data_handle: Arc<Mutex<ImportedData>>,
    db_table_data_handle: Arc<Mutex<Tables>>,
) -> Result<(), eframe::Error> {
    let win_option = NativeOptions {
        drag_and_drop_support: true,
        ..Default::default()
    };
    let (open_db_transaction_window_sender, open_db_transaction_window_receiver) = channel(2);
    run_native(
        "CSQL",
        win_option,
        Box::new(|cc| {
            Box::new(CSQL {
                spreadsheet_window: SpreadSheetWindow::default(
                    sender.clone(),
                    open_db_transaction_window_sender,
                    csv_data_handle.clone(),
                    db_table_data_handle.clone(),
                ),
                db_login_window: None,
                db_transaction_window: None,
                sender,
                csv_data_handle,
                db_table_data_handle,
                should_open_transaction_window: false,
                should_open_login_window: true,
            })
        }),
    )
}

struct CSQL {
    spreadsheet_window: SpreadSheetWindow,
    db_login_window: Option<DBLoginWindow>,
    db_transaction_window: Option<DBTransactionWindow>,
    sender: Sender<Communication>,
    csv_data_handle: Arc<Mutex<ImportedData>>,
    db_table_data_handle: Arc<Mutex<Tables>>,
    should_open_transaction_window: bool,
    should_open_login_window: bool,
}

impl App for CSQL {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(result) = self.spreadsheet_window.refresh(ctx, ui, frame) {
                match result {
                    ExitStatus::StartLoginWindow => self.should_open_transaction_window = true,
                    ExitStatus::StartTransactionWindow => self.should_open_login_window = true,
                    _ => (),
                }
            };

            if self.should_open_login_window {
                if let Some(db_login_window) = self.db_login_window.as_mut() {
                    if let Some(result) = db_login_window.refresh(ctx, ui, frame) {
                        match result {
                            ExitStatus::Ok => {
                                self.db_login_window = None;
                                self.should_open_login_window = false;
                            }
                            _ => (),
                        }
                    }
                }
            }
            if self.should_open_transaction_window {
                if let Some(db_transaction_window) = self.db_transaction_window.as_mut() {
                    if let Some(result) = db_transaction_window.refresh(ctx, ui, frame) {
                        match result {
                            ExitStatus::Ok => {
                                self.db_transaction_window = None;
                                self.should_open_transaction_window = false;
                            }
                            _ => (),
                        }
                    }
                }
            }
        });
    }
}

pub trait CSQLWindow {
    fn refresh(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        frame: &mut eframe::Frame,
    ) -> Option<ExitStatus>;
}
pub enum ExitStatus {
    StartTransactionWindow,
    StartLoginWindow,
    Ok,
    Err(Box<dyn std::error::Error>),
}
