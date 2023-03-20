use super::csv_handler::ImportedData;
use super::database_handler::{DBLoginData, Tables};
use super::parser::parse;
use sqlx::MySqlConnection;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{oneshot, Mutex};

pub struct BackendManger {
    pub db_login_data: DBLoginData,
    pub imported_data: ImportedData,
    pub csv_data: Arc<Mutex<ImportedData>>,
    pub db_connection: Option<MySqlConnection>,
    pub receiver: Receiver<Communication>,
    pub db_table_data: Arc<Mutex<Tables>>,
}

impl BackendManger {
    pub async fn listen(&mut self) {
        if let Some(command) = self.receiver.recv().await {
            match command {
                Communication::ValidateCreditentials(db_login_data, sender) => {
                    self.db_login_data = db_login_data;

                    let result = self.db_login_data.validate_creditentials().await;
                    match result {
                        Ok(connection) => {
                            self.db_connection = Some(connection);
                            sender.send(Ok(())).unwrap_or_else(|_e| {
                                println!("failed to send ValidateCreditentials err")
                            });
                            let mut db_table_data = self.db_table_data.lock().await;
                            db_table_data
                                .query_for_tables(&mut self.db_connection.as_mut().unwrap())
                                .await
                                .unwrap();
                        }
                        Err(e) => {
                            sender.send(Err(e)).unwrap_or_else(|_e| {
                                println!("failed to send ValidateCreditentials err")
                            });
                        }
                    }
                }
                Communication::AreCreditentialsValidated(sender) => sender
                    .send(self.db_login_data.is_verified)
                    .unwrap_or_else(|e| {
                        println!("Server - Failed answering if credit. validated");
                    }),
                Communication::LoadImportFilePath(path, sender) => {
                    self.imported_data.path = path;
                    match self.imported_data.load_csv() {
                        Ok(_) => {
                            let mut data = self.csv_data.lock().await;

                            data.are_headers = self.imported_data.are_headers.clone();
                            data.data = self.imported_data.data.clone();
                            data.path = self.imported_data.path.clone();

                            drop(data);
                            sender.send(true).unwrap_or_else(|err| {
                                println!("Server - failed to send loaded file callback, {}", err)
                            })
                        }
                        Err(e) => {
                            println!("{}", e);
                            sender.send(false).unwrap_or_else(|e| {
                                println!("Server - Failed answering if imported{}", e)
                            })
                        }
                    }
                }
                Communication::GetTableDescription(table_index) => {
                    let mut db_table_data = self.db_table_data.lock().await;
                    db_table_data
                        .tables
                        .get_mut(table_index)
                        .unwrap()
                        .describe_table(&mut self.db_connection.as_mut().unwrap())
                        .await;
                }
                Communication::TryParseCol(col_index) => {
                    let mut csv_data = self.csv_data.lock().await;
                    for cell in csv_data.data.iter_col_mut(col_index) {
                        if cell.curr_field_description.is_some() {
                            if let Some(desc) = &cell.curr_field_description {
                                println!("{} should be of type {}", cell.data, desc.field_type)
                            }
                            parse(cell);
                        }
                    }
                }
            }
        }
    }
}
pub enum Communication {
    ValidateCreditentials(
        DBLoginData,
        oneshot::Sender<Result<(), Box<dyn std::error::Error + Send>>>,
    ),
    AreCreditentialsValidated(oneshot::Sender<bool>),
    LoadImportFilePath(String, oneshot::Sender<bool>),
    GetTableDescription(usize),
    TryParseCol(usize),
}
