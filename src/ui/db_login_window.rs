use crate::backend::backend_manager::Communication;
use crate::backend::database_handler::DBLoginData;
use egui::{Context, Ui, Vec2};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

pub struct DBLoginWindow {
    db_login_data: DBLoginData,
    sender: Sender<Communication>,
    are_creditentials_verified_receiver:
        Option<oneshot::Receiver<Result<(), Box<dyn std::error::Error + Send>>>>,
    are_creditentials_verified_error: Option<Box<dyn std::error::Error + Send>>,
}
impl DBLoginWindow {
    pub fn default(sender: Sender<Communication>) -> DBLoginWindow {
        DBLoginWindow {
            db_login_data: DBLoginData {
                user_name: "root".to_string(),
                database: "quotes".to_string(),
                host: "127.0.0.1".to_string(),
                port: "4307".to_string(),
                password: "mauFJcuf5dhRMQrjj".to_string(),
                should_remember: true,
                ..Default::default()
            },
            sender,
            are_creditentials_verified_receiver: None,
            are_creditentials_verified_error: None,
        }
    }
}
impl DBLoginWindow {
    pub fn show(&mut self, ctx: &Context, ui: &mut Ui, is_db_connection_verified: bool) {
        let alignment: egui::Align2;

        match is_db_connection_verified {
            true => {
                alignment = egui::Align2::CENTER_TOP;
            }
            false => {
                alignment = egui::Align2::CENTER_CENTER;
            }
        }

        egui::Window::new("MySQL Login")
            .id(egui::Id::new("MySQL Login"))
            .resizable(false)
            .collapsible(true)
            .title_bar(true)
            .scroll2([false, false])
            .enabled(true)
            .default_open(!is_db_connection_verified)
            .anchor(alignment, Vec2::ZERO)
            .show(ctx, |ui| {
                self.ui(ctx, ui);
            });
    }

    fn ui(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.heading("Log into the database:");
        ui.group(|ui| {
            ui.horizontal(|ui| {
                let host_label = ui.label("Host:");
                ui.text_edit_singleline(&mut self.db_login_data.host)
                    .labelled_by(host_label.id);
            });

            ui.horizontal(|ui| {
                let port_label = ui.label("Port:");
                ui.text_edit_singleline(&mut self.db_login_data.port)
                    .labelled_by(port_label.id);
            });

            ui.horizontal(|ui| {
                let database_label = ui.label("Database:");
                ui.text_edit_singleline(&mut self.db_login_data.database)
                    .labelled_by(database_label.id);
            });

            ui.horizontal(|ui| {
                let user_name_label = ui.label("User name:");
                ui.text_edit_singleline(&mut self.db_login_data.user_name)
                    .labelled_by(user_name_label.id);
            });

            ui.horizontal(|ui| {
                let password_label = ui.label("Password:");
                ui.text_edit_singleline(&mut self.db_login_data.password)
                    .labelled_by(password_label.id);
            });
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.db_login_data.should_remember, "Remember");

            if ui.button("Connect").clicked() {
                let (sender, receiver) = oneshot::channel();
                self.are_creditentials_verified_receiver = Some(receiver);
                self.sender
                    .try_send(Communication::ValidateCreditentials(
                        self.db_login_data.clone(),
                        sender,
                    ))
                    .unwrap_or_else(|e| {
                        println!("Failed to send authenticator... {}", e);
                    })
            }
        });

        /* If db login bad, return sqlx error to UI */
        if let Some(err) = &mut self.are_creditentials_verified_error {
            ui.style_mut().visuals.override_text_color = Some(egui::Color32::RED);
            ui.label(format!("{}", err));
        }

        if let Some(receiver) = &mut self.are_creditentials_verified_receiver {
            if let Ok(response) = receiver.try_recv() {
                match response {
                    Ok(_) => self.are_creditentials_verified_error = None,
                    Err(err) => self.are_creditentials_verified_error = Some(err),
                }
                self.are_creditentials_verified_receiver = None;
            }
        }
    }
}
