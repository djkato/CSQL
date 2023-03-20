mod backend;
mod ui;
use backend::backend_manager::BackendManger;
use backend::csv_handler::ImportedData;
use backend::database_handler::DBLoginData;
use backend::database_handler::Tables;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use ui::window_manager::create_ui;
#[tokio::main]
async fn main() {
    let (sender, receiver) = mpsc::channel(8);

    let csv_data = Arc::new(Mutex::new(ImportedData::default()));
    let db_table_data = Arc::new(Mutex::new(Tables::default()));

    let csv_data_handle = csv_data.clone();
    let db_table_data_handle = db_table_data.clone();
    tokio::spawn(async move {
        let mut backend_manager = BackendManger {
            db_login_data: DBLoginData::default(),
            imported_data: ImportedData::default(),
            db_connection: None,
            csv_data,
            db_table_data,
            receiver,
        };
        loop {
            backend_manager.listen().await;
        }
    });
    create_ui(sender, csv_data_handle, db_table_data_handle).expect("Failed to start window");
}
