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
                db_login_window: DBLoginWindow::default(sender.clone()),
                db_transaction_window: None,
                is_db_connection_verified_receiver: None,
                sender,
                is_db_connection_verified: false,
                csv_data_handle,
                db_table_data_handle,
                open_db_transaction_window_receiver,
                should_open_db_transaction_window: None,
            })
        }),
    )
}

struct CSQL {
    spreadsheet_window: SpreadSheetWindow,
    db_login_window: DBLoginWindow,
    db_transaction_window: Option<DBTransactionWindow>,
    sender: Sender<Communication>,
    is_db_connection_verified_receiver: Option<oneshot::Receiver<bool>>,
    is_db_connection_verified: bool,
    csv_data_handle: Arc<Mutex<ImportedData>>,
    db_table_data_handle: Arc<Mutex<Tables>>,
    open_db_transaction_window_receiver: Receiver<usize>,
    should_open_db_transaction_window: Option<usize>,
}

impl App for CSQL {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.db_login_window
                .show(ctx, ui, self.is_db_connection_verified);

            self.spreadsheet_window
                .show(ctx, ui, self.is_db_connection_verified);

            if let Ok(resp) = self.open_db_transaction_window_receiver.try_recv() {
                self.should_open_db_transaction_window = Some(resp);
            }
            if self.should_open_db_transaction_window.is_some() {
                let db_transaction_window = DBTransactionWindow::default(
                    self.sender.clone(),
                    self.should_open_db_transaction_window.unwrap(),
                );
                self.db_transaction_window = Some(db_transaction_window);
                self.db_transaction_window.as_mut().unwrap().show(ctx, ui);
            }

            /* Changes self if db connection is verified */
            if let Some(oneshot_receiver) = &mut self.is_db_connection_verified_receiver {
                if let Ok(boole) = oneshot_receiver.try_recv() {
                    self.is_db_connection_verified = boole;
                    self.is_db_connection_verified_receiver = None;
                }
            } else {
                let (sed, rec) = oneshot::channel();
                self.is_db_connection_verified_receiver = Some(rec);
                match self
                    .sender
                    .try_send(Communication::AreCreditentialsValidated(sed))
                {
                    Ok(_) => (),
                    Err(e) => println!("Failed to send-verify db conn oneshot, {}", e),
                }
            }
        });
    }
}
